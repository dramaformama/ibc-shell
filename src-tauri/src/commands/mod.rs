//! `#[tauri::command]` handlers, one module per `ipc.md` section.

pub mod app;
pub mod catalog;
pub mod document;
pub mod host_window;
pub mod identity;
pub mod probe;
pub mod services;
pub mod session;
pub mod sftp;
pub mod stats;
pub mod tree;
pub mod vault;
