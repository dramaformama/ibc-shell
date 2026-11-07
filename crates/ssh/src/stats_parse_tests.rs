//! Tests of `stats_parse`.

use super::*;

const REPORT: &str = "@@IBC:STAT1@@
cpu  100 0 100 800 0 0 0 0 0 0
cpu0 50 0 50 400 0 0 0 0 0 0
cpu1 50 0 50 400 0 0 0 0 0 0
@@IBC:NET1@@
Inter-|   Receive
 face |bytes packets errs drop fifo frame compressed multicast|bytes packets
lo: 5000 1 0 0 0 0 0 0 5000 1 0 0 0 0 0 0
  eth0: 1000 1 0 0 0 0 0 0 2000 1 0 0 0 0 0 0
@@IBC:DISK1@@
   8       0 sda 1 0 100 0 1 0 200 0
   8       1 sda1 1 0 100 0 1 0 200 0
 259       0 nvme0n1 1 0 10 0 1 0 20 0
 259       1 nvme0n1p1 1 0 10 0 1 0 20 0
   7       0 loop0 1 0 999 0 1 0 999 0
@@IBC:UP1@@
100.00 50.00
@@IBC:STAT2@@
cpu  150 10 130 820 10 0 5 5 0 0
cpu0 100 0 80 410 0 0 0 0 0 0
cpu1 50 10 50 410 10 0 5 5 0 0
@@IBC:NET2@@
lo: 9000 1 0 0 0 0 0 0 9000 1 0 0 0 0 0 0
  eth0: 3000 1 0 0 0 0 0 0 6000 1 0 0 0 0 0 0
@@IBC:DISK2@@
   8       0 sda 1 0 300 0 1 0 700 0
   8       1 sda1 1 0 300 0 1 0 700 0
 259       0 nvme0n1 1 0 10 0 1 0 20 0
 259       1 nvme0n1p1 1 0 10 0 1 0 20 0
   7       0 loop0 1 0 9999 0 1 0 9999 0
@@IBC:UP2@@
101.00 50.00
@@IBC:MEM@@
MemTotal:       8000000 kB
MemFree:        1000000 kB
MemAvailable:   5000000 kB
Cached:         2000000 kB
SwapTotal:      1000000 kB
SwapFree:        400000 kB
@@IBC:LOAD@@
0.50 0.75 1.25 1/100 999
@@IBC:DF@@
Filesystem 1024-blocks Used Available Capacity Mounted on
/dev/vda1 100000 40000 60000 40% /
@@IBC:NPROC@@
2
@@IBC:OS@@
\"Ubuntu 24.04 LTS\"
@@IBC:PS@@
  PID USER %CPU RSS COMMAND
1 root  1.5 2048 systemd
  42 bob\x1b[31m 0.5 1024 my proc
@@IBC:TEMP@@
45500
@@IBC:END@@
";

#[test]
fn parses_full_report() {
    let h = parse_report(REPORT, true).unwrap();
    // delta: user 50, nice 10, sys 30, idle 20, iowait 10, softirq 5, steal 5 = 130
    assert!(
        (h.cpu_pct - 100.0 * 100.0 / 130.0).abs() < 0.1,
        "{}",
        h.cpu_pct
    );
    assert!((h.cpu_user - 100.0 * 50.0 / 130.0).abs() < 0.1);
    assert!((h.cpu_system - 100.0 * 35.0 / 130.0).abs() < 0.1);
    assert_eq!(h.cpu_cores.len(), 2);
    assert!((h.cpu_cores[0] - 100.0 * 80.0 / 90.0).abs() < 0.1);
    assert_eq!(h.cpu_core_split.len(), 2);
    let c = h.cpu_core_split[0];
    let sum = c.user + c.system + c.nice + c.iowait + c.steal;
    assert!(sum >= h.cpu_cores[0] - 0.1 && sum <= 100.1);
    assert_eq!(h.cores, 2);
    assert_eq!((h.net_rx_bps, h.net_tx_bps), (2000, 4000));
    assert_eq!((h.disk_read_bps, h.disk_write_bps), (200 * 512, 500 * 512));
    assert_eq!(h.mem_total, 8_000_000 * 1024);
    assert_eq!(h.mem_used, 3_000_000 * 1024);
    assert_eq!(h.mem_cached, 2_000_000 * 1024);
    assert_eq!(h.swap_used, 600_000 * 1024);
    assert_eq!((h.disk_total, h.disk_used), (100_000 * 1024, 40_000 * 1024));
    assert_eq!((h.load1, h.load5, h.load15), (0.5, 0.75, 1.25));
    assert_eq!(h.uptime_s, 101);
    assert_eq!(h.os_name.as_deref(), Some("Ubuntu 24.04 LTS"));
    assert_eq!(h.temp_c, Some(45.5));
    assert_eq!(h.mem_free, 1_000_000 * 1024);
    assert_eq!(h.ifaces.len(), 1);
    assert_eq!(h.ifaces[0].name, "eth0");
    assert_eq!((h.ifaces[0].rx_bps, h.ifaces[0].tx_bps), (2000, 4000));
    assert_eq!(h.ifaces[0].rx_total, 3000);
    assert!(h.full);
}

#[test]
fn fast_sample_has_no_slow_fields() {
    let fast: String = REPORT
        .split("@@IBC:DF@@")
        .next()
        .unwrap_or_default()
        .to_string()
        + "@@IBC:END@@\n";
    let h = parse_report(&fast, false).unwrap();
    assert!(!h.full);
    assert_eq!(h.disk_total, 0);
    assert_eq!(h.procs.len(), 0);
    assert_eq!(h.cores, 2); // falls back to the per-core count
    assert!(h.cpu_pct > 0.0);
}

#[test]
fn procs_are_sanitized_and_joined() {
    let h = parse_report(REPORT, true).unwrap();
    assert_eq!(h.procs.len(), 2);
    assert_eq!(h.procs[0].command, "systemd");
    assert_eq!(h.procs[0].mem_bytes, 2048 * 1024);
    assert_eq!(h.procs[1].user, "bob[31m");
    assert_eq!(h.procs[1].command, "my proc");
}

#[test]
fn unsupported_hosts() {
    assert!(matches!(
        parse_report("@@IBC:UNSUPPORTED@@\n", false),
        Err(SshError::Unsupported)
    ));
    assert!(matches!(
        parse_report("garbage", false),
        Err(SshError::Unsupported)
    ));
}

#[test]
fn disk_filter() {
    for ok in ["sda", "vdb", "xvda", "nvme0n1", "mmcblk0"] {
        assert!(is_whole_disk(ok), "{ok}");
    }
    for bad in [
        "sda1",
        "nvme0n1p2",
        "mmcblk0p1",
        "loop0",
        "ram0",
        "dm-0",
        "zram0",
        "sr0",
    ] {
        assert!(!is_whole_disk(bad), "{bad}");
    }
}

#[test]
fn sanitize_caps_and_strips() {
    assert_eq!(sanitize("a\x07b\n"), "ab");
    assert_eq!(sanitize(&"x".repeat(500)).chars().count(), 128);
}
