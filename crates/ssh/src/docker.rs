//! Docker containers over an existing connection (ADR-012): list with live usage (read-only),
//! logs (read-only) and start / stop / restart of one container chosen by the user.
//! Only the `docker` CLI already on the server is used; nothing is installed.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::stats_parse::{sanitize, sections};
use crate::{Connection, SshError};

const LIST_TIMEOUT: Duration = Duration::from_secs(15);
const LOGS_TIMEOUT: Duration = Duration::from_secs(12);
const ACTION_TIMEOUT: Duration = Duration::from_secs(70);
const MAX_LIST: usize = 512 * 1024;
const MAX_LOGS: usize = 512 * 1024;
/// Containers listed at most.
const MAX_CONTAINERS: usize = 150;

/// Whether the Docker CLI could be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum DockerAccess {
    /// Listed fine.
    Ok,
    /// No `docker` command on the server.
    Missing,
    /// The login user may not talk to the daemon (not root, not in the `docker` group).
    Denied,
    /// The daemon is not running.
    Down,
}

/// One container.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Container {
    /// Short id (12 hex chars).
    pub id: String,
    /// Name.
    pub name: String,
    /// Image.
    pub image: String,
    /// `running`, `exited`, `restarting`, `paused`, `created`, `dead`.
    pub state: String,
    /// Human status, e.g. `Up 3 days (healthy)`.
    pub status: String,
    /// Published ports as `docker ps` prints them.
    pub ports: String,
    /// CPU % (running containers only).
    pub cpu_pct: Option<f32>,
    /// Memory used, bytes.
    pub mem_bytes: Option<u64>,
    /// Memory limit, bytes.
    pub mem_limit: Option<u64>,
}

/// Result of [`docker_list`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct DockerReport {
    /// Access state; `containers` is empty unless `Ok`.
    pub access: DockerAccess,
    /// Containers, running first.
    pub containers: Vec<Container>,
}

/// A user-chosen container action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum DockerAction {
    /// `docker start`.
    Start,
    /// `docker stop`.
    Stop,
    /// `docker restart`.
    Restart,
}

const LIST_SCRIPT: &str = r"export LC_ALL=C
command -v docker >/dev/null 2>&1 || { echo '@@IBC:MISSING@@'; exit 0; }
echo '@@IBC:PS@@'
docker ps -a --format '{{.ID}}	{{.Names}}	{{.Image}}	{{.State}}	{{.Status}}	{{.Ports}}' 2>&1 | head -150
echo '@@IBC:USE@@'
docker stats --no-stream --format '{{.ID}}	{{.CPUPerc}}	{{.MemUsage}}' 2>/dev/null | head -150
echo '@@IBC:END@@'
";

/// `sh -c '<script>' sh <args…>` so scripts run under POSIX `sh` whatever the login shell is.
pub(crate) fn sh(script: &str, args: &[&str]) -> String {
    let mut cmd = format!("sh -c '{}' sh", script.replace('\'', "'\\''"));
    for a in args {
        cmd.push(' ');
        cmd.push_str(a);
    }
    cmd
}

/// A container id as `docker ps` prints it; the only thing ever passed to the CLI.
fn valid_id(id: &str) -> bool {
    (12..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_hexdigit())
}

pub(crate) async fn run(
    conn: &Connection,
    cmd: &str,
    max: usize,
    t: Duration,
) -> Result<String, SshError> {
    let out = tokio::time::timeout(t, conn.exec(cmd, max))
        .await
        .map_err(|_| SshError::Timeout)??;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(text)
}

/// Lists containers with CPU / memory (one `docker ps` and one `docker stats --no-stream`).
///
/// Errors: [`SshError::Timeout`] after 15 s, network errors of the exec channel.
pub async fn docker_list(conn: &Connection) -> Result<DockerReport, SshError> {
    let text = run(conn, &sh(LIST_SCRIPT, &[]), MAX_LIST, LIST_TIMEOUT).await?;
    Ok(parse_list(&text))
}

