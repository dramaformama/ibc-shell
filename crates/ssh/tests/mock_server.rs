#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    clippy::too_many_arguments,
    missing_docs
)]
//! Connect + shell path against an in-process russh server (no Docker, no OpenSSH).
//! The host key below is a throwaway test fixture, never used anywhere else.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ibc_ssh::{
    connect, AuthMethod, ConnectStep, Hop, HostKeyDecision, HostKeyInfo, KbdInteractiveRequest,
    KnownHosts, KnownKey, Prompter, PtyRequest, ShellOutput, SshError,
};
use russh::keys::decode_secret_key;
use russh::server::{Auth, Config, Handler, Msg, Server, Session};
use russh::{Channel, ChannelId};
use secrecy::SecretString;
use tokio::net::TcpListener;

const HOST_KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACAjWF9XMGonNvOZS5jQtwN2+OyShDB1KB0m2qfUAEySdgAAAIhasfTVWrH0
1QAAAAtzc2gtZWQyNTUxOQAAACAjWF9XMGonNvOZS5jQtwN2+OyShDB1KB0m2qfUAEySdg
AAAEBpExXls+OkMCOV+0lvM7tv4lCconrTA8VEu/lb4/uEkyNYX1cwaic285lLmNC3A3b4
7JKEMHUoHSbap9QATJJ2AAAABHRlc3QB
-----END OPENSSH PRIVATE KEY-----
";
const USER: &str = "tester";
const PASSWORD: &str = "correct horse";

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

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        Ok(if user == USER && password == PASSWORD {
            Auth::Accept
        } else {
            Auth::reject()
        })
    }

    async fn channel_open_session(
        &mut self,
        _: Channel<Msg>,
        reply: russh::server::ChannelOpenHandle,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _: &str,
        _: u32,
        _: u32,
        _: u32,
        _: u32,
        _: &[(russh::Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        session.data(channel, b"ready\r\n".to_vec())?;
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.data(channel, [b"echo:".as_slice(), data].concat())?;
        if data.ends_with(b"exit\n") {
            session.exit_status_request(channel, 0)?;
            session.close(channel)?;
        }
        Ok(())
    }
}

async fn start() -> u16 {
    let config = Config {
        keys: vec![decode_secret_key(HOST_KEY, None).expect("host key")],
        auth_rejection_time: Duration::from_millis(1),
        auth_rejection_time_initial: Some(Duration::from_millis(0)),
        ..Default::default()
    };
    let sock = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
    let port = sock.local_addr().expect("addr").port();
    let mut srv = Mock;
    tokio::spawn(async move {
        let _ = srv.run_on_socket(Arc::new(config), &sock).await;
    });
    port
}

#[derive(Default)]
struct Hosts(Mutex<Vec<(String, KnownKey)>>);

impl KnownHosts for Hosts {
    fn lookup(&self, p: &str) -> Result<Vec<KnownKey>, String> {
        let g = self.0.lock().map_err(|e| e.to_string())?;
        Ok(g.iter()
            .filter(|(h, _)| h == p)
            .map(|(_, k)| k.clone())
            .collect())
    }
    fn add(&self, p: &str, k: &KnownKey) -> Result<(), String> {
        self.0
            .lock()
            .map_err(|e| e.to_string())?
            .push((p.to_owned(), k.clone()));
        Ok(())
    }
}

struct Ui(HostKeyDecision);

impl Prompter for Ui {
    fn progress(&self, _: &str, _: ConnectStep) {}
    async fn confirm_unknown_host(&self, _: HostKeyInfo) -> HostKeyDecision {
        self.0
    }
    async fn keyboard_interactive(&self, _: KbdInteractiveRequest) -> Option<Vec<SecretString>> {
        None
    }
    async fn passphrase(&self, _: &str) -> Option<SecretString> {
        None
    }
}

fn hop(port: u16, password: &str) -> Hop {
    Hop {
        label: "mock".into(),
        host: "127.0.0.1".into(),
        port,
        username: USER.into(),
        auth: vec![AuthMethod::Password(SecretString::from(
            password.to_owned(),
        ))],
        keepalive_s: 0,
        legacy_algorithms: false,
    }
}

#[tokio::test]
async fn connect_and_shell_roundtrip() {
    let port = start().await;
    let hosts = Hosts::default();
    let conn = connect(
        vec![hop(port, PASSWORD)],
        &hosts,
        &Ui(HostKeyDecision::AcceptAndRemember),
    )
    .await
    .expect("connect");
    assert_eq!(
        hosts.0.lock().unwrap().len(),
        1,
        "host key remembered (TOFU)"
    );
    let mut sh = conn
        .open_shell(PtyRequest {
            term: "xterm-256color".into(),
            cols: 80,
            rows: 24,
        })
        .await
        .expect("shell");
    sh.write(b"hi\nexit\n").await.expect("write");
    let mut text = Vec::new();
    let mut exit = None;
    while let Ok(Some(item)) = tokio::time::timeout(Duration::from_secs(10), sh.read()).await {
        match item {
            ShellOutput::Data(d) => {
                sh.ack(u32::try_from(d.len()).unwrap_or(u32::MAX));
                text.extend_from_slice(&d);
            }
            ShellOutput::ExitStatus(s) => exit = Some(s),
        }
    }
    let s = String::from_utf8_lossy(&text);
    assert!(s.contains("ready") && s.contains("echo:hi"), "{s}");
    assert_eq!(exit, Some(0));
}

#[tokio::test]
async fn wrong_password_is_auth_failed() {
    let port = start().await;
    let r = connect(
        vec![hop(port, "nope")],
        &Hosts::default(),
        &Ui(HostKeyDecision::AcceptAndRemember),
    )
    .await;
    assert!(matches!(r, Err(SshError::AuthFailed { .. })), "{r:?}");
}

#[tokio::test]
async fn rejected_unknown_host_key_cancels() {
    let port = start().await;
    let r = connect(
        vec![hop(port, PASSWORD)],
        &Hosts::default(),
        &Ui(HostKeyDecision::Reject),
    )
    .await;
    assert!(matches!(r, Err(SshError::Cancelled)), "{r:?}");
}

#[tokio::test]
async fn changed_host_key_is_a_hard_block() {
    let port = start().await;
    let hosts = Hosts::default();
    hosts
        .add(
            &format!("[127.0.0.1]:{port}"),
            &KnownKey {
                id: "old".into(),
                key_type: "ssh-ed25519".into(),
                key_blob: vec![1, 2, 3],
                fingerprint: "SHA256:old".into(),
            },
        )
        .expect("add");
    let r = connect(
        vec![hop(port, PASSWORD)],
        &hosts,
        &Ui(HostKeyDecision::AcceptAndRemember),
    )
    .await;
    assert!(matches!(r, Err(SshError::HostKeyChanged(_))), "{r:?}");
}
