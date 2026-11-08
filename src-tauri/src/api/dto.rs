//! Bridge-owned IPC DTOs (types not owned by a core crate). Secret-bearing inputs use
//! `SecretString` (typed as `string` in TS) and redact `Debug`.

pub use crate::tray::{TrayItem, TrayMenu};
use ibc_shell_sftp::{ConflictPolicy, TransferId};
pub use ibc_shell_ssh::{
    CertInfo, CertReport, Container, CoreSplit, DockerAccess, DockerAction, DockerReport,
    HostStats, Listener, NetIface, PeerFlow, ProcRow,
};
use ibc_shell_ssh::{ConnectStep, ForwardState, HostKeyInfo, KbdPrompt};
use ibc_shell_store::{ForwardId, HostId, IdentityId, IdentityKind};
use ibc_shell_vault::{VaultMode, VaultState};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::ApiError;

macro_rules! bridge_id {
    ($($(#[$doc:meta])* $name:ident),+ $(,)?) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);
    )+};
}

bridge_id!(
    /// Open SSH session (one per terminal tab).
    SessionId,
    /// Open SFTP session.
    SftpId,
    /// Pending interactive prompt (host key, keyboard-interactive, passphrase).
    PendingId,
);

/// A user-typed secret (password / passphrase / OTP) arriving from JS. Deserialized straight
/// into `SecretString`; TS type `string`; `Debug` redacted; never serialized back.
#[derive(Deserialize)]
#[serde(transparent)]
pub struct SecretInput(pub SecretString);

impl std::fmt::Debug for SecretInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretInput([REDACTED])")
    }
}

impl specta::Type for SecretInput {
    fn definition(types: &mut specta::Types) -> specta::datatype::DataType {
        <String as specta::Type>::definition(types)
    }
}

/// Identity as seen by the UI: no secrets, no secret ids.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
pub struct IdentityDto {
    /// Id.
    pub id: IdentityId,
    /// Label.
    pub label: String,
    /// Username.
    pub username: Option<String>,
    /// Kind.
    pub kind: IdentityKind,
    /// e.g. `ssh-ed25519`.
    pub key_algo: Option<String>,
    /// `SHA256:…`.
    pub key_fingerprint: Option<String>,
    /// OpenSSH public key line.
    pub public_key: Option<String>,
    /// A password is stored.
    pub has_password: bool,
    /// A passphrase is stored.
    pub has_passphrase: bool,
    /// Unix millis.
    pub created_at: i64,
    /// Unix millis.
    pub updated_at: i64,
}

/// `identity_create` input. Secrets accepted once, moved into the vault, never returned.
#[derive(Deserialize, specta::Type)]
pub struct IdentityInput {
    /// Label.
    pub label: String,
    /// Username.
    #[serde(default)]
    pub username: Option<String>,
    /// Kind.
    pub kind: IdentityKind,
    /// Password.
    #[serde(default)]
    #[specta(type = Option<String>)]
    pub password: Option<SecretString>,
    /// OpenSSH / PEM private key text.
    #[serde(default)]
    #[specta(type = Option<String>)]
    pub private_key_pem: Option<SecretString>,
    /// Key passphrase.
    #[serde(default)]
    #[specta(type = Option<String>)]
    pub passphrase: Option<SecretString>,
}

/// `identity_update` input. Absent / `null` secret = keep; `clear_*` removes a stored secret.
#[derive(Default, Deserialize, specta::Type)]
#[serde(default)]
pub struct IdentityPatchInput {
    /// Label.
    pub label: Option<String>,
    /// New username (`null` / absent = keep); see `clear_username`.
    pub username: Option<String>,
    /// Remove the username.
    pub clear_username: bool,
    /// Replace the password.
    #[specta(type = Option<String>)]
    pub password: Option<SecretString>,
    /// Replace the private key (fingerprint/public key recomputed).
    #[specta(type = Option<String>)]
    pub private_key_pem: Option<SecretString>,
    /// Replace the passphrase.
    #[specta(type = Option<String>)]
    pub passphrase: Option<SecretString>,
    /// Remove the stored password.
    pub clear_password: bool,
    /// Remove the stored passphrase.
    pub clear_passphrase: bool,
}

macro_rules! redacted_debug {
    ($($t:ty),+) => {$(
        impl std::fmt::Debug for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($t), " { [REDACTED] }"))
            }
        }
    )+};
}
redacted_debug!(IdentityInput, IdentityPatchInput);

