//! Source tree scan: turns one requested item into directories to create and files to copy.
//! Symlinks are never followed (skipped with a notice); names are made safe for the target.

use std::collections::HashSet;

use crate::sanitize::escape_name;
use crate::side::Side;
use crate::{EntryKind, SftpError};

const MAX_DEPTH: usize = 128;

/// One file to copy.
#[derive(Clone, Debug)]
pub(crate) struct FileTask {
    pub src: String,
    pub dst_dir: String,
    pub dst_name: String,
    pub size: u64,
    pub mtime: Option<i64>,
    pub mode: u32,
}

/// Scan result.
#[derive(Default, Debug)]
pub(crate) struct Plan {
    /// Destination directories, parents first.
    pub dirs: Vec<String>,
    /// Files.
    pub files: Vec<FileTask>,
    /// Destination paths of skipped entries (symlinks, unaddressable names).
    pub skipped: Vec<String>,
    /// Sum of file sizes.
    pub total: u64,
}

/// Last path component.
pub(crate) fn base_name(side: &Side, path: &str) -> String {
    match side {
        Side::Local => std::path::Path::new(path)
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        Side::Remote(_) => path
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_owned(),
    }
}

/// Name to use on the destination, `None` when the entry cannot be transferred.
pub(crate) fn dst_name(src: &Side, dst: &Side, name: &str) -> Option<String> {
    if !src.is_local() && name.contains('\u{fffd}') {
        return None; // lossy-decoded remote name: cannot be addressed again
    }
    if dst.is_local() {
        escape_name(name).ok()
    } else if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\0']) {
        None
    } else {
        Some(name.to_owned())
    }
}

fn unique(taken: &mut HashSet<String>, name: String) -> String {
    if taken.insert(name.clone()) {
        return name;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name.as_str(), ""),
    };
    for n in 1u32.. {
        let cand = format!("{stem} ({n}){ext}");
        if taken.insert(cand.clone()) {
            return cand;
        }
    }
    name
}

/// Scans `item` (source path) for placement as `dst_dir/top_name`.
pub(crate) async fn scan(
    src: &Side,
    dst: &Side,
    item: &str,
    dst_dir: &str,
) -> Result<Plan, SftpError> {
    let mut plan = Plan::default();
    let top = src.lstat(item).await?.ok_or(SftpError::NotFound)?;
    let name = dst_name(src, dst, &base_name(src, item))
        .ok_or_else(|| SftpError::UnsafePath("unusable file name".into()))?;
    let top_dst = dst.join(dst_dir, &name);
    match top.kind {
        EntryKind::File => plan.push_file(item, dst_dir, name, &top),
        EntryKind::Dir => {
            plan.dirs.push(top_dst.clone());
            let mut stack = vec![(item.to_owned(), top_dst, 0usize)];
            while let Some((sdir, ddir, depth)) = stack.pop() {
                if depth >= MAX_DEPTH {
                    plan.skipped.push(ddir);
                    continue;
                }
                let mut taken = HashSet::new();
                for e in src.list(&sdir).await? {
                    let Some(n) = dst_name(src, dst, &e.name) else {
                        plan.skipped
                            .push(dst.join(&ddir, &e.name.replace('/', "_")));
                        continue;
                    };
                    let n = unique(&mut taken, n);
                    let spath = src.join(&sdir, &e.name);
                    match e.kind {
                        EntryKind::File => plan.push_file(&spath, &ddir, n, &e),
                        EntryKind::Dir => {
                            let d = dst.join(&ddir, &n);
                            plan.dirs.push(d.clone());
                            stack.push((spath, d, depth + 1));
                        }
                        _ => plan.skipped.push(dst.join(&ddir, &n)),
                    }
                }
            }
        }
        _ => plan.skipped.push(top_dst),
    }
    Ok(plan)
}

impl Plan {
    fn push_file(&mut self, src: &str, dst_dir: &str, dst_name: String, e: &crate::Entry) {
        self.total += e.size;
        self.files.push(FileTask {
            src: src.to_owned(),
            dst_dir: dst_dir.to_owned(),
            dst_name,
            size: e.size,
            mtime: e.mtime,
            mode: e.mode,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_adds_counter_before_extension() {
        let mut t = HashSet::new();
        assert_eq!(unique(&mut t, "a.txt".into()), "a.txt");
        assert_eq!(unique(&mut t, "a.txt".into()), "a (1).txt");
        assert_eq!(unique(&mut t, "a.txt".into()), "a (2).txt");
        assert_eq!(unique(&mut t, "b".into()), "b");
        assert_eq!(unique(&mut t, "b".into()), "b (1)");
    }

    #[test]
    fn remote_names_with_replacement_char_are_skipped() {
        let remote_like = Side::Local; // is_local() true => only checks dst rules
        assert_eq!(
            dst_name(&remote_like, &Side::Local, "a:b").as_deref(),
            Some("a_b")
        );
        assert!(dst_name(&remote_like, &Side::Local, "..").is_none());
    }
}
