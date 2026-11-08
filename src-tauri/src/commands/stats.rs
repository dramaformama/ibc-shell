//! Host metrics command (`ipc.md` §Stats, ADR-009).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use tauri::State;

use crate::api::dto::{HostStats, SessionId};
use crate::api::{ApiError, ApiResult};
use crate::commands::session::session_of;
use crate::state::AppState;

/// Slow fields (`df`, `nproc`, OS, `ps`) are refreshed at most this often (ADR-009 amendment).
const SLOW_EVERY: Duration = Duration::from_secs(15);
/// Cache entries of sessions not sampled for this long are dropped.
const FORGET_AFTER: Duration = Duration::from_secs(600);

/// Last full sample per session: (taken at, last used, sample).
type SlowCache = HashMap<SessionId, (Instant, Instant, HostStats)>;

fn cache() -> &'static Mutex<SlowCache> {
    static CACHE: OnceLock<Mutex<SlowCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Copies the slow fields of `from` into a fast sample.
fn merge_slow(into: &mut HostStats, from: &HostStats) {
    into.disk_total = from.disk_total;
    into.disk_used = from.disk_used;
    into.os_name.clone_from(&from.os_name);
    into.cpu_model.clone_from(&from.cpu_model);
    into.disk_dev_bytes = from.disk_dev_bytes;
    into.disk_kind.clone_from(&from.disk_kind);
    into.procs.clone_from(&from.procs);
    into.listen.clone_from(&from.listen);
    into.flows.clone_from(&from.flows);
    into.conns = from.conns;
    if from.cores > 0 {
        into.cores = from.cores;
    }
}

/// One-shot sample of the remote Linux host over the session's existing SSH connection
/// (exec channel, independent of the shell). Takes ~1 s. Passive for auto-lock. Slow fields
/// are re-read every 15 s and reused from the last full sample in between.
#[tauri::command]
#[specta::specta]
pub async fn stats_sample(
    state: State<'_, AppState>,
    session_id: SessionId,
) -> ApiResult<HostStats> {
    let conn = {
        let h = session_of(&state, session_id)?;
        let live = h
            .live
            .lock()
            .map_err(|_| ApiError::Internal("state poisoned".into()))?;
        live.conn
            .clone()
            .ok_or(ApiError::Network("session not connected".into()))?
    };
    let now = Instant::now();
    let need_full = {
        let mut c = cache()
            .lock()
            .map_err(|_| ApiError::Internal("state poisoned".into()))?;
        c.retain(|_, (_, used, _)| now.duration_since(*used) < FORGET_AFTER);
        c.get(&session_id)
            .is_none_or(|(at, _, _)| now.duration_since(*at) >= SLOW_EVERY)
    };
    let mut sample = ibc_shell_ssh::sample_stats(&conn, need_full).await?;
    let mut c = cache()
        .lock()
        .map_err(|_| ApiError::Internal("state poisoned".into()))?;
    if sample.full {
        c.insert(session_id, (now, now, sample.clone()));
    } else if let Some((_, used, prev)) = c.get_mut(&session_id) {
        *used = now;
        merge_slow(&mut sample, prev);
    }
    Ok(sample)
}
