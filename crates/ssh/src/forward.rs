//! Port forward types. Phase 3: [`crate::Connection::start_forward`] returns
//! `SshError::Unimplemented` until then.

use serde::{Deserialize, Serialize};

/// Port forward definition (mirrors store `PortForward`, without ids).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForwardSpec {
    /// `-L bind_addr:bind_port:dest_host:dest_port`.
    Local {
        /// Bind address.
        bind_addr: String,
        /// Bind port.
        bind_port: u16,
        /// Destination host.
        dest_host: String,
        /// Destination port.
        dest_port: u16,
    },
    /// `-R`.
    Remote {
        /// Remote bind address.
        bind_addr: String,
        /// Remote bind port.
        bind_port: u16,
        /// Local destination host.
        dest_host: String,
        /// Local destination port.
        dest_port: u16,
    },
    /// `-D` SOCKS5 (CONNECT only, no auth).
    Dynamic {
        /// Bind address (default 127.0.0.1).
        bind_addr: String,
        /// Bind port.
        bind_port: u16,
    },
}

/// Forward state. IPC: `SessionEvent::ForwardStatus.state`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum ForwardState {
    /// Listening / registered.
    Active,
    /// Stopped by the user or session end.
    Stopped,
    /// Bind or remote request failed.
    Failed,
}

/// Running forward; dropping it stops the forward.
#[derive(Debug)]
pub struct ForwardHandle {
    pub(crate) _private: (),
}

impl ForwardHandle {
    /// Actual bound port (useful when `bind_port` was 0).
    #[must_use]
    pub fn bound_port(&self) -> u16 {
        0
    }
}
