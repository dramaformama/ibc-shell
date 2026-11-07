# ADR-012: Docker containers and TLS certificates of a server

Status: accepted. Adds the IPC commands `docker_list` (passive), `docker_logs`, `docker_action`, `cert_list` (passive) and the `HostStats` field `cpu_core_split` (per-core user / system / nice / iowait / steal).

Context: owners of VPN and web servers want to see their containers and when their certificates expire without typing commands, for one server and for all connected servers at once (Docker and Domains pages, like SwiftServer's tabs).

Decision: everything runs over the session's existing SSH connection as one exec channel (like ADR-009), using only tools already on the server; nothing is installed, no port is opened or bound.
- `docker_list`: `docker ps -a` + `docker stats --no-stream`, 15 s. Access is reported as `Ok / Missing / Denied / Down`; polling stops for anything but `Ok`. Every 20 s while a view watches the session (one loop per session shared by all views).
- `docker_logs`: `docker logs --tail N --timestamps <id>`, N 20–2000, ≤ 512 KiB. Colour codes (SGR only) are kept so the UI can show the container's own colours; all other escapes are stripped. Rendered as text segments, never HTML.
- `docker_action`: `docker start|stop|restart <id>` — the only state-changing command. The id must be 12–64 hex chars and is passed as a positional argument to `sh -c`, never interpolated. Stop and restart are confirmed in the UI (the dialog warns that VPN clients drop). Unlocked vault required.
- `cert_list`: `openssl x509 -noout -enddate -subject -issuer` (and the DNS names) for certificate files of Let's Encrypt `live`, nginx / Apache `ssl_certificate` / `SSLCertificateFile`, Caddy and `/root/cert`; ≤ 48 files, 15 s, every 10 min. File contents never leave the server, private keys are never opened as keys. Unreadable places are counted (`denied`) so the UI can say root is needed.

Rejected: probing each domain's port 443 from the Mac (leaks which domains the user owns to the network path, misses certificates behind other ports); `sudo -n` (writes failed-sudo lines to auth logs); a Docker socket tunnel (needs a forward and gives full API access).
