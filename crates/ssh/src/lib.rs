//! SSH client built on russh: connect, auth, host keys, PTY, forwards, jump hosts.
//!
//! Contract: `docs/contracts/ipc.md` §crates/ssh public API + `security.md` §SSH policy
//! (FROZEN v1). Must not depend on `store`/`vault`: credentials and known hosts come in via
//! [`Hop`] / [`KnownHosts`]. Forwards are Phase 3 (`SshError::Unimplemented`).
mod auth;
mod certs;
mod config;
mod connect;
mod docker;
mod docker_more;
mod docker_more_types;
mod error;
mod forward;
mod hostkey;
pub mod policy;
mod session;
mod shell;
mod stats;
mod stats_net;
mod stats_parse;
mod verify;

pub use certs::{cert_list, CertInfo, CertReport};
pub use config::{
    AuthMethod, ConnectStep, Hop, KbdInteractiveRequest, KbdPrompt, Prompter, PtyRequest,
};
pub use docker::{
    docker_act, docker_list, docker_logs, Container, DockerAccess, DockerAction, DockerReport,
};
pub use docker_more::{
    docker_compose, docker_events, docker_images, docker_networks, docker_volumes,
};
pub use docker_more_types::{
    ComposeProject, ComposeService, DockerCompose, DockerEvent, DockerEvents, DockerImage,
    DockerImages, DockerNetwork, DockerNetworks, DockerVolume, DockerVolumes,
};
pub use forward::{ForwardHandle, ForwardSpec, ForwardState};
pub use hostkey::{
    host_pattern, HostKeyDecision, HostKeyInfo, HostKeyMismatch, KnownHosts, KnownKey,
};
pub use session::{Connection, ExecOutput, SubsystemStream, CREDIT_WINDOW};
pub use shell::{Shell, ShellOutput};
pub use stats::{sample_stats, CoreSplit, HostStats, NetIface, ProcRow};
pub use stats_net::{Listener, PeerFlow};

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
    /// Not implemented yet (Phase 3 forwards).
    #[error("not implemented")]
    Unimplemented,
    /// Remote host is not Linux (no usable `/proc`).
    #[error("unsupported remote system")]
    Unsupported,
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
    connect::connect_chain(chain, known_hosts, prompter).await
}
