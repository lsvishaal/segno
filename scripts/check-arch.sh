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
#   2. The pure crates — notation, capture, practice, analysis, feedback —
#      depend on no I/O, filesystem, MIDI, GUI, rendering or async crate.
#      These are the components 004 §12 relies on for correctness and
#      determinism, and they must stay testable with no device and no window
#      attached.
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
    hogwash-notation
    hogwash-capture
    hogwash-practice
    hogwash-analysis
    hogwash-feedback
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

if [ "$failed" -ne 0 ]; then
    echo
    echo "The dependency direction in 004 §6.3 has been violated."
    echo "This is a governance rule (004 §14), not a style preference:"
    echo "fix the dependency, or amend the architecture document first."
    exit 1
fi

echo
echo "Architecture guard passed."
