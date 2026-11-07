//! Tags, snippets, port forwards.

use rusqlite::{params, Connection, Row};

use crate::catalog::{
    ForwardInput, ForwardKind, ForwardPatch, PortForward, Snippet, SnippetInput, SnippetPatch, Tag,
    TagInput, TagPatch,
};
use crate::db::{now_ms, query_all, Store};
use crate::hosts::folder_exists;
use crate::ids::{FolderId, ForwardId, HostId, SnippetId, TagId};
use crate::search::{fts_add, fts_remove, hosts_with_tag};
use crate::sort_key::between;
use crate::{validate, Result, StoreError};

// ---------------- tags ----------------

fn tag_from_row(r: &Row<'_>) -> rusqlite::Result<Tag> {
    Ok(Tag {
        id: r.get(0)?,
        name: r.get(1)?,
        color: r.get(2)?,
    })
}

pub(crate) fn tag_get(conn: &Connection, id: TagId) -> Result<Tag> {
    Ok(conn.query_row(
        "SELECT id, name, color FROM tag WHERE id=?1",
        [id],
        tag_from_row,
    )?)
}

fn dup_tag(e: StoreError) -> StoreError {
    match e {
        StoreError::Conflict(_) => StoreError::Conflict("tag name already exists".into()),
        o => o,
    }
}

impl Store {
    /// All tags by name.
    pub fn tag_list(&self) -> Result<Vec<Tag>> {
        self.read(|c| {
            query_all(
                c,
                "SELECT id, name, color FROM tag ORDER BY name COLLATE NOCASE",
                [],
                tag_from_row,
            )
        })
    }

    /// Creates a tag (`Conflict` on duplicate name, case-insensitive).
    pub fn tag_create(&self, input: TagInput) -> Result<Tag> {
        validate::name("name", &input.name)?;
        self.write(|tx| {
            let id = TagId::new_v7();
            tx.execute(
                "INSERT INTO tag(id, name, color) VALUES(?1,?2,?3)",
                params![id, input.name, input.color],
            )
            .map_err(|e| dup_tag(e.into()))?;
            tag_get(tx, id)
        })
    }

    /// Updates a tag; renaming re-indexes the hosts that carry it.
    pub fn tag_update(&self, id: TagId, patch: TagPatch) -> Result<Tag> {
        if let Some(n) = &patch.name {
            validate::name("name", n)?;
        }
        self.write(|tx| {
            let mut t = tag_get(tx, id)?;
            let hosts = hosts_with_tag(tx, id)?;
            for h in &hosts {
                fts_remove(tx, *h)?;
            }
            if let Some(v) = patch.name {
                t.name = v;
            }
            if let Some(v) = patch.color {
                t.color = v;
            }
            tx.execute(
                "UPDATE tag SET name=?2, color=?3 WHERE id=?1",
                params![id, t.name, t.color],
            )
            .map_err(|e| dup_tag(e.into()))?;
            for h in &hosts {
                fts_add(tx, *h)?;
            }
            Ok(t)
        })
    }

    /// Deletes a tag (detaches it from hosts).
    pub fn tag_delete(&self, id: TagId) -> Result<()> {
        self.write(|tx| {
            tag_get(tx, id)?;
            let hosts = hosts_with_tag(tx, id)?;
            for h in &hosts {
                fts_remove(tx, *h)?;
            }
            tx.execute("DELETE FROM tag WHERE id=?1", [id])?;
            for h in &hosts {
                fts_add(tx, *h)?;
            }
            Ok(())
        })
    }

    // ---------------- snippets ----------------

    /// Snippets; `folder = Some` returns global + that folder's snippets.
    pub fn snippet_list(&self, folder: Option<FolderId>) -> Result<Vec<Snippet>> {
        self.read(|c| {
            let sql = format!(
                "{SNIPPET_SELECT} WHERE folder_id IS NULL OR folder_id=?1
                 ORDER BY folder_id IS NOT NULL, sort_key"
            );
            query_all(c, &sql, [folder], snippet_from_row)
        })
    }

