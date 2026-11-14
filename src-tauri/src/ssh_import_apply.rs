//! `ssh_config_preview` / `ssh_config_import` commands (ADR-023). The import re-reads the
//! config itself; the UI only picks aliases.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ibc_shell_ssh::{ConfigEntry, ForwardSpec};
use ibc_shell_store::{
    FolderId, FolderInput, ForwardInput, ForwardKind, HostId, HostInput, HostPatch, IdentityId,
    IdentityKind,
};
use secrecy::SecretString;
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_specta::Event;

use crate::api::dto::IdentityInput;
use crate::api::events::TreeChanged;
use crate::api::{ApiError, ApiResult};
use crate::ssh_import::{
    home, key_path, load_key, preview, read_config, SshImportPreview, SshKeyStatus,
};
use crate::state::AppState;

/// `ssh_config_import` result.
#[derive(Clone, Debug, Default, Serialize, specta::Type)]
pub struct SshImportSummary {
    /// Folder the hosts went into.
    pub folder_id: Option<FolderId>,
    /// Hosts created (jump hosts named only in `ProxyJump` included).
    pub hosts: u32,
    /// Keys added to the vault (already saved keys are reused, not counted).
    pub keys: u32,
    /// Forward rules created.
    pub forwards: u32,
    /// Keys left out: passphrase-protected, missing or unreadable.
    pub skipped_keys: u32,
}

struct Ctx<'a> {
    state: &'a AppState,
    folder: FolderId,
    summary: SshImportSummary,
    /// alias (lowercase) → host, for hosts created by this import.
    by_alias: HashMap<String, HostId>,
    /// key file → identity, so one key is stored once.
    keys: HashMap<PathBuf, Option<IdentityId>>,
}

fn bump(n: &mut u32) {
    *n = n.saturating_add(1);
}

impl Ctx<'_> {
    fn identity_for(&mut self, path: &Path, label: &str) -> Option<IdentityId> {
        if let Some(id) = self.keys.get(path) {
            return *id;
        }
        let id = self.store_key(path, label);
        if id.is_none() {
            bump(&mut self.summary.skipped_keys);
        }
        self.keys.insert(path.to_owned(), id);
        id
    }

    fn store_key(&mut self, path: &Path, label: &str) -> Option<IdentityId> {
        let (status, pem) = load_key(path);
        let pem = pem.filter(|_| status == SshKeyStatus::Ok)?;
        let info = ibc_shell_vault::keys::import_private_key(pem.as_bytes(), None).ok()?;
        let existing = self
            .state
            .store
            .identity_list()
            .ok()?
            .into_iter()
            .find(|i| i.key_fingerprint.as_deref() == Some(info.fingerprint.as_str()));
        if let Some(i) = existing {
            return Some(i.id);
        }
        let made = crate::commands::identity::create(
            self.state,
            IdentityInput {
                label: label.to_owned(),
                username: None,
                kind: IdentityKind::Key,
                password: None,
                private_key_pem: Some(SecretString::from(pem.as_str().to_owned())),
                passphrase: None,
            },
        )
        .ok()?;
        bump(&mut self.summary.keys);
        Some(made.id)
    }

    fn host(&mut self, input: HostInput) -> ApiResult<HostId> {
        let h = self.state.store.host_create(input)?;
        bump(&mut self.summary.hosts);
        Ok(h.id)
    }

    /// A `ProxyJump` hop: an alias of this import, a saved host label, or `[user@]host[:port]`.
    fn hop(&mut self, spec: &str) -> ApiResult<HostId> {
        if let Some(id) = self.by_alias.get(&spec.to_lowercase()) {
            return Ok(*id);
        }
        let tree = self.state.store.tree_list()?;
        if let Some(h) = tree
            .hosts
            .iter()
            .find(|h| h.label.eq_ignore_ascii_case(spec))
        {
            return Ok(h.id);
        }
        let (user, rest) = match spec.split_once('@') {
            Some((u, r)) => (Some(u.to_owned()), r),
            None => (None, spec),
        };
        let (hostname, port) = match rest.rsplit_once(':') {
            Some((h, p)) if p.parse::<u16>().is_ok() => (h.to_owned(), p.parse().ok()),
            _ => (rest.to_owned(), None),
        };
        let id = self.host(HostInput {
            folder_id: Some(self.folder),
            label: spec.to_owned(),
            hostname,
            port,
            username: user,
            ..HostInput::default()
        })?;
        self.by_alias.insert(spec.to_lowercase(), id);
        Ok(id)
    }

    fn forwards(&mut self, host: HostId, specs: &[ForwardSpec]) {
        for s in specs {
            let (kind, bind_addr, bind_port, dest) = match s {
                ForwardSpec::Local {
                    bind_addr,
                    bind_port,
                    dest_host,
                    dest_port,
                } => (
                    ForwardKind::Local,
                    bind_addr,
                    *bind_port,
                    Some((dest_host, *dest_port)),
                ),
                ForwardSpec::Remote {
                    bind_addr,
                    bind_port,
                    dest_host,
                    dest_port,
                } => (
                    ForwardKind::Remote,
                    bind_addr,
                    *bind_port,
                    Some((dest_host, *dest_port)),
                ),
                ForwardSpec::Dynamic {
                    bind_addr,
                    bind_port,
                } => (ForwardKind::Dynamic, bind_addr, *bind_port, None),
            };
            let made = self.state.store.forward_create(ForwardInput {
                host_id: host,
                kind,
                bind_addr: Some(bind_addr.clone()),
                bind_port,
                dest_host: dest.map(|d| d.0.clone()),
                dest_port: dest.map(|d| d.1),
                auto_start: false,
                label: None,
            });
            if made.is_ok() {
                bump(&mut self.summary.forwards);
            }
        }
    }
}

