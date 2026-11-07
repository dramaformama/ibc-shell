# E2E test servers
`just e2e` generates key fixtures, starts `tests/docker/compose.yml`, runs tests with `TL_E2E=1`, tears down.
User `tester`; password `test-pw-123`; keys `tests/fixtures/keys/id_*` (encrypted one: passphrase `test-pass-123`).
Ports (127.0.0.1): 2201 ssh-pass, 2202 ssh-key, 2203 ssh-kbd (password via keyboard-interactive), 2204 bastion (key; forwards to `internal:22`), 2205 ssh-evil (password).
