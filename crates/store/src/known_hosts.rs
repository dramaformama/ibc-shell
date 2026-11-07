//! Known hosts (TOFU store; deleting an entry is the only way to clear a changed-key block).

use rusqlite::{params, Connection, Row};

use crate::catalog::{KnownHost, KnownHostInput, KnownHostSummary};
use crate::db::{now_ms, query_all, Store};
use crate::ids::KnownHostId;
use crate::{Result, StoreError};

pub(crate) fn known_host_from_row(r: &Row<'_>) -> rusqlite::Result<KnownHost> {
    Ok(KnownHost {
        id: r.get(0)?,
        host_pattern: r.get(1)?,
        key_type: r.get(2)?,
        key_blob: r.get(3)?,
        fingerprint: r.get(4)?,
        added_at: r.get(5)?,
    })
}

const KH_COLS: &str = "id, host_pattern, key_type, key_blob, fingerprint, added_at";

impl Store {
    /// All entries for the Known hosts screen (no key blobs).
    pub fn known_host_list(&self) -> Result<Vec<KnownHostSummary>> {
        self.read(|c| {
            query_all(
                c,
                "SELECT id, host_pattern, key_type, fingerprint, added_at FROM known_host ORDER BY host_pattern, key_type",
                [],
                |r| {
                    Ok(KnownHostSummary {
                        id: r.get(0)?,
                        host_pattern: r.get(1)?,
                        key_type: r.get(2)?,
                        fingerprint: r.get(3)?,
                        added_at: r.get(4)?,
                    })
                },
            )
        })
    }

    /// Entries matching an exact host pattern (all key types).
    pub fn known_host_lookup(&self, pattern: &str) -> Result<Vec<KnownHost>> {
        self.read(|c| {
            let sql =
                format!("SELECT {KH_COLS} FROM known_host WHERE host_pattern=?1 ORDER BY key_type");
            query_all(c, &sql, [pattern], known_host_from_row)
        })
    }

    /// TOFU insert used by the connect path: stores `kh` only while the host has no other key.
    /// Identical key already stored → `Ok`. Any different key stored for the pattern (e.g. a
    /// concurrent prompt accepted another key first) → `Conflict`; never replaces a key.
    pub fn known_host_add_new(&self, kh: KnownHostInput) -> Result<()> {
        if kh.host_pattern.is_empty() || kh.key_type.is_empty() || kh.key_blob.is_empty() {
            return Err(StoreError::Validation("known host: empty field".into()));
        }
        self.write(|tx| {
            let sql = format!("SELECT {KH_COLS} FROM known_host WHERE host_pattern=?1");
            let stored = query_all(tx, &sql, [&kh.host_pattern], known_host_from_row)?;
            if stored.iter().any(|k| k.key_blob == kh.key_blob) {
                return Ok(());
            }
            if !stored.is_empty() {
                return Err(StoreError::Conflict("host key changed".into()));
            }
            tx.execute(
                &format!("INSERT INTO known_host({KH_COLS}) VALUES(?1,?2,?3,?4,?5,?6)"),
                params![KnownHostId::new_v7(), kh.host_pattern, kh.key_type, kh.key_blob, kh.fingerprint, now_ms()],
            )?;
            Ok(())
        })
    }

    /// Inserts or replaces on (`host_pattern`, `key_type`).
    pub fn known_host_upsert(&self, kh: KnownHostInput) -> Result<()> {
        if kh.host_pattern.is_empty() || kh.key_type.is_empty() || kh.key_blob.is_empty() {
            return Err(StoreError::Validation("known host: empty field".into()));
        }
        self.write(|tx| {
            tx.execute(
                "INSERT INTO known_host(id, host_pattern, key_type, key_blob, fingerprint, added_at)
                 VALUES(?1,?2,?3,?4,?5,?6)
                 ON CONFLICT(host_pattern, key_type) DO UPDATE SET
                   key_blob=excluded.key_blob, fingerprint=excluded.fingerprint, added_at=excluded.added_at",
                params![KnownHostId::new_v7(), kh.host_pattern, kh.key_type, kh.key_blob, kh.fingerprint, now_ms()],
            )?;
            Ok(())
        })
    }

    /// Deletes an entry (the only way to clear a changed-key block).
    pub fn known_host_delete(&self, id: KnownHostId) -> Result<()> {
        self.write(|tx| {
            let n = tx.execute("DELETE FROM known_host WHERE id=?1", [id])?;
            if n == 0 {
                return Err(StoreError::NotFound);
            }
            Ok(())
        })
    }
}

pub(crate) fn insert_known_host(conn: &Connection, k: &KnownHost, verb: &str) -> Result<bool> {
    let n = conn.execute(
        &format!("{verb} INTO known_host({KH_COLS}) VALUES(?1,?2,?3,?4,?5,?6)"),
        params![
            k.id,
            k.host_pattern,
            k.key_type,
            k.key_blob,
            k.fingerprint,
            k.added_at
        ],
    )?;
    Ok(n > 0)
}
