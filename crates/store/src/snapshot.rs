//! Backup support: consistent export and merge import.

use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::catalog::{Snapshot, Tag};
use crate::db::Store;
use crate::folders::{insert_folder, load_tree};
use crate::hosts::insert_host;
use crate::identity::{identity_from_row, insert_identity, IDENTITY_COLS};
use crate::ids::TagId;
use crate::known_hosts::{insert_known_host, known_host_from_row};
use crate::misc::{forward_from_row, insert_forward, snippet_from_row};
use crate::{ImportCounts, Result};

fn count(inserted: bool, n: &mut u32) {
    if inserted {
        *n += 1;
    }
}

fn collect<T>(
    conn: &Connection,
    sql: &str,
    f: fn(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    Ok(conn
        .prepare_cached(sql)?
        .query_map([], f)?
        .collect::<rusqlite::Result<_>>()?)
}

impl Store {
    /// Consistent snapshot of all non-secret data (single read transaction); no settings.
    pub fn export_snapshot(&self) -> Result<Snapshot> {
        self.read(|c| {
            let tx = c.unchecked_transaction()?;
            let tree = load_tree(&tx)?;
            let snap = Snapshot {
                folders: tree.folders,
                hosts: tree.hosts,
                identities: collect(&tx, &format!("SELECT {IDENTITY_COLS} FROM identity ORDER BY id"), identity_from_row)?,
                tags: collect(&tx, "SELECT id, name, color FROM tag ORDER BY id", |r| {
                    Ok(Tag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)? })
                })?,
                snippets: collect(
                    &tx,
                    "SELECT id, folder_id, name, body, sort_key, created_at, updated_at FROM snippet ORDER BY id",
                    snippet_from_row,
                )?,
                forwards: collect(
                    &tx,
                    "SELECT id, host_id, kind, bind_addr, bind_port, dest_host, dest_port, auto_start, label FROM port_forward ORDER BY id",
                    forward_from_row,
                )?,
                known_hosts: collect(
                    &tx,
                    "SELECT id, host_pattern, key_type, key_blob, fingerprint, added_at FROM known_host ORDER BY id",
                    known_host_from_row,
                )?,
            };
            tx.commit()?;
            Ok(snap)
        })
    }

    /// Merges a snapshot (ids kept; existing ids are skipped) in one transaction.
    pub fn import_snapshot(&self, snapshot: Snapshot) -> Result<ImportCounts> {
        self.write(|tx| {
            // Parents / referenced rows may arrive in any order; check references at commit.
            tx.execute_batch("PRAGMA defer_foreign_keys=ON;")?;
            let mut n = ImportCounts::default();
            for i in &snapshot.identities {
                count(insert_identity(tx, i, "INSERT OR IGNORE")?, &mut n.identities);
            }
            // Tags: an existing tag with the same name (case-insensitive) absorbs the imported one.
            let mut tag_map: HashMap<TagId, TagId> = HashMap::new();
            for t in &snapshot.tags {
                let existing: Option<TagId> = tx
                    .query_row(
                        "SELECT id FROM tag WHERE id=?1 OR name=?2 COLLATE NOCASE",
                        params![t.id, t.name],
                        |r| r.get(0),
                    )
                    .ok();
                if let Some(e) = existing {
                    tag_map.insert(t.id, e);
                } else {
                    tx.execute("INSERT INTO tag(id, name, color) VALUES(?1,?2,?3)", params![t.id, t.name, t.color])?;
                    tag_map.insert(t.id, t.id);
                    n.tags += 1;
                }
            }
            for f in &snapshot.folders {
                count(insert_folder(tx, f, "INSERT OR IGNORE")?, &mut n.folders);
            }
            for h in &snapshot.hosts {
                let mut h = h.clone();
                h.tag_ids = h.tag_ids.iter().map(|t| tag_map.get(t).copied().unwrap_or(*t)).collect();
                count(insert_host(tx, &h, "INSERT OR IGNORE")?, &mut n.hosts);
            }
            for s in &snapshot.snippets {
                let w = tx.execute(
                    "INSERT OR IGNORE INTO snippet(id, folder_id, name, body, sort_key, created_at, updated_at)
                     VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![s.id, s.folder_id, s.name, s.body, s.sort_key, s.created_at, s.updated_at],
                )?;
                count(w > 0, &mut n.snippets);
            }
            for f in &snapshot.forwards {
                count(insert_forward(tx, f, "INSERT OR IGNORE")?, &mut n.forwards);
            }
            for k in &snapshot.known_hosts {
                count(insert_known_host(tx, k, "INSERT OR IGNORE")?, &mut n.known_hosts);
            }
            Ok(n)
        })
    }
}
