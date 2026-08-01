# `just ci` is the gate. If it passes locally it passes in CI.

default: ci

ci: fmt-check lint test deny arch

check:
    cargo check --workspace --all-targets

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

deny:
    cargo deny check

arch:
    ./scripts/check-arch.sh

# Proves the licence gate actually fails on a forbidden licence.
# A gate nobody has seen fail is not a gate. The canary is added as a
# *normal* dependency of a throwaway crate, because cargo-deny does not
# evaluate dev-dependencies — see the note at the top of deny.toml.
verify-licence-gate:
    ./scripts/verify-licence-gate.sh

clean:
    cargo clean
