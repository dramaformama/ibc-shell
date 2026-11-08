//! Settings in a backup: which ones travel to another computer, and the checks on the way in.

use rusqlite::params;

use crate::db::Store;
use crate::Result;

/// Keys never exported or imported: a cache of the last metrics, and machine-bound security
/// choices (the lock timer belongs to this computer's vault).
fn local_only(k: &str) -> bool {
    k == "stats.last" || k.starts_with("security.")
}

const MAX_ITEMS: usize = 512;
const MAX_KEY: usize = 64;
const MAX_VALUE: usize = 64 * 1024;

fn key_ok(k: &str) -> bool {
    !k.is_empty()
        && k.len() <= MAX_KEY
        && k.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-')
        })
}

impl Store {
    /// Settings that go into a backup, as `(key, JSON text)`.
    pub fn settings_export(&self) -> Result<Vec<(String, String)>> {
        let all: Vec<(String, String)> = self.read(|c| {
            let mut st = c.prepare_cached("SELECT k, v FROM setting ORDER BY k")?;
            let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        })?;
        Ok(all.into_iter().filter(|(k, _)| !local_only(k)).collect())
    }

    /// Writes settings from a backup (replacing the same keys) and returns how many were
    /// taken. Keys that are not plain names, local-only keys, values over 64 KiB, text that is
    /// not JSON and everything past 512 entries are dropped.
    pub fn settings_import(&self, items: Vec<(String, String)>) -> Result<u32> {
        let ok: Vec<_> = items
            .into_iter()
            .take(MAX_ITEMS)
            .filter(|(k, v)| {
                key_ok(k)
                    && !local_only(k)
                    && v.len() <= MAX_VALUE
                    && serde_json::from_str::<serde_json::Value>(v).is_ok()
            })
            .collect();
        self.write(|tx| {
            for (k, v) in &ok {
                tx.execute(
                    "INSERT INTO setting(k, v) VALUES(?1, ?2) ON CONFLICT(k) DO UPDATE SET v=excluded.v",
                    params![k, v],
                )?;
            }
            Ok(())
        })?;
        Ok(u32::try_from(ok.len()).unwrap_or(u32::MAX))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn export_leaves_local_settings_out() {
        let s = Store::open_in_memory().unwrap();
        s.setting_set("term.font_size", &json!(14)).unwrap();
        s.setting_set("stats.last", &json!({"a": 1})).unwrap();
        s.setting_set("security.autolock_minutes", &json!(5))
            .unwrap();
        let keys: Vec<_> = s
            .settings_export()
            .unwrap()
            .into_iter()
            .map(|x| x.0)
            .collect();
        assert_eq!(keys, ["term.font_size"]);
    }

    #[test]
    fn import_replaces_and_rejects_bad_entries() {
        let s = Store::open_in_memory().unwrap();
        s.setting_set("term.font_size", &json!(12)).unwrap();
        let n = s
            .settings_import(vec![
                ("term.font_size".into(), "16".into()),
                ("ui.local".into(), r#"{"ibc.theme":"dark"}"#.into()),
                ("Bad Key".into(), "1".into()),
                ("stats.last".into(), "{}".into()),
                ("security.autolock_minutes".into(), "1".into()),
                ("not.json".into(), "{oops".into()),
                ("huge".into(), format!("\"{}\"", "x".repeat(70_000))),
            ])
            .unwrap();
        assert_eq!(n, 2);
        assert_eq!(s.setting_get("term.font_size").unwrap(), Some(json!(16)));
        assert_eq!(s.setting_get("stats.last").unwrap(), None);
        assert_eq!(s.setting_get("security.autolock_minutes").unwrap(), None);
    }
}
