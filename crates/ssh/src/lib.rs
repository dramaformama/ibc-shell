//! SSH client built on russh: connect, auth, host keys, PTY, forwards, jump hosts.
//!
//! Contract: `docs/contracts/ipc.md` §crates/ssh public API + `security.md` §SSH policy
//! (FROZEN v1). Must not depend on `store`/`vault`: credentials and known hosts come in via
//! [`Hop`] / [`KnownHosts`]. Phase 0 skeleton — fallible fns return [`SshError::Unimplemented`].
// Skeleton-only allowances; the ssh-core agent removes this line when implementing.
#![allow(
    unused_variables,
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    clippy::unused_self,
    clippy::needless_pass_by_value
)]

mod config;
mod hostkey;
mod session;

pub use config::{
    AuthMethod, ConnectStep, Hop, KbdInteractiveRequest, KbdPrompt, Prompter, PtyRequest,
};
pub use hostkey::{
    host_pattern, HostKeyDecision, HostKeyInfo, HostKeyMismatch, KnownHosts, KnownKey,
};
pub use session::{
    Connection, ExecOutput, ForwardHandle, ForwardSpec, ForwardState, Shell, ShellOutput,
    SubsystemStream, CREDIT_WINDOW,
};

/// SSH errors. Messages never contain credentials.
#[derive(Debug, thiserror::Error)]
pub enum SshError {
    /// Changed host key: hard block (no accept path exists). A rejected *unknown* key
    /// yields [`SshError::Cancelled`].
    #[error("host key changed for {}:{}", .0.host, .0.port)]
    HostKeyChanged(Box<HostKeyMismatch>),
    /// All auth methods failed; remaining server-advertised methods.
    #[error("authentication failed")]
    AuthFailed {
        /// Methods the server still offers.
        methods_left: Vec<String>,
    },
    /// DNS / TCP / IO failure (sanitized).
    #[error("network error: {0}")]
    Network(String),
    /// Connect / keepalive timeout.
    #[error("timeout")]
    Timeout,
    /// User cancelled a prompt.
    #[error("cancelled")]
    Cancelled,
    /// No common algorithm (policy refused the server's offer).
    #[error("no common algorithm: {0}")]
    Negotiation(String),
    /// Jump chain invalid (empty or > 4 hops).
    #[error("invalid jump chain")]
    InvalidChain,
    /// Known-hosts backend failure.
    #[error("known hosts: {0}")]
    KnownHosts(String),
    /// Protocol / channel error.
    #[error("ssh protocol error: {0}")]
    Protocol(String),
    /// Phase 0 skeleton placeholder.
    #[error("not implemented")]
    Unimplemented,
}

/// Connects through `chain` (jump hosts first, target last; each hop's host key is verified
/// independently) and authenticates. Changed key on any hop → [`SshError::HostKeyChanged`].
pub async fn connect<K, P>(
    chain: Vec<Hop>,
    known_hosts: &K,
    prompter: &P,
) -> Result<Connection, SshError>
where
    K: KnownHosts,
    P: Prompter,
{
    Err(SshError::Unimplemented)
}
