#!/usr/bin/env bash
#
# The architecture, enforced by the build.
#
# 004_System_Architecture.md §14 makes component boundaries a governance rule
# rather than a suggestion, and §6.3 requires dependencies to point inward,
# toward domain concepts and never outward toward presentation or storage.
# A rule nothing checks is a rule that erodes, so this script checks it.
#
# Two things are asserted:
#
#   1. hogwash-domain has NO dependencies at all — not on another hogwash
#      crate, not on anything from crates.io. It is shared vocabulary
#      (003 §7) and nothing else.
#
#   2. The pure crates — notation, capture, practice, analysis, feedback,
#      engraving, library and acquisition — depend on no I/O, filesystem,
#      MIDI, GUI, rendering or async crate.
#      These are the components 004 §12 relies on for correctness and
#      determinism, and they must stay testable with no device and no window
#      attached.
#
#      hogwash-records is NOT on that list, and hogwash-midi and
#      hogwash-engraving-verovio are not either. They are adapters: a
#      filesystem, a device, a C library. That is what an adapter is for.
#
#      hogwash-capture is on that list despite owning a concept fed by live
#      MIDI (004 §8.5). The concept is not the hardware: Observed Performance
#      is a record of sounding pitches, and the instrument belongs behind a
#      port in a separate adapter crate. Without that split, every test of
#      practice, analysis and feedback would need a keyboard attached.
#
# Crates that do not exist yet are skipped, so this is useful from the first
# commit rather than only once the workspace is complete.

set -euo pipefail

cd "$(dirname "$0")/.."

# Crates that must remain free of the world outside the domain.
PURE_CRATES=(
    hogwash-domain
    hogwash-notation
    hogwash-capture
    hogwash-practice
    hogwash-analysis
    hogwash-feedback
    hogwash-engraving
    hogwash-library
    hogwash-acquisition
)

# Anything that reaches a device, a disk, a screen or a scheduler.
FORBIDDEN=(
    tokio async-std smol futures
    midir alsa coremidi windows winapi
    wgpu winit egui iced slint dioxus tauri wry softbuffer
    resvg usvg tiny-skia
    serde_json toml ron rusqlite sled
    reqwest hyper ureq
)

failed=0

crate_exists() {
    cargo metadata --no-deps --format-version 1 2>/dev/null \
        | grep -q "\"name\":\"$1\""
}

deps_of() {
    # `--edges normal` excludes dev- and build-dependencies: a test-only or
    # build-time crate is not part of the shipped architecture.
    cargo tree --package "$1" --edges normal --prefix none 2>/dev/null \
        | tail -n +2 | awk '{print $1}' | sort -u
}

# --- 1. hogwash-domain depends on nothing -----------------------------------
if crate_exists hogwash-domain; then
    stray=$(deps_of hogwash-domain | grep -v '^$' || true)
    if [ -n "$stray" ]; then
        echo "ARCH VIOLATION: hogwash-domain must have zero dependencies."
        echo "  found:"
        echo "$stray" | sed 's/^/    /'
        failed=1
    else
        echo "ok  hogwash-domain has no dependencies"
    fi
else
    echo "--  hogwash-domain does not exist yet, skipped"
fi

# --- 2. pure crates stay pure -----------------------------------------------
for crate in "${PURE_CRATES[@]}"; do
    if ! crate_exists "$crate"; then
        echo "--  $crate does not exist yet, skipped"
        continue
    fi

    tree=$(deps_of "$crate")
    violations=""
    for dep in "${FORBIDDEN[@]}"; do
        if echo "$tree" | grep -qx "$dep"; then
            violations+="    $dep"$'\n'
        fi
    done

    if [ -n "$violations" ]; then
        echo "ARCH VIOLATION: $crate must stay free of I/O, UI and async."
        printf '%s' "$violations"
        failed=1
    else
        echo "ok  $crate is pure"
    fi
done

# --- 3. nothing in a pure crate is ordered by a hash -------------------------
#
# FR-406, NFR-300 and NFR-301 require identical results across separate
# executions, and MVP §9.2 makes that release-blocking. Rust's HashMap and
# HashSet seed their hasher randomly *per process*, so iterating one yields a
# different order on every run. That is the classic way a comparison which
# looks deterministic silently is not, and it fails only sometimes, which is
# worse than failing always.
#
# BTreeMap and BTreeSet order by the key and are safe. So is a Vec.
for crate in "${PURE_CRATES[@]}"; do
    src="crates/$crate/src"
    [ -d "$src" ] || continue

    # Comment lines are excluded: naming the hazard in a doc comment is how
    # it stays understood, and must not itself trip the guard.
    hashed=$(grep -rn 'HashMap\|HashSet' "$src" | grep -vE ':[0-9]+:[[:space:]]*//' || true)
    if [ -n "$hashed" ]; then
        echo "DETERMINISM VIOLATION: $crate uses a hash-ordered container."
        echo "$hashed" | sed 's/^/    /'
        echo "    Use BTreeMap/BTreeSet: hash iteration order is randomised"
        echo "    per process, which breaks FR-406 and NFR-301."
        failed=1
    fi
done

# --- 4. exactly one crate may step outside the workspace lints -------------
#
# The workspace forbids `unsafe_code`, and `forbid` cannot be lifted by an
# `allow` inside the crate — that is what distinguishes it from `deny`. So a
# crate that genuinely needs `unsafe` can only get it by declining the
# workspace lint table and declaring its own.
#
# Exactly one crate does: hogwash-engraving-verovio, which calls Verovio's C
# functions (ADR-0005). That exception is reviewed, documented at every call
# site, and quarantined behind a safe API. What must not happen is a second
# one appearing quietly, because "we already have an unsafe crate" is how a
# forbidden thing becomes an ordinary one.
#
# The check is on Cargo.toml rather than on the source: a crate cannot write
# `unsafe` at all without first opting out here, so this is the doorway.
EXPECTED_EXCEPTION="hogwash-engraving-verovio"

exceptions=()
for manifest in crates/*/Cargo.toml; do
    crate=$(basename "$(dirname "$manifest")")

    # `[lints]` followed by `workspace = true` is the inheriting form.
    # Anything else — including no [lints] section at all — is opting out.
    inherits=$(awk '
        /^\[lints\]$/            { in_lints = 1; next }
        /^\[/                    { in_lints = 0 }
        in_lints && /^workspace[[:space:]]*=[[:space:]]*true$/ { print "yes"; exit }
    ' "$manifest")

    if [ "$inherits" != "yes" ]; then
        exceptions+=("$crate")
    fi
done

if [ "${#exceptions[@]}" -eq 1 ] && [ "${exceptions[0]}" = "$EXPECTED_EXCEPTION" ]; then
    echo "ok  $EXPECTED_EXCEPTION is the only crate outside the workspace lints"
else
    echo "ARCH VIOLATION: the set of crates outside the workspace lints changed."
    echo "  expected exactly: $EXPECTED_EXCEPTION"
    echo "  found:"
    printf '    %s\n' "${exceptions[@]:-(none)}"
    echo
    echo "  Declining '[lints] workspace = true' is how a crate escapes"
    echo "  unsafe_code = \"forbid\". ADR-0005 permits one such crate, for the"
    echo "  Verovio C boundary. A second needs its own ADR, not a copy of that"
    echo "  Cargo.toml."
    failed=1
fi

if [ "$failed" -ne 0 ]; then
    echo
    echo "The dependency direction in 004 §6.3 has been violated."
    echo "This is a governance rule (004 §14), not a style preference:"
    echo "fix the dependency, or amend the architecture document first."
    exit 1
fi

echo
echo "Architecture guard passed."
