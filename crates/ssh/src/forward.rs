//! Port forwards over an established [`Connection`]: local (`-L`), remote (`-R`) and
//! dynamic SOCKS5 (`-D`). Dropping the [`ForwardHandle`] stops the forward and every
//! connection it carries.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use russh::client::Msg;
use russh::Channel;
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::{AbortHandle, JoinSet};

use crate::error::map_russh;
use crate::session::Connection;
use crate::{socks, SshError};

/// Simultaneous connections carried by one forward; extra ones are closed right away.
pub const MAX_CONNECTIONS: usize = 256;
/// Time for a SOCKS client to send its request, and for a local target to accept.
const SETUP_TIMEOUT: Duration = Duration::from_secs(10);

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

/// Where connections of one remote forward go, and the tasks carrying them.
pub(crate) struct RemoteTarget {
    host: String,
    port: u16,
    tasks: Vec<AbortHandle>,
}

/// Remote forwards registered on one hop, by the port the server listens on.
pub(crate) type RemoteMap = Arc<Mutex<HashMap<u32, RemoteTarget>>>;

enum Stop {
    Listener(AbortHandle),
    Remote {
        conn: Arc<Connection>,
        addr: String,
        port: u32,
    },
}

/// Running forward; dropping it stops the forward.
pub struct ForwardHandle {
    bound_port: u16,
    stop: Option<Stop>,
}

impl std::fmt::Debug for ForwardHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForwardHandle")
            .field("bound_port", &self.bound_port)
            .finish_non_exhaustive()
    }
}

impl ForwardHandle {
    /// Actual bound port (useful when `bind_port` was 0).
    #[must_use]
    pub fn bound_port(&self) -> u16 {
        self.bound_port
    }
}

impl Drop for ForwardHandle {
    fn drop(&mut self) {
        match self.stop.take() {
            Some(Stop::Listener(task)) => task.abort(),
            Some(Stop::Remote { conn, addr, port }) => {
                if let Some(t) = lock(conn.remote_map()).remove(&port) {
                    t.tasks.iter().for_each(AbortHandle::abort);
                }
                if let Ok(rt) = tokio::runtime::Handle::try_current() {
                    rt.spawn(async move {
                        let _ = conn.handle().cancel_tcpip_forward(addr, port).await;
                    });
                }
            }
            None => {}
        }
    }
}

fn lock(map: &RemoteMap) -> std::sync::MutexGuard<'_, HashMap<u32, RemoteTarget>> {
    map.lock().unwrap_or_else(PoisonError::into_inner)
}

fn io_err(e: &std::io::Error) -> SshError {
    SshError::Network(format!("{:?}", e.kind()))
}

/// See [`Connection::start_forward`].
pub(crate) async fn start(
    conn: &Arc<Connection>,
    spec: ForwardSpec,
) -> Result<ForwardHandle, SshError> {
    let (bind_addr, bind_port, dest) = match spec {
        ForwardSpec::Remote {
            bind_addr,
            bind_port,
            dest_host,
            dest_port,
        } => return start_remote(conn, bind_addr, bind_port, dest_host, dest_port).await,
        ForwardSpec::Local {
            bind_addr,
            bind_port,
            dest_host,
            dest_port,
        } => (bind_addr, bind_port, Some((dest_host, dest_port))),
        ForwardSpec::Dynamic {
            bind_addr,
            bind_port,
        } => (bind_addr, bind_port, None),
    };
    let listener = TcpListener::bind((bind_addr.as_str(), bind_port))
        .await
        .map_err(|e| io_err(&e))?;
    let bound_port = listener.local_addr().map_or(bind_port, |a| a.port());
    let task = tokio::spawn(serve(Arc::clone(conn), listener, dest));
    Ok(ForwardHandle {
        bound_port,
        stop: Some(Stop::Listener(task.abort_handle())),
    })
}

