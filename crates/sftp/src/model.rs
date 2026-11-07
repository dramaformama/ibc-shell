//! Directory listing types shared by remote (SFTP) and local file systems.

use serde::{Deserialize, Serialize};

/// Entry kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    /// Regular file.
    File,
    /// Directory.
    Dir,
    /// Symbolic link (not followed in recursive transfers).
    Symlink,
    /// Anything else (device, socket, fifo).
    Other,
}

/// One directory entry. `name` is the raw server name (display as plain text only).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Entry {
    /// File name (no path).
    pub name: String,
    /// Kind.
    pub kind: EntryKind,
    /// Size in bytes (0 for dirs if unknown).
    pub size: u64,
    /// Modification time, unix millis.
    pub mtime: Option<i64>,
    /// POSIX mode bits (permissions + type); 0 if unknown.
    pub mode: u32,
    /// Symlink target, if `kind == symlink` and readable.
    pub link_target: Option<String>,
}

/// A directory listing. `path` is absolute and normalized (remote: POSIX).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DirListing {
    /// Absolute path of the directory.
    pub path: String,
    /// Entries, unsorted (`.` and `..` excluded).
    pub entries: Vec<Entry>,
}
