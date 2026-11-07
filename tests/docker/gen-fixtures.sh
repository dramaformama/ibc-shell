#!/usr/bin/env bash
# Generates TEST-ONLY keys into tests/fixtures/keys (gitignored). Idempotent.
set -euo pipefail
D="$(cd "$(dirname "$0")/.." && pwd)/fixtures/keys"
mkdir -p "$D"; cd "$D"
gen() { [ -f "$1" ] || ssh-keygen -q -t "$2" -b "$3" -N "$4" -C "ibc-test-$1" -f "$1"; }
gen id_ed25519 ed25519 256 ""
gen id_rsa4096 rsa 4096 ""
gen id_ecdsa_p256 ecdsa 256 ""
gen id_ed25519_enc ed25519 256 "test-pass-123"   # passphrase: test-pass-123
chmod 600 id_*
ls "$D"
