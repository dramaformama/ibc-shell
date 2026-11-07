//! Remote file name sanitization (security.md §Terminal / remote-data hardening).

use std::path::{Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

use crate::SftpError;

/// Longest local name the engine will create (leaves room for the `.tlpart` suffix).
pub(crate) const MAX_LOCAL_NAME: usize = 240;

const RESERVED: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];

fn is_reserved(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("").trim_end_matches(' ');
    let up = stem.to_ascii_uppercase();
    if RESERVED.contains(&up.as_str()) {
        return true;
    }
    let b = up.as_bytes();
    b.len() == 4
        && (up.starts_with("COM") || up.starts_with("LPT"))
        && (b'1'..=b'9').contains(&b[3])
}

fn bad_char(c: char) -> bool {
    c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\\' | '/')
}

/// Names that can never be used, whatever the replacement.
fn structurally_bad(name: &str) -> bool {
    name.is_empty() || name == "." || name == ".." || name.contains(['/', '\0'])
}

fn problem(name: &str) -> Option<&'static str> {
    if structurally_bad(name) {
        Some("empty, dot or separator in name")
    } else if name.chars().any(bad_char) {
        Some("forbidden character")
    } else if name.ends_with(['.', ' ']) {
        Some("trailing dot or space")
    } else if is_reserved(name) {
        Some("reserved device name")
    } else if name.len() > 255 {
        Some("name too long")
    } else {
        None
    }
}

/// Validates a remote entry name for use as a local file name: rejects empty, `.`, `..`,
/// `/`, `\`, NUL, drive letters, Windows reserved names (`CON`, `PRN`, `AUX`, `NUL`,
/// `COM1-9`, `LPT1-9`), trailing dots/spaces, `<>:"|?*`, names > 255 bytes. NFC-normalizes.
pub fn sanitize_name(name: &str) -> Result<String, SftpError> {
    let nfc: String = name.nfc().collect();
    match problem(&nfc) {
        None => Ok(nfc),
        Some(why) => Err(SftpError::UnsafePath(why.into())),
    }
}

/// Lenient variant used by the transfer engine: instead of rejecting, replaces forbidden
/// characters with `_`, prefixes reserved names, fixes trailing dots/spaces and truncates to
/// 240 bytes. Still fails for names that cannot be repaired (`""`, `.`, `..`, anything with
/// `/` or NUL).
pub fn escape_name(name: &str) -> Result<String, SftpError> {
    if structurally_bad(name) {
        return Err(SftpError::UnsafePath(
            "empty, dot or separator in name".into(),
        ));
    }
    let mut s: String = name
        .nfc()
        .map(|c| if bad_char(c) { '_' } else { c })
        .collect();
    if s.ends_with(['.', ' ']) {
        s.push('_');
    }
    if is_reserved(&s) {
        s.insert(0, '_');
    }
    while s.len() > MAX_LOCAL_NAME {
        s.pop();
    }
    if s.ends_with(['.', ' ']) {
        s.push('_');
    }
    Ok(s)
}

/// Joins a sanitized `name` onto `base` and asserts the canonical result stays inside `base`.
pub fn safe_join(base: &Path, name: &str) -> Result<PathBuf, SftpError> {
    let clean = sanitize_name(name)?;
    let canon_base = base
        .canonicalize()
        .map_err(|e| crate::local::io_err(&e))?;
    let target = canon_base.join(clean);
    if let Ok(real) = target.canonicalize() {
        if !real.starts_with(&canon_base) {
            return Err(SftpError::UnsafePath(
                "path escapes target directory".into(),
            ));
        }
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_and_separators() {
        for n in ["", ".", "..", "a/b", "..\\..\\x", "a\0b", "C:evil", "../x"] {
            assert!(sanitize_name(n).is_err(), "{n:?}");
        }
        assert!(sanitize_name("..%2f..%2fx").is_ok());
    }

    #[test]
    fn rejects_windows_reserved_and_chars() {
        for n in [
            "CON", "con", "NUL.txt", "com1", "LPT9.log", "aux ", "a<b", "q?", "x|y", "t.", "t ",
            "a*b", "a\"b",
        ] {
            assert!(sanitize_name(n).is_err(), "{n:?}");
        }
        for n in ["COM0", "COM10", "console", "lpt", "ok.txt", " leading", "-rf"] {
            assert!(sanitize_name(n).is_ok(), "{n:?}");
        }
    }

    #[test]
    fn nfc_normalizes() {
        let nfd = "cafe\u{301}";
        assert_eq!(sanitize_name(nfd).unwrap(), "caf\u{e9}");
    }

    #[test]
    fn rejects_too_long() {
        assert!(sanitize_name(&"a".repeat(255)).is_ok());
        assert!(sanitize_name(&"a".repeat(256)).is_err());
        assert!(sanitize_name(&"я".repeat(128)).is_err()); // 256 bytes
    }

    #[test]
    fn escape_repairs() {
        assert_eq!(escape_name("CON").unwrap(), "_CON");
        assert_eq!(escape_name("a:b").unwrap(), "a_b");
        assert_eq!(escape_name("..\\..\\x").unwrap(), ".._.._x");
        assert_eq!(escape_name("trailing.").unwrap(), "trailing._");
        assert_eq!(escape_name("new\nline").unwrap(), "new_line");
        let long = escape_name(&"я".repeat(200)).unwrap();
        assert!(long.len() <= MAX_LOCAL_NAME);
        assert!(escape_name("..").is_err());
        assert!(escape_name("a/b").is_err());
        for n in ["CON", "a:b", "x.", "NUL.txt", "new\nline"] {
            let e = escape_name(n).unwrap();
            assert!(sanitize_name(&e).is_ok(), "{e:?}");
        }
    }

    #[test]
    fn safe_join_stays_inside() {
        let d = tempfile::tempdir().unwrap();
        assert!(safe_join(d.path(), "../x").is_err());
        let p = safe_join(d.path(), "ok.txt").unwrap();
        assert!(p.starts_with(d.path().canonicalize().unwrap()));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc", d.path().join("esc")).unwrap();
            assert!(safe_join(d.path(), "esc").is_err());
        }
    }
}