    /// Creates a snippet (appended last in its scope).
    pub fn snippet_create(&self, input: SnippetInput) -> Result<Snippet> {
        validate::name("name", &input.name)?;
        self.write(|tx| {
            folder_exists(tx, input.folder_id)?;
            let last: Option<String> = tx.query_row(
                "SELECT max(sort_key) FROM snippet WHERE folder_id IS ?1",
                [input.folder_id],
                |r| r.get(0),
            )?;
            let key = between(last.as_deref(), None)
                .ok_or_else(|| StoreError::Db("sort key space exhausted".into()))?;
            let id = SnippetId::new_v7();
            let now = now_ms();
            tx.execute(
                "INSERT INTO snippet(id, folder_id, name, body, sort_key, created_at, updated_at)
                 VALUES(?1,?2,?3,?4,?5,?6,?6)",
                params![id, input.folder_id, input.name, input.body, key, now],
            )?;
            snippet_get(tx, id)
        })
    }

    /// Updates a snippet.
    pub fn snippet_update(&self, id: SnippetId, patch: SnippetPatch) -> Result<Snippet> {
        if let Some(n) = &patch.name {
            validate::name("name", n)?;
        }
        self.write(|tx| {
            let mut s = snippet_get(tx, id)?;
            if let Some(v) = patch.folder_id {
                folder_exists(tx, v)?;
                s.folder_id = v;
            }
            if let Some(v) = patch.name {
                s.name = v;
            }
            if let Some(v) = patch.body {
                s.body = v;
            }
            tx.execute(
                "UPDATE snippet SET folder_id=?2, name=?3, body=?4, updated_at=?5 WHERE id=?1",
                params![id, s.folder_id, s.name, s.body, now_ms()],
            )?;
            snippet_get(tx, id)
        })
    }

    /// Deletes a snippet.
    pub fn snippet_delete(&self, id: SnippetId) -> Result<()> {
        self.write(|tx| delete_one(tx, "snippet", id))
    }
}

const SNIPPET_SELECT: &str =
    "SELECT id, folder_id, name, body, sort_key, created_at, updated_at FROM snippet";

pub(crate) fn snippet_from_row(r: &Row<'_>) -> rusqlite::Result<Snippet> {
    Ok(Snippet {
        id: r.get(0)?,
        folder_id: r.get(1)?,
        name: r.get(2)?,
        body: r.get(3)?,
        sort_key: r.get(4)?,
        created_at: r.get(5)?,
        updated_at: r.get(6)?,
    })
}

fn snippet_get(conn: &Connection, id: SnippetId) -> Result<Snippet> {
    Ok(conn.query_row(
        &format!("{SNIPPET_SELECT} WHERE id=?1"),
        [id],
        snippet_from_row,
    )?)
}

fn delete_one(conn: &Connection, table: &str, id: impl rusqlite::ToSql) -> Result<()> {
    let n = conn.execute(&format!("DELETE FROM {table} WHERE id=?1"), [id])?;
    if n == 0 {
        return Err(StoreError::NotFound);
    }
    Ok(())
}

// ---------------- forwards ----------------

const FORWARD_SELECT: &str =
    "SELECT id, host_id, kind, bind_addr, bind_port, dest_host, dest_port, auto_start, label FROM port_forward";

fn kind_str(k: ForwardKind) -> &'static str {
    match k {
        ForwardKind::Local => "local",
        ForwardKind::Remote => "remote",
        ForwardKind::Dynamic => "dynamic",
    }
}

pub(crate) fn forward_from_row(r: &Row<'_>) -> rusqlite::Result<PortForward> {
    let kind: String = r.get(2)?;
    let kind = match kind.as_str() {
        "local" => ForwardKind::Local,
        "remote" => ForwardKind::Remote,
        "dynamic" => ForwardKind::Dynamic,
        _ => {
            return Err(rusqlite::Error::InvalidColumnType(
                2,
                "kind".into(),
                rusqlite::types::Type::Text,
            ))
        }
    };
    Ok(PortForward {
        id: r.get(0)?,
        host_id: r.get(1)?,
        kind,
        bind_addr: r.get(3)?,
        bind_port: r.get(4)?,
        dest_host: r.get(5)?,
        dest_port: r.get(6)?,
        auto_start: r.get(7)?,
        label: r.get(8)?,
    })
}