/// Last `lines` log lines (stdout + stderr, with timestamps) of a container, ANSI stripped.
///
/// Errors: [`SshError::Protocol`] for a malformed id, [`SshError::Timeout`] after 12 s.
pub async fn docker_logs(conn: &Connection, id: &str, lines: u32) -> Result<String, SshError> {
    if !valid_id(id) {
        return Err(SshError::Protocol("bad container id".into()));
    }
    let n = lines.clamp(20, 2000).to_string();
    let cmd = sh(
        r#"docker logs --tail "$1" --timestamps "$2" 2>&1"#,
        &[&n, id],
    );
    let text = run(conn, &cmd, MAX_LOGS, LOGS_TIMEOUT).await?;
    Ok(clean_log(&text))
}

/// Starts, stops or restarts one container. Returns the CLI's error line on failure.
///
/// Errors: [`SshError::Protocol`] with the (sanitized) first error line, or for a bad id.
pub async fn docker_act(conn: &Connection, id: &str, action: DockerAction) -> Result<(), SshError> {
    if !valid_id(id) {
        return Err(SshError::Protocol("bad container id".into()));
    }
    let verb = match action {
        DockerAction::Start => "start",
        DockerAction::Stop => "stop",
        DockerAction::Restart => "restart",
    };
    let out = tokio::time::timeout(
        ACTION_TIMEOUT,
        conn.exec(&sh(r#"docker "$1" "$2""#, &[verb, id]), 16 * 1024),
    )
    .await
    .map_err(|_| SshError::Timeout)??;
    if out.exit_status.unwrap_or(0) == 0 {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let line = err
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("docker failed");
    Err(SshError::Protocol(sanitize(line)))
}

/// Keeps colour codes (SGR, `ESC[…m` with digits and `;` only) so the UI can show the
/// container's own colours; every other escape and control character except newline and tab
/// is dropped.
fn clean_log(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                let mut params = String::new();
                for d in chars.by_ref() {
                    if ('@'..='~').contains(&d) {
                        let sgr =
                            d == 'm' && params.chars().all(|p| p.is_ascii_digit() || p == ';');
                        if sgr && params.len() <= 32 {
                            out.push_str("\u{1b}[");
                            out.push_str(&params);
                            out.push('m');
                        }
                        break;
                    }
                    params.push(d);
                }
            }
            continue;
        }
        if c == '\n' || c == '\t' || !c.is_control() {
            out.push(c);
        }
    }
    out
}

/// Parses a size such as `12.5MiB`, `1.944GiB`, `512kB`, `0B` into bytes.
pub(crate) fn size(s: &str) -> Option<u64> {
    let s = s.trim();
    let split = s.find(|c: char| c.is_ascii_alphabetic())?;
    let (num, unit) = s.split_at(split);
    let v: f64 = num.trim().parse().ok()?;
    let mul: f64 = match unit {
        "B" => 1.0,
        "kB" | "KB" => 1e3,
        "KiB" => 1024.0,
        "MB" => 1e6,
        "MiB" => 1024.0 * 1024.0,
        "GB" => 1e9,
        "GiB" => 1024.0 * 1024.0 * 1024.0,
        "TB" => 1e12,
        "TiB" => 1024f64.powi(4),
        _ => return None,
    };
    // Sizes fit easily: f64 → u64 of a non-negative finite value.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some((v * mul).max(0.0) as u64)
}

fn rank(state: &str) -> u8 {
    match state {
        "running" => 0,
        "restarting" => 1,
        "paused" => 2,
        _ => 3,
    }
}

