//! Input validation helpers.

use crate::{Result, StoreError, KNOWN_FLAGS};

fn bad<T>(msg: &str) -> Result<T> {
    Err(StoreError::Validation(msg.to_string()))
}

/// Name / label: 1..=128 chars, not blank.
pub(crate) fn name(field: &str, v: &str) -> Result<()> {
    let n = v.chars().count();
    if n == 0 || n > 128 || v.trim().is_empty() {
        return bad(&format!("{field}: must be 1..=128 characters"));
    }
    Ok(())
}

/// Hostname: non-empty, no whitespace, no brackets.
pub(crate) fn hostname(v: &str) -> Result<()> {
    if v.is_empty()
        || v.len() > 255
        || v.chars()
            .any(|c| c.is_whitespace() || c == '[' || c == ']' || c.is_control())
    {
        return bad("hostname: invalid");
    }
    Ok(())
}

/// Port 1..=65535 (0 is rejected).
pub(crate) fn port(field: &str, v: Option<u16>) -> Result<()> {
    if v == Some(0) {
        return bad(&format!("{field}: must be 1..=65535"));
    }
    Ok(())
}

/// Lowercases and checks a flag code. When `KNOWN_FLAGS` is still the empty placeholder only the
/// shape (2..=6 ASCII letters) is checked.
pub(crate) fn country(v: Option<String>) -> Result<Option<String>> {
    let Some(v) = v else { return Ok(None) };
    let v = v.to_ascii_lowercase();
    let shape = (2..=6).contains(&v.len()) && v.bytes().all(|b| b.is_ascii_lowercase());
    if !shape || (!KNOWN_FLAGS.is_empty() && !KNOWN_FLAGS.contains(&v.as_str())) {
        return bad("country_code: unknown flag");
    }
    Ok(Some(v))
}
