#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    missing_docs
)]
//! Integration tests against `tests/docker` OpenSSH servers. All `#[ignore]`: run with
//! `TL_E2E=1 cargo test -p ibc-shell-ssh -- --ignored`.
//!
//! Env: `TL_SSH_HOST` (default 127.0.0.1), `TL_SSH_PORT` (password server), `TL_SSH_USER`,
//! `TL_SSH_PASSWORD`, `TL_SSH_KEY_PORT` + `TL_SSH_KEY_FILE` (key-only server),
//! `TL_SSH_KBD_PORT`, `TL_SSH_BASTION_PORT` + `TL_SSH_INTERNAL_HOST`.

use std::sync::Mutex;

use ibc_shell_ssh::{
    connect, AuthMethod, ConnectStep, Hop, HostKeyDecision, HostKeyInfo, KbdInteractiveRequest,
    KnownHosts, KnownKey, Prompter, PtyRequest, ShellOutput, SshError,
};
use secrecy::SecretString;

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok()
}

fn need(k: &str) -> String {
    env(k).unwrap_or_else(|| panic!("env {k} is required for this test"))
}

fn enabled() -> bool {
    env("TL_E2E").as_deref() == Some("1")
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
        let mut g = self.0.lock().map_err(|e| e.to_string())?;
        let mut k = k.clone();
        k.id = format!("id{}", g.len());
        g.push((p.to_owned(), k));
        Ok(())
    }
}

struct Ui {
    decision: HostKeyDecision,
    kbd_answer: Option<String>,
}

impl Prompter for Ui {
    fn progress(&self, _: &str, _: ConnectStep) {}
    async fn confirm_unknown_host(&self, _: HostKeyInfo) -> HostKeyDecision {
        self.decision
    }
    async fn keyboard_interactive(&self, r: KbdInteractiveRequest) -> Option<Vec<SecretString>> {
        let a = self.kbd_answer.clone()?;
        Some(
            r.prompts
                .iter()
                .map(|_| SecretString::from(a.clone()))
                .collect(),
        )
    }
    async fn passphrase(&self, _: &str) -> Option<SecretString> {
        None
    }
}

fn accept() -> Ui {
    Ui {
        decision: HostKeyDecision::AcceptAndRemember,
        kbd_answer: env("TL_SSH_PASSWORD"),
    }
}

fn hop(port: &str, host: &str, auth: Vec<AuthMethod>) -> Hop {
    Hop {
        label: host.to_owned(),
        host: host.to_owned(),
        port: port.parse().expect("port"),
        username: need("TL_SSH_USER"),
        auth,
        keepalive_s: 5,
        legacy_algorithms: false,
    }
}

fn host() -> String {
    env("TL_SSH_HOST").unwrap_or_else(|| "127.0.0.1".into())
}

fn pw(p: &str) -> AuthMethod {
    AuthMethod::Password(SecretString::from(p.to_owned()))
}

#[tokio::test]
#[ignore = "needs docker (TL_E2E=1)"]
async fn password_auth_exec() {
    if !enabled() {
        return;
    }
    let hosts = Hosts::default();
    let c = connect(
        vec![hop(
            &need("TL_SSH_PORT"),
            &host(),
            vec![pw(&need("TL_SSH_PASSWORD"))],
        )],
        &hosts,
        &accept(),
    )
    .await
    .expect("connect");
    let out = c.exec("echo hello", 1024).await.expect("exec");
    assert_eq!(out.stdout, b"hello\n");
    assert_eq!(out.exit_status, Some(0));
}

#[tokio::test]
#[ignore = "needs docker (TL_E2E=1)"]
async fn wrong_password_fails() {
    if !enabled() {
        return;
    }
    let r = connect(
        vec![hop(
            &need("TL_SSH_PORT"),
            &host(),
            vec![pw("definitely-wrong")],
        )],
        &Hosts::default(),
        &accept(),
    )
    .await;
    assert!(matches!(r, Err(SshError::AuthFailed { .. })), "{r:?}");
}

#[tokio::test]
#[ignore = "needs docker (TL_E2E=1)"]
async fn unknown_key_rejected_is_cancelled() {
    if !enabled() {
        return;
    }
    let ui = Ui {
        decision: HostKeyDecision::Reject,
        kbd_answer: None,
    };
    let r = connect(
        vec![hop(
            &need("TL_SSH_PORT"),
            &host(),
            vec![pw(&need("TL_SSH_PASSWORD"))],
        )],
        &Hosts::default(),
        &ui,
    )
    .await;
    assert!(matches!(r, Err(SshError::Cancelled)), "{r:?}");
}

