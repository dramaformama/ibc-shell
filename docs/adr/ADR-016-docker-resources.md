# ADR-016: Docker compose projects, images, volumes, networks and events

Status: accepted. Adds the IPC commands `docker_compose`, `docker_images`, `docker_volumes`, `docker_networks`, `docker_events` (all read-only and passive for auto-lock). Extends ADR-012.

Context: the Docker card only showed containers. Owners of VPN and web servers also ask which compose project a container belongs to, which images and volumes are unused and eat disk, how the networks are laid out and what just happened (a container died, an image was pulled).

Decision: same rules as ADR-012: one exec channel per call over the session's existing connection, only the `docker` CLI already on the server, nothing installed, no port opened or bound.
- `docker_compose`: no `docker compose` plugin needed; projects are grouped from the labels `com.docker.compose.project`, `.service`, `.project.working_dir` that `docker ps -a` prints. Only reading, no compose actions.
- `docker_images` / `docker_volumes` / `docker_networks`: `docker images`, `docker volume ls`, `docker network ls` + `inspect`, joined with `docker ps -a` to count users (0 = unused). Volume sizes come from `docker system df -v`, which walks the volumes, so the call has 25 s and the UI asks for it only while the tab is open (every 60 s, shown only when visible).
- `docker_events`: `docker events --since <now - window_s> --until <now>`: the CLI prints the history and exits, so no long-lived process stays on a server that may be running a VPN. The UI asks every 5 s only while the events tab is visible. Command lines carried by `exec_*` events (they may hold secrets) are cut at the colon and the exec / health / attach noise is dropped.
- Every list is capped (≤ 150 rows, ≤ 200 events); all server strings lose control characters and are capped like the other services; they are shown as text, never HTML.
- Values reaching the shell are only the numeric `window_s` (clamped, decimal) passed as a positional argument; ids are never taken from the UI here.

Rejected: `docker events` as a live stream (a process that lives on the server, needs reconnect logic); Docker socket tunnel (full API access, needs a forward); compose start / stop (state-changing, not needed for visibility yet).
