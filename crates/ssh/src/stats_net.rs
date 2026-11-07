//! Pure parsing of `ss` output into listening ports and per-peer TCP traffic (ADR-009
//! amendment 3). No IO. Addresses are kept as text: the UI only shows them.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::stats_parse::sanitize;

/// At most this many listening sockets are reported.
const MAX_LISTEN: usize = 32;
/// At most this many peer rows are reported (busiest first).
const MAX_PEERS: usize = 40;

/// A socket accepting connections (`ss -ltun`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Listener {
    /// `tcp` or `udp`.
    pub proto: String,
    /// Bound address (`0.0.0.0`, `::`, `127.0.0.1`…).
    pub addr: String,
    /// Port.
    pub port: u16,
    /// Owning program when the login user may see it (root), else `None`.
    pub process: Option<String>,
}

/// Established TCP traffic between this host and one remote address on one port.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PeerFlow {
    /// True when the remote side connected to us (our port is a listening one).
    pub inbound: bool,
    /// Our listening port for inbound flows, the remote port for outbound ones.
    pub port: u16,
    /// Remote IP address.
    pub peer: String,
    /// Open connections in this group.
    pub conns: u32,
    /// Bytes received over these connections' lifetime.
    pub bytes_in: u64,
    /// Bytes sent over these connections' lifetime.
    pub bytes_out: u64,
}

/// Splits `addr:port`, `[v6]:port` or `v6%iface:port`; strips IPv4-mapped prefixes.
fn split_endpoint(s: &str) -> Option<(String, u16)> {
    let (host, port) = s.rsplit_once(':')?;
    let port = port.parse().ok()?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let host = host.split('%').next().unwrap_or(host);
    let host = host.strip_prefix("::ffff:").unwrap_or(host);
    Some((sanitize(host), port))
}

/// `users:(("sshd",pid=1,fd=3),…)` → `sshd`.
fn process_name(rest: &str) -> Option<String> {
    let start = rest.find("((\"")? + 3;
    let end = rest[start..].find('"')? + start;
    Some(sanitize(&rest[start..end]))
}

/// Parses `ss -Hltunp`: `Netid State Recv-Q Send-Q Local Peer [Process]`.
pub(crate) fn parse_listen(s: &str) -> Vec<Listener> {
    let mut seen = HashSet::new();
    let mut out: Vec<Listener> = s
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            let proto = *f.first()?;
            if proto != "tcp" && proto != "udp" {
                return None;
            }
            let (addr, port) = split_endpoint(f.get(4)?)?;
            let process = f.get(6..).and_then(|r| process_name(&r.join(" ")));
            seen.insert((proto, port, process.clone()))
                .then(|| Listener {
                    proto: proto.to_string(),
                    addr,
                    port,
                    process,
                })
        })
        .collect();
    out.sort_by(|a, b| (a.port, &a.proto).cmp(&(b.port, &b.proto)));
    out.truncate(MAX_LISTEN);
    out
}

