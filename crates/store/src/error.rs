//! Store error type.

/// Errors returned by [`crate::Store`]. Messages never contain secret material.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Entity with the given id does not exist.
    #[error("not found")]
    NotFound,
    /// Input failed validation (field name + reason).
    #[error("validation failed: {0}")]
    Validation(String),
    /// Folder move would create a cycle (invariant 1).
    #[error("folder tree cycle")]
    Cycle,
    /// Folder depth would exceed 8 (invariant 2).
    #[error("folder tree too deep")]
    TooDeep,
    /// Jump chain has a cycle or exceeds 3 hops (invariant 3).
    #[error("jump host chain cycle or too long")]
    JumpCycle,
    /// Unique constraint violated (e.g. tag name, known host pattern+type).
    #[error("conflict: {0}")]
    Conflict(String),
    /// Underlying SQLite / migration / IO failure.
    #[error("database error: {0}")]
    Db(String),
    /// Phase 0 skeleton placeholder; must not be returned once the crate is implemented (gate G1).
    #[error("not implemented")]
    Unimplemented,
}
