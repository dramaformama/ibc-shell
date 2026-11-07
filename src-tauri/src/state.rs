//! Shared application state (`AppState`) and its registries.

use std::collections::HashMap;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use ibc_sftp::Sftp;
use ibc_ssh::Connection;
use ibc_store::{HostId, Store};
use ibc_vault::Vault;
use tauri::ipc::Channel;
use tokio::sync::mpsc;

use crate::api::dto::{QuickConnect, SessionEvent, SessionId, SftpId};
use crate::api::stream::RawBytes;
use crate::prompts::PendingPrompts;
use crate::transfers::TransferManager;

/// Small concurrent map (std `Mutex<HashMap>`; avoids a `dashmap` dependency — the maps are
/// tiny and never held across `.await`).
pub struct Registry<K, V>(Mutex<HashMap<K, V>>);

impl<K, V> Default for Registry<K, V> {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

impl<K: Eq + Hash + Copy, V: Clone> Registry<K, V> {
    /// Inserts / replaces.
    pub fn insert(&self, k: K, v: V) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).insert(k, v);
    }
    /// Clone of the value.
    pub fn get(&self, k: &K) -> Option<V> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).get(k).cloned()
    }
    /// Removes and returns.
    pub fn remove(&self, k: &K) -> Option<V> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).remove(k)
    }
    /// Number of entries.
    pub fn len(&self) -> usize {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).len()
    }
    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Input to the session pump task.
#[derive(Debug)]
pub enum ShellCmd {
    /// Keystrokes.
    Write(Vec<u8>),
    /// PTY resize.
    Resize(u16, u16),
    /// Credit return.
    Ack(u32),
    /// Close the shell and disconnect.
    Close,
}

/// What a session connects to (kept for `session_reconnect`).
#[derive(Clone, Debug)]
pub enum SessionTarget {
    /// Stored host.
    Host(HostId),
    /// Quick connect.
    Adhoc(QuickConnect),
}

/// Immutable session parameters.
pub struct SessionSpec {
    /// Target.
    pub target: SessionTarget,
    /// Terminal output channel.
    pub on_data: Channel<RawBytes>,
    /// Event channel.
    pub on_event: Channel<SessionEvent>,
}

/// Live parts of a session, replaced on reconnect.
#[derive(Default)]
pub struct Live {
    /// Command channel into the pump (closed when the pump ended).
    pub cmd: Option<mpsc::UnboundedSender<ShellCmd>>,
    /// Authenticated connection (shared with SFTP).
    pub conn: Option<Arc<Connection>>,
    /// Last known PTY size.
    pub size: (u16, u16),
}

/// One terminal session.
pub struct SessionHandle {
    /// Id.
    pub id: SessionId,
    /// Parameters.
    pub spec: SessionSpec,
    /// Mutable parts.
    pub live: Mutex<Live>,
}

impl SessionHandle {
    /// Sends a command to the pump; `false` when the session is not running.
    pub fn send(&self, cmd: ShellCmd) -> bool {
        let live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        live.cmd.as_ref().is_some_and(|tx| tx.send(cmd).is_ok())
    }
    /// True while the pump is alive.
    pub fn is_running(&self) -> bool {
        let live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        live.cmd.as_ref().is_some_and(|tx| !tx.is_closed())
    }
}

/// Open SFTP session: keeps its connection alive.
pub struct SftpHandle {
    /// SFTP client.
    pub sftp: Arc<Sftp>,
    /// Underlying connection.
    pub conn: Arc<Connection>,
}

/// Local-FS allow list (`ipc.md` Rules): home, paths listed before, and their ancestors.
#[derive(Default)]
pub struct AllowedRoots(Mutex<Vec<PathBuf>>);

impl AllowedRoots {
    /// Registers a root (canonicalized).
    pub fn add(&self, p: &Path) {
        if let Ok(c) = std::fs::canonicalize(p) {
            let mut g = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if !g.contains(&c) {
                g.push(c);
            }
        }
    }
    /// True when `p` is under a root, or an ancestor of one (so the UI can go "up").
    #[must_use]
    pub fn allows(&self, p: &Path, allow_ancestors: bool) -> bool {
        let Ok(c) = std::fs::canonicalize(p) else { return false };
        let g = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        g.iter().any(|r| c.starts_with(r) || (allow_ancestors && r.starts_with(&c)))
    }
}

/// Everything commands need.
pub struct Inner {
    /// SQLite store.
    pub store: Arc<Store>,
    /// Vault (internally synchronized, so no outer `RwLock`).
    pub vault: Arc<Vault>,
    /// Terminal sessions.
    pub sessions: Registry<SessionId, Arc<SessionHandle>>,
    /// SFTP sessions.
    pub sftps: Registry<SftpId, Arc<SftpHandle>>,
    /// Transfer engine + subscriber.
    pub transfers: TransferManager,
    /// Unanswered prompts.
    pub pending: Arc<PendingPrompts>,
    /// Local FS roots.
    pub roots: AllowedRoots,
}

/// Cheap-to-clone handle managed by Tauri.
#[derive(Clone)]
pub struct AppState(pub Arc<Inner>);

impl std::ops::Deref for AppState {
    type Target = Inner;
    fn deref(&self) -> &Inner {
        &self.0
    }
}

impl AppState {
    /// Builds the state over an opened store and vault.
    #[must_use]
    pub fn new(store: Arc<Store>, vault: Arc<Vault>) -> Self {
        Self(Arc::new(Inner {
            store,
            vault,
            sessions: Registry::default(),
            sftps: Registry::default(),
            transfers: TransferManager::new(),
            pending: Arc::new(PendingPrompts::default()),
            roots: AllowedRoots::default(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roots_allow_descendants_and_ancestors_only() {
        let tmp = std::env::temp_dir().join(format!("ibc-roots-{}", std::process::id()));
        let inner = tmp.join("a/b");
        std::fs::create_dir_all(&inner).unwrap();
        let other = std::env::temp_dir().join(format!("ibc-roots-other-{}", std::process::id()));
        std::fs::create_dir_all(&other).unwrap();
        let r = AllowedRoots::default();
        r.add(&tmp.join("a"));
        assert!(r.allows(&inner, false));
        assert!(r.allows(&tmp, true));
        assert!(!r.allows(&tmp, false));
        assert!(!r.allows(&other, true));
        assert!(!r.allows(&tmp.join("a/../../nonexistent"), true));
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&other);
    }

    #[test]
    fn registry_roundtrip() {
        let r: Registry<u8, u8> = Registry::default();
        r.insert(1, 2);
        assert_eq!(r.get(&1), Some(2));
        assert_eq!(r.remove(&1), Some(2));
        assert!(r.is_empty());
    }
}
