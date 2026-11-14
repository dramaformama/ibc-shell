//! iBC Shell app shell: commands, channels, state, capabilities.

use std::sync::Arc;

use ibc_shell_store::Store;
use ibc_shell_vault::{OsKeyring, Vault};
use tauri::Manager;

pub mod api;
pub mod autolock;
pub mod commands;
pub mod connect;
pub mod forwards;
#[cfg(desktop)]
pub mod local_shell;
pub mod prompts;
mod session_output;
pub mod session_task;
pub mod ssh_import;
pub mod ssh_import_apply;
pub mod state;
pub mod transfers;
#[cfg(desktop)]
pub mod tray;
#[cfg(not(desktop))]
#[path = "tray_stub.rs"]
pub mod tray;
pub mod window;

/// Opens the store in the app data dir and builds [`state::AppState`].
fn init_state(app: &tauri::AppHandle) -> Result<state::AppState, Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let store = Arc::new(Store::open(&dir.join("ibc.db"))?);
    let vault = Arc::new(Vault::new(store.clone(), Arc::new(OsKeyring::new())));
    Ok(state::AppState::new(store, vault))
}

/// Release builds drop the panic message (it could carry data from a secret-handling frame);
/// the location stays for diagnosis. Debug builds keep the default hook.
fn install_panic_hook() {
    if cfg!(debug_assertions) {
        return;
    }
    std::panic::set_hook(Box::new(|info| {
        if let Some(loc) = info.location() {
            eprintln!("ibc-shell panicked at {}:{}", loc.file(), loc.line());
        } else {
            eprintln!("ibc-shell panicked");
        }
    }));
}

/// Builds and runs the Tauri application.
///
/// # Panics
/// Tauri aborts the process if the runtime cannot start.
pub fn run() {
    install_panic_hook();
    let specta = api::builder();
    let typed = specta.invoke_handler();
    let raw: Box<dyn Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync> =
        Box::new(tauri::generate_handler![commands::session::shell_write]);
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(move |invoke| {
            let cmd = invoke.message.command();
            // Backend auto-lock (ADR-007): user-driven commands count as activity; output
            // acks are driven by the server and do not.
            if !api::PASSIVE_COMMANDS.contains(&cmd) {
                if let Some(st) = invoke.message.webview().try_state::<state::AppState>() {
                    st.vault.touch();
                }
            }
            if api::RAW_COMMANDS.contains(&cmd) {
                raw(invoke)
            } else {
                typed(invoke)
            }
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                window::cleanup(window);
            }
        })
        .setup(move |app| {
            specta.mount_events(app);
            let st = init_state(app.handle())?;
            autolock::apply_from_store(&st.store, &st.vault);
            autolock::spawn_sweeper(app.handle().clone(), st.vault.clone());
            app.manage(st);
            app.manage(commands::host_window::Handoffs::default());
            #[cfg(desktop)]
            app.manage(commands::document::OpenDocuments::default());
            window::create_main(app)?;
            Ok(())
        })
        .build(tauri::generate_context!());
    match app {
        Ok(app) => app.run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                #[cfg(desktop)]
                app.state::<commands::document::OpenDocuments>().clear();
                #[cfg(not(desktop))]
                let _ = app;
            }
        }),
        Err(err) => {
            eprintln!("ibc-shell failed to start: {err}");
            std::process::exit(1);
        }
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