/// Accepts local connections until aborted; aborting drops the set and every carried connection.
async fn serve(conn: Arc<Connection>, listener: TcpListener, dest: Option<(String, u16)>) {
    let mut set = JoinSet::new();
    loop {
        while set.try_join_next().is_some() {}
        let Ok((sock, peer)) = listener.accept().await else {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        };
        if set.len() >= MAX_CONNECTIONS {
            continue; // `sock` dropped: closed right away
        }
        let (conn, dest) = (Arc::clone(&conn), dest.clone());
        set.spawn(async move {
            let _ = carry_local(&conn, sock, peer, dest).await;
        });
    }
}

async fn carry_local(
    conn: &Connection,
    mut sock: TcpStream,
    peer: std::net::SocketAddr,
    dest: Option<(String, u16)>,
) -> std::io::Result<()> {
    let socks = dest.is_none();
    let (host, port) = match dest {
        Some(d) => d,
        None => tokio::time::timeout(SETUP_TIMEOUT, socks::accept(&mut sock))
            .await
            .map_err(|_| std::io::ErrorKind::TimedOut)??,
    };
    let opened = conn
        .handle()
        .channel_open_direct_tcpip(
            host,
            u32::from(port),
            peer.ip().to_string(),
            u32::from(peer.port()),
        )
        .await;
    let Ok(ch) = opened else {
        if socks {
            socks::reply(&mut sock, 5).await?;
        }
        return Ok(());
    };
    if socks {
        socks::reply(&mut sock, 0).await?;
    }
    let mut stream = ch.into_stream();
    tokio::io::copy_bidirectional(&mut sock, &mut stream).await?;
    Ok(())
}

async fn start_remote(
    conn: &Arc<Connection>,
    bind_addr: String,
    bind_port: u16,
    dest_host: String,
    dest_port: u16,
) -> Result<ForwardHandle, SshError> {
    let target = || RemoteTarget {
        host: dest_host.clone(),
        port: dest_port,
        tasks: Vec::new(),
    };
    let asked = u32::from(bind_port);
    // Registered before the request: the server may open a channel as soon as it replies.
    if asked != 0 {
        lock(conn.remote_map()).insert(asked, target());
    }
    let granted = match conn.handle().tcpip_forward(bind_addr.clone(), asked).await {
        Ok(p) => p,
        Err(e) => {
            lock(conn.remote_map()).remove(&asked);
            return Err(match e {
                russh::Error::RequestDenied => {
                    SshError::Protocol("remote forward refused by server".into())
                }
                e => map_russh(&e),
            });
        }
    };
    let port = if granted == 0 { asked } else { granted };
    if port != asked {
        let mut map = lock(conn.remote_map());
        map.remove(&asked);
        map.insert(port, target());
    }
    Ok(ForwardHandle {
        bound_port: u16::try_from(port).unwrap_or(bind_port),
        stop: Some(Stop::Remote {
            conn: Arc::clone(conn),
            addr: bind_addr,
            port,
        }),
    })
}

/// Destination of a channel the server opened for remote forward `port`, if one is registered.
pub(crate) fn remote_target(map: &RemoteMap, port: u32) -> Option<(String, u16)> {
    lock(map).get(&port).map(|t| (t.host.clone(), t.port))
}

/// Carries one server-opened channel to its local destination.
pub(crate) fn carry_remote(map: &RemoteMap, port: u32, channel: Channel<Msg>, dest: (String, u16)) {
    let mut guard = lock(map);
    let Some(entry) = guard.get_mut(&port) else {
        return;
    };
    entry.tasks.retain(|t| !t.is_finished());
    if entry.tasks.len() >= MAX_CONNECTIONS {
        return; // `channel` dropped: closed right away
    }
    let task = tokio::spawn(async move {
        let connect = TcpStream::connect((dest.0.as_str(), dest.1));
        let Ok(Ok(mut sock)) = tokio::time::timeout(SETUP_TIMEOUT, connect).await else {
            return;
        };
        let mut stream = channel.into_stream();
        let _ = tokio::io::copy_bidirectional(&mut sock, &mut stream).await;
    });
    entry.tasks.push(task.abort_handle());
}
