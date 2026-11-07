//! Connection establishment: TCP / jump-host tunnelling, handshake with host key verification,
//! authentication.

use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use russh::client::{self, AuthResult, Handle};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::auth::authenticate;
use crate::config::{ConnectStep, Hop, Prompter};
use crate::error::map_russh;
use crate::hostkey::KnownHosts;
use crate::policy::client_config;
use crate::session::Connection;
use crate::verify::{ClientHandler, VerifyCtx, VerifyReq};
use crate::SshError;

/// TCP connect timeout per address.
const TCP_TIMEOUT: Duration = Duration::from_secs(15);
/// Maximum chain length (up to 3 jump hosts + target).
pub(crate) const MAX_HOPS: usize = 4;

/// Runs `fut` while serving host key verification requests from the handler.
async fn drive<T, F, K, P>(
    fut: F,
    rx: &mut mpsc::UnboundedReceiver<VerifyReq>,
    ctx: &mut VerifyCtx<'_, K, P>,
) -> T
where
    F: Future<Output = T>,
    K: KnownHosts,
    P: Prompter,
{
    tokio::pin!(fut);
    loop {
        tokio::select! {
            out = &mut fut => return out,
            Some(req) = rx.recv() => ctx.handle(req).await,
        }
    }
}

async fn tcp(host: &str, port: u16) -> Result<TcpStream, SshError> {
    let addrs: Vec<_> = tokio::time::timeout(TCP_TIMEOUT, tokio::net::lookup_host((host, port)))
        .await
        .map_err(|_| SshError::Timeout)?
        .map_err(|e| SshError::Network(format!("{:?}", e.kind())))?
        .collect();
    let mut last = SshError::Network("no address".into());
    for a in addrs {
        match tokio::time::timeout(TCP_TIMEOUT, TcpStream::connect(a)).await {
            Ok(Ok(s)) => return Ok(s),
            Ok(Err(e)) => last = SshError::Network(format!("{:?}", e.kind())),
            Err(_) => last = SshError::Timeout,
        }
    }
    Err(last)
}

async fn handshake<S, K, P>(
    stream: S,
    hop: &Hop,
    known: &K,
    prompter: &P,
) -> Result<(Handle<ClientHandler>, Option<String>), SshError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    K: KnownHosts,
    P: Prompter,
{
    let (tx, mut rx) = mpsc::unbounded_channel();
    let banner = Arc::new(Mutex::new(None));
    let handler = ClientHandler {
        tx,
        banner: Arc::clone(&banner),
    };
    let mut ctx = VerifyCtx {
        host: &hop.host,
        port: hop.port,
        known,
        prompter,
        error: None,
    };
    let cfg = client_config(hop.keepalive_s, hop.legacy_algorithms);
    prompter.progress(&hop.label, ConnectStep::Handshake);
    let connected = drive(client::connect_stream(cfg, stream, handler), &mut rx, &mut ctx).await;
    let mut handle = match connected {
        Ok(h) => h,
        Err(e) => return Err(ctx.error.take().unwrap_or_else(|| map_russh(&e))),
    };
    // The `none` probe waits for the key exchange and tells us which methods are offered.
    let probe = drive(handle.authenticate_none(hop.username.clone()), &mut rx, &mut ctx).await;
    prompter.progress(&hop.label, ConnectStep::Authenticating);
    let remaining = match probe {
        Ok(AuthResult::Success) => None,
        Ok(AuthResult::Failure {
            remaining_methods, ..
        }) => Some(remaining_methods),
        Err(e) => return Err(ctx.error.take().unwrap_or_else(|| map_russh(&e))),
    };
    if let Some(remaining) = remaining {
        authenticate(
            &mut handle,
            &hop.username,
            &hop.auth,
            remaining,
            &hop.label,
            prompter,
        )
        .await?;
    }
    let banner = banner.lock().ok().and_then(|mut b| b.take());
    Ok((handle, banner))
}

async fn connect_hop<K, P>(
    hop: &Hop,
    previous: Option<&Handle<ClientHandler>>,
    known: &K,
    prompter: &P,
) -> Result<(Handle<ClientHandler>, Option<String>), SshError>
where
    K: KnownHosts,
    P: Prompter,
{
    match previous {
        None => {
            prompter.progress(&hop.label, ConnectStep::Resolving);
            let s = tcp(&hop.host, hop.port).await?;
            prompter.progress(&hop.label, ConnectStep::Connecting);
            handshake(s, hop, known, prompter).await
        }
        Some(prev) => {
            prompter.progress(&hop.label, ConnectStep::Connecting);
            let ch = prev
                .channel_open_direct_tcpip(hop.host.clone(), u32::from(hop.port), "127.0.0.1", 0)
                .await
                .map_err(|e| map_russh(&e))?;
            handshake(ch.into_stream(), hop, known, prompter).await
        }
    }
}

/// See [`crate::connect`].
pub(crate) async fn connect_chain<K, P>(
    chain: Vec<Hop>,
    known: &K,
    prompter: &P,
) -> Result<Connection, SshError>
where
    K: KnownHosts,
    P: Prompter,
{
    if chain.is_empty() || chain.len() > MAX_HOPS {
        return Err(SshError::InvalidChain);
    }
    let mut handles: Vec<Handle<ClientHandler>> = Vec::with_capacity(chain.len());
    let mut banner = None;
    for hop in &chain {
        let (h, b) = connect_hop(hop, handles.last(), known, prompter).await?;
        banner = b;
        handles.push(h);
    }
    let last = handles.pop().ok_or(SshError::InvalidChain)?;
    Ok(Connection::new(last, handles, banner))
}