/// `vault_status` result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
pub struct VaultStatus {
    /// Lock state.
    pub state: VaultState,
    /// `None` while uninitialized.
    pub mode: Option<VaultMode>,
    /// Biometric unlock possible.
    pub biometric_available: bool,
}

/// Quick connect target (`user@host:port` from the palette).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, specta::Type)]
pub struct QuickConnect {
    /// Username; `None` → UI asked already or agent default.
    #[serde(default)]
    pub user: Option<String>,
    /// Hostname / IP.
    pub hostname: String,
    /// Port (default 22).
    #[serde(default)]
    pub port: Option<u16>,
}

/// Per-session events (`on_event` channel). TS: tag `type`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "type")]
pub enum SessionEvent {
    /// Progress; `hop` = label of the host being connected (jump hosts first).
    Connecting {
        /// Step.
        step: ConnectStep,
        /// Hop label.
        hop: String,
    },
    /// One TCP attempt at an address of the hop (several when the name has many addresses).
    Attempt {
        /// Hop label.
        hop: String,
        /// `ip:port` that was tried.
        addr: String,
        /// `None` = connected; else `Timeout` or the IO error kind (`ConnectionRefused`, …).
        failure: Option<String>,
    },
    /// TOFU: reply with `hostkey_accept` or `session_close`.
    HostKeyPrompt {
        /// Reply id.
        pending_id: PendingId,
        /// Key info.
        info: HostKeyInfo,
    },
    /// Reply with `kbd_interactive_reply`.
    KbdInteractive {
        /// Reply id.
        pending_id: PendingId,
        /// Server name (plain text).
        name: String,
        /// Server instruction (plain text).
        instruction: String,
        /// Prompts.
        prompts: Vec<KbdPrompt>,
    },
    /// Reply with `passphrase_reply`.
    PassphrasePrompt {
        /// Reply id.
        pending_id: PendingId,
        /// Identity label.
        identity_label: String,
    },
    /// Shell is open.
    Connected {
        /// Server auth banner (plain text).
        server_banner: Option<String>,
    },
    /// Connect failed before `Connected` (host key changed, auth failed, network…).
    Failed {
        /// Reason.
        error: ApiError,
    },
    /// Connection closed after `Connected`.
    Disconnected {
        /// Reason code: `closed`, `shell_exited`, `keepalive`, `inactivity`,
        /// `server:<DisconnectCode>`, `network:<IoKind>`, `protocol` (the UI localizes them).
        reason: String,
        /// `session_reconnect` is possible.
        can_reconnect: bool,
    },
    /// Remote shell exited.
    ExitStatus {
        /// Exit code.
        code: u32,
    },
    /// Port forward state change (Phase 3).
    ForwardStatus {
        /// Forward rule.
        forward_id: ForwardId,
        /// State.
        state: ForwardState,
        /// Error (sanitized).
        error: Option<String>,
    },
}

/// `sftp_open` target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SftpTarget {
    /// Reuse the SSH connection of an open terminal session.
    Session {
        /// Session.
        session_id: SessionId,
    },
    /// Open a new connection to a stored host (prompts go to `on_event`).
    Host {
        /// Host.
        host_id: HostId,
    },
}

/// Transfer endpoint. TS: `{kind:"local"} | {kind:"remote", sftp_id}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Endpoint {
    /// Local FS (desktop only).
    Local,
    /// Open SFTP session.
    Remote {
        /// Session.
        sftp_id: SftpId,
    },
}

/// `transfer_enqueue` input: copy `items` (absolute `src` paths) into `dst_dir`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, specta::Type)]
pub struct TransferRequest {
    /// Source side.
    pub src: Endpoint,
    /// Destination side.
    pub dst: Endpoint,
    /// Absolute source paths.
    pub items: Vec<String>,
    /// Absolute destination directory.
    pub dst_dir: String,
    /// Conflict policy.
    pub conflict: ConflictPolicy,
}

/// `app_info` result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
pub struct AppInfo {
    /// App version.
    pub version: String,
    /// `macos` | `windows` | `linux` | `ios` | `android`.
    pub platform: String,
    /// `aarch64` | `x86_64` | ….
    pub arch: String,
}

/// Re-exported so commands can name it without importing sftp.
pub type TransferIds = Vec<TransferId>;
