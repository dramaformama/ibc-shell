//! Connection establishment: TCP / jump-host tunnelling, handshake with host key verification,
//! authentication.

use std::future::Future;
use std::net::SocketAddr;
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
use crate::verify::{ClientHandler, CloseSlot, VerifyCtx, VerifyReq};
use crate::SshError;

/// TCP connect timeout per address.
const TCP_TIMEOUT: Duration = Duration::from_secs(15);
/// Maximum chain length (up to 3 jump hosts + target).
pub(crate) const MAX_HOPS: usize = 4;

/// Network phases without user interaction (key exchange, `none` probe) must finish within this.
/// Time spent waiting for the user (host key prompts) does not count.
const PHASE_TIMEOUT: Duration = Duration::from_secs(30);

/// Runs `fut` while serving host key verification requests from the handler. Fails with
/// [`SshError::Timeout`] if `limit` passes without progress; the clock restarts after every
/// served verification request so user prompts never time out.
async fn drive<T, F, K, P>(
    fut: F,
    rx: &mut mpsc::UnboundedReceiver<VerifyReq>,
    ctx: &mut VerifyCtx<'_, K, P>,
    limit: Duration,
) -> Result<T, SshError>
where
    F: Future<Output = T>,
    K: KnownHosts,
    P: Prompter,
{
    tokio::pin!(fut);
    let sleep = tokio::time::sleep(limit);
    tokio::pin!(sleep);
    loop {
        tokio::select! {
            out = &mut fut => return Ok(out),
            Some(req) = rx.recv() => {
                ctx.handle(req).await;
                sleep.as_mut().reset(tokio::time::Instant::now() + limit);
            }
            () = &mut sleep => return Err(SshError::Timeout),
        }
    }
}

/// DNS lookup (also accepts an IP literal).
async fn resolve(host: &str, port: u16) -> Result<Vec<SocketAddr>, SshError> {
    let addrs: Vec<_> = tokio::time::timeout(TCP_TIMEOUT, tokio::net::lookup_host((host, port)))
        .await
        .map_err(|_| SshError::Timeout)?
        .map_err(|e| SshError::Network(format!("{:?}", e.kind())))?
        .collect();
    if addrs.is_empty() {
        return Err(SshError::Network("no address".into()));
    }
    Ok(addrs)
}

/// Tries the addresses in order and reports every attempt, so the UI can show which address
/// answered and which one did not.
async fn tcp_connect<P: Prompter>(
    addrs: &[SocketAddr],
    label: &str,
    prompter: &P,
) -> Result<TcpStream, SshError> {
    let mut last = SshError::Network("no address".into());
    for a in addrs {
        let (err, failure) = match tokio::time::timeout(TCP_TIMEOUT, TcpStream::connect(a)).await {
            Ok(Ok(s)) => {
                prompter.attempt(label, &a.to_string(), None);
                return Ok(s);
            }
            Ok(Err(e)) => {
                let kind = format!("{:?}", e.kind());
                (SshError::Network(kind.clone()), kind)
            }
            Err(_) => (SshError::Timeout, "Timeout".to_owned()),
        };
        prompter.attempt(label, &a.to_string(), Some(&failure));
        last = err;
    }
    Err(last)
}