fn counter(line: &str, key: &str) -> u64 {
    line.split_whitespace()
        .find_map(|w| w.strip_prefix(key))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Parses `ss -Htni state established`: a socket line `Recv-Q Send-Q Local Peer` followed by
/// an indented `tcp_info` line with `bytes_sent:` / `bytes_received:`. Groups by direction,
/// port and peer; loopback traffic is skipped.
pub(crate) fn parse_flows(s: &str, listen: &[Listener]) -> (Vec<PeerFlow>, u32) {
    let ports: HashSet<u16> = listen
        .iter()
        .filter(|l| l.proto == "tcp")
        .map(|l| l.port)
        .collect();
    let mut groups: HashMap<(bool, u16, String), PeerFlow> = HashMap::new();
    let mut total = 0u32;
    let mut current: Option<(bool, u16, String)> = None;
    for line in s.lines() {
        if line.starts_with(char::is_whitespace) {
            if let Some(g) = current.take().and_then(|k| groups.get_mut(&k)) {
                g.bytes_in = g.bytes_in.saturating_add(counter(line, "bytes_received:"));
                // bytes_acked is what actually reached the peer; older kernels only have sent.
                let out = match counter(line, "bytes_acked:") {
                    0 => counter(line, "bytes_sent:"),
                    acked => acked,
                };
                g.bytes_out = g.bytes_out.saturating_add(out);
            }
            continue;
        }
        current = None;
        let f: Vec<&str> = line.split_whitespace().collect();
        let (Some(local), Some(peer)) = (f.get(2), f.get(3)) else {
            continue;
        };
        let (Some((_, lport)), Some((paddr, pport))) =
            (split_endpoint(local), split_endpoint(peer))
        else {
            continue;
        };
        if paddr.starts_with("127.") || paddr == "::1" {
            continue;
        }
        total = total.saturating_add(1);
        let inbound = ports.contains(&lport);
        let key = (inbound, if inbound { lport } else { pport }, paddr.clone());
        let g = groups.entry(key.clone()).or_insert(PeerFlow {
            inbound,
            port: key.1,
            peer: paddr,
            conns: 0,
            bytes_in: 0,
            bytes_out: 0,
        });
        g.conns += 1;
        current = Some(key);
    }
    let mut out: Vec<PeerFlow> = groups.into_values().collect();
    out.sort_by(|a, b| {
        (b.bytes_in + b.bytes_out)
            .cmp(&(a.bytes_in + a.bytes_out))
            .then(b.conns.cmp(&a.conns))
            .then(a.peer.cmp(&b.peer))
    });
    out.truncate(MAX_PEERS);
    (out, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTEN: &str = "tcp   LISTEN 0 4096 0.0.0.0:22 0.0.0.0:* users:((\"sshd\",pid=1,fd=3))
tcp   LISTEN 0 4096    [::]:22    [::]:* users:((\"sshd\",pid=1,fd=4))
tcp   LISTEN 0 511  *:443 *:* users:((\"xray\",pid=9,fd=7))
udp   UNCONN 0 0 0.0.0.0:51820 0.0.0.0:*
tcp   LISTEN 0 4096 127.0.0.53%lo:53 0.0.0.0:* users:((\"systemd-resolve\",pid=4,fd=14))
";

    const CONN: &str = "0 0 10.0.0.5:443 [::ffff:91.1.2.3]:51000
\t cubic rtt:1 bytes_sent:900 bytes_acked:800 bytes_received:5000 segs_out:3
0 0 10.0.0.5:443 91.1.2.3:51001
\t cubic bytes_acked:200 bytes_received:1000
0 36 10.0.0.5:22 85.9.9.9:60000
\t cubic bytes_sent:100 bytes_received:50
0 0 10.0.0.5:41000 1.1.1.1:443
\t cubic bytes_acked:10 bytes_received:20
0 0 127.0.0.1:5000 127.0.0.1:41234
\t cubic bytes_acked:99999 bytes_received:99999
";

    #[test]
    fn listeners_keep_ports_and_programs() {
        let l = parse_listen(LISTEN);
        assert_eq!(l.len(), 4, "v4/v6 duplicates of sshd collapse");
        assert_eq!(l[0].port, 22);
        assert_eq!(l[0].process.as_deref(), Some("sshd"));
        let dns = l.iter().find(|x| x.port == 53).map(|x| x.addr.as_str());
        assert_eq!(dns, Some("127.0.0.53"));
        assert!(l
            .iter()
            .any(|x| x.proto == "udp" && x.port == 51820 && x.process.is_none()));
    }

    #[test]
    fn flows_group_by_direction_port_and_peer() {
        let (f, total) = parse_flows(CONN, &parse_listen(LISTEN));
        assert_eq!(total, 4, "loopback is skipped");
        assert_eq!(f.len(), 3);
        let vpn = &f[0];
        assert!(vpn.inbound);
        assert_eq!(
            (vpn.port, vpn.peer.as_str(), vpn.conns),
            (443, "91.1.2.3", 2)
        );
        assert_eq!((vpn.bytes_in, vpn.bytes_out), (6000, 1000));
        let out = f
            .iter()
            .find(|x| !x.inbound)
            .map(|x| (x.port, x.peer.clone()));
        assert_eq!(out, Some((443, "1.1.1.1".into())));
    }

    #[test]
    fn garbage_is_ignored() {
        assert_eq!(parse_listen("nonsense\n\n"), vec![]);
        assert_eq!(parse_flows("x y\n\tbytes_received:5\n", &[]), (vec![], 0));
    }
}
