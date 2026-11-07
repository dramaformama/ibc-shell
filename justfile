set shell := ["bash", "-cu"]

default:
    @just --list

# Run the desktop app in dev mode
dev:
    pnpm tauri dev

# Rust + UI lint
lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    pnpm -C ui check
    pnpm -C ui lint

# All unit tests
test:
    cargo test --workspace
    pnpm -C ui test

# Regenerate ui/src/lib/api/bindings.ts from the tauri-specta builder, then check mock coverage
bindings:
    cargo run -q -p ibc-shell --example export_bindings
    node scripts/check-mock-coverage.mjs

# Docker integration tests: fixtures, servers up, tests, servers down
e2e:
    tests/docker/gen-fixtures.sh
    docker compose -f tests/docker/compose.yml up -d --build --wait
    trap 'docker compose -f tests/docker/compose.yml down -v' EXIT; TL_E2E=1 cargo test -p ibc-shell-ssh -p ibc-shell-sftp --test '*'

# Rename the project (placeholder)
rename name:
    @echo "rename -> {{name}}: not implemented yet"
