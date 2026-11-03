#!/usr/bin/env bash
#
# The engraving half of the determinism criterion, proven across processes.
#
# scripts/check-determinism.sh covers the pure chain — score in, feedback out.
# This covers the part that is not pure, and that was measured to be random by
# default: Verovio generates a fresh set of element identifiers on every
# render, and they differ even between two loads inside one process. Seeding
# the generator before loading makes them reproducible, and the adapter does
# that on every load (see `Toolkit::seed`).
#
# WHY TWO PROCESSES: a seed that was only reset per-process would pass an
# in-process comparison and fail on the first restart. The identifiers reach
# what the performer sees — the design highlights a note by pointing at one — so
# The requirements apply, and the design makes it release-blocking.
#
# Not part of `just ci`. The design loads Verovio at run time precisely so the
# ordinary build needs no C++ toolchain; this needs a real library and says so
# rather than skipping quietly.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -z "${SEGNO_VEROVIO_LIB:-}" ] || [ -z "${SEGNO_VEROVIO_DATA:-}" ]; then
    echo "check-engraving-determinism: set SEGNO_VEROVIO_LIB and SEGNO_VEROVIO_DATA." >&2
    echo "  SEGNO_VEROVIO_LIB   path to libverovio.so / .dll / .dylib" >&2
    echo "  SEGNO_VEROVIO_DATA  path to verovio's data/ directory" >&2
    exit 1
fi

cargo build --quiet --package segno-engraving-verovio --example engraved-ids

binary="target/debug/examples/engraved-ids"
if [ ! -x "$binary" ]; then
    echo "check-engraving-determinism: $binary was not built" >&2
    exit 1
fi

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT

# Two separate process invocations. Not a loop inside one.
"$binary" > "$workdir/first"
"$binary" > "$workdir/second"

if ! cmp -s "$workdir/first" "$workdir/second"; then
    echo "DETERMINISM VIOLATION: the same score engraved twice produced different identifiers."
    echo
    diff "$workdir/first" "$workdir/second" || true
    echo
    echo "verovio's identifiers are random unless resetXmlIdSeed is called"
    echo "BEFORE loadData. Check Toolkit::seed still runs on every load."
    exit 1
fi

# A run that produced nothing at all would satisfy the comparison above while
# proving nothing. This is what notices.
if [ ! -s "$workdir/first" ] || ! grep -q "^bar " "$workdir/first"; then
    echo "HARNESS VACUOUS: the engraving produced no placements to compare."
    echo "The determinism check above therefore proves nothing."
    cat "$workdir/first"
    exit 1
fi

# The analysis of a real piece with a pickup, replayed in two processes: its
# own notes, from the beginning and as a loop from the pickup to bar 1.
cargo build --quiet --package segno-cli
cli="target/debug/segno-cli"
pickup="app/src-tauri/tests/fixtures/schlusslied-des-narren.musicxml"
"$cli" expected "$pickup" > "$workdir/pickup-notes.json" 2>/dev/null
for run in first second; do
    "$cli" replay "$pickup" "$workdir/pickup-notes.json" 2>/dev/null > "$workdir/pickup-$run"
    "$cli" replay "$pickup" "$workdir/pickup-notes.json" --loop 0-1 2>/dev/null >> "$workdir/pickup-$run"
done
if ! cmp -s "$workdir/pickup-first" "$workdir/pickup-second"; then
    echo "DETERMINISM VIOLATION: the piece with a pickup replayed twice differed."
    diff "$workdir/pickup-first" "$workdir/pickup-second" || true
    exit 1
fi
if ! grep -q "Pickup\|pickup" "$workdir/pickup-first"; then
    echo "HARNESS VACUOUS: the pickup replay never reached the pickup."
    head -20 "$workdir/pickup-first"
    exit 1
fi

echo "ok  the same score engraved identically in two separate processes"
echo "ok  a piece with a pickup replayed identically in two separate processes"
echo "ok  the engraving placed notes, so the comparison is meaningful"
echo
echo "Engraving determinism check passed."
