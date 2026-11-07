//! User authentication: password, key (encrypted ok), keyboard-interactive, agent.

use russh::client::{Handle, KeyboardInteractiveAuthResponse};
use russh::keys::agent::client::AgentClient;
use russh::keys::{decode_secret_key, HashAlg, PrivateKeyWithHashAlg};
use russh::MethodSet;
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use zeroize::Zeroizing;

use crate::config::{AuthMethod, KbdInteractiveRequest, KbdPrompt, Prompter};
use crate::error::map_russh;
use crate::verify::ClientHandler;
use crate::SshError;

/// Max keyboard-interactive rounds per attempt.
const KBD_ROUNDS: usize = 10;
/// Max prompts the server may put into one keyboard-interactive round.
const KBD_MAX_PROMPTS: usize = 16;

/// Removes control characters (terminal escape injection) from server-provided prompt text.
fn strip_control(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}

/// Bounded, sanitized keyboard-interactive request; `None` if the server sent too many prompts.
fn build_kbd_request(
    hop_label: &str,
    name: &str,
    instructions: &str,
    prompts: Vec<(String, bool)>,
) -> Option<KbdInteractiveRequest> {
    if prompts.len() > KBD_MAX_PROMPTS {
        return None;
    }
    Some(KbdInteractiveRequest {
        hop_label: hop_label.to_owned(),
        name: strip_control(name),
        instruction: strip_control(instructions),
        prompts: prompts
            .into_iter()
            .map(|(text, echo)| KbdPrompt {
                text: strip_control(&text),
                echo,
            })
            .collect(),
    })
}

