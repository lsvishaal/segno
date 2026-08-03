# `just ci` is the gate. If it passes locally it passes in CI.

default: ci

ci: fmt-check lint test deny arch native-licences determinism

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

# FR-406 / NFR-300 / NFR-301, which MVP §9.2 makes release-blocking.
# Runs the harness twice in SEPARATE PROCESSES and diffs the bytes: the
# failures this catches (hash iteration order, anything address-derived) are
# invisible inside a single run.
determinism:
    ./scripts/check-determinism.sh

# The licence obligations cargo-deny cannot see, because Verovio is not a
# cargo dependency — it is a shared library opened by name (ADR-0005).
native-licences:
    ./scripts/check-native-licences.sh

# --- Verovio -----------------------------------------------------------------
#
# Deliberately OUTSIDE `just ci`. ADR-0005 loads Verovio at run time so that
# building and testing Hogwash needs no C++ toolchain and no shared library;
# putting these in `ci` would give that away. Both need:
#
#     HOGWASH_VEROVIO_LIB   path to libverovio.so / .dll / .dylib
#     HOGWASH_VEROVIO_DATA  path to verovio's data/ directory
#
# Build one with `just verovio-build ../verovio`.

# The adapter against a real engine. Everything else about it is pure and
# tested against captured output, with no library present.
engraving-test:
    cargo test --package hogwash-engraving-verovio -- --ignored

# FR-406 for the engraving: the same score must place its notes identically
# across separate processes, which S0 found is false unless the identifier
# generator is seeded before every load.
engraving-determinism:
    ./scripts/check-engraving-determinism.sh

# Builds libverovio.so from a clone, OUTSIDE this repository.
# Verovio is never vendored here and never committed (THIRD_PARTY.md).
verovio-build clone output=".verovio-build":
    cmake -S "{{clone}}/cmake" -B "{{output}}" -DBUILD_AS_LIBRARY=ON -DCMAKE_BUILD_TYPE=Release
    cmake --build "{{output}}" -j
    @echo
    @echo "Built. Point the recipes above at it:"
    @echo "  export HOGWASH_VEROVIO_LIB=$(realpath {{output}})/libverovio.so"
    @echo "  export HOGWASH_VEROVIO_DATA=$(realpath {{clone}})/data"

# Proves the licence gate actually fails on a forbidden licence.
# A gate nobody has seen fail is not a gate. The canary is added as a
# *normal* dependency of a throwaway crate, because cargo-deny does not
# evaluate dev-dependencies — see the note at the top of deny.toml.
verify-licence-gate:
    ./scripts/verify-licence-gate.sh

clean:
    cargo clean
