//! Reading `~/.ssh/config` for import (ADR-023). Paths come only from the user's own config,
//! never from the UI. Key files are read here; their contents never leave the backend.

use std::path::{Path, PathBuf};

use ibc_shell_ssh::{parse_ssh_config, ConfigEntry};
use ibc_shell_store::Store;
use ibc_shell_vault::keys::{import_private_key, KeyError};
use serde::Serialize;

/// Largest config / include file read.
const MAX_CONFIG_FILE: u64 = 1024 * 1024;
/// Largest private key file read.
const MAX_KEY_FILE: u64 = 64 * 1024;
/// Files one `Include` pattern may pull in.
const MAX_INCLUDE_FILES: usize = 64;
/// Keys ssh tries when a host names none.
const DEFAULT_KEYS: [&str; 3] = ["id_ed25519", "id_ecdsa", "id_rsa"];

/// State of a host's key file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SshKeyStatus {
    /// No key named and no default key found.
    None,
    /// Readable, unencrypted: will be imported.
    Ok,
    /// Protected by a passphrase: not imported (the host is, and can use ssh-agent).
    Encrypted,
    /// Named in the config but not on disk.
    Missing,
    /// Not a private key we understand.
    Invalid,
}

/// One host the config defines.
#[derive(Clone, Debug, Serialize, specta::Type)]
pub struct SshImportEntry {
    /// `Host` alias (becomes the label).
    pub alias: String,
    /// Address.
    pub hostname: String,
    /// Port.
    pub port: Option<u16>,
    /// User.
    pub user: Option<String>,
    /// Key file for display, `~/` shortened.
    pub key_path: Option<String>,
    /// Key state.
    pub key: SshKeyStatus,
    /// `ProxyJump` as written.
    pub proxy_jump: Option<String>,
    /// Forward lines that will become rules.
    pub forwards: u32,
    /// Settings that are not imported (e.g. `ProxyCommand`).
    pub unsupported: Vec<String>,
    /// A saved host with the same address, port and user exists.
    pub duplicate: bool,
}

/// `ssh_config_preview` result.
#[derive(Clone, Debug, Serialize, specta::Type)]
pub struct SshImportPreview {
    /// `~/.ssh/config` exists.
    pub found: bool,
    /// Hosts in file order.
    pub entries: Vec<SshImportEntry>,
}

pub(crate) fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn read_small(path: &Path, max: u64) -> Option<Vec<u8>> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > max {
        return None;
    }
    std::fs::read(path).ok()
}

/// `~` and `%d` → home; relative paths are under `~/.ssh` (as ssh resolves `Include`).
fn expand(home: &Path, raw: &str) -> PathBuf {
    let s = raw.replace("%d", &home.to_string_lossy());
    if let Some(rest) = s.strip_prefix("~/") {
        return home.join(rest);
    }
    let p = PathBuf::from(&s);
    if p.is_absolute() {
        p
    } else {
        home.join(".ssh").join(p)
    }
}

fn wildcard(name: &str, pattern: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut rest = name;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            let Some(r) = rest.strip_prefix(part) else {
                return false;
            };
            rest = r;
        } else if i == parts.len() - 1 {
            return rest.ends_with(part);
        } else if let Some(at) = rest.find(part) {
            rest = &rest[at + part.len()..];
        } else {
            return false;
        }
    }
    rest.is_empty()
}

/// Contents of the files an `Include` pattern names (`*` only in the last component).
fn include(home: &Path, pattern: &str) -> Vec<String> {
    let path = expand(home, pattern);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let files: Vec<PathBuf> = if name.contains('*') {
        let Some(dir) = path.parent() else {
            return Vec::new();
        };
        let Ok(rd) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut v: Vec<PathBuf> = rd
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| wildcard(&n.to_string_lossy(), &name))
            })
            .collect();
        v.sort();
        v.truncate(MAX_INCLUDE_FILES);
        v
    } else {
        vec![path]
    };
    files
        .iter()
        .filter_map(|f| read_small(f, MAX_CONFIG_FILE))
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .collect()
}

/// Parsed `~/.ssh/config`; `None` when the file does not exist.
pub(crate) fn read_config(home: &Path) -> Option<Vec<ConfigEntry>> {
    let bytes = read_small(&home.join(".ssh").join("config"), MAX_CONFIG_FILE)?;
    let text = String::from_utf8_lossy(&bytes);
    Some(parse_ssh_config(&text, &|p| include(home, p)))
}

