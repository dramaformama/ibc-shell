//! Sibling ordering: folders and hosts under one parent share one `sort_key` space.

use rusqlite::Connection;

use crate::ids::{FolderId, HostId};
use crate::sort_key::between;
use crate::tree::NodeRef;
use crate::{Result, StoreError};

fn exhausted() -> StoreError {
    StoreError::Db("sort key space exhausted or corrupt".into())
}

/// Ordered `(node, sort_key)` children of `parent` (folders and hosts interleaved).
fn siblings(conn: &Connection, parent: Option<FolderId>) -> Result<Vec<(NodeRef, String)>> {
    let mut st = conn.prepare_cached(
        "SELECT 0, id, sort_key FROM folder WHERE parent_id IS ?1
         UNION ALL SELECT 1, id, sort_key FROM host WHERE folder_id IS ?1 ORDER BY 3",
    )?;
    let rows = st.query_map([parent], |r| {
        let kind: i64 = r.get(0)?;
        let key: String = r.get(2)?;
        let node = if kind == 0 {
            NodeRef::Folder(r.get::<_, FolderId>(1)?)
        } else {
            NodeRef::Host(r.get::<_, HostId>(1)?)
        };
        Ok((node, key))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Key placing a new node last under `parent`.
pub(crate) fn append_key(conn: &Connection, parent: Option<FolderId>) -> Result<String> {
    let sib = siblings(conn, parent)?;
    between(sib.last().map(|s| s.1.as_str()), None).ok_or_else(exhausted)
}

/// Key placing `moving` under `parent` right after `after` (`None` = first). `moving` itself is
/// ignored when listing siblings.
pub(crate) fn key_after(
    conn: &Connection,
    parent: Option<FolderId>,
    moving: Option<NodeRef>,
    after: Option<NodeRef>,
) -> Result<String> {
    let sib: Vec<_> = siblings(conn, parent)?
        .into_iter()
        .filter(|(n, _)| Some(*n) != moving)
        .collect();
    let key = match after {
        None => between(None, sib.first().map(|s| s.1.as_str())),
        Some(a) => {
            let i = sib
                .iter()
                .position(|(n, _)| *n == a)
                .ok_or_else(|| StoreError::Validation("after: not a sibling".into()))?;
            between(Some(&sib[i].1), sib.get(i + 1).map(|s| s.1.as_str()))
        }
    };
    key.ok_or_else(exhausted)
}
