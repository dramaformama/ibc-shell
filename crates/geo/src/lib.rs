//! Optional offline IP-to-country lookup (feature-gated, off by default; Phase 3).
//!
//! Contract: `docs/contracts/data-model.md` §crates/geo (FROZEN v1). Skeleton.

use std::net::IpAddr;

/// Geo lookup errors.
#[derive(Debug, thiserror::Error)]
pub enum GeoError {
    /// Database not bundled (feature off).
    #[error("geo database unavailable")]
    Unavailable,
    /// Phase 0 skeleton placeholder.
    #[error("not implemented")]
    Unimplemented,
}

/// Lowercase ISO 3166-1 alpha-2 code for `ip`, `None` for private/unknown ranges.
pub fn lookup_country(ip: IpAddr) -> Result<Option<String>, GeoError> {
    let _ = ip;
    Err(GeoError::Unimplemented)
}
