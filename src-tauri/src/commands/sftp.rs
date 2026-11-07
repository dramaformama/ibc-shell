//! SFTP, local FS and transfer commands (`ipc.md` §SFTP).

use tauri::ipc::Channel;
use termlite_sftp::{ConflictChoice, ConflictId, DirListing, Entry, TransferEvent, TransferId};

use crate::api::dto::{SessionEvent, SftpId, SftpTarget, TransferIds, TransferRequest};
use crate::api::{ApiError, ApiResult};

macro_rules! stub {
    ($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*) -> $ret:ty) => {
        $(#[$doc])*
        #[tauri::command]
        #[specta::specta]
        pub async fn $name($($arg: $ty),*) -> ApiResult<$ret> {
            Err(ApiError::unimplemented())
        }
    };
}

stub!(
    /// Opens SFTP over an existing session or a new connection (prompts → `on_event`).
    sftp_open(target: SftpTarget, on_event: Channel<SessionEvent>) -> SftpId
);
stub!(
    /// Closes an SFTP session (transfers using it fail with `Cancelled`).
    sftp_close(sftp_id: SftpId) -> ()
);
stub!(
    /// Lists a remote directory.
    sftp_list(sftp_id: SftpId, path: String) -> DirListing
);
stub!(
    /// `lstat`.
    sftp_stat(sftp_id: SftpId, path: String) -> Entry
);
stub!(
    /// `mkdir`.
    sftp_mkdir(sftp_id: SftpId, path: String) -> ()
);
stub!(
    /// Rename / move.
    sftp_rename(sftp_id: SftpId, from: String, to: String) -> ()
);
stub!(
    /// Remove a file, or a tree when `recursive`.
    sftp_remove(sftp_id: SftpId, path: String, recursive: bool) -> ()
);
stub!(
    /// Change permissions.
    sftp_chmod(sftp_id: SftpId, path: String, mode: u32) -> ()
);
stub!(
    /// Canonical absolute path (`"."` → remote home).
    sftp_realpath(sftp_id: SftpId, path: String) -> String
);
stub!(
    /// Local home directory; registers it as an allowed root. Desktop only (`Unsupported` on mobile).
    local_home() -> String
);
stub!(
    /// Lists a local directory (must be under an allowed root).
    local_list(path: String) -> DirListing
);
stub!(
    /// Creates a local directory.
    local_mkdir(path: String) -> ()
);
stub!(
    /// Renames a local entry.
    local_rename(from: String, to: String) -> ()
);
stub!(
    /// Removes a local entry.
    local_remove(path: String, recursive: bool) -> ()
);
stub!(
    /// Queues transfers; one id per item.
    transfer_enqueue(request: TransferRequest) -> TransferIds
);
stub!(
    /// Pauses a transfer.
    transfer_pause(id: TransferId) -> ()
);
stub!(
    /// Resumes a transfer.
    transfer_resume(id: TransferId) -> ()
);
stub!(
    /// Cancels a transfer.
    transfer_cancel(id: TransferId) -> ()
);
stub!(
    /// Retries a failed / cancelled transfer.
    transfer_retry(id: TransferId) -> ()
);
stub!(
    /// Answers a `TransferEvent::Conflict`.
    transfer_conflict_reply(pending_id: ConflictId, choice: ConflictChoice, apply_to_all: bool) -> ()
);
stub!(
    /// Subscribes to all transfer events (replaces the previous subscriber).
    transfers_subscribe(on_event: Channel<TransferEvent>) -> ()
);