fn run(
    state: &AppState,
    home: &Path,
    entries: &[ConfigEntry],
    folder_name: &str,
) -> ApiResult<SshImportSummary> {
    let name = folder_name.trim();
    let folder = state.store.folder_create(FolderInput {
        name: if name.is_empty() { "SSH config" } else { name }.to_owned(),
        ..FolderInput::default()
    })?;
    let mut cx = Ctx {
        state,
        folder: folder.id,
        summary: SshImportSummary {
            folder_id: Some(folder.id),
            ..SshImportSummary::default()
        },
        by_alias: HashMap::new(),
        keys: HashMap::new(),
    };
    for e in entries {
        let identity_id = key_path(home, e).and_then(|p| {
            let label = p
                .file_name()
                .map_or_else(|| e.alias.clone(), |n| n.to_string_lossy().into_owned());
            cx.identity_for(&p, &label)
        });
        let id = cx.host(HostInput {
            folder_id: Some(folder.id),
            label: e.alias.clone(),
            hostname: e.hostname.clone(),
            port: e.port,
            username: e.user.clone(),
            identity_id,
            ..HostInput::default()
        })?;
        cx.by_alias.insert(e.alias.to_lowercase(), id);
        cx.forwards(id, &e.forwards);
    }
    // Jumps after every host exists: `ProxyJump a,b` = reach b through a, then the target.
    let created: Vec<HostId> = cx.by_alias.values().copied().collect();
    for e in entries {
        let Some(chain) = &e.proxy_jump else { continue };
        let Some(&target) = cx.by_alias.get(&e.alias.to_lowercase()) else {
            continue;
        };
        let hops: Vec<HostId> = chain
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| cx.hop(s))
            .collect::<ApiResult<_>>()?;
        for pair in hops.windows(2) {
            let own_jump = state
                .store
                .tree_list()?
                .hosts
                .iter()
                .any(|h| h.id == pair[1] && h.jump_host_id.is_some());
            if created.contains(&pair[1]) && !own_jump {
                let _ = state.store.host_update(pair[1], jump(pair[0]));
            }
        }
        if let Some(&last) = hops.last() {
            // A cycle or a chain over 3 hops is refused by the store; the host stays direct.
            let _ = state.store.host_update(target, jump(last));
        }
    }
    Ok(cx.summary)
}

fn jump(id: HostId) -> HostPatch {
    HostPatch {
        jump_host_id: Some(Some(id)),
        ..HostPatch::default()
    }
}

/// Hosts of `~/.ssh/config` with key state and duplicates, for the import dialog.
#[tauri::command]
#[specta::specta]
pub async fn ssh_config_preview(state: State<'_, AppState>) -> ApiResult<SshImportPreview> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || preview(&state.store))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))
}

