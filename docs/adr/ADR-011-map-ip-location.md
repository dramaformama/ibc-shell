# ADR-011: Server location on the map by IP

Status: accepted. Adds the IPC command `host_lookup_ip` (passive), the CSP `connect-src` origin `https://api.ipapi.is` and the settings keys `map.geoip` (bool, off by default) and `map.geo` (cache).

Context: pins placed by the server's flag are only as good as the flag, and a folder's inherited flag puts all its servers in one spot. Owners want the real location of each server.

Decision: opt-in, per device. When the user turns on "Location by IP" on the map (the switch states what is sent and to whom), the UI resolves each saved host name through `host_lookup_ip` (DNS on the backend, no connection), then asks `https://api.ipapi.is/?q=<ip>` for the city, country and coordinates — the same read-only UI-side GET pattern as ADR-008. Only server IP addresses leave the device; no host names, labels, credentials or identifiers. Results are cached in settings `map.geo` per host (`addr, ip, cc, city, lat, lon, at`), kept until the host is deleted or its address is edited (then looked up again), at most 4 lookups in flight; a failed lookup is retried next session only. Private and reserved addresses are never sent. Hosts without a result fall back to their flag. Pins are grouped by city.

Rejected: a bundled offline database (`crates/geo`: tens of MB for city precision, licence attribution, staleness) — may still come later for country-level fallback; resolving on the server over SSH (needs a connection and an outbound request from the server).
