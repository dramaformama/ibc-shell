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

/// Longest server banner kept (chars).
const MAX_BANNER: usize = 4096;

/// Server banners are plain text: drop control characters (escape injection) except line
/// breaks / tabs, and cap the length.
pub(crate) fn clean_banner(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .take(MAX_BANNER)
        .collect()
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
    /// Why the transport ended (see [`close_code`]); read through `Connection::close_reason`.
    pub closed: CloseSlot,
}

/// Shared slot for the reason a connection ended.
pub(crate) type CloseSlot = std::sync::Arc<std::sync::Mutex<Option<String>>>;

/// Short, fixed-vocabulary code for the end of a connection (never the server's own text):
/// `server:<DisconnectCode>`, `keepalive`, `inactivity`, `network:<IoKind>`, `protocol`.
pub(crate) fn close_code(reason: &russh::client::DisconnectReason<russh::Error>) -> String {
    use russh::client::DisconnectReason as R;
    match reason {
        R::ReceivedDisconnect(info) => format!("server:{:?}", info.reason_code),
        R::Error(russh::Error::KeepaliveTimeout) => "keepalive".into(),
        R::Error(russh::Error::InactivityTimeout) => "inactivity".into(),
        R::Error(russh::Error::IO(io)) => format!("network:{:?}", io.kind()),
        R::Error(_) => "protocol".into(),
    }
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
            *b = Some(clean_banner(banner));
        }
        Ok(())
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn disconnected(
        &mut self,
        reason: russh::client::DisconnectReason<Self::Error>,
    ) -> Result<(), Self::Error> {
        if let Ok(mut c) = self.closed.lock() {
            c.get_or_insert_with(|| close_code(&reason));
        }
        match reason {
            russh::client::DisconnectReason::ReceivedDisconnect(_) => Ok(()),
            russh::client::DisconnectReason::Error(e) => Err(e),
        }
    }

    async fn check_server_key(
        &mut self,
        key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let (reply, rx) = oneshot::channel();
        // Certificates are never offered (host_key_certificates is empty) so only plain keys
        // reach the verifier; clone via re-wrap.
        let key = match key {
            PublicKeyOrCertificate::PublicKey { key, hash_alg } => {
                PublicKeyOrCertificate::PublicKey {
                    key: key.clone(),
                    hash_alg: *hash_alg,
                }
            }
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

    fn changed(&self, key_type: String, old: KnownKey, actual: String) -> SshError {
        SshError::HostKeyChanged(Box::new(HostKeyMismatch {
            host: self.host.to_owned(),
            port: self.port,
            key_type,
            expected_fingerprint: old.fingerprint,
            actual_fingerprint: actual,
            known_host_id: old.id,
        }))
    }

    /// Looks the host up and compares; sets `self.error` on a backend failure.
    fn verdict(&mut self, pattern: &str, key_type: &str, blob: &[u8]) -> Option<Verdict> {
        match self.known.lookup(pattern) {
            Ok(stored) => Some(evaluate(&stored, key_type, blob)),
            Err(e) => {
                self.error = Some(SshError::KnownHosts(e));
                None
            }
        }
    }

    async fn decide(&mut self, key: &PublicKey) -> bool {
        let pattern = host_pattern(self.host, self.port);
        let key_type = key.algorithm().as_str().to_owned();
        let Ok(blob) = key.to_bytes() else {
            self.error = Some(SshError::Protocol("unencodable host key".into()));
            return false;
        };
        let fp = fingerprint(key);
        match self.verdict(&pattern, &key_type, &blob) {
            None => false,
            Some(Verdict::Known) => true,
            Some(Verdict::Changed(old)) => {
                self.error = Some(self.changed(key_type, old, fp));
                false
            }
            Some(Verdict::Unknown) => {
                let info = HostKeyInfo {
                    host: self.host.to_owned(),
                    port: self.port,
                    key_type: key_type.clone(),
                    randomart: key.fingerprint(HashAlg::Sha256).to_randomart(&key_type),
                    fingerprint: fp.clone(),
                };
                let decision = self.prompter.confirm_unknown_host(info).await;
                if decision == HostKeyDecision::Reject {
                    self.error = Some(SshError::Cancelled);
                    return false;
                }
                self.after_accept(decision, &pattern, key_type, &blob, fp)
            }
        }
    }

    /// The prompt may have waited minutes: another connection could have stored a different
    /// key for this host meanwhile. Re-check, and never let an "unknown" answer override it.
    fn after_accept(
        &mut self,
        decision: HostKeyDecision,
        pattern: &str,
        key_type: String,
        blob: &[u8],
        fp: String,
    ) -> bool {
        match self.verdict(pattern, &key_type, blob) {
            None => false,
            Some(Verdict::Known) => true,
            Some(Verdict::Changed(old)) => {
                self.error = Some(self.changed(key_type, old, fp));
                false
            }
            Some(Verdict::Unknown) if decision == HostKeyDecision::AcceptOnce => true,
            Some(Verdict::Unknown) => {
                let entry = KnownKey {
                    id: String::new(),
                    key_type: key_type.clone(),
                    key_blob: blob.to_vec(),
                    fingerprint: fp.clone(),
                };
                let Err(e) = self.known.add(pattern, &entry) else {
                    return true;
                };
                // Lost a race with another writer: report a change if that is what it was.
                self.error = Some(match self.verdict(pattern, &key_type, blob) {
                    Some(Verdict::Changed(old)) => self.changed(key_type, old, fp),
                    _ => SshError::KnownHosts(e),
                });
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn close_codes_are_fixed_vocabulary() {
        use russh::client::{DisconnectReason as R, RemoteDisconnectInfo};
        let io = |k| R::Error(russh::Error::IO(std::io::Error::new(k, "secret detail")));
        assert_eq!(
            close_code(&R::Error(russh::Error::KeepaliveTimeout)),
            "keepalive"
        );
        assert_eq!(
            close_code(&io(std::io::ErrorKind::ConnectionReset)),
            "network:ConnectionReset"
        );
        assert_eq!(close_code(&R::Error(russh::Error::Kex)), "protocol");
        let info = RemoteDisconnectInfo {
            reason_code: russh::Disconnect::TooManyConnections,
            message: "evil\x1b[31m text".into(),
            lang_tag: String::new(),
        };
        assert_eq!(
            close_code(&R::ReceivedDisconnect(info)),
            "server:TooManyConnections"
        );
    }

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

    #[test]
    fn banner_is_plain_text_and_capped() {
        assert_eq!(
            clean_banner("Hi\x1b]52;c;ZXZpbA==\x07\r\nthere\t!"),
            "Hi]52;c;ZXZpbA==\nthere\t!"
        );
        assert_eq!(clean_banner(&"a".repeat(10_000)).len(), MAX_BANNER);
    }

    const KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACAjWF9XMGonNvOZS5jQtwN2+OyShDB1KB0m2qfUAEySdgAAAIhasfTVWrH0
1QAAAAtzc2gtZWQyNTUxOQAAACAjWF9XMGonNvOZS5jQtwN2+OyShDB1KB0m2qfUAEySdg
AAAEBpExXls+OkMCOV+0lvM7tv4lCconrTA8VEu/lb4/uEkyNYX1cwaic285lLmNC3A3b4
7JKEMHUoHSbap9QATJJ2AAAABHRlc3QB
-----END OPENSSH PRIVATE KEY-----
";

    /// Store whose content a rival connection changes while the prompt is open.
    #[derive(Default)]
    struct Racy(std::sync::Mutex<Vec<KnownKey>>);

    impl KnownHosts for Racy {
        fn lookup(&self, _: &str) -> Result<Vec<KnownKey>, String> {
            Ok(self.0.lock().map_err(|_| "poisoned")?.clone())
        }
        fn add(&self, _: &str, key: &KnownKey) -> Result<(), String> {
            let mut rows = self.0.lock().map_err(|_| "poisoned")?;
            if rows.iter().any(|r| r.key_blob != key.key_blob) {
                return Err("conflict".into());
            }
            rows.push(key.clone());
            Ok(())
        }
    }

    /// Accepts, but first lets a rival store a different key for the same host.
    struct Sneaky<'a>(&'a Racy, HostKeyDecision);

    #[allow(clippy::unused_async_trait_impl)]
    impl Prompter for Sneaky<'_> {
        fn progress(&self, _: &str, _: crate::config::ConnectStep) {}
        async fn confirm_unknown_host(&self, _: HostKeyInfo) -> HostKeyDecision {
            if let Ok(mut rows) = self.0 .0.lock() {
                rows.push(k("rival", "ssh-ed25519", b"rival"));
            }
            self.1
        }
        async fn keyboard_interactive(
            &self,
            _: crate::config::KbdInteractiveRequest,
        ) -> Option<Vec<secrecy::SecretString>> {
            None
        }
        async fn passphrase(&self, _: &str) -> Option<secrecy::SecretString> {
            None
        }
    }

    #[tokio::test]
    async fn key_stored_during_prompt_blocks_every_accept() {
        let key = russh::keys::decode_secret_key(KEY, None)
            .unwrap()
            .public_key()
            .clone();
        for decision in [
            HostKeyDecision::AcceptOnce,
            HostKeyDecision::AcceptAndRemember,
        ] {
            let store = Racy::default();
            let prompter = Sneaky(&store, decision);
            let mut ctx = VerifyCtx {
                host: "h",
                port: 22,
                known: &store,
                prompter: &prompter,
                error: None,
            };
            assert!(!ctx.decide(&key).await, "{decision:?}");
            assert!(
                matches!(&ctx.error, Some(SshError::HostKeyChanged(m)) if m.known_host_id == "rival"),
                "{decision:?}: {:?}",
                ctx.error
            );
            assert_eq!(store.0.lock().unwrap().len(), 1, "rival key stays alone");
        }
    }
}
