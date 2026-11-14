//! Key generation and installing a public key on a server (ADR-024).

use std::sync::PoisonError;

use ibc_shell_store::{IdentityId, IdentityKind};
use ibc_shell_vault::keys::{generate_key, KeyGenAlgo};
use secrecy::SecretString;
use serde::Deserialize;
use tauri::State;

use crate::api::dto::{IdentityDto, IdentityInput, SessionId};
use crate::api::{ApiError, ApiResult};
use crate::commands::session::session_of;
use crate::state::AppState;

/// Markers the install script prints.
const ADDED: &str = "IBC_KEY_ADDED";
const ALREADY: &str = "IBC_KEY_ALREADY";

/// Algorithm for `identity_generate`.
#[derive(Clone, Copy, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum KeyAlgo {
    /// Ed25519 (default).
    Ed25519,
    /// RSA 4096, for old servers.
    Rsa4096,
}

/// Generates a key pair with the OS RNG and stores the private half in the vault.
#[tauri::command]
#[specta::specta]
pub async fn identity_generate(
    state: State<'_, AppState>,
    label: String,
    algo: KeyAlgo,
) -> ApiResult<IdentityDto> {
    let label = label.trim().to_owned();
    if label.is_empty() {
        return Err(ApiError::Validation("label is required".into()));
    }
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {
        let key = generate_key(match algo {
            KeyAlgo::Ed25519 => KeyGenAlgo::Ed25519,
            KeyAlgo::Rsa4096 => KeyGenAlgo::Rsa4096,
        })
        .map_err(|_| ApiError::Internal("key generation failed".into()))?;
        let pem = std::str::from_utf8(&key.private_openssh)
            .map_err(|_| ApiError::Internal("key generation failed".into()))?
            .to_owned();
        crate::commands::identity::create(
            &state,
            IdentityInput {
                label,
                username: None,
                kind: IdentityKind::Key,
                password: None,
                private_key_pem: Some(SecretString::from(pem)),
                passphrase: None,
            },
        )
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
}

/// `algo base64` of an OpenSSH public key line, when it looks like one (no shell metacharacters).
fn key_body(public: &str) -> Option<String> {
    let mut parts = public.split_whitespace();
    let algo = parts.next()?;
    let blob = parts.next()?;
    let algo_ok = algo
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-@.".contains(c));
    let blob_ok = blob
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c));
    (algo_ok && blob_ok && blob.len() >= 16).then(|| format!("{algo} {blob}"))
}

/// Shell script that appends `body` to `~/.ssh/authorized_keys` unless it is already there.
/// `body` is validated by [`key_body`]: only `[A-Za-z0-9+/=@.-]` and one space.
fn install_script(body: &str) -> String {
    let line = format!("{body} ibc-shell");
    format!(
        "umask 077; mkdir -p ~/.ssh && touch ~/.ssh/authorized_keys && \
         chmod 700 ~/.ssh && chmod 600 ~/.ssh/authorized_keys || exit 3; \
         if grep -qF '{body}' ~/.ssh/authorized_keys; then echo {ALREADY}; else \
         if [ -s ~/.ssh/authorized_keys ] && [ \"$(tail -c1 ~/.ssh/authorized_keys)\" != \"\" ]; \
         then echo >> ~/.ssh/authorized_keys; fi; \
         printf '%s\\n' '{line}' >> ~/.ssh/authorized_keys && echo {ADDED}; fi"
    )
}

/// Adds a key identity's public key to the server's `~/.ssh/authorized_keys` through the
/// session's connection. `true` = added, `false` = it was already there.
#[tauri::command]
#[specta::specta]
pub async fn identity_install(
    state: State<'_, AppState>,
    session_id: SessionId,
    identity_id: IdentityId,
) -> ApiResult<bool> {
    let handle = session_of(&state, session_id)?;
    let conn = handle
        .live
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .conn
        .clone()
        .ok_or_else(|| ApiError::Validation("session is not connected".into()))?;
    let ident = state.store.identity_get(identity_id)?;
    let body = ident
        .public_key
        .as_deref()
        .and_then(key_body)
        .ok_or_else(|| ApiError::Validation("identity has no public key".into()))?;
    let out = conn.exec(&install_script(&body), 4096).await?;
    let text = String::from_utf8_lossy(&out.stdout);
    if text.contains(ADDED) {
        Ok(true)
    } else if text.contains(ALREADY) {
        Ok(false)
    } else {
        Err(ApiError::Validation(
            "could not write ~/.ssh/authorized_keys on the server".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_body_accepts_openssh_lines_only() {
        let k = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICNYX1cwaic285lLmNC3A3b4 user@host";
        assert_eq!(
            key_body(k).as_deref(),
            Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICNYX1cwaic285lLmNC3A3b4")
        );
        assert!(key_body("ssh-ed25519 AAAA'; rm -rf ~").is_none());
        assert!(key_body("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5$(id)xxxxxxxx").is_none());
        assert!(key_body("ssh-ed25519").is_none());
    }

    #[test]
    fn script_embeds_only_the_validated_body() {
        let s = install_script("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIC");
        assert!(s.contains("grep -qF 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIC'"));
        assert!(s.contains("'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIC ibc-shell'"));
    }

    /// Runs the real script in a temporary HOME with `sh`: adds once, then reports "already".
    #[cfg(unix)]
    #[test]
    fn script_is_idempotent_under_sh() {
        let home = tempfile::tempdir().unwrap_or_else(|_| std::process::abort());
        let ssh = home.path().join(".ssh");
        std::fs::create_dir_all(&ssh).unwrap_or_else(|_| std::process::abort());
        std::fs::write(ssh.join("authorized_keys"), "ssh-rsa AAAAold no-newline")
            .unwrap_or_else(|_| std::process::abort());
        let body = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIC";
        let run = || {
            let out = std::process::Command::new("sh")
                .arg("-c")
                .arg(install_script(body))
                .env("HOME", home.path())
                .output()
                .unwrap_or_else(|_| std::process::abort());
            String::from_utf8_lossy(&out.stdout).into_owned()
        };
        assert!(run().contains(ADDED));
        assert!(run().contains(ALREADY));
        let file = std::fs::read_to_string(ssh.join("authorized_keys"))
            .unwrap_or_else(|_| std::process::abort());
        assert_eq!(
            file,
            "ssh-rsa AAAAold no-newline\nssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIC ibc-shell\n"
        );
    }
}
