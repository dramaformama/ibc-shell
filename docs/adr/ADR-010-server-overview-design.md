# ADR-010: Server-first overview and shared desktop design

Status: accepted, 2026-11-07. UI contract v1.1, IPC addition `host_probe`.

The saved host catalog, not the open-tab list, defines the overview. A fleet module joins catalog, active sessions and observation results. Terminal and Files remain views of a server. SFTP reuses its terminal connection when connected; independent SFTP remains available otherwise. File views remain mounted across navigation, preserving the connection, directory, selection and pane configuration; closing the server/vault unmounts them.

Reachability and authentication are separate facts: `host_probe(host_id) -> HostReachability { reachable: bool, latency_ms: u32|null, checked_at: i64 }` performs a bounded (3 s) TCP connection to the resolved SSH port, without login, remote commands or accepting host keys. Online means this port is reachable, not authenticated or healthy. Hosts behind jump hosts return `Unsupported` (unknown without the tunnel); a live SSH session proves reachability. Latency measures TCP connect including name resolution, not ICMP RTT. Vault must be unlocked; passive command, never resets auto-lock. No external service. Maximum four concurrent probes, visible cards poll every 30 s; navigation/lock removes subscribers. Errors `NotFound`, `VaultLocked`, `Unsupported`, `Internal`; connection failure/timeouts return reachable=false.

Metrics continue using `stats_sample` on authenticated sessions (ADR-009). Missing, stale, unreachable and authentication failure are separate states; unavailable measurements never become zero. Opening a host does not imply successful connection. The overview does not automatically authenticate hosts or start shells. It provides Connect / Terminal / Files / Edit actions.

Design: shared tokens and controls across overview, sidebar, tabs, forms, dialogs and SFTP. Light default, system typography, 4 pt spacing; control heights 32 (regular), 28 (compact), 36 (prominent); radius 8 controls, 12 grouped fields, 16 cards, 20 sheets. Quiet hairlines, subtle shadows, semantic colors and consistent keyboard focus. Full text status accompanies color. No third-party branding/assets. Virtualized server grid supports large catalogs. Search and status filters operate on the same fleet model.

The Rust/store/vault/SSH separation remains intact; this ADR adds no core crate dependencies and does not replace russh with Traversio. Reference findings in `docs/swiftserver-reference.md` distinguish published behavior from inferred internal design.