#[tokio::test]
#[ignore = "needs docker (TL_E2E=1)"]
async fn changed_host_key_blocks() {
    if !enabled() {
        return;
    }
    let hosts = Hosts::default();
    let port = need("TL_SSH_PORT");
    let h = host();
    let pat = if port == "22" {
        h.clone()
    } else {
        format!("[{h}]:{port}")
    };
    hosts
        .add(
            &pat,
            &KnownKey {
                id: "old".into(),
                key_type: "ssh-ed25519".into(),
                key_blob: vec![1, 2, 3],
                fingerprint: "SHA256:old".into(),
            },
        )
        .expect("add");
    let r = connect(
        vec![hop(&port, &h, vec![pw(&need("TL_SSH_PASSWORD"))])],
        &hosts,
        &accept(),
    )
    .await;
    assert!(matches!(r, Err(SshError::HostKeyChanged(_))), "{r:?}");
}

#[tokio::test]
#[ignore = "needs docker (TL_E2E=1)"]
async fn key_auth() {
    if !enabled() {
        return;
    }
    let pem = std::fs::read_to_string(need("TL_SSH_KEY_FILE")).expect("key file");
    let m = AuthMethod::Key {
        pem: zeroize::Zeroizing::new(pem),
        passphrase: None,
        identity_label: "test".into(),
    };
    let c = connect(
        vec![hop(&need("TL_SSH_KEY_PORT"), &host(), vec![m])],
        &Hosts::default(),
        &accept(),
    )
    .await
    .expect("connect");
    assert_eq!(c.exec("echo ok", 64).await.expect("exec").stdout, b"ok\n");
}

#[tokio::test]
#[ignore = "needs docker (TL_E2E=1)"]
async fn keyboard_interactive_auth() {
    if !enabled() {
        return;
    }
    let c = connect(
        vec![hop(
            &need("TL_SSH_KBD_PORT"),
            &host(),
            vec![AuthMethod::KeyboardInteractive],
        )],
        &Hosts::default(),
        &accept(),
    )
    .await
    .expect("connect");
    assert_eq!(
        c.exec("echo ok", 64).await.expect("exec").exit_status,
        Some(0)
    );
}

#[tokio::test]
#[ignore = "needs docker (TL_E2E=1)"]
async fn jump_through_bastion() {
    if !enabled() {
        return;
    }
    let key = |label: &str| AuthMethod::Key {
        pem: zeroize::Zeroizing::new(
            std::fs::read_to_string(need("TL_SSH_KEY_FILE")).expect("key file"),
        ),
        passphrase: None,
        identity_label: label.into(),
    };
    let chain = vec![
        hop(&need("TL_SSH_BASTION_PORT"), &host(), vec![key("bastion")]),
        hop("22", &need("TL_SSH_INTERNAL_HOST"), vec![key("internal")]),
    ];
    let c = connect(chain, &Hosts::default(), &accept())
        .await
        .expect("connect");
    assert_eq!(
        c.exec("echo inner", 64).await.expect("exec").stdout,
        b"inner\n"
    );
}

#[tokio::test]
#[ignore = "needs docker (TL_E2E=1)"]
async fn shell_resize_and_throughput() {
    if !enabled() {
        return;
    }
    let c = connect(
        vec![hop(
            &need("TL_SSH_PORT"),
            &host(),
            vec![pw(&need("TL_SSH_PASSWORD"))],
        )],
        &Hosts::default(),
        &accept(),
    )
    .await
    .expect("connect");
    let mut sh = c
        .open_shell(PtyRequest {
            term: "xterm-256color".into(),
            cols: 80,
            rows: 24,
        })
        .await
        .expect("shell");
    sh.resize(120, 40).await.expect("resize");
    sh.write(b"stty size; head -c 50000000 /dev/zero | wc -c; exit\n")
        .await
        .expect("write");
    let mut text = Vec::new();
    while let Some(item) = sh.read().await {
        if let ShellOutput::Data(d) = item {
            sh.ack(u32::try_from(d.len()).unwrap_or(u32::MAX));
            text.extend_from_slice(&d);
        }
    }
    let s = String::from_utf8_lossy(&text);
    assert!(s.contains("40 120"), "resize not applied: {s}");
    assert!(s.contains("50000000"));
}