/// Tries `methods` in order until one succeeds. `remaining` is the server's current method set
/// (from the initial `none` probe); methods the server does not offer are skipped.
pub(crate) async fn authenticate<P: Prompter>(
    handle: &mut Handle<ClientHandler>,
    user: &str,
    methods: &[AuthMethod],
    mut remaining: MethodSet,
    hop_label: &str,
    prompter: &P,
) -> Result<(), SshError> {
    for m in methods {
        let name = match m {
            AuthMethod::Password(_) => "password",
            AuthMethod::Key { .. } | AuthMethod::Agent => "publickey",
            AuthMethod::KeyboardInteractive => "keyboard-interactive",
        };
        if !offers(&remaining, name) {
            continue;
        }
        let res = match m {
            AuthMethod::Password(p) => try_password(handle, user, p).await?,
            AuthMethod::Key {
                pem,
                passphrase,
                identity_label,
            } => {
                try_key(
                    handle,
                    user,
                    pem,
                    passphrase.as_ref(),
                    identity_label,
                    prompter,
                )
                .await?
            }
            AuthMethod::Agent => try_agent(handle, user).await?,
            AuthMethod::KeyboardInteractive => try_kbd(handle, user, hop_label, prompter).await?,
        };
        match res {
            Outcome::Success => return Ok(()),
            Outcome::Failed(Some(left)) => remaining = left,
            Outcome::Failed(None) => {}
        }
    }
    Err(SshError::AuthFailed {
        methods_left: remaining
            .iter()
            .map(|k| <&'static str>::from(k).to_owned())
            .collect(),
    })
}

fn offers(set: &MethodSet, name: &str) -> bool {
    set.iter().any(|k| <&'static str>::from(k) == name)
}

enum Outcome {
    Success,
    /// Failed; the server's updated method list when it sent one.
    Failed(Option<MethodSet>),
}

fn from_result(r: russh::client::AuthResult) -> Outcome {
    match r {
        russh::client::AuthResult::Success => Outcome::Success,
        russh::client::AuthResult::Failure {
            remaining_methods, ..
        } => Outcome::Failed(Some(remaining_methods)),
    }
}

async fn try_password(
    handle: &mut Handle<ClientHandler>,
    user: &str,
    password: &SecretString,
) -> Result<Outcome, SshError> {
    let r = handle
        .authenticate_password(user, password.expose_secret())
        .await
        .map_err(|e| map_russh(&e))?;
    Ok(from_result(r))
}

async fn rsa_hash(handle: &Handle<ClientHandler>) -> Option<HashAlg> {
    // `Ok(Some(None))` means the server only speaks SHA-1 ssh-rsa: we refuse that.
    handle
        .best_supported_rsa_hash()
        .await
        .ok()
        .flatten()
        .flatten()
}

async fn try_key<P: Prompter>(
    handle: &mut Handle<ClientHandler>,
    user: &str,
    pem: &str,
    passphrase: Option<&SecretString>,
    label: &str,
    prompter: &P,
) -> Result<Outcome, SshError> {
    let key = match decode_secret_key(pem, None) {
        Ok(k) => k,
        Err(russh::keys::Error::KeyIsEncrypted) => {
            // Encrypted: stored passphrase first, then ask.
            let pass = match passphrase {
                Some(p) => p.clone(),
                None => prompter
                    .passphrase(label)
                    .await
                    .ok_or(SshError::Cancelled)?,
            };
            match decode_secret_key(pem, Some(pass.expose_secret())) {
                Ok(k) => k,
                // Wrong passphrase / unsupported key: skip this identity.
                Err(_) => return Ok(Outcome::Failed(None)),
            }
        }
        // Unreadable / unsupported / corrupt: never ask for a passphrase.
        Err(_) => return Ok(Outcome::Failed(None)),
    };
    let hash = if key.algorithm().is_rsa() {
        match rsa_hash(handle).await {
            Some(h) => Some(h),
            None => return Ok(Outcome::Failed(None)),
        }
    } else {
        None
    };
    let r = handle
        .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
        .await
        .map_err(|e| map_russh(&e))?;
    Ok(from_result(r))
}

#[cfg(unix)]
async fn connect_agent(
) -> Option<AgentClient<Box<dyn russh::keys::agent::client::AgentStream + Send + Unpin>>> {
    AgentClient::connect_env()
        .await
        .ok()
        .map(AgentClient::dynamic)
}

#[cfg(windows)]
async fn connect_agent(
) -> Option<AgentClient<Box<dyn russh::keys::agent::client::AgentStream + Send + Unpin>>> {
    if let Ok(c) = AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent").await {
        return Some(c.dynamic());
    }
    AgentClient::connect_pageant()
        .await
        .ok()
        .map(AgentClient::dynamic)
}

#[cfg(not(any(unix, windows)))]
async fn connect_agent(
) -> Option<AgentClient<Box<dyn russh::keys::agent::client::AgentStream + Send + Unpin>>> {
    None
}

async fn try_agent(handle: &mut Handle<ClientHandler>, user: &str) -> Result<Outcome, SshError> {
    let Some(mut agent) = connect_agent().await else {
        return Ok(Outcome::Failed(None));
    };
    let Ok(ids) = agent.request_identities().await else {
        return Ok(Outcome::Failed(None));
    };
    let mut last = None;
    for id in ids {
        let pk = id.public_key().into_owned();
        let hash = if pk.algorithm().is_rsa() {
            match rsa_hash(handle).await {
                Some(h) => Some(h),
                None => continue,
            }
        } else {
            None
        };
        // Signer errors (agent refused) just move on to the next identity.
        if let Ok(r) = handle
            .authenticate_publickey_with(user, pk, hash, &mut agent)
            .await
        {
            match from_result(r) {
                Outcome::Success => return Ok(Outcome::Success),
                Outcome::Failed(l) => last = l.or(last),
            }
        }
    }
    Ok(Outcome::Failed(last))
}

async fn try_kbd<P: Prompter>(
    handle: &mut Handle<ClientHandler>,
    user: &str,
    hop_label: &str,
    prompter: &P,
) -> Result<Outcome, SshError> {
    let mut resp = handle
        .authenticate_keyboard_interactive_start(user, None)
        .await
        .map_err(|e| map_russh(&e))?;
    for _ in 0..KBD_ROUNDS {
        match resp {
            KeyboardInteractiveAuthResponse::Success => return Ok(Outcome::Success),
            KeyboardInteractiveAuthResponse::Failure {
                remaining_methods, ..
            } => return Ok(Outcome::Failed(Some(remaining_methods))),
            KeyboardInteractiveAuthResponse::InfoRequest {
                name,
                instructions,
                prompts,
            } => {
                let answers: Vec<String> = if prompts.is_empty() {
                    Vec::new()
                } else {
                    let req = build_kbd_request(
                        hop_label,
                        &name,
                        &instructions,
                        prompts.into_iter().map(|p| (p.prompt, p.echo)).collect(),
                    )
                    .ok_or_else(|| SshError::Protocol("too many prompts".into()))?;
                    let given = prompter
                        .keyboard_interactive(req)
                        .await
                        .ok_or(SshError::Cancelled)?;
                    let mut wrapped: Vec<Zeroizing<String>> = given
                        .iter()
                        .map(|s| Zeroizing::new(s.expose_secret().to_owned()))
                        .collect();
                    wrapped
                        .iter_mut()
                        .map(|z| std::mem::take(&mut **z))
                        .collect()
                };
                resp = handle
                    .authenticate_keyboard_interactive_respond(answers)
                    .await
                    .map_err(|e| map_russh(&e))?;
            }
        }
    }
    Ok(Outcome::Failed(None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_control_chars() {
        assert_eq!(strip_control("Pass\x1b[2Jword:\r\n\u{7}"), "Pass[2Jword:");
    }

    #[test]
    fn caps_prompt_count_and_sanitizes() {
        let many: Vec<(String, bool)> = (0..17).map(|i| (format!("p{i}"), false)).collect();
        assert!(build_kbd_request("h", "n", "i", many).is_none());
        let ok: Vec<(String, bool)> = (0..16).map(|i| (format!("p{i}\x1b"), true)).collect();
        let r = build_kbd_request("h", "na\x07me", "in\nstr", ok).unwrap_or_else(|| unreachable!());
        assert_eq!(r.prompts.len(), 16);
        assert_eq!(r.prompts[0].text, "p0");
        assert_eq!(r.name, "name");
        assert_eq!(r.instruction, "instr");
    }

    #[test]
    fn garbage_key_is_not_reported_as_encrypted() {
        assert!(!matches!(
            decode_secret_key("not a key", None),
            Err(russh::keys::Error::KeyIsEncrypted)
        ));
    }
}
