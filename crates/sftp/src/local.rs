//! Local file system operations (desktop only). Path validation (allowed roots) is the
//! bridge's job; these functions assume an already-validated absolute path.

use std::fs::Metadata;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::{DirListing, Entry, EntryKind, SftpError};

pub(crate) fn io_err(e: &std::io::Error) -> SftpError {
    match e.kind() {
        std::io::ErrorKind::NotFound => SftpError::NotFound,
        std::io::ErrorKind::PermissionDenied => SftpError::PermissionDenied,
        _ => SftpError::Io(e.to_string()),
    }
}

/// The user's home directory (initial local pane path).
pub fn home() -> Result<String, SftpError> {
    let key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var(key).map_err(|_| SftpError::NotFound)
}

#[cfg(unix)]
fn mode_of(m: &Metadata) -> u32 {
    std::os::unix::fs::MetadataExt::mode(m)
}
#[cfg(not(unix))]
fn mode_of(_: &Metadata) -> u32 {
    0
}

pub(crate) fn entry_from(name: String, path: &Path, m: &Metadata) -> Entry {
    let ft = m.file_type();
    let kind = if ft.is_symlink() {
        EntryKind::Symlink
    } else if ft.is_dir() {
        EntryKind::Dir
    } else if ft.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    };
    let mtime = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_millis()).ok());
    let link_target = if kind == EntryKind::Symlink {
        std::fs::read_link(path)
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    } else {
        None
    };
    Entry {
        name,
        kind,
        size: if kind == EntryKind::Dir { 0 } else { m.len() },
        mtime,
        mode: mode_of(m),
        link_target,
    }
}

/// `lstat` of one path (symlinks are not followed).
pub(crate) fn lstat(path: &Path) -> Result<Entry, SftpError> {
    let m = std::fs::symlink_metadata(path).map_err(|e| io_err(&e))?;
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    Ok(entry_from(name, path, &m))
}

/// Lists a directory.
pub fn list(path: &Path) -> Result<DirListing, SftpError> {
    let mut entries = Vec::new();
    for item in std::fs::read_dir(path).map_err(|e| io_err(&e))? {
        let item = item.map_err(|e| io_err(&e))?;
        let Ok(m) = item.metadata() else { continue };
        entries.push(entry_from(
            item.file_name().to_string_lossy().into_owned(),
            &item.path(),
            &m,
        ));
    }
    Ok(DirListing {
        path: path.to_string_lossy().into_owned(),
        entries,
    })
}

/// Creates a directory.
pub fn mkdir(path: &Path) -> Result<(), SftpError> {
    std::fs::create_dir(path).map_err(|e| io_err(&e))
}

/// Renames within the same file system.
pub fn rename(from: &Path, to: &Path) -> Result<(), SftpError> {
    std::fs::rename(from, to).map_err(|e| io_err(&e))
}

/// Removes a file or (with `recursive`) a directory tree; symlinks are removed, not followed.
pub fn remove(path: &Path, recursive: bool) -> Result<(), SftpError> {
    let m = std::fs::symlink_metadata(path).map_err(|e| io_err(&e))?;
    let r = if m.is_dir() {
        if recursive {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_dir(path)
        }
    } else {
        std::fs::remove_file(path)
    };
    r.map_err(|e| io_err(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_mkdir_rename_remove() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a");
        mkdir(&a).unwrap();
        std::fs::write(a.join("f"), b"hi").unwrap();
        let l = list(&a).unwrap();
        assert_eq!(l.entries.len(), 1);
        assert_eq!(l.entries[0].size, 2);
        let b = d.path().join("b");
        rename(&a, &b).unwrap();
        assert!(remove(&b, false).is_err());
        remove(&b, true).unwrap();
        assert!(matches!(list(&b), Err(SftpError::NotFound)));
    }

    #[cfg(unix)]
    #[test]
    fn remove_does_not_follow_symlinks() {
        let d = tempfile::tempdir().unwrap();
        let keep = d.path().join("keep");
        std::fs::create_dir(&keep).unwrap();
        std::fs::write(keep.join("x"), b"1").unwrap();
        let t = d.path().join("t");
        std::fs::create_dir(&t).unwrap();
        std::os::unix::fs::symlink(&keep, t.join("l")).unwrap();
        remove(&t, true).unwrap();
        assert!(keep.join("x").exists());
    }
}