/// The key file a host uses: its first `IdentityFile` that exists (or the first named),
/// else the first default key on disk.
pub(crate) fn key_path(home: &Path, e: &ConfigEntry) -> Option<PathBuf> {
    let user = e.user.clone().unwrap_or_default();
    let named: Vec<PathBuf> = e
        .identity_files
        .iter()
        .map(|f| expand(home, &f.replace("%h", &e.hostname).replace("%r", &user)))
        .collect();
    if let Some(p) = named.iter().find(|p| p.is_file()) {
        return Some(p.clone());
    }
    if let Some(p) = named.into_iter().next() {
        return Some(p);
    }
    DEFAULT_KEYS
        .iter()
        .map(|k| home.join(".ssh").join(k))
        .find(|p| p.is_file())
}

/// Reads and checks a key file; `Ok` carries the PEM text.
pub(crate) fn load_key(path: &Path) -> (SshKeyStatus, Option<zeroize::Zeroizing<String>>) {
    if !path.is_file() {
        return (SshKeyStatus::Missing, None);
    }
    let Some(bytes) = read_small(path, MAX_KEY_FILE).map(zeroize::Zeroizing::new) else {
        return (SshKeyStatus::Invalid, None);
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return (SshKeyStatus::Invalid, None);
    };
    match import_private_key(text.as_bytes(), None) {
        Ok(_) => (
            SshKeyStatus::Ok,
            Some(zeroize::Zeroizing::new(text.to_owned())),
        ),
        Err(KeyError::PassphraseRequired | KeyError::WrongPassphrase) => {
            (SshKeyStatus::Encrypted, None)
        }
        Err(_) => (SshKeyStatus::Invalid, None),
    }
}

fn display(home: &Path, p: &Path) -> String {
    match p.strip_prefix(home) {
        Ok(rest) => format!("~/{}", rest.to_string_lossy()),
        Err(_) => p.to_string_lossy().into_owned(),
    }
}

fn same(a: Option<&str>, b: Option<&str>) -> bool {
    a.unwrap_or_default() == b.unwrap_or_default()
}

/// Builds the preview: every host, its key state and whether it is already saved.
pub(crate) fn preview(store: &Store) -> SshImportPreview {
    let Some(home) = home() else {
        return SshImportPreview {
            found: false,
            entries: Vec::new(),
        };
    };
    let Some(entries) = read_config(&home) else {
        return SshImportPreview {
            found: false,
            entries: Vec::new(),
        };
    };
    let saved = store.tree_list().map(|t| t.hosts).unwrap_or_default();
    let entries = entries
        .into_iter()
        .map(|e| {
            let path = key_path(&home, &e);
            let key = path
                .as_deref()
                .map_or(SshKeyStatus::None, |p| load_key(p).0);
            let duplicate = saved.iter().any(|h| {
                h.hostname.eq_ignore_ascii_case(&e.hostname)
                    && h.port.unwrap_or(22) == e.port.unwrap_or(22)
                    && same(h.username.as_deref(), e.user.as_deref())
            });
            SshImportEntry {
                key_path: path.as_deref().map(|p| display(&home, p)),
                key,
                hostname: e.hostname,
                port: e.port,
                user: e.user,
                proxy_jump: e.proxy_jump,
                forwards: u32::try_from(e.forwards.len()).unwrap_or(u32::MAX),
                unsupported: e.unsupported,
                duplicate,
                alias: e.alias,
            }
        })
        .collect();
    SshImportPreview {
        found: true,
        entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn include_wildcards() {
        assert!(wildcard("work.conf", "*.conf"));
        assert!(wildcard("a", "*"));
        assert!(!wildcard("work.txt", "*.conf"));
        assert!(wildcard("x-1-y", "x-*-y"));
    }

    #[test]
    fn paths_expand_like_ssh() {
        let h = Path::new("/home/u");
        assert_eq!(expand(h, "~/.ssh/id"), PathBuf::from("/home/u/.ssh/id"));
        assert_eq!(expand(h, "%d/k"), PathBuf::from("/home/u/k"));
        assert_eq!(
            expand(h, "conf.d/a"),
            PathBuf::from("/home/u/.ssh/conf.d/a")
        );
        assert_eq!(expand(h, "/etc/x"), PathBuf::from("/etc/x"));
    }
}
