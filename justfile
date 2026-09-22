# Bare `just` lists commands and does nothing else — never build, test, or
# launch anything just because you typed the shortest possible thing.

# lists every command below — this is what bare `just` runs
default:
    @just --list --unsorted

# `verovio_lib` / `verovio_data` default to the sibling checkout this repo has
# always been developed against (`../verovio`, built into `../verovio-build`
# by `just verovio-build`). An explicit HOGWASH_VEROVIO_LIB/DATA in the
# environment always wins — these defaults exist so a machine that already
# has Verovio built beside this repo needs no manual exporting at all.
verovio_lib := env_var_or_default("HOGWASH_VEROVIO_LIB", justfile_directory() + "/../verovio-build/libverovio.so")
verovio_data := env_var_or_default("HOGWASH_VEROVIO_DATA", justfile_directory() + "/../verovio/data")

# --- run it ------------------------------------------------------------------

# launches the app with live reload — the command you actually want (Verovio is found automatically)
[group('run')]
run: _require-verovio
    cd app && HOGWASH_VEROVIO_LIB="{{verovio_lib}}" HOGWASH_VEROVIO_DATA="{{verovio_data}}" npx tauri dev

# alias for `run`
[group('run')]
dev: run

# --- check your work ---------------------------------------------------------

# the full gate — if this passes locally it passes in CI
[group('check')]
ci: fmt-check lint test deny arch native-licences determinism ui

# typecheck everything without building
[group('check')]
check:
    cargo check --workspace --all-targets

# reformat the whole workspace
[group('check')]
fmt:
    cargo fmt --all

# fail if anything is unformatted
[group('check')]
fmt-check:
    cargo fmt --all --check

# clippy, warnings deny
[group('check')]
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# every workspace test (fast — no Verovio needed, see the [verovio] group)
[group('check')]
test:
    cargo test --workspace

# dependency licence/advisory check
[group('check')]
deny:
    cargo deny check

# the crate-dependency-direction guard
[group('check')]
arch:
    ./scripts/check-arch.sh

# FR-406: runs the harness twice in separate processes and diffs the bytes (catches non-determinism a single run can't)
[group('check')]
determinism:
    ./scripts/check-determinism.sh

# licence obligations cargo-deny can't see, since Verovio is a shared library opened by name (ADR-0005)
[group('check')]
native-licences:
    ./scripts/check-native-licences.sh

# proves the licence gate actually fails on a forbidden licence
[group('check')]
verify-licence-gate:
    ./scripts/verify-licence-gate.sh

# frontend typecheck + tests, both — part of `just ci`. Needs `npm install` in `app/` once
[group('check')]
ui: ui-check ui-test

# frontend typecheck only
[group('check')]
ui-check:
    cd app && npx tsc --noEmit

# frontend tests only
[group('check')]
ui-test:
    cd app && npx vitest run --passWithNoTests

# cargo clean
[group('check')]
clean:
    cargo clean

# --- Verovio -------------------------------------------------------------
#
# ADR-0005 loads Verovio at run time, so building and testing the rest of
# Hogwash needs no C++ toolchain and no shared library — that's why these
# stay separate from `just ci` rather than folded into it. All of them
# default to the sibling `../verovio` / `../verovio-build` checkout; an
# explicit HOGWASH_VEROVIO_LIB/DATA always overrides that.

# builds libverovio.so from a clone, OUTSIDE this repo (never vendored, never committed)
[group('verovio')]
verovio-build clone output=".verovio-build":
    cmake -S "{{clone}}/cmake" -B "{{output}}" -DBUILD_AS_LIBRARY=ON -DCMAKE_BUILD_TYPE=Release
    cmake --build "{{output}}" -j
    @echo
    @echo "Built. Point the recipes above at it:"
    @echo "  export HOGWASH_VEROVIO_LIB=$(realpath {{output}})/libverovio.so"
    @echo "  export HOGWASH_VEROVIO_DATA=$(realpath {{clone}})/data"

# fails with one clear message when Verovio isn't built yet, instead of a
# confusing runtime error several layers down
[group('verovio')]
[private]
_require-verovio:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -f "{{verovio_lib}}" ] || [ ! -d "{{verovio_data}}" ]; then
        echo "Verovio not found:" >&2
        echo "  library: {{verovio_lib}}" >&2
        echo "  data:    {{verovio_data}}" >&2
        echo >&2
        echo "Build it with:   just verovio-build ../verovio" >&2
        echo "...or point at an existing build:" >&2
        echo "  export HOGWASH_VEROVIO_LIB=/path/to/libverovio.so" >&2
        echo "  export HOGWASH_VEROVIO_DATA=/path/to/verovio/data" >&2
        exit 1
    fi

# the engraving adapter against a real engine (everything else about it is tested without one)
[group('verovio')]
engraving-test: _require-verovio
    HOGWASH_VEROVIO_LIB="{{verovio_lib}}" HOGWASH_VEROVIO_DATA="{{verovio_data}}" \
        cargo test --package hogwash-engraving-verovio -- --ignored

# the shipped practice pieces, against a real engine — guards that they actually load
[group('verovio')]
provided-test: _require-verovio
    HOGWASH_VEROVIO_LIB="{{verovio_lib}}" HOGWASH_VEROVIO_DATA="{{verovio_data}}" \
        cargo test --package hogwash-app -- --ignored

# FR-406 for engraving: the same score must place its notes identically across processes
[group('verovio')]
engraving-determinism: _require-verovio
    HOGWASH_VEROVIO_LIB="{{verovio_lib}}" HOGWASH_VEROVIO_DATA="{{verovio_data}}" \
        ./scripts/check-engraving-determinism.sh
