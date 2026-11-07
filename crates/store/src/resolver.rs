//! Inheritance resolution: host field → nearest ancestor folder default → built-in default.

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension};

use crate::db::{query_all, Store};
use crate::folders::{folder_from_row, FOLDER_COLS};
use crate::hosts::host_get;
use crate::ids::{FolderId, HostId};
use crate::resolve::{Resolved, ResolvedHost, Source};
use crate::tree::{Folder, Host};
use crate::{Result, StoreError};

const MAX_JUMPS: usize = 3;
const DEFAULT_PORT: u16 = 22;
const DEFAULT_KEEPALIVE: u32 = 30;

fn load_folders(conn: &Connection) -> Result<HashMap<FolderId, Folder>> {
    let sql = format!("SELECT {FOLDER_COLS} FROM folder");
    Ok(query_all(conn, &sql, [], folder_from_row)?
        .into_iter()
        .map(|f| (f.id, f))
        .collect())
}

/// Folders from the host's folder up to the root (nearest first).
fn ancestors(folders: &HashMap<FolderId, Folder>, start: Option<FolderId>) -> Vec<&Folder> {
    let mut out = Vec::new();
    let mut cur = start;
    while let Some(id) = cur {
        match folders.get(&id) {
            Some(f) if out.len() < 64 => {
                out.push(f);
                cur = f.parent_id;
            }
            _ => break,
        }
    }
    out
}

fn pick<T: Clone>(
    own: Option<T>,
    chain: &[&Folder],
    get: impl Fn(&Folder) -> Option<T>,
) -> Option<(T, Source)> {
    if let Some(v) = own {
        return Some((v, Source::Host));
    }
    chain
        .iter()
        .find_map(|f| get(f).map(|v| (v, Source::Folder(f.id))))
}

fn or_default<T>(p: Option<(T, Source)>, default: T) -> Resolved<T> {
    match p {
        Some((value, source)) => Resolved { value, source },
        None => Resolved {
            value: default,
            source: Source::Default,
        },
    }
}

fn opt<T>(p: Option<(T, Source)>) -> Resolved<Option<T>> {
    match p {
        Some((v, source)) => Resolved {
            value: Some(v),
            source,
        },
        None => Resolved {
            value: None,
            source: Source::Default,
        },
    }
}

fn effective_jump(folders: &HashMap<FolderId, Folder>, h: &Host) -> Option<HostId> {
    pick(h.jump_host_id, &ancestors(folders, h.folder_id), |f| {
        f.defaults.jump_host_id
    })
    .map(|p| p.0)
}

/// Jump chain of `id` (outermost first), following inherited jump hosts. Max 3 hops.
pub(crate) fn jump_chain(conn: &Connection, id: HostId) -> Result<Vec<HostId>> {
    let folders = load_folders(conn)?;
    let mut host = host_get(conn, id)?;
    let mut seen = HashSet::from([id]);
    let mut chain = Vec::new();
    while let Some(j) = effective_jump(&folders, &host) {
        if !seen.insert(j) || chain.len() >= MAX_JUMPS {
            return Err(StoreError::JumpCycle);
        }
        chain.push(j);
        host = host_get(conn, j)?;
    }
    chain.reverse();
    Ok(chain)
}

/// Checks the jump-chain invariant for every host (cycles, > 3 hops). Run after any write that
/// can change an effective jump host, since it may break hosts upstream of the edited one.
pub(crate) fn validate_jumps(conn: &Connection) -> Result<()> {
    let folders = load_folders(conn)?;
    let rows = query_all(
        conn,
        "SELECT id, folder_id, jump_host_id FROM host",
        [],
        |r| {
            Ok((
                r.get::<_, HostId>(0)?,
                (
                    r.get::<_, Option<FolderId>>(1)?,
                    r.get::<_, Option<HostId>>(2)?,
                ),
            ))
        },
    )?;
    let hosts: HashMap<HostId, (Option<FolderId>, Option<HostId>)> = rows.into_iter().collect();
    let next = |(folder, own): &(Option<FolderId>, Option<HostId>)| {
        pick(*own, &ancestors(&folders, *folder), |f| {
            f.defaults.jump_host_id
        })
        .map(|p| p.0)
    };
    for (&id, entry) in &hosts {
        let mut seen = HashSet::from([id]);
        let mut hops = 0;
        let mut cur = next(entry);
        while let Some(j) = cur {
            hops += 1;
            if !seen.insert(j) || hops > MAX_JUMPS {
                return Err(StoreError::JumpCycle);
            }
            cur = hosts.get(&j).and_then(next);
        }
    }
    Ok(())
}

impl Store {
    /// Resolves inheritance for a host.
    pub fn host_resolve(&self, id: HostId) -> Result<ResolvedHost> {
        self.read(|c| {
            let h = host_get(c, id)?;
            let folders = load_folders(c)?;
            let chain = ancestors(&folders, h.folder_id);

            let identity = pick(h.identity_id, &chain, |f| f.defaults.identity_id);
            let username = if let Some((v, s)) =
                pick(h.username.clone(), &chain, |f| f.defaults.username.clone())
            {
                Resolved {
                    value: Some(v),
                    source: s,
                }
            } else {
                let from_identity: Option<String> = match identity {
                    Some((iid, _)) => c
                        .query_row("SELECT username FROM identity WHERE id=?1", [iid], |r| {
                            r.get(0)
                        })
                        .optional()?
                        .flatten(),
                    None => None,
                };
                Resolved {
                    value: from_identity,
                    source: Source::Default,
                }
            };
            Ok(ResolvedHost {
                id,
                label: h.label.clone(),
                hostname: h.hostname.clone(),
                port: or_default(pick(h.port, &chain, |f| f.defaults.port), DEFAULT_PORT),
                username,
                identity_id: opt(identity),
                jump_chain: jump_chain(c, id)?,
                country_code: opt(pick(h.country_code.clone(), &chain, |f| {
                    f.country_code.clone()
                })),
                startup_cmd: opt(pick(h.startup_cmd.clone(), &chain, |f| {
                    f.defaults.startup_cmd.clone()
                })),
                keepalive_s: or_default(
                    pick(h.keepalive_s, &chain, |f| f.defaults.keepalive_s),
                    DEFAULT_KEEPALIVE,
                ),
                term_theme: opt(pick(h.term_theme.clone(), &chain, |f| {
                    f.defaults.term_theme.clone()
                })),
            })
        })
    }
}
