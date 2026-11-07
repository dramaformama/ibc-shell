//! Connection management: one writer `Mutex<Connection>` + two read connections.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags, Transaction, TransactionBehavior};

use crate::{schema, Result, StoreError};

const READERS: usize = 2;

/// Database handle: one writer `Mutex<Connection>` + read connections. `Send + Sync`.
/// In-memory stores have no separate readers (each in-memory connection is its own database).
#[derive(Debug)]
pub struct Store {
    writer: Mutex<Connection>,
    readers: Vec<Mutex<Connection>>,
}

/// Unix millis now.
pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

fn lock(m: &Mutex<Connection>) -> MutexGuard<'_, Connection> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.busy_timeout(std::time::Duration::from_millis(5000))?;
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL;")?;
    Ok(())
}

impl Store {
    /// Opens (creating + migrating) the database at `path`; WAL, `foreign_keys=ON`, 0600.
    pub fn open(path: &Path) -> Result<Self> {
        let mut writer = Connection::open(path)?;
        configure(&writer)?;
        writer.execute_batch("PRAGMA journal_mode=WAL;")?;
        schema::migrations().to_latest(&mut writer)?;
        #[cfg(unix)]
        restrict_permissions(path)?;
        let mut readers = Vec::with_capacity(READERS);
        for _ in 0..READERS {
            let r = Connection::open_with_flags(
                path,
                OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?;
            configure(&r)?;
            r.execute_batch("PRAGMA query_only=ON;")?;
            readers.push(Mutex::new(r));
        }
        #[cfg(unix)]
        restrict_permissions(path)?;
        Ok(Self {
            writer: Mutex::new(writer),
            readers,
        })
    }

    /// In-memory database for tests.
    pub fn open_in_memory() -> Result<Self> {
        let mut writer = Connection::open_in_memory()?;
        configure(&writer)?;
        schema::migrations().to_latest(&mut writer)?;
        Ok(Self {
            writer: Mutex::new(writer),
            readers: Vec::new(),
        })
    }

    /// Runs `f` on a read connection.
    pub(crate) fn read<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        for r in &self.readers {
            if let Ok(g) = r.try_lock() {
                return f(&g);
            }
        }
        match self.readers.first() {
            Some(r) => f(&lock(r)),
            None => f(&lock(&self.writer)),
        }
    }

    /// Runs `f` inside one IMMEDIATE write transaction; commits on `Ok`, rolls back on `Err`.
    pub(crate) fn write<T>(&self, f: impl FnOnce(&Transaction<'_>) -> Result<T>) -> Result<T> {
        let mut conn = lock(&self.writer);
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let out = f(&tx)?;
        tx.commit()?;
        Ok(out)
    }
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for suffix in ["", "-wal", "-shm"] {
        let mut p = path.as_os_str().to_owned();
        p.push(suffix);
        let p = std::path::PathBuf::from(p);
        if p.exists() {
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| StoreError::Db(format!("chmod: {e}")))?;
        }
    }
    Ok(())
}

/// Prepares (cached), runs and collects a query.
pub(crate) fn query_all<T>(
    conn: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
    f: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    let mut st = conn.prepare_cached(sql)?;
    let rows = st.query_map(params, f)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}
