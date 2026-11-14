#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    clippy::too_many_arguments,
    missing_docs
)]
//! Local, remote and SOCKS5 forwards against an in-process russh server that carries
//! `direct-tcpip` channels and `tcpip-forward` requests (no Docker, no OpenSSH).
//! The host key below is a throwaway test fixture, never used anywhere else.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ibc_shell_ssh::{
    connect, AuthMethod, ConnectStep, Connection, ForwardSpec, Hop, HostKeyDecision, HostKeyInfo,
    KbdInteractiveRequest, KnownHosts, KnownKey, Prompter,
};
use russh::keys::decode_secret_key;
use russh::server::{Auth, Config, Handler, Msg, Server, Session};
use russh::Channel;
use secrecy::SecretString;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const HOST_KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACAjWF9XMGonNvOZS5jQtwN2+OyShDB1KB0m2qfUAEySdgAAAIhasfTVWrH0
1QAAAAtzc2gtZWQyNTUxOQAAACAjWF9XMGonNvOZS5jQtwN2+OyShDB1KB0m2qfUAEySdg
AAAEBpExXls+OkMCOV+0lvM7tv4lCconrTA8VEu/lb4/uEkyNYX1cwaic285lLmNC3A3b4
7JKEMHUoHSbap9QATJJ2AAAABHRlc3QB
-----END OPENSSH PRIVATE KEY-----
";
const T: Duration = Duration::from_secs(10);

#[derive(Clone)]
struct Mock;

impl Server for Mock {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        Self
    }
}

impl Handler for Mock {
    type Error = russh::Error;

    async fn auth_password(&mut self, _: &str, _: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    /// Like OpenSSH: connect to the requested target and carry bytes both ways.
    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<Msg>,
        host: &str,
        port: u32,
        _: &str,
        _: u32,
        reply: russh::server::ChannelOpenHandle,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        let Ok(mut sock) = TcpStream::connect((host, u16::try_from(port).unwrap())).await else {
            reply.reject(russh::ChannelOpenFailure::ConnectFailed).await;
            return Ok(());
        };
        reply.accept().await;
        tokio::spawn(async move {
            let mut s = channel.into_stream();
            let _ = tokio::io::copy_bidirectional(&mut sock, &mut s).await;
        });
        Ok(())
    }

    /// Listens on 127.0.0.1 and opens a `forwarded-tcpip` channel per connection.
    async fn tcpip_forward(
        &mut self,
        address: &str,
        port: &mut u32,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let l = TcpListener::bind(("127.0.0.1", u16::try_from(*port).unwrap()))
            .await
            .unwrap();
        let bound = u32::from(l.local_addr().unwrap().port());
        *port = bound;
        let (h, addr) = (session.handle(), address.to_owned());
        tokio::spawn(async move {
            while let Ok((mut sock, peer)) = l.accept().await {
                let ch = h
                    .channel_open_forwarded_tcpip(
                        addr.clone(),
                        bound,
                        peer.ip().to_string(),
                        u32::from(peer.port()),
                    )
                    .await;
                let Ok(ch) = ch else { continue };
                tokio::spawn(async move {
                    let mut s = ch.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut sock, &mut s).await;
                });
            }
        });
        Ok(true)
    }
}

async fn start() -> u16 {
    let config = Config {
        keys: vec![decode_secret_key(HOST_KEY, None).expect("host key")],
        ..Default::default()
    };
    let sock = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
    let port = sock.local_addr().expect("addr").port();
    tokio::spawn(async move {
        let _ = Mock.run_on_socket(Arc::new(config), &sock).await;
    });
    port
}

/// Echo server on 127.0.0.1 that prefixes every reply with `tag`.
async fn echo(tag: &'static [u8]) -> u16 {
    let l = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = l.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 256];
                while let Ok(n) = s.read(&mut buf).await {
                    if n == 0 || s.write_all(&[tag, &buf[..n]].concat()).await.is_err() {
                        break;
                    }
                }
            });
        }
    });
    port
}

#[derive(Default)]
struct Hosts(Mutex<Vec<KnownKey>>);

