//! Termlite app shell: commands, channels, state, capabilities.

/// Builds and runs the Tauri application.
///
/// # Panics
/// Tauri aborts the process if the runtime cannot start.
pub fn run() {
    if let Err(err) = tauri::Builder::default().run(tauri::generate_context!()) {
        eprintln!("termlite failed to start: {err}");
        std::process::exit(1);
    }
}
