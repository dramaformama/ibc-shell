//! Local file system operations (desktop only). Path validation (allowed roots) is the
//! bridge's job; these functions assume an already-validated absolute path.

use std::path::Path;

use crate::{DirListing, SftpError};

/// The user's home directory (initial local pane path).
pub fn home() -> Result<String, SftpError> {
    Err(SftpError::Unimplemented)
}
/// Lists a directory.
pub fn list(path: &Path) -> Result<DirListing, SftpError> {
    Err(SftpError::Unimplemented)
}
/// Creates a directory.
pub fn mkdir(path: &Path) -> Result<(), SftpError> {
    Err(SftpError::Unimplemented)
}
/// Renames within the same file system.
pub fn rename(from: &Path, to: &Path) -> Result<(), SftpError> {
    Err(SftpError::Unimplemented)
}
/// Removes a file or (with `recursive`) a directory tree; symlinks are removed, not followed.
pub fn remove(path: &Path, recursive: bool) -> Result<(), SftpError> {
    Err(SftpError::Unimplemented)
}
