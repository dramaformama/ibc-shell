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

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        use rusqlite::ffi::{
            SQLITE_CONSTRAINT_CHECK, SQLITE_CONSTRAINT_FOREIGNKEY, SQLITE_CONSTRAINT_PRIMARYKEY,
            SQLITE_CONSTRAINT_UNIQUE,
        };
        match &e {
            rusqlite::Error::QueryReturnedNoRows => Self::NotFound,
            rusqlite::Error::SqliteFailure(f, msg) => {
                let text = msg.clone().unwrap_or_else(|| f.to_string());
                match f.extended_code {
                    SQLITE_CONSTRAINT_UNIQUE | SQLITE_CONSTRAINT_PRIMARYKEY => Self::Conflict(text),
                    SQLITE_CONSTRAINT_FOREIGNKEY => {
                        Self::Validation("reference to a missing or in-use row".into())
                    }
                    SQLITE_CONSTRAINT_CHECK => Self::Validation(text),
                    _ => Self::Db(text),
                }
            }
            _ => Self::Db(e.to_string()),
        }
    }
}

impl From<rusqlite_migration::Error> for StoreError {
    fn from(e: rusqlite_migration::Error) -> Self {
        Self::Db(e.to_string())
    }
}
