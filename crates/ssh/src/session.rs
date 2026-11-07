//! Established connection, interactive shell, subsystem stream, port forwards.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::config::PtyRequest;
use crate::SshError;

/// Credit window: the shell stops reading from the SSH channel while unacked bytes exceed this.
pub const CREDIT_WINDOW: u32 = 1 << 20;

/// An authenticated connection (to the last hop of the chain). Cheap to share via `Arc`;
/// dropping the last reference disconnects.
#[derive(Debug)]
pub struct Connection {
    _private: (),
}

impl Connection {
    /// Server banner received during auth, if any (plain text).
    #[must_use]
    pub fn server_banner(&self) -> Option<&str> {
        None
    }
    /// Opens a PTY + shell channel.
    pub async fn open_shell(&self, pty: PtyRequest) -> Result<Shell, SshError> {
        Err(SshError::Unimplemented)
    }
    /// Runs a command without PTY, collecting at most `max_output` bytes of stdout.
    pub async fn exec(&self, command: &str, max_output: usize) -> Result<ExecOutput, SshError> {
        Err(SshError::Unimplemented)
    }
    /// Opens the `sftp` subsystem; the stream feeds `russh-sftp` in `crates/sftp`.
    pub async fn open_sftp(&self) -> Result<SubsystemStream, SshError> {
        Err(SshError::Unimplemented)
    }
    /// Starts a port forward (Phase 3). Stops when the handle is dropped.
    pub async fn start_forward(&self, spec: ForwardSpec) -> Result<ForwardHandle, SshError> {
        Err(SshError::Unimplemented)
    }
    /// Graceful disconnect.
    pub async fn disconnect(&self) {}
}

/// Output of [`Connection::exec`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecOutput {
    /// Exit status, if the server sent one.
    pub exit_status: Option<u32>,
    /// Stdout (truncated to `max_output`).
    pub stdout: Vec<u8>,
    /// Stderr (truncated to `max_output`).
    pub stderr: Vec<u8>,
}

/// Item read from a shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellOutput {
    /// stdout/stderr bytes (merged, as a terminal sees them).
    Data(Vec<u8>),
    /// Remote process exit status.
    ExitStatus(u32),
}

/// Interactive shell with credit-based backpressure ([`CREDIT_WINDOW`]).
#[derive(Debug)]
pub struct Shell {
    _private: (),
}

impl Shell {
    /// Next output item; `None` = channel closed (EOF / disconnect). Waits while the credit
    /// window is exhausted.
    pub async fn read(&mut self) -> Option<ShellOutput> {
        None
    }
    /// Sends input bytes.
    pub async fn write(&self, data: &[u8]) -> Result<(), SshError> {
        Err(SshError::Unimplemented)
    }
    /// Window change.
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<(), SshError> {
        Err(SshError::Unimplemented)
    }
    /// Returns credit for `bytes` consumed by the UI (`shell_ack`).
    pub fn ack(&self, bytes: u32) {}
    /// Sends EOF + close.
    pub async fn close(self) {}
}

/// Bidirectional byte stream of a subsystem channel.
#[derive(Debug)]
pub struct SubsystemStream {
    _private: (),
}

fn unimplemented_io() -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, "not implemented")
}

impl AsyncRead for SubsystemStream {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Err(unimplemented_io()))
    }
}

impl AsyncWrite for SubsystemStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Err(unimplemented_io()))
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Err(unimplemented_io()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Err(unimplemented_io()))
    }
}

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
    _private: (),
}

impl ForwardHandle {
    /// Actual bound port (useful when `bind_port` was 0).
    #[must_use]
    pub fn bound_port(&self) -> u16 {
        0
    }
}
