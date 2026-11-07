//! Backend idle auto-lock (ADR-007): applies `security.autolock_minutes` to the vault and runs
//! the sweeper that locks + emits `vault://locked` once the deadline passes.

use std::sync::Arc;
use std::time::Duration;

use ibc_shell_store::Store;
use ibc_shell_vault::Vault;
use tauri::AppHandle;
use tauri_specta::Event;

use crate::api::events::VaultLocked;

/// Settings key (shared with the UI).
pub const AUTOLOCK_KEY: &str = "security.autolock_minutes";
/// Default minutes (security.md).
const DEFAULT_MINUTES: u64 = 15;
/// Largest accepted value (24 h), same bound as the UI.
const MAX_MINUTES: f64 = 24.0 * 60.0;
/// Sweeper period.
const SWEEP: Duration = Duration::from_secs(5);

/// Setting value → timeout (`None` = never). Anything unusual falls back to the default.
#[must_use]
pub fn timeout_from_setting(v: Option<&serde_json::Value>) -> Option<Duration> {
    let minutes = match v.and_then(serde_json::Value::as_f64) {
        Some(m) if m.is_finite() && (0.0..=MAX_MINUTES).contains(&m) => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let m = m.round() as u64;
            m
        }
        _ => DEFAULT_MINUTES,
    };
    (minutes > 0).then(|| Duration::from_secs(minutes * 60))
}

/// Reads the setting from the store and applies it (default on a read error). Returns it.
pub fn apply_from_store(store: &Store, vault: &Vault) -> Option<Duration> {
    let v = store.setting_get(AUTOLOCK_KEY).ok().flatten();
    let t = timeout_from_setting(v.as_ref());
    vault.set_idle_timeout(t);
    t
}

/// Spawns the sweeper: locks an idle vault and tells the UI.
pub fn spawn_sweeper(app: AppHandle, vault: Arc<Vault>) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(SWEEP).await;
            if vault.lock_if_idle() {
                let _ = VaultLocked.emit(&app);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_like_the_ui() {
        let m = |v: serde_json::Value| timeout_from_setting(Some(&v));
        assert_eq!(m(json!(5)), Some(Duration::from_mins(5)));
        assert_eq!(m(json!(0)), None);
        assert_eq!(m(json!(1.4)), Some(Duration::from_mins(1)));
        for bad in [json!(-1), json!(100_000), json!("5"), json!(null)] {
            assert_eq!(m(bad), Some(Duration::from_mins(15)));
        }
        assert_eq!(timeout_from_setting(None), Some(Duration::from_mins(15)));
    }

    #[test]
    fn store_setting_reaches_the_vault() {
        let store = Arc::new(Store::open_in_memory().unwrap());
        let vault = Vault::new(
            store.clone(),
            Arc::new(ibc_shell_vault::MemoryKeyStore::default()),
        );
        vault.init(ibc_shell_vault::VaultMode::OsKeychain, None).unwrap();
        assert_eq!(
            apply_from_store(&store, &vault),
            Some(Duration::from_mins(15))
        );
        store.setting_set(AUTOLOCK_KEY, &json!(5)).unwrap();
        assert_eq!(
            apply_from_store(&store, &vault),
            Some(Duration::from_mins(5))
        );
        store.setting_set(AUTOLOCK_KEY, &json!(0)).unwrap();
        assert_eq!(apply_from_store(&store, &vault), None);
        assert!(!vault.lock_if_idle());
    }
}
