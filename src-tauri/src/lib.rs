//! ibc server app shell: commands, channels, state, capabilities.

use std::sync::Arc;

use ibc_store::Store;
use ibc_vault::{OsKeyring, Vault};
use tauri::Manager;

pub mod api;
pub mod commands;
pub mod connect;
pub mod prompts;
pub mod session_task;
pub mod state;
pub mod transfers;

/// Opens the store in the app data dir and builds [`state::AppState`].
fn init_state(app: &tauri::AppHandle) -> Result<state::AppState, Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let store = Arc::new(Store::open(&dir.join("ibc.db"))?);
    let vault = Arc::new(Vault::new(store.clone(), Arc::new(OsKeyring::new())));
    Ok(state::AppState::new(store, vault))
}

/// Builds and runs the Tauri application.
///
/// # Panics
/// Tauri aborts the process if the runtime cannot start.
pub fn run() {
    let specta = api::builder();
    let typed = specta.invoke_handler();
    let raw: Box<dyn Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync> =
        Box::new(tauri::generate_handler![commands::session::shell_write]);
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(move |invoke| {
            if api::RAW_COMMANDS.contains(&invoke.message.command()) {
                raw(invoke)
            } else {
                typed(invoke)
            }
        })
        .setup(move |app| {
            specta.mount_events(app);
            app.manage(init_state(app.handle())?);
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(err) = result {
        eprintln!("ibc-server failed to start: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    /// Fails when `ui/src/lib/api/bindings.ts` is stale — run `just bindings`.
    #[test]
    fn bindings_are_fresh() {
        let committed =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/src/lib/api/bindings.ts");
        let tmp = std::env::temp_dir().join(format!("ibc-bindings-{}.ts", std::process::id()));
        crate::api::export_bindings(&tmp).unwrap();
        let fresh = std::fs::read_to_string(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        let current = std::fs::read_to_string(committed).unwrap_or_default();
        assert!(
            fresh == current,
            "bindings.ts is stale: run `just bindings`"
        );
    }
}
