//! Remote file name sanitization (security.md §Terminal / remote-data hardening).

use std::path::{Path, PathBuf};

use crate::SftpError;

/// Validates a remote entry name for use as a local file name: rejects empty, `.`, `..`,
/// `/`, `\`, NUL, drive letters, Windows reserved names (`CON`, `PRN`, `AUX`, `NUL`,
/// `COM1-9`, `LPT1-9`), trailing dots/spaces, `<>:"|?*`, names > 255 bytes. NFC-normalizes.
pub fn sanitize_name(name: &str) -> Result<String, SftpError> {
    Err(SftpError::Unimplemented)
}

/// Joins a sanitized `name` onto `base` and asserts the canonical result stays inside `base`.
pub fn safe_join(base: &Path, name: &str) -> Result<PathBuf, SftpError> {
    Err(SftpError::Unimplemented)
}
