//! Transfer engine: queue (N parallel files, default 3), pipelining, resume, conflicts.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Sftp;

/// Transfer id (one per top-level item of a request).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct TransferId(pub Uuid);

/// Pending conflict decision id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct ConflictId(pub Uuid);

/// What to do when the destination exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum ConflictPolicy {
    /// Emit `TransferEvent::Conflict` and wait for `conflict_reply`.
    Ask,
    /// Replace.
    Overwrite,
    /// Keep destination, skip file.
    Skip,
    /// Write as `name (1).ext`, `name (2).ext`, …
    Rename,
    /// Append if destination is smaller, otherwise ask.
    ResumeIfSmaller,
}

/// Answer to a conflict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum ConflictChoice {
    /// Replace.
    Overwrite,
    /// Skip this file.
    Skip,
    /// Keep both.
    Rename,
    /// Append from destination size.
    Resume,
}

/// Both sides of a conflict (`TransferEvent::Conflict`, `ApiError::Conflict`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ConflictInfo {
    /// Destination path.
    pub path: String,
    /// Source size.
    pub src_size: u64,
    /// Source mtime, unix millis.
    pub src_mtime: Option<i64>,
    /// Destination size.
    pub dst_size: u64,
    /// Destination mtime, unix millis.
    pub dst_mtime: Option<i64>,
}

/// Engine-side endpoint (the bridge maps IPC `{kind:"remote", sftp_id}` to an `Arc<Sftp>`).
#[derive(Clone, Debug)]
pub enum Endpoint {
    /// Local file system (paths already validated by the bridge).
    Local,
    /// Remote SFTP session.
    Remote(Arc<Sftp>),
}

/// A transfer request: copy `items` (absolute source paths) into directory `dst_dir`.
#[derive(Clone, Debug)]
pub struct TransferJob {
    /// Source.
    pub src: Endpoint,
    /// Destination.
    pub dst: Endpoint,
    /// Absolute source paths (files or dirs, recursive).
    pub items: Vec<String>,
    /// Absolute destination directory.
    pub dst_dir: String,
    /// Conflict policy.
    pub conflict: ConflictPolicy,
}

/// Progress events. IPC: `TransferEvent` (tag `type`). Progress ≤ 4 Hz per transfer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum TransferEvent {
    /// Accepted into the queue.
    Queued {
        /// Transfer.
        id: TransferId,
        /// Source path.
        src_path: String,
        /// Destination path.
        dst_path: String,
    },
    /// Scanning finished, bytes flowing.
    Started {
        /// Transfer.
        id: TransferId,
        /// Total bytes.
        total_bytes: u64,
        /// File count.
        files: u32,
    },
    /// Progress.
    Progress {
        /// Transfer.
        id: TransferId,
        /// Bytes done.
        bytes_done: u64,
        /// Smoothed speed.
        bytes_per_s: u64,
    },
    /// Waiting for `transfer_conflict_reply`.
    Conflict {
        /// Transfer.
        id: TransferId,
        /// Reply id.
        pending_id: ConflictId,
        /// Details.
        info: ConflictInfo,
    },
    /// One file finished (or skipped).
    FileDone {
        /// Transfer.
        id: TransferId,
        /// Destination path of the file.
        path: String,
        /// Skipped by policy / symlink not followed.
        skipped: bool,
    },
    /// All files done.
    Done {
        /// Transfer.
        id: TransferId,
    },
    /// Failed; may be retried.
    Failed {
        /// Transfer.
        id: TransferId,
        /// Sanitized message.
        message: String,
    },
    /// Paused.
    Paused {
        /// Transfer.
        id: TransferId,
    },
    /// Cancelled (partial files kept for resume).
    Cancelled {
        /// Transfer.
        id: TransferId,
    },
}

/// Engine tuning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransferConfig {
    /// Parallel files (default 3).
    pub parallel_files: usize,
    /// Outstanding requests per file (default 64).
    pub pipeline_depth: usize,
    /// Request size, bytes (default 32 KiB; raised up to `limits@openssh.com`).
    pub chunk_size: u32,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            parallel_files: 3,
            pipeline_depth: 64,
            chunk_size: 32 * 1024,
        }
    }
}
