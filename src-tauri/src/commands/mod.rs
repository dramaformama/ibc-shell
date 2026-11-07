//! `#[tauri::command]` handlers, one module per `ipc.md` section.
//! Phase 0 stubs: every command returns `ApiError::Internal("not implemented")`.
// Skeleton-only allowances; the tauri-bridge agent removes this line when implementing.
#![allow(unused_variables, clippy::unused_async, clippy::needless_pass_by_value)]

pub mod app;
pub mod catalog;
pub mod session;
pub mod sftp;
pub mod tree;
pub mod vault;