impl KnownHosts for Hosts {
    fn lookup(&self, _: &str) -> Result<Vec<KnownKey>, String> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn add(&self, _: &str, k: &KnownKey) -> Result<(), String> {
        self.0.lock().unwrap().push(k.clone());
        Ok(())
    }
}

struct Ui;

impl Prompter for Ui {
    fn progress(&self, _: &str, _: ConnectStep) {}
    async fn confirm_unknown_host(&self, _: HostKeyInfo) -> HostKeyDecision {
        HostKeyDecision::AcceptOnce
    }
    async fn keyboard_interactive(&self, _: KbdInteractiveRequest) -> Option<Vec<SecretString>> {
        None
    }
    async fn passphrase(&self, _: &str) -> Option<SecretString> {
        None
    }
}

async fn conn() -> Arc<Connection> {
    let hop = Hop {
        label: "mock".into(),
        host: "127.0.0.1".into(),
        port: start().await,
        username: "tester".into(),
        auth: vec![AuthMethod::Password(SecretString::from("x".to_owned()))],
        keepalive_s: 0,
        legacy_algorithms: false,
    };
    Arc::new(
        connect(vec![hop], &Hosts::default(), &Ui)
            .await
            .expect("connect"),
    )
}

async fn roundtrip(port: u16, prefix: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut s = TcpStream::connect(("127.0.0.1", port)).await.expect("dial");
    s.write_all(prefix).await.unwrap();
    s.write_all(msg).await.unwrap();
    let mut out = Vec::new();
    let mut buf = [0u8; 256];
    while !out.ends_with(msg) {
        let n = tokio::time::timeout(T, s.read(&mut buf))
            .await
            .unwrap()
            .unwrap();
        assert!(n > 0, "closed early after {out:?}");
        out.extend_from_slice(&buf[..n]);
    }
    out
}

#[tokio::test]
async fn local_forward_carries_bytes_and_stops_on_drop() {
    let target = echo(b"L:").await;
    let c = conn().await;
    let fwd = c
        .start_forward(ForwardSpec::Local {
            bind_addr: "127.0.0.1".into(),
            bind_port: 0,
            dest_host: "127.0.0.1".into(),
            dest_port: target,
        })
        .await
        .expect("local forward");
    let port = fwd.bound_port();
    assert_ne!(port, 0);
    assert_eq!(roundtrip(port, b"", b"ping").await, b"L:ping");
    drop(fwd);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        TcpStream::connect(("127.0.0.1", port)).await.is_err(),
        "listener closed"
    );
}

#[tokio::test]
async fn local_forward_port_in_use_is_an_error() {
    let busy = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let c = conn().await;
    let r = c
        .start_forward(ForwardSpec::Local {
            bind_addr: "127.0.0.1".into(),
            bind_port: busy.local_addr().unwrap().port(),
            dest_host: "127.0.0.1".into(),
            dest_port: 1,
        })
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn dynamic_forward_speaks_socks5() {
    let target = echo(b"D:").await;
    let c = conn().await;
    let fwd = c
        .start_forward(ForwardSpec::Dynamic {
            bind_addr: "127.0.0.1".into(),
            bind_port: 0,
        })
        .await
        .expect("socks");
    let mut req = vec![5, 1, 0, 5, 1, 0, 3, 9];
    req.extend_from_slice(b"localhost");
    req.extend_from_slice(&target.to_be_bytes());
    let out = roundtrip(fwd.bound_port(), &req, b"hello").await;
    // greeting reply, CONNECT success, then the echoed payload
    assert_eq!(out[..4], [5, 0, 5, 0]);
    assert!(out.ends_with(b"D:hello"), "{out:?}");
}

#[tokio::test]
async fn remote_forward_reaches_local_target() {
    let target = echo(b"R:").await;
    let c = conn().await;
    let fwd = c
        .start_forward(ForwardSpec::Remote {
            bind_addr: "127.0.0.1".into(),
            bind_port: 0,
            dest_host: "127.0.0.1".into(),
            dest_port: target,
        })
        .await
        .expect("remote forward");
    // The mock "server" listens on this machine, so its port is reachable directly.
    assert_eq!(roundtrip(fwd.bound_port(), b"", b"pong").await, b"R:pong");
}
