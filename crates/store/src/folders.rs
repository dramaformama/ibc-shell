//! Folder CRUD, move (cycle / depth checks), delete modes, and `tree_list`.

use std::collections::HashMap;

use rusqlite::{params, Connection, Row};

use crate::db::{now_ms, Store};
use crate::hosts::{all_host_tags, folder_exists, host_from_row, HOST_COLS};
use crate::ids::{FolderId, HostId};
use crate::search::fts_remove;
use crate::siblings::{append_key, key_after};
use crate::tree::{
    Folder, FolderDefaults, FolderDeleteMode, FolderInput, FolderPatch, NodeRef, Tree,
};
use crate::{resolver, validate, Result, StoreError};

const MAX_DEPTH: usize = 8;

pub(crate) const FOLDER_COLS: &str = "id,parent_id,name,country_code,color,icon,sort_key,expanded,\
    d_username,d_port,d_identity_id,d_jump_host_id,d_startup_cmd,d_keepalive_s,d_term_theme,\
    created_at,updated_at";

pub(crate) fn folder_from_row(r: &Row<'_>) -> rusqlite::Result<Folder> {
    Ok(Folder {
        id: r.get(0)?,
        parent_id: r.get(1)?,
        name: r.get(2)?,
        country_code: r.get(3)?,
        color: r.get(4)?,
        icon: r.get(5)?,
        sort_key: r.get(6)?,
        expanded: r.get(7)?,
        defaults: FolderDefaults {
            username: r.get(8)?,
            port: r.get(9)?,
            identity_id: r.get(10)?,
            jump_host_id: r.get(11)?,
            startup_cmd: r.get(12)?,
            keepalive_s: r.get(13)?,
            term_theme: r.get(14)?,
        },
        created_at: r.get(15)?,
        updated_at: r.get(16)?,
    })
}

pub(crate) fn folder_get(conn: &Connection, id: FolderId) -> Result<Folder> {
    let sql = format!("SELECT {FOLDER_COLS} FROM folder WHERE id=?1");
    Ok(conn
        .prepare_cached(&sql)?
        .query_row([id], folder_from_row)?)
}

