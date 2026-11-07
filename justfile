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
    cargo run -q -p termlite --example export_bindings
    node scripts/check-mock-coverage.mjs

# Docker integration tests (placeholder until tests/docker exists)
e2e:
    @echo "e2e: not implemented yet"

# Rename the project (placeholder)
rename name:
    @echo "rename -> {{name}}: not implemented yet"
