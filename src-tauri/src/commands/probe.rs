//! Passive SSH-port reachability, distinct from authentication and metrics (ADR-010).

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ibc_shell_store::HostId;
use serde::Serialize;
use tauri::State;
use tokio::{net::TcpStream, sync::Semaphore, time::timeout};

use crate::{
    api::{ApiError, ApiResult},
    commands::vault::require_unlocked,
    state::AppState,
};

static PROBES: Semaphore = Semaphore::const_new(4);

/// Reachability of the configured SSH port; does not establish an authenticated session.
#[derive(Clone, Debug, Serialize, specta::Type)]
pub struct HostReachability {
    /// True if a TCP connection was established, not a guarantee of successful SSH login.
    pub reachable: bool,
    /// TCP connect time including DNS, not ICMP RTT. Absent when unreachable.
    pub latency_ms: Option<u32>,
    /// Observation timestamp in Unix milliseconds.
    pub checked_at: i64,
}

async fn probe(hostname: &str, port: u16) -> HostReachability {
    let start = Instant::now();
    let reachable = matches!(
        timeout(Duration::from_secs(3), TcpStream::connect((hostname, port))).await,
        Ok(Ok(_))
    );
    HostReachability {
        reachable,
        latency_ms: reachable
            .then(|| u32::try_from(start.elapsed().as_millis()).unwrap_or(u32::MAX)),
        checked_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX)),
    }
}

/// Checks a saved host's SSH port without using credentials or modifying host-key trust.
#[tauri::command]
#[specta::specta]
pub async fn host_probe(
    state: State<'_, AppState>,
    host_id: HostId,
) -> ApiResult<HostReachability> {
    require_unlocked(&state)?;
    let host = state.store.host_resolve(host_id)?;
    if !host.jump_chain.is_empty() {
        return Err(ApiError::Unsupported);
    }
    let _permit = PROBES
        .acquire()
        .await
        .map_err(|_| ApiError::Internal("probe unavailable".into()))?;
    require_unlocked(&state)?;
    let observation = probe(&host.hostname, host.port.value).await;
    require_unlocked(&state)?;
    Ok(observation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reachable_port_has_latency_and_timestamp() {
        let server = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let result = probe("127.0.0.1", server.local_addr().unwrap().port()).await;
        assert!(result.reachable);
        assert!(result.latency_ms.is_some());
        assert!(result.checked_at > 0);
    }

    #[tokio::test]
    async fn closed_port_is_unreachable_not_an_authenticated_session() {
        let server = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = server.local_addr().unwrap().port();
        drop(server);
        let result = probe("127.0.0.1", port).await;
        assert!(!result.reachable);
        assert_eq!(result.latency_ms, None);
    }
}