pub(crate) fn parse_list(text: &str) -> DockerReport {
    let sec = sections(text);
    if sec.contains_key("MISSING") {
        return DockerReport {
            access: DockerAccess::Missing,
            containers: vec![],
        };
    }
    let ps = sec.get("PS").map_or("", String::as_str);
    let lower = ps.to_ascii_lowercase();
    if lower.contains("permission denied") {
        return DockerReport {
            access: DockerAccess::Denied,
            containers: vec![],
        };
    }
    if lower.contains("cannot connect to the docker daemon")
        || lower.contains("is the docker daemon running")
    {
        return DockerReport {
            access: DockerAccess::Down,
            containers: vec![],
        };
    }
    let mut use_map = std::collections::HashMap::new();
    for line in sec.get("USE").map_or("", String::as_str).lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if let [id, cpu, mem] = f[..] {
            let cpu = cpu.trim().trim_end_matches('%').parse::<f32>().ok();
            let (used, limit) = mem.split_once('/').unwrap_or((mem, ""));
            use_map.insert(id.trim().to_string(), (cpu, size(used), size(limit)));
        }
    }
    let mut containers: Vec<Container> = ps
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            let [id, name, image, state, status, ports] = f[..] else {
                return None;
            };
            let id = id.trim();
            if !valid_id(id) {
                return None;
            }
            let short: String = id.chars().take(12).collect();
            let (cpu_pct, mem_bytes, mem_limit) = use_map
                .iter()
                .find(|(k, _)| k.starts_with(&short) || short.starts_with(k.as_str()))
                .map_or((None, None, None), |(_, v)| *v);
            let running = state.trim() == "running";
            Some(Container {
                id: short,
                name: sanitize(name),
                image: sanitize(image),
                state: sanitize(state),
                status: sanitize(status),
                ports: sanitize(ports),
                cpu_pct: if running { cpu_pct } else { None },
                mem_bytes: if running { mem_bytes } else { None },
                mem_limit: if running { mem_limit } else { None },
            })
        })
        .take(MAX_CONTAINERS)
        .collect();
    containers.sort_by(|a, b| {
        rank(&a.state)
            .cmp(&rank(&b.state))
            .then(a.name.cmp(&b.name))
    });
    DockerReport {
        access: DockerAccess::Ok,
        containers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_and_merges_usage() {
        let text = "@@IBC:PS@@\n\
            a1b2c3d4e5f6\tweb\tnginx:1.27\trunning\tUp 3 days\t0.0.0.0:80->80/tcp\n\
            0123456789ab\tjob\talpine\texited\tExited (0) 2 hours ago\t\n\
            @@IBC:USE@@\n\
            a1b2c3d4e5f6\t1.50%\t12.5MiB / 1.944GiB\n\
            @@IBC:END@@\n";
        let r = parse_list(text);
        assert_eq!(r.access, DockerAccess::Ok);
        assert_eq!(r.containers.len(), 2);
        let web = &r.containers[0];
        assert_eq!(web.name, "web");
        assert_eq!(web.cpu_pct, Some(1.5));
        assert_eq!(web.mem_bytes, Some(13_107_200));
        assert_eq!(r.containers[1].cpu_pct, None);
    }

    #[test]
    fn access_states() {
        assert_eq!(
            parse_list("@@IBC:MISSING@@\n").access,
            DockerAccess::Missing
        );
        let denied = "@@IBC:PS@@\npermission denied while trying to connect to the Docker daemon socket\n@@IBC:USE@@\n@@IBC:END@@\n";
        assert_eq!(parse_list(denied).access, DockerAccess::Denied);
        let down = "@@IBC:PS@@\nCannot connect to the Docker daemon at unix:///var/run/docker.sock. Is the docker daemon running?\n@@IBC:END@@\n";
        assert_eq!(parse_list(down).access, DockerAccess::Down);
    }

    #[test]
    fn ids_and_logs() {
        assert!(valid_id("a1b2c3d4e5f6"));
        assert!(!valid_id("web; rm -rf /"));
        assert!(!valid_id("abc"));
        assert_eq!(
            clean_log("\u{1b}[31mred\u{1b}[0m ok\r\n"),
            "\u{1b}[31mred\u{1b}[0m ok\n"
        );
        assert_eq!(clean_log("\u{1b}[2Jx\u{1b}]0;t\u{7}y"), "x]0;ty");
        assert_eq!(size("1.5KiB"), Some(1536));
    }
}
