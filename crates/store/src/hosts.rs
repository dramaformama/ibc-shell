//! Host CRUD, move, duplicate, delete.

use std::collections::HashMap;

use rusqlite::{params, Connection, Row};

use crate::db::{now_ms, Store};
use crate::ids::{FolderId, HostId, TagId};
use crate::search::{fts_add, fts_remove};
use crate::siblings::{append_key, key_after};
use crate::tree::{Host, HostInput, HostPatch, NodeRef};
use crate::{resolver, validate, Result, StoreError};

pub(crate) const HOST_COLS: &str = "id,folder_id,label,hostname,port,username,identity_id,\
    jump_host_id,country_code,startup_cmd,keepalive_s,term_theme,notes,sort_key,\
    last_connected_at,created_at,updated_at";

/// Maps a `HOST_COLS` row; `tag_ids` is filled by the caller.
pub(crate) fn host_from_row(r: &Row<'_>) -> rusqlite::Result<Host> {
    Ok(Host {
        id: r.get(0)?,
        folder_id: r.get(1)?,
        label: r.get(2)?,
        hostname: r.get(3)?,
        port: r.get(4)?,
        username: r.get(5)?,
        identity_id: r.get(6)?,
        jump_host_id: r.get(7)?,
        country_code: r.get(8)?,
        startup_cmd: r.get(9)?,
        keepalive_s: r.get(10)?,
        term_theme: r.get(11)?,
        notes: r.get(12)?,
        tag_ids: Vec::new(),
        sort_key: r.get(13)?,
        last_connected_at: r.get(14)?,
        created_at: r.get(15)?,
        updated_at: r.get(16)?,
    })
}

pub(crate) fn host_tags(conn: &Connection, id: HostId) -> Result<Vec<TagId>> {
    let mut st =
        conn.prepare_cached("SELECT tag_id FROM host_tag WHERE host_id=?1 ORDER BY tag_id")?;
    let rows = st.query_map([id], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub(crate) fn host_get(conn: &Connection, id: HostId) -> Result<Host> {
    let sql = format!("SELECT {HOST_COLS} FROM host WHERE id=?1");
    let mut h = conn.prepare_cached(&sql)?.query_row([id], host_from_row)?;
    h.tag_ids = host_tags(conn, id)?;
    Ok(h)
}

/// All `(host, tags)` links grouped by host.
pub(crate) fn all_host_tags(conn: &Connection) -> Result<HashMap<HostId, Vec<TagId>>> {
    let mut st = conn.prepare_cached("SELECT host_id, tag_id FROM host_tag ORDER BY tag_id")?;
    let mut map: HashMap<HostId, Vec<TagId>> = HashMap::new();
    for row in st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))? {
        let (h, t) = row?;
        map.entry(h).or_default().push(t);
    }
    Ok(map)
}

pub(crate) fn folder_exists(conn: &Connection, id: Option<FolderId>) -> Result<()> {
    if let Some(id) = id {
        let n: i64 = conn.query_row("SELECT count(*) FROM folder WHERE id=?1", [id], |r| {
            r.get(0)
        })?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
    }
    Ok(())
}

/// Inserts a host row and its tag links. `verb` is `INSERT` or `INSERT OR IGNORE`; returns
/// whether a row was written.
pub(crate) fn insert_host(conn: &Connection, h: &Host, verb: &str) -> Result<bool> {
    let sql = format!(
        "{verb} INTO host({HOST_COLS}) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)"
    );
    let n = conn.execute(
        &sql,
        params![
            h.id,
            h.folder_id,
            h.label,
            h.hostname,
            h.port,
            h.username,
            h.identity_id,
            h.jump_host_id,
            h.country_code,
            h.startup_cmd,
            h.keepalive_s,
            h.term_theme,
            h.notes,
            h.sort_key,
            h.last_connected_at,
            h.created_at,
            h.updated_at
        ],
    )?;
    if n == 0 {
        return Ok(false);
    }
    for t in &h.tag_ids {
        conn.execute(
            "INSERT OR IGNORE INTO host_tag(host_id, tag_id) VALUES(?1,?2)",
            params![h.id, t],
        )?;
    }
    fts_add(conn, h.id)?;
    Ok(true)
}

fn write_back(conn: &Connection, h: &Host) -> Result<()> {
    conn.execute(
        "UPDATE host SET label=?2, hostname=?3, port=?4, username=?5, identity_id=?6,
         jump_host_id=?7, country_code=?8, startup_cmd=?9, keepalive_s=?10, term_theme=?11,
         notes=?12, updated_at=?13 WHERE id=?1",
        params![
            h.id,
            h.label,
            h.hostname,
            h.port,
            h.username,
            h.identity_id,
            h.jump_host_id,
            h.country_code,
            h.startup_cmd,
            h.keepalive_s,
            h.term_theme,
            h.notes,
            h.updated_at
        ],
    )?;
    Ok(())
}