/// Imports the chosen aliases into a new folder; emits `tree://changed`.
#[tauri::command]
#[specta::specta]
pub async fn ssh_config_import(
    app: AppHandle,
    state: State<'_, AppState>,
    aliases: Vec<String>,
    folder_name: String,
) -> ApiResult<SshImportSummary> {
    let state = (*state).clone();
    let summary = tauri::async_runtime::spawn_blocking(move || {
        let home = home().ok_or(ApiError::NotFound)?;
        let wanted: Vec<String> = aliases.iter().map(|a| a.to_lowercase()).collect();
        let entries: Vec<ConfigEntry> = read_config(&home)
            .ok_or(ApiError::NotFound)?
            .into_iter()
            .filter(|e| wanted.contains(&e.alias.to_lowercase()))
            .collect();
        if entries.is_empty() {
            return Err(ApiError::Validation("nothing to import".into()));
        }
        run(&state, &home, &entries, &folder_name)
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;
    let _ = TreeChanged.emit(&app);
    Ok(summary)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use std::sync::Arc;

    use ibc_shell_store::Store;
    use ibc_shell_vault::{MemoryKeyStore, Vault, VaultMode};

    use super::*;

    /// Throwaway ed25519 test key (same fixture as the SSH mock server).
    const KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACAjWF9XMGonNvOZS5jQtwN2+OyShDB1KB0m2qfUAEySdgAAAIhasfTVWrH0
1QAAAAtzc2gtZWQyNTUxOQAAACAjWF9XMGonNvOZS5jQtwN2+OyShDB1KB0m2qfUAEySdg
AAAEBpExXls+OkMCOV+0lvM7tv4lCconrTA8VEu/lb4/uEkyNYX1cwaic285lLmNC3A3b4
7JKEMHUoHSbap9QATJJ2AAAABHRlc3QB
-----END OPENSSH PRIVATE KEY-----
";

    fn state() -> AppState {
        let store = Arc::new(Store::open_in_memory().unwrap());
        let vault = Arc::new(Vault::new(
            store.clone(),
            Arc::new(MemoryKeyStore::default()),
        ));
        vault.init(VaultMode::OsKeychain, None).unwrap();
        AppState::new(store, vault)
    }

    #[test]
    fn imports_hosts_keys_jumps_and_forwards() {
        let home = tempfile::tempdir().unwrap();
        let ssh = home.path().join(".ssh");
        std::fs::create_dir_all(&ssh).unwrap();
        std::fs::write(ssh.join("k"), KEY).unwrap();
        std::fs::write(
            ssh.join("config"),
            "Host bastion\n  HostName 10.0.0.1\n  User jump\n\
             Host app\n  HostName app.internal\n  User deploy\n  ProxyJump bastion\n  IdentityFile ~/.ssh/k\n  LocalForward 5432 db:5432\n\
             Host other\n  ProxyJump admin@gw.example.com:2222\n  IdentityFile ~/.ssh/k\n\
             Host missing\n  IdentityFile ~/.ssh/nope\n",
        )
        .unwrap();
        let st = state();
        let entries = read_config(home.path()).unwrap();
        let sum = run(&st, home.path(), &entries, "Imported").unwrap();
        assert_eq!(sum.hosts, 5, "four aliases + the gw jump host");
        assert_eq!(sum.keys, 1, "one key stored once for two hosts");
        assert_eq!(sum.skipped_keys, 1, "missing key file");
        assert_eq!(sum.forwards, 1);

        let hosts = st.store.tree_list().unwrap().hosts;
        let by = |l: &str| hosts.iter().find(|h| h.label == l).unwrap();
        assert_eq!(by("app").jump_host_id, Some(by("bastion").id));
        let gw = by("admin@gw.example.com:2222");
        assert_eq!(
            (gw.hostname.as_str(), gw.port),
            ("gw.example.com", Some(2222))
        );
        assert_eq!(gw.username.as_deref(), Some("admin"));
        assert_eq!(by("other").jump_host_id, Some(gw.id));
        assert!(by("app").identity_id.is_some());
        assert_eq!(by("app").identity_id, by("other").identity_id);
        assert!(by("missing").identity_id.is_none());
        assert!(hosts.iter().all(|h| h.folder_id == sum.folder_id));

        // A second import reuses the stored key instead of adding it again.
        let again = run(&st, home.path(), &entries[1..2], "Again").unwrap();
        assert_eq!(again.keys, 0);
    }
}