async fn handshake<S, K, P>(
    stream: S,
    hop: &Hop,
    known: &K,
    prompter: &P,
) -> Result<(Handle<ClientHandler>, Option<String>, CloseSlot), SshError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    K: KnownHosts,
    P: Prompter,
{
    let (tx, mut rx) = mpsc::unbounded_channel();
    let banner = Arc::new(Mutex::new(None));
    let closed: CloseSlot = Arc::new(Mutex::new(None));
    let handler = ClientHandler {
        tx,
        banner: Arc::clone(&banner),
        closed: Arc::clone(&closed),
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
    let connected = drive(
        client::connect_stream(cfg, stream, handler),
        &mut rx,
        &mut ctx,
        PHASE_TIMEOUT,
    )
    .await?;
    let mut handle = match connected {
        Ok(h) => h,
        Err(e) => return Err(ctx.error.take().unwrap_or_else(|| map_russh(&e))),
    };
    // The `none` probe waits for the key exchange and tells us which methods are offered.
    let probe = drive(
        handle.authenticate_none(hop.username.clone()),
        &mut rx,
        &mut ctx,
        PHASE_TIMEOUT,
    )
    .await?;
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
    Ok((handle, banner, closed))
}

async fn connect_hop<K, P>(
    hop: &Hop,
    previous: Option<&Handle<ClientHandler>>,
    known: &K,
    prompter: &P,
) -> Result<(Handle<ClientHandler>, Option<String>, CloseSlot), SshError>
where
    K: KnownHosts,
    P: Prompter,
{
    match previous {
        None => {
            prompter.progress(&hop.label, ConnectStep::Resolving);
            let addrs = resolve(&hop.host, hop.port).await?;
            prompter.progress(&hop.label, ConnectStep::Connecting);
            let s = tcp_connect(&addrs, &hop.label, prompter).await?;
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
    let mut slots = Vec::with_capacity(chain.len());
    for hop in &chain {
        let (h, b, c) = connect_hop(hop, handles.last(), known, prompter).await?;
        banner = b;
        handles.push(h);
        slots.push(c);
    }
    let last = handles.pop().ok_or(SshError::InvalidChain)?;
    slots.reverse(); // the target hop first
    Ok(Connection::new(last, handles, banner, slots))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KbdInteractiveRequest;
    use crate::hostkey::{HostKeyDecision, HostKeyInfo, KnownKey};
    use secrecy::SecretString;

    struct NoKnown;
    impl KnownHosts for NoKnown {
        fn lookup(&self, _: &str) -> Result<Vec<KnownKey>, String> {
            Ok(Vec::new())
        }
        fn add(&self, _: &str, _: &KnownKey) -> Result<(), String> {
            Ok(())
        }
    }
    struct NoPrompt;
    #[allow(clippy::unused_async_trait_impl)]
    impl Prompter for NoPrompt {
        fn progress(&self, _: &str, _: ConnectStep) {}
        async fn confirm_unknown_host(&self, _: HostKeyInfo) -> HostKeyDecision {
            HostKeyDecision::Reject
        }
        async fn keyboard_interactive(
            &self,
            _: KbdInteractiveRequest,
        ) -> Option<Vec<SecretString>> {
            None
        }
        async fn passphrase(&self, _: &str) -> Option<SecretString> {
            None
        }
    }

    /// Records progress and attempts, in order.
    #[derive(Default)]
    struct Recorder(Mutex<Vec<String>>);
    #[allow(clippy::unused_async_trait_impl)]
    impl Prompter for Recorder {
        fn progress(&self, hop: &str, step: ConnectStep) {
            self.0.lock().unwrap().push(format!("{hop}:{step:?}"));
        }
        fn attempt(&self, hop: &str, addr: &str, failure: Option<&str>) {
            self.0
                .lock()
                .unwrap()
                .push(format!("{hop}:{addr}:{}", failure.unwrap_or("ok")));
        }
        async fn confirm_unknown_host(&self, _: HostKeyInfo) -> HostKeyDecision {
            HostKeyDecision::Reject
        }
        async fn keyboard_interactive(
            &self,
            _: KbdInteractiveRequest,
        ) -> Option<Vec<SecretString>> {
            None
        }
        async fn passphrase(&self, _: &str) -> Option<SecretString> {
            None
        }
    }

    #[tokio::test]
    async fn refused_port_reports_steps_then_the_failed_attempt() {
        // A port nobody listens on: bind, note it, drop.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let hop = Hop {
            label: "h".into(),
            host: "127.0.0.1".into(),
            port,
            username: "u".into(),
            auth: Vec::new(),
            keepalive_s: 0,
            legacy_algorithms: false,
        };
        let rec = Recorder::default();
        let r = connect_chain(vec![hop], &NoKnown, &rec).await;
        assert!(matches!(r, Err(SshError::Network(k)) if k == "ConnectionRefused"));
        let log = rec.0.lock().unwrap().clone();
        assert_eq!(
            log,
            [
                "h:Resolving".to_owned(),
                "h:Connecting".to_owned(),
                format!("h:127.0.0.1:{port}:ConnectionRefused"),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn drive_times_out_on_silent_peer() {
        let (_tx, mut rx) = mpsc::unbounded_channel::<VerifyReq>();
        let (known, prompter) = (NoKnown, NoPrompt);
        let mut ctx = VerifyCtx {
            host: "h",
            port: 22,
            known: &known,
            prompter: &prompter,
            error: None,
        };
        let r = drive(
            std::future::pending::<()>(),
            &mut rx,
            &mut ctx,
            Duration::from_secs(30),
        )
        .await;
        assert!(matches!(r, Err(SshError::Timeout)));
    }

    #[tokio::test(start_paused = true)]
    async fn drive_returns_value_in_time() {
        let (_tx, mut rx) = mpsc::unbounded_channel::<VerifyReq>();
        let (known, prompter) = (NoKnown, NoPrompt);
        let mut ctx = VerifyCtx {
            host: "h",
            port: 22,
            known: &known,
            prompter: &prompter,
            error: None,
        };
        let fut = async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            7
        };
        let r = drive(fut, &mut rx, &mut ctx, Duration::from_secs(30)).await;
        assert!(matches!(r, Ok(7)));
    }
}
