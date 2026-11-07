//! Termlite app shell: commands, channels, state, capabilities.

pub mod api;
pub mod commands;

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
        .invoke_handler(move |invoke| {
            if api::RAW_COMMANDS.contains(&invoke.message.command()) {
                raw(invoke)
            } else {
                typed(invoke)
            }
        })
        .setup(move |app| {
            specta.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(err) = result {
        eprintln!("termlite failed to start: {err}");
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
        let tmp = std::env::temp_dir().join(format!("termlite-bindings-{}.ts", std::process::id()));
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
