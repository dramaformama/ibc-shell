//! Host → `Hop` chain resolution (store + vault) and the store-backed `KnownHosts`.

use std::sync::Arc;

use ibc_shell_ssh::{AuthMethod, Hop, KnownHosts, KnownKey};
use ibc_shell_store::{HostId, IdentityKind, KnownHostInput, ResolvedHost, SecretId, Store};
use ibc_shell_vault::{Vault, VaultState};
use secrecy::SecretString;
use zeroize::Zeroizing;

use crate::api::dto::QuickConnect;
use crate::api::{ApiError, ApiResult};
use crate::state::SessionTarget;

/// `KnownHosts` over the SQLite store.
pub struct StoreKnownHosts(pub Arc<Store>);

impl KnownHosts for StoreKnownHosts {
    fn lookup(&self, host_pattern: &str) -> Result<Vec<KnownKey>, String> {
        let rows = self
            .0
            .known_host_lookup(host_pattern)
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|r| KnownKey {
                id: r.id.0.to_string(),
                key_type: r.key_type,
                key_blob: r.key_blob,
                fingerprint: r.fingerprint,
            })
            .collect())
    }

    /// Never replaces a stored key (a concurrent prompt may have stored another one first).
    fn add(&self, host_pattern: &str, key: &KnownKey) -> Result<(), String> {
        self.0
            .known_host_add_new(KnownHostInput {
                host_pattern: host_pattern.to_owned(),
                key_type: key.key_type.clone(),
                key_blob: key.key_blob.clone(),
                fingerprint: key.fingerprint.clone(),
            })
            .map_err(|e| e.to_string())
    }
}

fn secret_text(vault: &Vault, id: SecretId) -> ApiResult<Zeroizing<String>> {
    vault
        .with_secret(id, |b| {
            std::str::from_utf8(b).map(|s| Zeroizing::new(s.to_owned()))
        })?
        .map_err(|_| ApiError::Internal("stored secret is not valid text".into()))
}

fn auth_methods(store: &Store, vault: &Vault, rh: &ResolvedHost) -> ApiResult<Vec<AuthMethod>> {
    let Some(identity_id) = rh.identity_id.value else {
        return Ok(vec![AuthMethod::Agent, AuthMethod::KeyboardInteractive]);
    };
    let ident = store.identity_get(identity_id)?;
    let mut out = Vec::new();
    match ident.kind {
        IdentityKind::Agent => out.push(AuthMethod::Agent),
        IdentityKind::Password => {}
        IdentityKind::Key | IdentityKind::KeyPassword => {
            if let Some(kid) = ident.key_secret_id {
                let pem = secret_text(vault, kid)?;
                let passphrase = match ident.passphrase_secret_id {
                    Some(pid) => Some(SecretString::from(
                        secret_text(vault, pid)?.as_str().to_owned(),
                    )),
                    None => None,
                };
                out.push(AuthMethod::Key {
                    pem,
                    passphrase,
                    identity_label: ident.label.clone(),
                });
            }
        }
    }
    if let Some(pid) = ident.password_secret_id {
        out.push(AuthMethod::Password(SecretString::from(
            secret_text(vault, pid)?.as_str().to_owned(),
        )));
    }
    out.push(AuthMethod::KeyboardInteractive);
    Ok(out)
}

fn hop_for(store: &Store, vault: &Vault, id: HostId) -> ApiResult<Hop> {
    let rh = store.host_resolve(id)?;
    let username = rh
        .username
        .value
        .clone()
        .ok_or_else(|| ApiError::Validation("username required".into()))?;
    Ok(Hop {
        label: rh.label.clone(),
        host: rh.hostname.clone(),
        port: rh.port.value,
        username,
        auth: auth_methods(store, vault, &rh)?,
        keepalive_s: rh.keepalive_s.value,
        legacy_algorithms: false,
    })
}

fn adhoc_hop(q: &QuickConnect) -> ApiResult<Hop> {
    let username = q
        .user
        .clone()
        .filter(|u| !u.is_empty())
        .ok_or_else(|| ApiError::Validation("username required".into()))?;
    if q.hostname.trim().is_empty() {
        return Err(ApiError::Validation("hostname required".into()));
    }
    Ok(Hop {
        label: q.hostname.clone(),
        host: q.hostname.clone(),
        port: q.port.unwrap_or(22),
        username,
        auth: vec![AuthMethod::Agent, AuthMethod::KeyboardInteractive],
        keepalive_s: 30,
        legacy_algorithms: false,
    })
}

/// Requires an unlocked vault, then resolves `target` into the hop chain (jump hosts first).
pub fn build_chain(store: &Store, vault: &Vault, target: &SessionTarget) -> ApiResult<Vec<Hop>> {
    if vault.state()? != VaultState::Unlocked {
        return Err(ApiError::VaultLocked);
    }
    match target {
        SessionTarget::Adhoc(q) => Ok(vec![adhoc_hop(q)?]),
        SessionTarget::Host(id) => {
            let rh = store.host_resolve(*id)?;
            let mut chain = Vec::with_capacity(rh.jump_chain.len() + 1);
            for j in rh.jump_chain.iter().chain(std::iter::once(id)) {
                chain.push(hop_for(store, vault, *j)?);
            }
            Ok(chain)
        }
    }
}
