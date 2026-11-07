//! Connection configuration, credentials and interactive prompts.

use std::future::Future;

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::hostkey::{HostKeyDecision, HostKeyInfo};

/// One hop of a connection chain (jump hosts first, target last; max 4 entries).
#[derive(Debug)]
pub struct Hop {
    /// Display label (used in prompts / progress).
    pub label: String,
    /// Hostname / IP (no brackets).
    pub host: String,
    /// Port.
    pub port: u16,
    /// Username (already resolved; UI prompts before connecting when absent).
    pub username: String,
    /// Auth methods to try, in order.
    pub auth: Vec<AuthMethod>,
    /// Keepalive interval seconds; 3 missed → disconnect. 0 = off.
    pub keepalive_s: u32,
    /// Allow legacy algorithms (Phase 3 toggle). Default false.
    pub legacy_algorithms: bool,
}

/// A credential. `Debug` prints `[REDACTED]` for secret material.
pub enum AuthMethod {
    /// Password.
    Password(SecretString),
    /// Private key PEM/OpenSSH; passphrase asked via [`Prompter::passphrase`] if needed and absent.
    Key {
        /// Key text (zeroized on drop).
        pem: Zeroizing<String>,
        /// Stored passphrase.
        passphrase: Option<SecretString>,
        /// Identity label for the passphrase prompt.
        identity_label: String,
    },
    /// System agent (`SSH_AUTH_SOCK`, `\\.\pipe\openssh-ssh-agent`, Pageant).
    Agent,
    /// keyboard-interactive via [`Prompter::keyboard_interactive`].
    KeyboardInteractive,
}

impl std::fmt::Debug for AuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Password(_) => f.write_str("Password([REDACTED])"),
            Self::Key { identity_label, .. } => {
                write!(
                    f,
                    "Key {{ identity_label: {identity_label:?}, pem: [REDACTED] }}"
                )
            }
            Self::Agent => f.write_str("Agent"),
            Self::KeyboardInteractive => f.write_str("KeyboardInteractive"),
        }
    }
}

/// Progress step. IPC: `SessionEvent::Connecting.step`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum ConnectStep {
    /// DNS lookup.
    Resolving,
    /// TCP connect (or direct-tcpip through the previous hop).
    Connecting,
    /// KEX + host key check.
    Handshake,
    /// User auth.
    Authenticating,
    /// Opening PTY / shell / subsystem.
    OpeningChannel,
}

/// One keyboard-interactive prompt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct KbdPrompt {
    /// Prompt text (plain text, server-provided).
    pub text: String,
    /// Whether the answer may be echoed.
    pub echo: bool,
}

/// keyboard-interactive round.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KbdInteractiveRequest {
    /// Hop label.
    pub hop_label: String,
    /// Server-provided name.
    pub name: String,
    /// Server-provided instruction.
    pub instruction: String,
    /// Prompts.
    pub prompts: Vec<KbdPrompt>,
}

/// UI round-trips during connect, implemented by the bridge (pending ids + `SessionEvent`s).
/// Returning `None` / `Reject` aborts with `SshError::Cancelled`.
pub trait Prompter: Send + Sync {
    /// Progress notification (non-blocking).
    fn progress(&self, hop_label: &str, step: ConnectStep);
    /// Unknown host key (never called for a changed key).
    fn confirm_unknown_host(
        &self,
        info: HostKeyInfo,
    ) -> impl Future<Output = HostKeyDecision> + Send;
    /// keyboard-interactive answers, same order as prompts.
    fn keyboard_interactive(
        &self,
        req: KbdInteractiveRequest,
    ) -> impl Future<Output = Option<Vec<SecretString>>> + Send;
    /// Passphrase for an encrypted key.
    fn passphrase(&self, identity_label: &str)
        -> impl Future<Output = Option<SecretString>> + Send;
}

/// PTY request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PtyRequest {
    /// `TERM`, default `xterm-256color`.
    pub term: String,
    /// Columns.
    pub cols: u16,
    /// Rows.
    pub rows: u16,
}
