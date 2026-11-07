//! FTS5 index maintenance (contentless `host_fts`, rowid = `host.rowid`) and search.
//!
//! A contentless table cannot be updated in place: before any change to a host's label,
//! hostname, notes or tag set, call [`fts_remove`] (it reads the currently indexed values from
//! the base tables) and after the change [`fts_add`] — both inside the same transaction.

use rusqlite::Connection;

use crate::db::Store;
use crate::ids::{HostId, TagId};
use crate::Result;

const VALUES_SQL: &str = "SELECT h.rowid, h.label, h.hostname,
    COALESCE((SELECT group_concat(n, ' ') FROM
        (SELECT t.name AS n FROM host_tag ht JOIN tag t ON t.id = ht.tag_id
         WHERE ht.host_id = h.id ORDER BY t.name)), ''),
    COALESCE(h.notes, '')
    FROM host h WHERE h.id = ?1";

type Values = (i64, String, String, String, String);

fn values(conn: &Connection, id: HostId) -> Result<Option<Values>> {
    let mut st = conn.prepare_cached(VALUES_SQL)?;
    let mut rows = st.query([id])?;
    match rows.next()? {
        Some(r) => Ok(Some((
            r.get(0)?,
            r.get(1)?,
            r.get(2)?,
            r.get(3)?,
            r.get(4)?,
        ))),
        None => Ok(None),
    }
}

/// Removes the host's current index entry (no-op for unknown hosts).
pub(crate) fn fts_remove(conn: &Connection, id: HostId) -> Result<()> {
    if let Some(v) = values(conn, id)? {
        conn.execute(
            "INSERT INTO host_fts(host_fts, rowid, label, hostname, tags, notes)
             VALUES('delete', ?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![v.0, v.1, v.2, v.3, v.4],
        )?;
    }
    Ok(())
}

/// Indexes the host's current values.
pub(crate) fn fts_add(conn: &Connection, id: HostId) -> Result<()> {
    if let Some(v) = values(conn, id)? {
        conn.execute(
            "INSERT INTO host_fts(rowid, label, hostname, tags, notes) VALUES(?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![v.0, v.1, v.2, v.3, v.4],
        )?;
    }
    Ok(())
}

/// Hosts carrying `tag`.
pub(crate) fn hosts_with_tag(conn: &Connection, tag: TagId) -> Result<Vec<HostId>> {
    let mut st = conn.prepare_cached("SELECT host_id FROM host_tag WHERE tag_id = ?1")?;
    let rows = st.query_map([tag], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Builds an FTS5 query: every whitespace-separated word becomes a quoted prefix phrase, `AND`ed.
fn match_expr(q: &str) -> Option<String> {
    let parts: Vec<String> = q
        .split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .map(|w| format!("\"{}\"*", w.replace('"', "\"\"")))
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

impl Store {
    /// FTS5 search over label, hostname, tags, notes; best match first.
    pub fn host_search(&self, q: &str, limit: u32) -> Result<Vec<HostId>> {
        let Some(expr) = match_expr(q) else {
            return Ok(Vec::new());
        };
        self.read(|c| {
            let mut st = c.prepare_cached(
                "SELECT h.id FROM host_fts JOIN host h ON h.rowid = host_fts.rowid
                 WHERE host_fts MATCH ?1 ORDER BY bm25(host_fts), h.label LIMIT ?2",
            )?;
            let rows = st.query_map(rusqlite::params![expr, limit], |r| r.get(0))?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        })
    }
}
