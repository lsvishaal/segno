#!/usr/bin/env bash
#
# The architecture, enforced by the build.
#
# the design makes component boundaries a governance rule
# rather than a suggestion, and the design requires dependencies to point inward,
# toward domain concepts and never outward toward presentation or storage.
# A rule nothing checks is a rule that erodes, so this script checks it.
#
# Two things are asserted:
#
#   1. segno-domain has NO dependencies at all — not on another hogwash
#      crate, not on anything from crates.io. It is shared vocabulary
# and nothing else.
#
#   2. The pure crates — notation, capture, practice, analysis, feedback,
#      engraving, library and acquisition — depend on no I/O, filesystem,
#      MIDI, GUI, rendering or async crate.
#      These are the components the design relies on for correctness and
#      determinism, and they must stay testable with no device and no window
#      attached.
#
#      segno-records is NOT on that list, and segno-midi and
#      segno-engraving-verovio are not either. They are adapters: a
#      filesystem, a device, a C library. That is what an adapter is for.
#
#      segno-capture is on that list despite owning a concept fed by live
#      MIDI. The concept is not the hardware: Observed Performance
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
    segno-domain
    segno-notation
    segno-capture
    segno-practice
    segno-analysis
    segno-feedback
    segno-engraving
    segno-library
    segno-acquisition
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

# --- 1. segno-domain depends on nothing -----------------------------------
if crate_exists segno-domain; then
    stray=$(deps_of segno-domain | grep -v '^$' || true)
    if [ -n "$stray" ]; then
        echo "ARCH VIOLATION: segno-domain must have zero dependencies."
        echo "  found:"
        echo "$stray" | sed 's/^/    /'
        failed=1
    else
        echo "ok  segno-domain has no dependencies"
    fi
else
    echo "--  segno-domain does not exist yet, skipped"
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
# The requirement requires identical results across separate
# executions, and the design makes that release-blocking. Rust's HashMap and
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
        echo "    per process, which breaks identical results across runs."
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
# Exactly one crate does: segno-engraving-verovio, which calls Verovio's C
# functions. That exception is reviewed, documented at every call
# site, and quarantined behind a safe API. What must not happen is a second
# one appearing quietly, because "we already have an unsafe crate" is how a
# forbidden thing becomes an ordinary one.
#
# The check is on Cargo.toml rather than on the source: a crate cannot write
# `unsafe` at all without first opting out here, so this is the doorway.
EXPECTED_EXCEPTION="segno-engraving-verovio"

# `app/src-tauri` is included explicitly. It is a workspace member like any
# other, and it is where an `unsafe` block would be least surprising and most
# dangerous — the application shell is the one crate touching a window system.
# Globbing only `crates/*` would have quietly stopped covering it the day the
# interface moved out of that directory.
exceptions=()
for manifest in crates/*/Cargo.toml app/src-tauri/Cargo.toml; do
    [ -f "$manifest" ] || continue

    # From the manifest, not the directory: `app/src-tauri` holds a crate
    # called `segno-app`, and a violation message naming the folder would
    # send someone looking for a crate that does not exist.
    crate=$(awk -F'"' '/^name *= *"/ { print $2; exit }' "$manifest")

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
    echo "  unsafe_code = \"forbid\". The design permits one such crate, for the"
    echo "  Verovio C boundary. A second needs its own ADR, not a copy of that"
    echo "  Cargo.toml."
    failed=1
fi

if [ "$failed" -ne 0 ]; then
    echo
    echo "The dependency direction (pure crates point inward) has been violated."
    echo "This is a governance rule, not a style preference:"
    echo "fix the dependency, or amend the architecture document first."
    exit 1
fi

echo
echo "Architecture guard passed."