/// `id -> parent` for every folder.
fn parent_map(conn: &Connection) -> Result<HashMap<FolderId, Option<FolderId>>> {
    let mut st = conn.prepare_cached("SELECT id, parent_id FROM folder")?;
    let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Depth of `id` (root folder = 1).
fn depth(map: &HashMap<FolderId, Option<FolderId>>, id: FolderId) -> usize {
    let mut d = 1;
    let mut cur = map.get(&id).copied().flatten();
    while let Some(p) = cur {
        d += 1;
        if d > 64 {
            break;
        }
        cur = map.get(&p).copied().flatten();
    }
    d
}

/// Number of levels in the subtree rooted at `id` (leaf = 1).
fn height(map: &HashMap<FolderId, Option<FolderId>>, id: FolderId) -> usize {
    let mut best = 1;
    for (&child, &parent) in map {
        if parent == Some(id) && child != id {
            best = best.max(1 + height(map, child));
        }
    }
    best
}

/// Subtree ids of `id` including itself, parents before children.
fn subtree(map: &HashMap<FolderId, Option<FolderId>>, id: FolderId) -> Vec<FolderId> {
    let mut out = vec![id];
    let mut i = 0;
    while i < out.len() {
        let cur = out[i];
        let mut kids: Vec<FolderId> = map
            .iter()
            .filter(|(_, p)| **p == Some(cur))
            .map(|(c, _)| *c)
            .collect();
        kids.sort();
        out.extend(kids);
        i += 1;
    }
    out
}

fn check_defaults(d: &FolderDefaults) -> Result<()> {
    validate::port("port", d.port)
}

impl Store {
    /// Whole tree, flat, sorted by (parent, `sort_key`).
    pub fn tree_list(&self) -> Result<Tree> {
        self.read(load_tree)
    }

    /// Creates a folder (appended last among siblings).
    pub fn folder_create(&self, input: FolderInput) -> Result<Folder> {
        validate::name("name", &input.name)?;
        check_defaults(&input.defaults)?;
        let country = validate::country(input.country_code)?;
        self.write(|tx| {
            if let Some(p) = input.parent_id {
                folder_exists(tx, Some(p))?;
                if depth(&parent_map(tx)?, p) + 1 > MAX_DEPTH {
                    return Err(StoreError::TooDeep);
                }
            }
            let now = now_ms();
            let f = Folder {
                id: FolderId::new_v7(),
                parent_id: input.parent_id,
                name: input.name,
                country_code: country,
                color: input.color,
                icon: input.icon,
                sort_key: append_key(tx, input.parent_id)?,
                expanded: true,
                defaults: input.defaults,
                created_at: now,
                updated_at: now,
            };
            insert_folder(tx, &f, "INSERT")?;
            folder_get(tx, f.id)
        })
    }

    /// Updates folder fields (parent changes go through `folder_move`).
    pub fn folder_update(&self, id: FolderId, patch: FolderPatch) -> Result<Folder> {
        if let Some(n) = &patch.name {
            validate::name("name", n)?;
        }
        validate::port("port", patch.defaults.port.flatten())?;
        let country = patch.country_code.map(validate::country).transpose()?;
        self.write(|tx| {
            let mut f = folder_get(tx, id)?;
            if let Some(v) = patch.name {
                f.name = v;
            }
            if let Some(v) = country {
                f.country_code = v;
            }
            if let Some(v) = patch.color {
                f.color = v;
            }
            if let Some(v) = patch.icon {
                f.icon = v;
            }
            if let Some(v) = patch.expanded {
                f.expanded = v;
            }
            let p_jump_touched = patch.defaults.jump_host_id.is_some();
            let (d, p) = (&mut f.defaults, patch.defaults);
            if let Some(v) = p.username {
                d.username = v;
            }
            if let Some(v) = p.port {
                d.port = v;
            }
            if let Some(v) = p.identity_id {
                d.identity_id = v;
            }
            if let Some(v) = p.jump_host_id {
                d.jump_host_id = v;
            }
            if let Some(v) = p.startup_cmd {
                d.startup_cmd = v;
            }
            if let Some(v) = p.keepalive_s {
                d.keepalive_s = v;
            }
            if let Some(v) = p.term_theme {
                d.term_theme = v;
            }
            f.updated_at = now_ms();
            let jump_touched = p_jump_touched;
            tx.execute(
                "UPDATE folder SET name=?2, country_code=?3, color=?4, icon=?5, expanded=?6,
                 d_username=?7, d_port=?8, d_identity_id=?9, d_jump_host_id=?10,
                 d_startup_cmd=?11, d_keepalive_s=?12, d_term_theme=?13, updated_at=?14
                 WHERE id=?1",
                params![
                    id,
                    f.name,
                    f.country_code,
                    f.color,
                    f.icon,
                    f.expanded,
                    f.defaults.username,
                    f.defaults.port,
                    f.defaults.identity_id,
                    f.defaults.jump_host_id,
                    f.defaults.startup_cmd,
                    f.defaults.keepalive_s,
                    f.defaults.term_theme,
                    f.updated_at
                ],
            )?;
            if jump_touched {
                resolver::validate_jumps(tx)?;
            }
            Ok(f)
        })
    }

    /// Moves a folder under `parent`, after `after` (`None` = first). Rejects cycles / depth > 8.
    pub fn folder_move(
        &self,
        id: FolderId,
        parent: Option<FolderId>,
        after: Option<NodeRef>,
    ) -> Result<()> {
        self.write(|tx| {
            folder_get(tx, id)?;
            if let Some(p) = parent {
                folder_exists(tx, Some(p))?;
                let map = parent_map(tx)?;
                // Walk up from the new parent: meeting `id` means it is a descendant (or itself).
                let mut cur = Some(p);
                let mut guard = 0;
                while let Some(c) = cur {
                    if c == id {
                        return Err(StoreError::Cycle);
                    }
                    guard += 1;
                    if guard > 64 {
                        return Err(StoreError::Cycle);
                    }
                    cur = map.get(&c).copied().flatten();
                }
                if depth(&map, p) + height(&map, id) > MAX_DEPTH {
                    return Err(StoreError::TooDeep);
                }
            }
            let key = key_after(tx, parent, Some(NodeRef::Folder(id)), after)?;
            tx.execute(
                "UPDATE folder SET parent_id=?2, sort_key=?3, updated_at=?4 WHERE id=?1",
                params![id, parent, key, now_ms()],
            )?;
            Ok(())
        })
    }

    /// Deletes a folder per `mode` (invariant 4). Identities are global and never touched.
    pub fn folder_delete(&self, id: FolderId, mode: FolderDeleteMode) -> Result<()> {
        self.write(|tx| {
            let f = folder_get(tx, id)?;
            match mode {
                FolderDeleteMode::MoveToParent => {
                    // Children keep their relative order, appended after the grandparent's own.
                    let kids = ordered_children(tx, id)?;
                    for k in kids {
                        let key = append_key(tx, f.parent_id)?;
                        match k {
                            NodeRef::Folder(c) => tx.execute(
                                "UPDATE folder SET parent_id=?2, sort_key=?3 WHERE id=?1",
                                params![c, f.parent_id, key],
                            )?,
                            NodeRef::Host(h) => tx.execute(
                                "UPDATE host SET folder_id=?2, sort_key=?3 WHERE id=?1",
                                params![h, f.parent_id, key],
                            )?,
                        };
                    }
                    // Snippets would be cascade-deleted: hand them to the parent instead.
                    tx.execute(
                        "UPDATE snippet SET folder_id=?2 WHERE folder_id=?1",
                        params![id, f.parent_id],
                    )?;
                    tx.execute("DELETE FROM folder WHERE id=?1", [id])?;
                }
                FolderDeleteMode::DeleteContents => {
                    let map = parent_map(tx)?;
                    let mut ids = subtree(&map, id);
                    for fid in &ids {
                        let hosts: Vec<HostId> = tx
                            .prepare_cached("SELECT id FROM host WHERE folder_id=?1")?
                            .query_map([fid], |r| r.get(0))?
                            .collect::<rusqlite::Result<_>>()?;
                        for h in hosts {
                            fts_remove(tx, h)?;
                            tx.execute("DELETE FROM host WHERE id=?1", [h])?;
                        }
                    }
                    ids.reverse(); // children first (parent_id is ON DELETE RESTRICT)
                    for fid in ids {
                        tx.execute("DELETE FROM folder WHERE id=?1", [fid])?;
                    }
                }
            }
            Ok(())
        })
    }
}

fn ordered_children(conn: &Connection, id: FolderId) -> Result<Vec<NodeRef>> {
    let mut st = conn.prepare_cached(
        "SELECT 0, id, sort_key FROM folder WHERE parent_id=?1
         UNION ALL SELECT 1, id, sort_key FROM host WHERE folder_id=?1 ORDER BY 3",
    )?;
    let rows = st.query_map([id], |r| {
        Ok(if r.get::<_, i64>(0)? == 0 {
            NodeRef::Folder(r.get(1)?)
        } else {
            NodeRef::Host(r.get(1)?)
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Inserts a folder row; `verb` is `INSERT` or `INSERT OR IGNORE`. Returns whether it was written.
pub(crate) fn insert_folder(conn: &Connection, f: &Folder, verb: &str) -> Result<bool> {
    let sql = format!(
        "{verb} INTO folder({FOLDER_COLS}) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)"
    );
    let d = &f.defaults;
    let n = conn.execute(
        &sql,
        params![
            f.id,
            f.parent_id,
            f.name,
            f.country_code,
            f.color,
            f.icon,
            f.sort_key,
            f.expanded,
            d.username,
            d.port,
            d.identity_id,
            d.jump_host_id,
            d.startup_cmd,
            d.keepalive_s,
            d.term_theme,
            f.created_at,
            f.updated_at
        ],
    )?;
    Ok(n > 0)
}

/// Reads every folder and host (hosts with tags), sorted by (parent, `sort_key`).
pub(crate) fn load_tree(c: &Connection) -> Result<Tree> {
    let sql = format!("SELECT {FOLDER_COLS} FROM folder ORDER BY parent_id, sort_key");
    let folders = c
        .prepare_cached(&sql)?
        .query_map([], folder_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let sql = format!("SELECT {HOST_COLS} FROM host ORDER BY folder_id, sort_key");
    let mut hosts = c
        .prepare_cached(&sql)?
        .query_map([], host_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut tags = all_host_tags(c)?;
    for h in &mut hosts {
        h.tag_ids = tags.remove(&h.id).unwrap_or_default();
    }
    Ok(Tree { folders, hosts })
}
