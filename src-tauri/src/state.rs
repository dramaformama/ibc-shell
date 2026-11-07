//! Shared application state (`AppState`) and its registries.

use std::collections::HashMap;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use ibc_shell_sftp::Sftp;
use ibc_shell_ssh::Connection;
use ibc_shell_store::{HostId, Store};
use ibc_shell_vault::Vault;
use tauri::ipc::Channel;
use tokio::sync::{mpsc, Notify};

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
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(k, v);
    }
    /// Clone of the value.
    pub fn get(&self, k: &K) -> Option<V> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(k)
            .cloned()
    }
    /// Removes and returns.
    pub fn remove(&self, k: &K) -> Option<V> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(k)
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
    /// Keystrokes (counted against the input queue budget while queued).
    Write(WriteBytes),
    /// PTY resize.
    Resize(u16, u16),
    /// Credit return.
    Ack(u32),
    /// Close the shell and disconnect.
    Close,
}

/// Max keystroke bytes waiting in a session's input queue (a stalled server must not make
/// the queue grow without bound).
pub const MAX_QUEUED_INPUT: usize = 1024 * 1024;

/// Queued keystrokes; releases its share of the queue budget when dropped.
pub struct WriteBytes {
    data: Vec<u8>,
    queued: Arc<AtomicUsize>,
}

impl WriteBytes {
    /// The bytes to send.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
}

impl std::fmt::Debug for WriteBytes {
    /// Never prints the keystrokes (they may be a password).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WriteBytes(<{} bytes>)", self.data.len())
    }
}

impl Drop for WriteBytes {
    fn drop(&mut self) {
        self.queued.fetch_sub(self.data.len(), Ordering::AcqRel);
    }
}

/// Sender side of a session's command queue: unbounded for small control commands, byte-bounded
/// for keystrokes, plus an out-of-band close signal that works even while a write is stalled.
#[derive(Clone)]
pub struct ShellTx {
    tx: mpsc::UnboundedSender<ShellCmd>,
    queued: Arc<AtomicUsize>,
    closed: Arc<Notify>,
}

impl ShellTx {
    /// New queue: sender plus receiver and the close signal for the pump.
    #[must_use]
    pub fn channel() -> (Self, mpsc::UnboundedReceiver<ShellCmd>, Arc<Notify>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let closed = Arc::new(Notify::new());
        let me = Self {
            tx,
            queued: Arc::new(AtomicUsize::new(0)),
            closed: closed.clone(),
        };
        (me, rx, closed)
    }
    /// Queues keystrokes; `false` when the pump ended or the queue budget is exhausted.
    #[must_use]
    pub fn write(&self, data: &[u8]) -> bool {
        let prev = self.queued.fetch_add(data.len(), Ordering::AcqRel);
        let msg = WriteBytes {
            data: data.to_vec(),
            queued: self.queued.clone(),
        };
        if prev.saturating_add(data.len()) > MAX_QUEUED_INPUT {
            return false; // `msg` drop returns the budget
        }
        self.tx.send(ShellCmd::Write(msg)).is_ok()
    }
    /// Queues a control command; `Close` additionally raises the out-of-band signal.
    #[must_use]
    pub fn send(&self, cmd: ShellCmd) -> bool {
        let close = matches!(cmd, ShellCmd::Close);
        let ok = self.tx.send(cmd).is_ok();
        if close {
            self.closed.notify_one();
        }
        ok
    }
    /// True when the pump has dropped its receiver.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
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
    pub cmd: Option<ShellTx>,
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
        live.cmd.as_ref().is_some_and(|tx| tx.send(cmd))
    }
    /// Queues keystrokes; `false` when not running or the input queue is full.
    #[must_use]
    pub fn write(&self, data: &[u8]) -> bool {
        let live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        live.cmd.as_ref().is_some_and(|tx| tx.write(data))
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
        let Ok(c) = std::fs::canonicalize(p) else {
            return false;
        };
        let g = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        g.iter()
            .any(|r| c.starts_with(r) || (allow_ancestors && r.starts_with(&c)))
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
    fn input_queue_is_byte_bounded_and_refills() {
        let (tx, mut rx, _closed) = ShellTx::channel();
        let chunk = vec![0u8; MAX_QUEUED_INPUT / 2];
        assert!(tx.write(&chunk));
        assert!(tx.write(&chunk));
        assert!(!tx.write(b"x"), "over budget must be rejected");
        drop(rx.try_recv().unwrap());
        assert!(tx.write(b"x"), "budget returns once the pump consumed a chunk");
    }

    #[tokio::test]
    async fn close_signal_survives_a_full_queue() {
        let (tx, _rx, closed) = ShellTx::channel();
        assert!(tx.send(ShellCmd::Close));
        tokio::time::timeout(std::time::Duration::from_secs(1), closed.notified())
            .await
            .unwrap();
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
