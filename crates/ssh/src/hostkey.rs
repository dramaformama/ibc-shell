//! Host key verification types. Policy (security.md): unknown → ask, changed → hard block
//! (there is intentionally no API to accept a changed key).

use serde::{Deserialize, Serialize};

/// Shown in the TOFU dialog (`HostKeyUnknown`, `SessionEvent::HostKeyPrompt`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct HostKeyInfo {
    /// Hostname as connected (no brackets).
    pub host: String,
    /// Port.
    pub port: u16,
    /// e.g. `ssh-ed25519`.
    pub key_type: String,
    /// `SHA256:<base64, no padding>`.
    pub fingerprint: String,
    /// OpenSSH-style randomart (plain text, render in monospace).
    pub randomart: String,
}

/// Shown in the red block (`HostKeyChanged`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct HostKeyMismatch {
    /// Hostname.
    pub host: String,
    /// Port.
    pub port: u16,
    /// Key type presented by the server.
    pub key_type: String,
    /// Fingerprint stored in known hosts.
    pub expected_fingerprint: String,
    /// Fingerprint presented now.
    pub actual_fingerprint: String,
    /// Store id of the stored entry (UUID string) so the UI can link to Known hosts.
    pub known_host_id: String,
}

/// A stored key, as provided by the [`KnownHosts`] implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownKey {
    /// Store id (UUID string).
    pub id: String,
    /// Key type.
    pub key_type: String,
    /// SSH wire-format public key.
    pub key_blob: Vec<u8>,
    /// Fingerprint.
    pub fingerprint: String,
}

/// User decision for an unknown host key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostKeyDecision {
    /// Abort the connection (`SshError::Cancelled`).
    Reject,
    /// Continue without storing.
    AcceptOnce,
    /// Continue and call [`KnownHosts::add`].
    AcceptAndRemember,
}

/// Known-hosts storage, implemented by the bridge over `crates/store`.
pub trait KnownHosts: Send + Sync {
    /// Entries for `host_pattern` (`[host]:port`, bare host for 22), any key type.
    fn lookup(&self, host_pattern: &str) -> Result<Vec<KnownKey>, String>;
    /// Stores a newly accepted key.
    fn add(&self, host_pattern: &str, key: &KnownKey) -> Result<(), String>;
}

/// OpenSSH host pattern for `host:port` (`[host]:port`, or bare host when port is 22).
#[must_use]
pub fn host_pattern(host: &str, port: u16) -> String {
    if port == 22 {
        host.to_owned()
    } else {
        format!("[{host}]:{port}")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn pattern() {
        assert_eq!(super::host_pattern("a.b", 22), "a.b");
        assert_eq!(super::host_pattern("::1", 2222), "[::1]:2222");
    }
}
