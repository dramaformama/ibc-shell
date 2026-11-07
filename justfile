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

# Regenerate ui/src/lib/api/bindings.ts (placeholder until P0.3)
bindings:
    @echo "bindings: not implemented yet (P0.3)"

# Docker integration tests (placeholder until tests/docker exists)
e2e:
    @echo "e2e: not implemented yet"

# Rename the project (placeholder)
rename name:
    @echo "rename -> {{name}}: not implemented yet"