fn forward_get(conn: &Connection, id: ForwardId) -> Result<PortForward> {
    Ok(conn.query_row(
        &format!("{FORWARD_SELECT} WHERE id=?1"),
        [id],
        forward_from_row,
    )?)
}

fn check_forward(f: &PortForward) -> Result<()> {
    validate::port("bind_port", Some(f.bind_port))?;
    validate::port("dest_port", f.dest_port)?;
    if f.bind_addr.trim().is_empty() {
        return Err(StoreError::Validation("bind_addr: empty".into()));
    }
    if f.kind != ForwardKind::Dynamic && (f.dest_host.is_none() || f.dest_port.is_none()) {
        return Err(StoreError::Validation(
            "destination required for local/remote forward".into(),
        ));
    }
    Ok(())
}

impl Store {
    /// Forwards; `host = Some` filters by host.
    pub fn forward_list(&self, host: Option<HostId>) -> Result<Vec<PortForward>> {
        self.read(|c| {
            let sql = format!(
                "{FORWARD_SELECT} WHERE ?1 IS NULL OR host_id=?1 ORDER BY host_id, bind_port, id"
            );
            query_all(c, &sql, [host], forward_from_row)
        })
    }

    /// Creates a forward rule.
    pub fn forward_create(&self, input: ForwardInput) -> Result<PortForward> {
        let f = PortForward {
            id: ForwardId::new_v7(),
            host_id: input.host_id,
            kind: input.kind,
            bind_addr: input.bind_addr.unwrap_or_else(|| "127.0.0.1".into()),
            bind_port: input.bind_port,
            dest_host: input.dest_host,
            dest_port: input.dest_port,
            auto_start: input.auto_start,
            label: input.label,
        };
        check_forward(&f)?;
        self.write(|tx| {
            insert_forward(tx, &f, "INSERT")?;
            forward_get(tx, f.id)
        })
    }

    /// Updates a forward rule.
    pub fn forward_update(&self, id: ForwardId, patch: ForwardPatch) -> Result<PortForward> {
        self.write(|tx| {
            let mut f = forward_get(tx, id)?;
            if let Some(v) = patch.kind {
                f.kind = v;
            }
            if let Some(v) = patch.bind_addr {
                f.bind_addr = v;
            }
            if let Some(v) = patch.bind_port {
                f.bind_port = v;
            }
            if let Some(v) = patch.dest_host {
                f.dest_host = v;
            }
            if let Some(v) = patch.dest_port {
                f.dest_port = v;
            }
            if let Some(v) = patch.auto_start {
                f.auto_start = v;
            }
            if let Some(v) = patch.label {
                f.label = v;
            }
            check_forward(&f)?;
            tx.execute(
                "UPDATE port_forward SET kind=?2, bind_addr=?3, bind_port=?4, dest_host=?5,
                 dest_port=?6, auto_start=?7, label=?8 WHERE id=?1",
                params![
                    id,
                    kind_str(f.kind),
                    f.bind_addr,
                    f.bind_port,
                    f.dest_host,
                    f.dest_port,
                    f.auto_start,
                    f.label
                ],
            )?;
            Ok(f)
        })
    }

    /// Deletes a forward rule.
    pub fn forward_delete(&self, id: ForwardId) -> Result<()> {
        self.write(|tx| delete_one(tx, "port_forward", id))
    }
}

pub(crate) fn insert_forward(conn: &Connection, f: &PortForward, verb: &str) -> Result<bool> {
    let n = conn.execute(
        &format!(
            "{verb} INTO port_forward(id, host_id, kind, bind_addr, bind_port, dest_host, dest_port, auto_start, label)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)"
        ),
        params![f.id, f.host_id, kind_str(f.kind), f.bind_addr, f.bind_port, f.dest_host, f.dest_port, f.auto_start, f.label],
    )?;
    Ok(n > 0)
}