impl Store {
    /// Creates a host (appended last in its folder).
    pub fn host_create(&self, input: HostInput) -> Result<Host> {
        validate::name("label", &input.label)?;
        validate::hostname(&input.hostname)?;
        validate::port("port", input.port)?;
        let country = validate::country(input.country_code)?;
        self.write(|tx| {
            folder_exists(tx, input.folder_id)?;
            let now = now_ms();
            let h = Host {
                id: HostId::new_v7(),
                folder_id: input.folder_id,
                label: input.label,
                hostname: input.hostname,
                port: input.port,
                username: input.username,
                identity_id: input.identity_id,
                jump_host_id: input.jump_host_id,
                country_code: country,
                startup_cmd: input.startup_cmd,
                keepalive_s: input.keepalive_s,
                term_theme: input.term_theme,
                notes: input.notes,
                tag_ids: input.tag_ids,
                sort_key: append_key(tx, input.folder_id)?,
                last_connected_at: None,
                created_at: now,
                updated_at: now,
            };
            insert_host(tx, &h, "INSERT")?;
            if h.jump_host_id.is_some() {
                resolver::validate_jumps(tx)?;
            }
            host_get(tx, h.id)
        })
    }

    /// Updates host fields; `tag_ids: Some(v)` replaces the tag set.
    pub fn host_update(&self, id: HostId, patch: HostPatch) -> Result<Host> {
        if let Some(l) = &patch.label {
            validate::name("label", l)?;
        }
        if let Some(h) = &patch.hostname {
            validate::hostname(h)?;
        }
        validate::port("port", patch.port.flatten())?;
        let country = patch.country_code.map(validate::country).transpose()?;
        self.write(|tx| {
            let mut h = host_get(tx, id)?;
            fts_remove(tx, id)?;
            if let Some(v) = patch.label {
                h.label = v;
            }
            if let Some(v) = patch.hostname {
                h.hostname = v;
            }
            if let Some(v) = patch.port {
                h.port = v;
            }
            if let Some(v) = patch.username {
                h.username = v;
            }
            if let Some(v) = patch.identity_id {
                h.identity_id = v;
            }
            if let Some(v) = patch.jump_host_id {
                h.jump_host_id = v;
            }
            if let Some(v) = country {
                h.country_code = v;
            }
            if let Some(v) = patch.startup_cmd {
                h.startup_cmd = v;
            }
            if let Some(v) = patch.keepalive_s {
                h.keepalive_s = v;
            }
            if let Some(v) = patch.term_theme {
                h.term_theme = v;
            }
            if let Some(v) = patch.notes {
                h.notes = v;
            }
            h.updated_at = now_ms();
            write_back(tx, &h)?;
            if let Some(tags) = patch.tag_ids {
                tx.execute("DELETE FROM host_tag WHERE host_id=?1", [id])?;
                for t in tags {
                    tx.execute(
                        "INSERT OR IGNORE INTO host_tag(host_id, tag_id) VALUES(?1,?2)",
                        params![id, t],
                    )?;
                }
            }
            fts_add(tx, id)?;
            if h.jump_host_id.is_some() {
                resolver::validate_jumps(tx)?;
            }
            host_get(tx, id)
        })
    }

    /// Moves a host into `folder`, placed after `after` (`None` = first).
    pub fn host_move(
        &self,
        id: HostId,
        folder: Option<FolderId>,
        after: Option<NodeRef>,
    ) -> Result<()> {
        self.write(|tx| {
            host_get(tx, id)?;
            folder_exists(tx, folder)?;
            let key = key_after(tx, folder, Some(NodeRef::Host(id)), after)?;
            tx.execute(
                "UPDATE host SET folder_id=?2, sort_key=?3, updated_at=?4 WHERE id=?1",
                params![id, folder, key, now_ms()],
            )?;
            Ok(())
        })
    }

    /// Copies a host (label suffixed " copy"), placed right after the original.
    pub fn host_duplicate(&self, id: HostId) -> Result<Host> {
        self.write(|tx| {
            let mut h = host_get(tx, id)?;
            let mut label: String = h.label.chars().take(123).collect();
            label.push_str(" copy");
            h.sort_key = key_after(tx, h.folder_id, None, Some(NodeRef::Host(id)))?;
            h.id = HostId::new_v7();
            h.label = label;
            h.last_connected_at = None;
            h.created_at = now_ms();
            h.updated_at = h.created_at;
            insert_host(tx, &h, "INSERT")?;
            host_get(tx, h.id)
        })
    }

    /// Deletes hosts (one transaction); unknown ids are ignored.
    pub fn host_delete(&self, ids: &[HostId]) -> Result<()> {
        self.write(|tx| {
            for id in ids {
                fts_remove(tx, *id)?;
                tx.execute("DELETE FROM host WHERE id=?1", [id])?;
            }
            Ok(())
        })
    }

    /// Sets `last_connected_at = now`.
    pub fn host_touch_connected(&self, id: HostId) -> Result<()> {
        self.write(|tx| {
            let n = tx.execute(
                "UPDATE host SET last_connected_at=?2 WHERE id=?1",
                params![id, now_ms()],
            )?;
            if n == 0 {
                return Err(StoreError::NotFound);
            }
            Ok(())
        })
    }
}
