//! Host key verification: pure decision logic + the per-hop context that talks to
//! [`KnownHosts`] and [`Prompter`] while the handshake runs.

use russh::client::Handler;
use russh::keys::{HashAlg, PublicKey, PublicKeyOrCertificate};
use tokio::sync::{mpsc, oneshot};

use crate::config::Prompter;
use crate::hostkey::{
    host_pattern, HostKeyDecision, HostKeyInfo, HostKeyMismatch, KnownHosts, KnownKey,
};
use crate::SshError;

/// Outcome of comparing a presented key with the stored ones.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Presented key equals a stored one.
    Known,
    /// Nothing stored for this host.
    Unknown,
    /// Something is stored but the presented key matches none of it (hard block).
    /// Holds the stored entry to report (same key type preferred).
    Changed(KnownKey),
}

/// Strict rule: if the host has any stored key and none matches, it is a change, even when the
/// key *type* differs (otherwise a MITM could dodge the check by offering another key type).
pub(crate) fn evaluate(stored: &[KnownKey], key_type: &str, blob: &[u8]) -> Verdict {
    if stored.iter().any(|k| k.key_blob == blob) {
        return Verdict::Known;
    }
    match stored
        .iter()
        .find(|k| k.key_type == key_type)
        .or_else(|| stored.first())
    {
        Some(k) => Verdict::Changed(k.clone()),
        None => Verdict::Unknown,
    }
}

/// Fingerprint (`SHA256:...`) of a key.
pub(crate) fn fingerprint(key: &PublicKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

/// Request from the russh handler (running in the session task) to the connect task.
pub(crate) struct VerifyReq {
    pub key: PublicKeyOrCertificate,
    pub reply: oneshot::Sender<bool>,
}

/// russh handler: forwards the server key to the connecting task, which owns the borrowed
/// `KnownHosts` / `Prompter`. Also records the auth banner.
pub(crate) struct ClientHandler {
    pub tx: mpsc::UnboundedSender<VerifyReq>,
    pub banner: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl Handler for ClientHandler {
    type Error = russh::Error;

    #[allow(clippy::unused_async_trait_impl)]
    async fn auth_banner(
        &mut self,
        banner: &str,
        _session: &mut russh::client::Session,
    ) -> Result<(), Self::Error> {
        if let Ok(mut b) = self.banner.lock() {
            *b = Some(banner.to_owned());
        }
        Ok(())
    }

    async fn check_server_key(
        &mut self,
        key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let (reply, rx) = oneshot::channel();
        // Certificates are never offered (host_key_certificates is empty) so only plain keys
        // reach the verifier; clone via re-wrap.
        let key = match key {
            PublicKeyOrCertificate::PublicKey { key, hash_alg } => PublicKeyOrCertificate::PublicKey {
                key: key.clone(),
                hash_alg: *hash_alg,
            },
            PublicKeyOrCertificate::Certificate(_) => return Ok(false),
        };
        if self.tx.send(VerifyReq { key, reply }).is_err() {
            return Ok(false);
        }
        Ok(rx.await.unwrap_or(false))
    }
}

/// Per-hop verification context living on the connecting task.
pub(crate) struct VerifyCtx<'a, K, P> {
    pub host: &'a str,
    pub port: u16,
    pub known: &'a K,
    pub prompter: &'a P,
    /// Why a key was refused (reported instead of russh's generic error).
    pub error: Option<SshError>,
}

impl<K: KnownHosts, P: Prompter> VerifyCtx<'_, K, P> {
    /// Decides on one request and answers the handler.
    pub async fn handle(&mut self, req: VerifyReq) {
        let ok = match &req.key {
            PublicKeyOrCertificate::PublicKey { key, .. } => self.decide(key).await,
            PublicKeyOrCertificate::Certificate(_) => false,
        };
        let _ = req.reply.send(ok);
    }

    async fn decide(&mut self, key: &PublicKey) -> bool {
        let pattern = host_pattern(self.host, self.port);
        let key_type = key.algorithm().as_str().to_owned();
        let Ok(blob) = key.to_bytes() else {
            self.error = Some(SshError::Protocol("unencodable host key".into()));
            return false;
        };
        let stored = match self.known.lookup(&pattern) {
            Ok(s) => s,
            Err(e) => {
                self.error = Some(SshError::KnownHosts(e));
                return false;
            }
        };
        let fp = fingerprint(key);
        match evaluate(&stored, &key_type, &blob) {
            Verdict::Known => true,
            Verdict::Changed(old) => {
                self.error = Some(SshError::HostKeyChanged(Box::new(HostKeyMismatch {
                    host: self.host.to_owned(),
                    port: self.port,
                    key_type,
                    expected_fingerprint: old.fingerprint,
                    actual_fingerprint: fp,
                    known_host_id: old.id,
                })));
                false
            }
            Verdict::Unknown => {
                let info = HostKeyInfo {
                    host: self.host.to_owned(),
                    port: self.port,
                    key_type: key_type.clone(),
                    randomart: key.fingerprint(HashAlg::Sha256).to_randomart(&key_type),
                    fingerprint: fp.clone(),
                };
                match self.prompter.confirm_unknown_host(info).await {
                    HostKeyDecision::Reject => {
                        self.error = Some(SshError::Cancelled);
                        false
                    }
                    HostKeyDecision::AcceptOnce => true,
                    HostKeyDecision::AcceptAndRemember => {
                        let entry = KnownKey {
                            id: String::new(),
                            key_type,
                            key_blob: blob,
                            fingerprint: fp,
                        };
                        if let Err(e) = self.known.add(&pattern, &entry) {
                            self.error = Some(SshError::KnownHosts(e));
                            return false;
                        }
                        true
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(id: &str, ty: &str, blob: &[u8]) -> KnownKey {
        KnownKey {
            id: id.into(),
            key_type: ty.into(),
            key_blob: blob.to_vec(),
            fingerprint: format!("SHA256:{id}"),
        }
    }

    #[test]
    fn unknown_when_nothing_stored() {
        assert_eq!(evaluate(&[], "ssh-ed25519", b"a"), Verdict::Unknown);
    }

    #[test]
    fn known_on_exact_match_any_entry() {
        let s = [k("1", "ssh-rsa", b"r"), k("2", "ssh-ed25519", b"e")];
        assert_eq!(evaluate(&s, "ssh-ed25519", b"e"), Verdict::Known);
    }

    #[test]
    fn changed_same_type_reports_that_entry() {
        let s = [k("1", "ssh-rsa", b"r"), k("2", "ssh-ed25519", b"e")];
        assert_eq!(
            evaluate(&s, "ssh-ed25519", b"other"),
            Verdict::Changed(k("2", "ssh-ed25519", b"e"))
        );
    }

    #[test]
    fn different_key_type_is_still_a_change() {
        let s = [k("1", "ssh-ed25519", b"e")];
        assert_eq!(
            evaluate(&s, "ecdsa-sha2-nistp256", b"x"),
            Verdict::Changed(k("1", "ssh-ed25519", b"e"))
        );
    }
}
