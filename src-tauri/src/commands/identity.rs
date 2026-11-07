//! Identity commands. Secrets are accepted once, sealed into the vault, never returned.

use ibc_store::{Identity, IdentityId, IdentityKind, IdentityMetaInput, IdentityMetaPatch, SecretId};
use ibc_vault::keys::{import_private_key, KeyError, KeyInfo};
use ibc_vault::Vault;
use secrecy::{ExposeSecret, SecretString};
use tauri::State;

use crate::api::dto::{IdentityDto, IdentityInput, IdentityPatchInput, SecretInput};
use crate::api::{ApiError, ApiResult};
use crate::state::AppState;

fn dto(i: Identity) -> IdentityDto {
    IdentityDto {
        id: i.id,
        label: i.label,
        username: i.username,
        kind: i.kind,
        key_algo: i.key_algo,
        key_fingerprint: i.key_fingerprint,
        public_key: i.public_key,
        has_password: i.password_secret_id.is_some(),
        has_passphrase: i.passphrase_secret_id.is_some(),
        created_at: i.created_at,
        updated_at: i.updated_at,
    }
}

fn key_err(e: &KeyError) -> ApiError {
    ApiError::Validation(e.to_string())
}

/// Secret rows created during one command; deleted again when the command fails.
struct Created<'a>(&'a Vault, Vec<SecretId>);

impl Created<'_> {
    fn put(&mut self, s: &SecretString) -> ApiResult<SecretId> {
        let id = self.0.put_secret(s.expose_secret().as_bytes())?;
        self.1.push(id);
        Ok(id)
    }
    fn commit(mut self) {
        self.1.clear();
    }
}

impl Drop for Created<'_> {
    fn drop(&mut self) {
        for id in self.1.drain(..) {
            let _ = self.0.delete_secret(id);
        }
    }
}

fn create(
    state: &AppState,
    input: IdentityInput,
) -> ApiResult<IdentityDto> {
    let mut made = Created(&state.vault, Vec::new());
    let mut info: Option<KeyInfo> = None;
    let mut key_id = None;
    if let Some(pem) = &input.private_key_pem {
        info = Some(
            import_private_key(pem.expose_secret().as_bytes(), input.passphrase.as_ref())
                .map_err(|e| key_err(&e))?,
        );
        key_id = Some(made.put(pem)?);
    }
    let password_secret_id = input.password.as_ref().map(|p| made.put(p)).transpose()?;
    let passphrase_secret_id = input.passphrase.as_ref().map(|p| made.put(p)).transpose()?;
    let ident = state.store.identity_create(IdentityMetaInput {
        label: input.label,
        username: input.username,
        kind: input.kind,
        key_algo: info.as_ref().map(|i| i.algo.clone()),
        key_fingerprint: info.as_ref().map(|i| i.fingerprint.clone()),
        public_key: info.map(|i| i.public_key),
        password_secret_id,
        key_secret_id: key_id,
        passphrase_secret_id,
    })?;
    made.commit();
    Ok(dto(ident))
}

/// All identities (metadata only).
#[tauri::command]
#[specta::specta]
pub async fn identity_list(state: State<'_, AppState>) -> ApiResult<Vec<IdentityDto>> {
    Ok(state.store.identity_list()?.into_iter().map(dto).collect())
}

/// Creates an identity; secrets go straight into the vault.
#[tauri::command]
#[specta::specta]
pub async fn identity_create(state: State<'_, AppState>, input: IdentityInput) -> ApiResult<IdentityDto> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || create(&state, input))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
}

/// Updates an identity; replaced secret rows are removed by the store in the same transaction.
#[tauri::command]
#[specta::specta]
pub async fn identity_update(
    state: State<'_, AppState>,
    id: IdentityId,
    patch: IdentityPatchInput,
) -> ApiResult<IdentityDto> {
    let mut made = Created(&state.vault, Vec::new());
    let mut p = IdentityMetaPatch {
        label: patch.label,
        username: if patch.clear_username { Some(None) } else { patch.username.map(Some) },
        ..Default::default()
    };
    if let Some(pem) = &patch.private_key_pem {
        let info = import_private_key(pem.expose_secret().as_bytes(), patch.passphrase.as_ref())
            .map_err(|e| key_err(&e))?;
        p.key_algo = Some(Some(info.algo));
        p.key_fingerprint = Some(Some(info.fingerprint));
        p.public_key = Some(Some(info.public_key));
        p.key_secret_id = Some(Some(made.put(pem)?));
    }
    if let Some(pw) = &patch.password {
        p.password_secret_id = Some(Some(made.put(pw)?));
    } else if patch.clear_password {
        p.password_secret_id = Some(None);
    }
    if let Some(pp) = &patch.passphrase {
        p.passphrase_secret_id = Some(Some(made.put(pp)?));
    } else if patch.clear_passphrase {
        p.passphrase_secret_id = Some(None);
    }
    let ident = state.store.identity_update(id, p)?;
    made.commit();
    Ok(dto(ident))
}

/// Deletes an identity and its secrets.
#[tauri::command]
#[specta::specta]
pub async fn identity_delete(state: State<'_, AppState>, id: IdentityId) -> ApiResult<()> {
    Ok(state.store.identity_delete(id)?)
}

/// Imports a private key file chosen via the dialog plugin.
#[tauri::command]
#[specta::specta]
pub async fn identity_import_file(
    state: State<'_, AppState>,
    path: String,
    passphrase: Option<SecretInput>,
) -> ApiResult<IdentityDto> {
    let p = std::path::PathBuf::from(&path);
    let bytes = zeroize::Zeroizing::new(
        std::fs::read(&p).map_err(|_| ApiError::Validation("cannot read key file".into()))?,
    );
    if bytes.len() > 64 * 1024 {
        return Err(ApiError::Validation("key file too large".into()));
    }
    let pem = String::from_utf8(bytes.to_vec())
        .map_err(|_| ApiError::Validation("invalid key".into()))?;
    let label = p
        .file_name()
        .map_or_else(|| "key".to_owned(), |n| n.to_string_lossy().into_owned());
    let passphrase = passphrase.map(|s| s.0);
    let input = IdentityInput {
        label,
        username: None,
        kind: if passphrase.is_some() { IdentityKind::KeyPassword } else { IdentityKind::Key },
        password: None,
        private_key_pem: Some(SecretString::from(pem)),
        passphrase,
    };
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || create(&state, input))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
}
