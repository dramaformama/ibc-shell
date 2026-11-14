//! Established connection: shell / exec / subsystem channels.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use russh::client::{Handle, Msg};
use russh::{ChannelMsg, ChannelStream, Disconnect};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::config::PtyRequest;
use crate::error::map_russh;
use crate::forward::{ForwardHandle, ForwardSpec, RemoteMap};
use crate::shell::Shell;
use crate::verify::{ClientHandler, CloseSlot};
use crate::SshError;

/// Credit window: the shell stops reading from the SSH channel while unacked bytes exceed this.
pub const CREDIT_WINDOW: u32 = 1 << 20;

/// An authenticated connection (to the last hop of the chain). Cheap to share via `Arc`;
/// dropping the last reference disconnects (earlier hops are kept alive inside).
pub struct Connection {
    handle: Handle<ClientHandler>,
    _upstream: Vec<Handle<ClientHandler>>,
    banner: Option<String>,
    /// Close reasons, target hop first.
    closed: Vec<CloseSlot>,
    /// Remote forwards registered on the target hop.
    remote: RemoteMap,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("closed", &self.handle.is_closed())
            .finish_non_exhaustive()
    }
}

impl Connection {
    pub(crate) fn new(
        handle: Handle<ClientHandler>,
        upstream: Vec<Handle<ClientHandler>>,
        banner: Option<String>,
        closed: Vec<CloseSlot>,
        remote: RemoteMap,
    ) -> Self {
        Self {
            handle,
            _upstream: upstream,
            banner,
            closed,
            remote,
        }
    }

    pub(crate) fn handle(&self) -> &Handle<ClientHandler> {
        &self.handle
    }

    pub(crate) fn remote_map(&self) -> &RemoteMap {
        &self.remote
    }

    /// Why the connection ended, once it has: `server:<DisconnectCode>`, `keepalive`,
    /// `inactivity`, `network:<IoKind>` or `protocol` (fixed vocabulary, no server text).
    /// A jump host that dropped is reported when the target itself said nothing.
    #[must_use]
    pub fn close_reason(&self) -> Option<String> {
        self.closed
            .iter()
            .find_map(|c| c.lock().ok().and_then(|g| g.clone()))
    }

    /// Server banner received during auth, if any (plain text).
    #[must_use]
    pub fn server_banner(&self) -> Option<&str> {
        self.banner.as_deref()
    }

    /// True once the connection is gone (disconnect, keepalive loss).
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.handle.is_closed()
    }

    /// Opens a PTY + shell channel.
    pub async fn open_shell(&self, pty: PtyRequest) -> Result<Shell, SshError> {
        let ch = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| map_russh(&e))?;
        ch.request_pty(
            false,
            &pty.term,
            u32::from(pty.cols),
            u32::from(pty.rows),
            0,
            0,
            &[],
        )
        .await
        .map_err(|e| map_russh(&e))?;
        ch.request_shell(false).await.map_err(|e| map_russh(&e))?;
        let (rx, tx) = ch.split();
        Ok(Shell::new(rx, tx))
    }

    /// Runs a command without PTY, collecting at most `max_output` bytes of stdout.
    pub async fn exec(&self, command: &str, max_output: usize) -> Result<ExecOutput, SshError> {
        let mut ch = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| map_russh(&e))?;
        ch.exec(true, command.as_bytes().to_vec())
            .await
            .map_err(|e| map_russh(&e))?;
        let mut out = ExecOutput {
            exit_status: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        while let Some(msg) = ch.wait().await {
            match msg {
                ChannelMsg::Data { data } => append_capped(&mut out.stdout, &data, max_output),
                ChannelMsg::ExtendedData { data, ext: 1 } => {
                    append_capped(&mut out.stderr, &data, max_output);
                }
                ChannelMsg::ExitStatus { exit_status } => out.exit_status = Some(exit_status),
                ChannelMsg::Failure => {
                    return Err(SshError::Protocol("exec refused".into()));
                }
                ChannelMsg::Close => break,
                _ => {}
            }
        }
        Ok(out)
    }

    /// Opens the `sftp` subsystem; the stream feeds `russh-sftp` in `crates/sftp`.
    pub async fn open_sftp(&self) -> Result<SubsystemStream, SshError> {
        let mut ch = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| map_russh(&e))?;
        ch.request_subsystem(true, "sftp")
            .await
            .map_err(|e| map_russh(&e))?;
        loop {
            match ch.wait().await {
                Some(ChannelMsg::Success) => break,
                Some(ChannelMsg::Failure) | None => {
                    return Err(SshError::Protocol("sftp subsystem refused".into()));
                }
                Some(_) => {}
            }
        }
        Ok(SubsystemStream {
            inner: ch.into_stream(),
        })
    }

    /// Starts a port forward; it stops when the handle is dropped. Local and dynamic forwards
    /// listen on this machine, remote ones ask the server to listen.
    pub async fn start_forward(
        self: &std::sync::Arc<Self>,
        spec: ForwardSpec,
    ) -> Result<ForwardHandle, SshError> {
        crate::forward::start(self, spec).await
    }

    /// Graceful disconnect.
    pub async fn disconnect(&self) {
        let _ = self
            .handle
            .disconnect(Disconnect::ByApplication, "", "en")
            .await;
    }
}

fn append_capped(buf: &mut Vec<u8>, data: &[u8], cap: usize) {
    let room = cap.saturating_sub(buf.len());
    buf.extend_from_slice(&data[..data.len().min(room)]);
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

/// Bidirectional byte stream of a subsystem channel.
pub struct SubsystemStream {
    inner: ChannelStream<Msg>,
}

impl std::fmt::Debug for SubsystemStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubsystemStream").finish_non_exhaustive()
    }
}

impl AsyncRead for SubsystemStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for SubsystemStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::append_capped;

    #[test]
    fn capped_append() {
        let mut b = Vec::new();
        append_capped(&mut b, b"abcdef", 4);
        append_capped(&mut b, b"xyz", 4);
        assert_eq!(b, b"abcd");
    }
}
