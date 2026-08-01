#!/usr/bin/env bash
#
# Proves the licence allow-list in deny.toml actually rejects something.
#
# A gate nobody has seen fail is not a gate. This creates a throwaway
# workspace member with a copyleft dependency, asserts `cargo deny check
# licenses` fails on it, and removes it again.
#
# The canary must be a NORMAL dependency. cargo-deny does not evaluate
# dev-dependencies, so a canary placed under [dev-dependencies] reports
# "licenses ok" and silently proves nothing.

set -euo pipefail

cd "$(dirname "$0")/.."

CANARY_DIR="crates/zz-licence-gate-canary"

cleanup() {
    rm -rf "$CANARY_DIR"
}
trap cleanup EXIT

if [ -e "$CANARY_DIR" ]; then
    echo "error: $CANARY_DIR already exists; refusing to overwrite it." >&2
    exit 1
fi

mkdir -p "$CANARY_DIR/src"

cat > "$CANARY_DIR/Cargo.toml" <<'EOF'
[package]
name = "zz-licence-gate-canary"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
# GPL-family. Must be rejected by deny.toml's allow-list.
sequoia-openpgp = "1"
EOF

echo '' > "$CANARY_DIR/src/lib.rs"

echo "Checking that a copyleft dependency is rejected..."

# Captured rather than piped: cargo-deny exits non-zero on rejection, which
# under `set -o pipefail` would fail the pipeline no matter what grep found.
# That is exactly backwards here — a non-zero exit is the outcome we want.
output=$(cargo deny check licenses 2>&1 || true)
echo "$output"

if echo "$output" | grep -q "licenses FAILED"; then
    echo
    echo "PASS: the licence gate rejected the copyleft canary."
    exit 0
fi

echo
echo "FAIL: the licence gate did NOT reject a copyleft dependency." >&2
echo "The clean-room boundary is not being enforced. Do not trust it." >&2
exit 1
