//! Live Linux host metrics over an existing connection (ADR-009): one read-only exec of a
//! POSIX `sh` script that samples `/proc` twice, one second apart. Nothing is installed.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::stats_parse::parse_report;
use crate::{Connection, SshError};

/// Upper bound of the script output.
const MAX_OUTPUT: usize = 256 * 1024;
/// Whole-sample deadline (the script itself sleeps 1 s).
const TIMEOUT: Duration = Duration::from_secs(8);

/// One process row (top by CPU).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ProcRow {
    /// Process id.
    pub pid: u32,
    /// Owner (sanitized).
    pub user: String,
    /// CPU % as reported by `ps`.
    pub cpu: f32,
    /// Resident memory, bytes.
    pub mem_bytes: u64,
    /// Command name (sanitized, ≤ 128 chars).
    pub command: String,
}

/// Snapshot of host metrics. Percentages 0–100, memory/disk in bytes, rates in bytes/s.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct HostStats {
    /// Total CPU busy %.
    pub cpu_pct: f32,
    /// Logical cores.
    pub cores: u32,
    /// Busy % per core.
    pub cpu_cores: Vec<f32>,
    /// User %.
    pub cpu_user: f32,
    /// System (kernel + irq) %.
    pub cpu_system: f32,
    /// Nice %.
    pub cpu_nice: f32,
    /// IO-wait %.
    pub cpu_iowait: f32,
    /// Steal %.
    pub cpu_steal: f32,
    /// Load average, 1 min.
    pub load1: f32,
    /// Load average, 5 min.
    pub load5: f32,
    /// Load average, 15 min.
    pub load15: f32,
    /// RAM total.
    pub mem_total: u64,
    /// RAM used (total − available).
    pub mem_used: u64,
    /// Page cache.
    pub mem_cached: u64,
    /// Swap total.
    pub swap_total: u64,
    /// Swap used.
    pub swap_used: u64,
    /// Root filesystem size.
    pub disk_total: u64,
    /// Root filesystem used.
    pub disk_used: u64,
    /// Network receive rate (all non-loopback interfaces).
    pub net_rx_bps: u64,
    /// Network transmit rate.
    pub net_tx_bps: u64,
    /// Disk read rate (whole disks).
    pub disk_read_bps: u64,
    /// Disk write rate (whole disks).
    pub disk_write_bps: u64,
    /// Uptime, seconds.
    pub uptime_s: u64,
    /// `PRETTY_NAME` of the OS.
    pub os_name: Option<String>,
    /// Temperature of thermal zone 0, °C.
    pub temp_c: Option<f32>,
    /// Top 8 processes by CPU.
    pub procs: Vec<ProcRow>,
}

/// The remote script. Read-only; sections are delimited by `@@IBC:NAME@@` lines.
pub(crate) const SCRIPT: &str = r#"export LC_ALL=C
[ -r /proc/stat ] && [ -r /proc/net/dev ] || { echo '@@IBC:UNSUPPORTED@@'; exit 0; }
snap() {
echo "@@IBC:STAT$1@@"; cat /proc/stat
echo "@@IBC:NET$1@@"; cat /proc/net/dev
echo "@@IBC:DISK$1@@"; cat /proc/diskstats 2>/dev/null
echo "@@IBC:UP$1@@"; cat /proc/uptime
}
snap 1
sleep 1
snap 2
echo '@@IBC:MEM@@'; cat /proc/meminfo
echo '@@IBC:LOAD@@'; cat /proc/loadavg
echo '@@IBC:DF@@'; df -kP / 2>/dev/null
echo '@@IBC:NPROC@@'; nproc 2>/dev/null
echo '@@IBC:OS@@'; sed -n 's/^PRETTY_NAME=//p' /etc/os-release 2>/dev/null | head -1
echo '@@IBC:PS@@'; ps -eo pid,user,pcpu,rss,comm --sort=-pcpu 2>/dev/null | head -9
echo '@@IBC:TEMP@@'; cat /sys/class/thermal/thermal_zone0/temp 2>/dev/null
echo '@@IBC:END@@'
"#;

/// Samples the host over `conn` (exec channel, independent of any shell).
///
/// Errors: [`SshError::Unsupported`] for non-Linux hosts, [`SshError::Timeout`] after 8 s.
pub async fn sample_stats(conn: &Connection) -> Result<HostStats, SshError> {
    let out = tokio::time::timeout(TIMEOUT, conn.exec(&wrap_command(), MAX_OUTPUT))
        .await
        .map_err(|_| SshError::Timeout)??;
    let text = String::from_utf8_lossy(&out.stdout);
    parse_report(&text)
}

/// `sh -c '<script>'`, so the script runs under POSIX `sh` whatever the login shell is.
fn wrap_command() -> String {
    format!("sh -c '{}'", SCRIPT.replace('\'', "'\\''"))
}
