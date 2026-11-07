//! Pure parsing of the stats script output (see `stats::SCRIPT`). No IO.

use std::collections::HashMap;

use crate::stats::{HostStats, NetIface, ProcRow};
use crate::SshError;

const MAX_STR: usize = 128;

/// Strips control characters and caps the length.
pub(crate) fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(MAX_STR)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Splits `@@IBC:NAME@@`-delimited output into sections.
pub(crate) fn sections(text: &str) -> HashMap<&str, String> {
    let mut map: HashMap<&str, String> = HashMap::new();
    let mut cur: Option<&str> = None;
    for line in text.lines() {
        let t = line.trim_end();
        if let Some(name) = t.strip_prefix("@@IBC:").and_then(|r| r.strip_suffix("@@")) {
            cur = Some(name);
            map.entry(name).or_default();
        } else if let Some(name) = cur {
            if let Some(buf) = map.get_mut(name) {
                buf.push_str(line);
                buf.push('\n');
            }
        }
    }
    map
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn pct(part: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (part as f64 * 100.0 / total as f64).clamp(0.0, 100.0) as f32
    }
}

/// Jiffies of one `cpu*` line: user, nice, system, idle, iowait, irq, softirq, steal.
type Jiffies = [u64; 8];

fn parse_stat(s: &str) -> (Option<Jiffies>, Vec<Jiffies>) {
    let (mut all, mut cores) = (None, Vec::new());
    for line in s.lines() {
        let mut it = line.split_whitespace();
        let Some(name) = it.next() else { continue };
        if !name.starts_with("cpu") {
            continue;
        }
        let mut j = [0u64; 8];
        for slot in &mut j {
            *slot = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        }
        if name == "cpu" {
            all = Some(j);
        } else {
            cores.push(j);
        }
    }
    (all, cores)
}

fn delta(a: &Jiffies, b: &Jiffies) -> Jiffies {
    let mut d = [0u64; 8];
    for (i, slot) in d.iter_mut().enumerate() {
        *slot = b[i].saturating_sub(a[i]);
    }
    d
}

fn busy_pct(d: &Jiffies) -> f32 {
    let total: u64 = d.iter().sum();
    pct(total.saturating_sub(d[3] + d[4]), total)
}

/// Interfaces shown at most.
const MAX_IFACES: usize = 8;

/// (name, rx bytes, tx bytes) of every non-loopback interface.
fn parse_net(s: &str) -> Vec<(String, u64, u64)> {
    s.lines()
        .filter_map(|line| {
            let (name, rest) = line.split_once(':')?;
            let name = name.trim();
            if name == "lo" || name.contains(' ') {
                return None;
            }
            let f: Vec<&str> = rest.split_whitespace().collect();
            let rx = f.first().and_then(|v| v.parse::<u64>().ok())?;
            let tx = f.get(8).and_then(|v| v.parse::<u64>().ok())?;
            Some((sanitize(name), rx, tx))
        })
        .collect()
}

/// Per-interface rates between two snapshots, busiest first.
fn ifaces(a: &[(String, u64, u64)], b: &[(String, u64, u64)], dt: f64) -> Vec<NetIface> {
    let mut out: Vec<NetIface> = b
        .iter()
        .map(|(name, rx2, tx2)| {
            let (rx1, tx1) = a
                .iter()
                .find(|(n, _, _)| n == name)
                .map_or((*rx2, *tx2), |(_, r, t)| (*r, *t));
            NetIface {
                name: name.clone(),
                rx_bps: rate(rx1, *rx2, dt),
                tx_bps: rate(tx1, *tx2, dt),
                rx_total: *rx2,
                tx_total: *tx2,
            }
        })
        .collect();
    out.sort_by_key(|i| std::cmp::Reverse(i.rx_bps.saturating_add(i.tx_bps)));
    out.truncate(MAX_IFACES);
    out
}

fn is_whole_disk(name: &str) -> bool {
    let digit_end = name.ends_with(|c: char| c.is_ascii_digit());
    if name.starts_with("nvme") || name.starts_with("mmcblk") {
        return match name.rsplit_once('p') {
            Some((head, tail)) => {
                !(head.ends_with(|c: char| c.is_ascii_digit())
                    && !tail.is_empty()
                    && tail.chars().all(|c| c.is_ascii_digit()))
            }
            None => true,
        };
    }
    ["sd", "vd", "xvd", "hd"]
        .iter()
        .any(|p| name.starts_with(p))
        && !digit_end
}

/// Total (read, written) bytes over whole disks (sectors × 512).
fn parse_disks(s: &str) -> (u64, u64) {
    let (mut r, mut w) = (0u64, 0u64);
    for line in s.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 10 || !is_whole_disk(f[2]) {
            continue;
        }
        r += f[5].parse::<u64>().unwrap_or(0).saturating_mul(512);
        w += f[9].parse::<u64>().unwrap_or(0).saturating_mul(512);
    }
    (r, w)
}

