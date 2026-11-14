#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    missing_docs
)]
//! Port forwards against the real OpenSSH bastion of `tests/docker` (the only server there
//! with `AllowTcpForwarding yes`). `#[ignore]`: run via `just e2e`.
//! Env: `TL_SSH_HOST`, `TL_SSH_USER`, `TL_SSH_KEY_FILE`, `TL_SSH_BASTION_PORT`,
//! `TL_SSH_INTERNAL_HOST`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ibc_shell_ssh::{
    connect, AuthMethod, ConnectStep, Connection, ForwardSpec, Hop, HostKeyDecision, HostKeyInfo,
    KbdInteractiveRequest, KnownHosts, KnownKey, Prompter,
};
use secrecy::SecretString;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const T: Duration = Duration::from_secs(10);

fn env(k: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| panic!("env {k} is required for this test"))
}

fn enabled() -> bool {
    std::env::var("TL_E2E").as_deref() == Ok("1")
}

#[derive(Default)]
struct Hosts(Mutex<Vec<KnownKey>>);

impl KnownHosts for Hosts {
    fn lookup(&self, _: &str) -> Result<Vec<KnownKey>, String> {
        Ok(Vec::new())
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

async fn bastion() -> Arc<Connection> {
    let hop = Hop {
        label: "bastion".into(),
        host: std::env::var("TL_SSH_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
        port: env("TL_SSH_BASTION_PORT").parse().unwrap(),
        username: env("TL_SSH_USER"),
        auth: vec![AuthMethod::Key {
            pem: zeroize::Zeroizing::new(std::fs::read_to_string(env("TL_SSH_KEY_FILE")).unwrap()),
            passphrase: None,
            identity_label: "e2e".into(),
        }],
        keepalive_s: 0,
        legacy_algorithms: false,
    };
    Arc::new(
        connect(vec![hop], &Hosts::default(), &Ui)
            .await
            .expect("connect"),
    )
}

/// Reads until the SSH identification line of the server behind the forward arrives.
async fn banner(mut s: TcpStream) -> String {
    let mut out = Vec::new();
    let mut buf = [0u8; 256];
    while !out.contains(&b'\n') {
        let n = tokio::time::timeout(T, s.read(&mut buf))
            .await
            .unwrap()
            .unwrap();
        assert!(n > 0, "closed before the banner");
        out.extend_from_slice(&buf[..n]);
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[tokio::test]
#[ignore = "needs docker (TL_E2E=1)"]
async fn local_forward_reaches_the_internal_server() {
    if !enabled() {
        return;
    }
    let c = bastion().await;
    let f = c
        .start_forward(ForwardSpec::Local {
            bind_addr: "127.0.0.1".into(),
            bind_port: 0,
            dest_host: env("TL_SSH_INTERNAL_HOST"),
            dest_port: 22,
        })
        .await
        .expect("local forward");
    let s = TcpStream::connect(("127.0.0.1", f.bound_port()))
        .await
        .unwrap();
    assert!(banner(s).await.starts_with("SSH-2.0-OpenSSH"));
}

#[tokio::test]
#[ignore = "needs docker (TL_E2E=1)"]
async fn socks_reaches_the_internal_server_by_name() {
    if !enabled() {
        return;
    }
    let c = bastion().await;
    let f = c
        .start_forward(ForwardSpec::Dynamic {
            bind_addr: "127.0.0.1".into(),
            bind_port: 0,
        })
        .await
        .expect("socks");
    let mut s = TcpStream::connect(("127.0.0.1", f.bound_port()))
        .await
        .unwrap();
    let name = env("TL_SSH_INTERNAL_HOST");
    let mut req = vec![5, 1, 0, 5, 1, 0, 3, u8::try_from(name.len()).unwrap()];
    req.extend_from_slice(name.as_bytes());
    req.extend_from_slice(&22u16.to_be_bytes());
    s.write_all(&req).await.unwrap();
    let mut head = [0u8; 12];
    tokio::time::timeout(T, s.read_exact(&mut head))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(head[..4], [5, 0, 5, 0], "greeting + CONNECT success");
    assert!(banner(s).await.starts_with("SSH-2.0-OpenSSH"));
}

#[tokio::test]
#[ignore = "needs docker (TL_E2E=1)"]
async fn remote_forward_brings_server_traffic_here() {
    if !enabled() {
        return;
    }
    // Local echo target the server should reach through the forward.
    let l = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let target = l.local_addr().unwrap().port();
    tokio::spawn(async move {
        if let Ok((mut s, _)) = l.accept().await {
            let mut buf = [0u8; 64];
            let n = s.read(&mut buf).await.unwrap_or(0);
            let _ = s.write_all(&[b"R:", &buf[..n]].concat()).await;
        }
    });
    let c = bastion().await;
    let f = c
        .start_forward(ForwardSpec::Remote {
            bind_addr: "127.0.0.1".into(),
            bind_port: 0,
            dest_host: "127.0.0.1".into(),
            dest_port: target,
        })
        .await
        .expect("remote forward");
    let port = f.bound_port();
    assert_ne!(port, 0, "server allocated a port");
    let cmd = format!(
        "bash -c 'exec 3<>/dev/tcp/127.0.0.1/{port}; printf ping >&3; timeout 5 head -c 6 <&3'"
    );
    let out = c.exec(&cmd, 64).await.expect("exec");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "R:ping");
    drop(f);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let after = c
        .exec(
            &format!("bash -c 'exec 3<>/dev/tcp/127.0.0.1/{port}' 2>/dev/null; echo $?"),
            64,
        )
        .await
        .expect("exec");
    assert_ne!(
        String::from_utf8_lossy(&after.stdout).trim(),
        "0",
        "cancelled on drop"
    );
}

/// The same `sh` script `identity_install` runs (src-tauri `commands/keys.rs`), against a
/// real server: installs once, then reports the key as present.
#[tokio::test]
#[ignore = "needs docker (TL_E2E=1)"]
async fn authorized_keys_append_is_idempotent() {
    if !enabled() {
        return;
    }
    let c = bastion().await;
    let body = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIE2eTestOnlyKeyBodyForE2E";
    let script = format!(
        "umask 077; mkdir -p ~/.ssh && touch ~/.ssh/authorized_keys && \
         chmod 700 ~/.ssh && chmod 600 ~/.ssh/authorized_keys || exit 3; \
         if grep -qF '{body}' ~/.ssh/authorized_keys; then echo IBC_KEY_ALREADY; else \
         if [ -s ~/.ssh/authorized_keys ] && [ \"$(tail -c1 ~/.ssh/authorized_keys)\" != \"\" ]; \
         then echo >> ~/.ssh/authorized_keys; fi; \
         printf '%s\\n' '{body} ibc-shell' >> ~/.ssh/authorized_keys && echo IBC_KEY_ADDED; fi"
    );
    let first = c.exec(&script, 256).await.expect("exec");
    let second = c.exec(&script, 256).await.expect("exec");
    assert!(String::from_utf8_lossy(&first.stdout).contains("IBC_KEY_ADDED"));
    assert!(String::from_utf8_lossy(&second.stdout).contains("IBC_KEY_ALREADY"));
    let count = c
        .exec(&format!("grep -cF '{body}' ~/.ssh/authorized_keys"), 64)
        .await
        .expect("exec");
    assert_eq!(String::from_utf8_lossy(&count.stdout).trim(), "1");
    // The test key from the fixtures must still be there: nothing was overwritten.
    let still = c
        .exec("wc -l < ~/.ssh/authorized_keys", 64)
        .await
        .expect("exec");
    assert!(
        String::from_utf8_lossy(&still.stdout)
            .trim()
            .parse::<u32>()
            .unwrap()
            >= 2
    );
}