fn first_f64(s: &str) -> f64 {
    s.split_whitespace()
        .next()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn rate(a: u64, b: u64, dt: f64) -> u64 {
    (b.saturating_sub(a) as f64 / dt) as u64
}

fn meminfo(s: &str) -> HashMap<&str, u64> {
    s.lines()
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            let kb: u64 = v.split_whitespace().next()?.parse().ok()?;
            Some((k, kb.saturating_mul(1024)))
        })
        .collect()
}

fn parse_procs(s: &str) -> Vec<ProcRow> {
    s.lines()
        .skip(1)
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let pid = it.next()?.parse().ok()?;
            let user = sanitize(it.next()?);
            let cpu = it.next()?.parse().ok()?;
            let rss_kb: u64 = it.next()?.parse().ok()?;
            let command = sanitize(&it.collect::<Vec<_>>().join(" "));
            Some(ProcRow {
                pid,
                user,
                cpu,
                mem_bytes: rss_kb.saturating_mul(1024),
                command,
            })
        })
        .take(8)
        .collect()
}

/// Parses the whole script output; `full` = the slow sections were requested.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn parse_report(text: &str, full: bool) -> Result<HostStats, SshError> {
    let sec = sections(text);
    let get = |k: &str| sec.get(k).map_or("", String::as_str);
    if sec.contains_key("UNSUPPORTED") || !sec.contains_key("STAT2") {
        return Err(SshError::Unsupported);
    }
    let mut h = HostStats {
        full,
        ..HostStats::default()
    };

    let (a1, c1) = parse_stat(get("STAT1"));
    let (a2, c2) = parse_stat(get("STAT2"));
    if let (Some(a), Some(b)) = (a1, a2) {
        let d = delta(&a, &b);
        let total: u64 = d.iter().sum();
        h.cpu_pct = busy_pct(&d);
        h.cpu_user = pct(d[0], total);
        h.cpu_nice = pct(d[1], total);
        h.cpu_system = pct(d[2] + d[5] + d[6], total);
        h.cpu_iowait = pct(d[4], total);
        h.cpu_steal = pct(d[7], total);
    }
    h.cpu_cores = c1
        .iter()
        .zip(&c2)
        .map(|(a, b)| busy_pct(&delta(a, b)))
        .collect();
    h.cores = get("NPROC")
        .trim()
        .parse()
        .unwrap_or(u32::try_from(c2.len()).unwrap_or(0));

    let dt = (first_f64(get("UP2")) - first_f64(get("UP1"))).max(0.0);
    let dt = if dt < 0.1 { 1.0 } else { dt };
    let (n1, n2) = (parse_net(get("NET1")), parse_net(get("NET2")));
    let all = ifaces(&n1, &n2, dt);
    h.net_rx_bps = all.iter().map(|i| i.rx_bps).sum();
    h.net_tx_bps = all.iter().map(|i| i.tx_bps).sum();
    h.ifaces = all;
    let (dr1, dw1) = parse_disks(get("DISK1"));
    let (dr2, dw2) = parse_disks(get("DISK2"));
    h.disk_read_bps = rate(dr1, dr2, dt);
    h.disk_write_bps = rate(dw1, dw2, dt);

    let m = meminfo(get("MEM"));
    let g = |k: &str| m.get(k).copied().unwrap_or(0);
    h.mem_total = g("MemTotal");
    let avail = m
        .get("MemAvailable")
        .copied()
        .unwrap_or_else(|| g("MemFree") + g("Buffers") + g("Cached"));
    h.mem_used = h.mem_total.saturating_sub(avail);
    h.mem_cached = g("Cached");
    h.mem_free = g("MemFree");
    h.swap_total = g("SwapTotal");
    h.swap_used = h.swap_total.saturating_sub(g("SwapFree"));

    let load: Vec<f32> = get("LOAD")
        .split_whitespace()
        .take(3)
        .filter_map(|v| v.parse().ok())
        .collect();
    if let [l1, l5, l15] = load[..] {
        (h.load1, h.load5, h.load15) = (l1, l5, l15);
    }
    h.uptime_s = first_f64(get("UP2")) as u64;

    if let Some(line) = get("DF").lines().nth(1) {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() >= 3 {
            h.disk_total = f[1].parse::<u64>().unwrap_or(0).saturating_mul(1024);
            h.disk_used = f[2].parse::<u64>().unwrap_or(0).saturating_mul(1024);
        }
    }
    let os = sanitize(get("OS").trim().trim_matches(|c| c == '"' || c == '\''));
    h.os_name = (!os.is_empty()).then_some(os);
    h.temp_c = get("TEMP").trim().parse::<f32>().ok().map(|t| t / 1000.0);
    h.procs = parse_procs(get("PS"));
    h.listen = crate::stats_net::parse_listen(get("LISTEN"));
    (h.flows, h.conns) = crate::stats_net::parse_flows(get("CONN"), &h.listen);
    Ok(h)
}

#[cfg(test)]
#[path = "stats_parse_tests.rs"]
mod tests;
