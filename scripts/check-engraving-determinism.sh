#!/usr/bin/env bash
#
# The engraving half of the determinism criterion, proven across processes.
#
# scripts/check-determinism.sh covers the pure chain — score in, feedback out.
# This covers the part that is not pure, and that S0 measured to be random by
# default: Verovio generates a fresh set of element identifiers on every
# render, and they differ even between two loads inside one process. Seeding
# the generator before loading makes them reproducible, and the adapter does
# that on every load (see `Toolkit::seed`).
#
# WHY TWO PROCESSES: a seed that was only reset per-process would pass an
# in-process comparison and fail on the first restart. The identifiers reach
# what the performer sees — MVP §6.6 highlights a note by pointing at one — so
# FR-406 and NFR-300/301 apply, and MVP §9.2 makes it release-blocking.
#
# Not part of `just ci`. ADR-0005 loads Verovio at run time precisely so the
# ordinary build needs no C++ toolchain; this needs a real library and says so
# rather than skipping quietly.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -z "${HOGWASH_VEROVIO_LIB:-}" ] || [ -z "${HOGWASH_VEROVIO_DATA:-}" ]; then
    echo "check-engraving-determinism: set HOGWASH_VEROVIO_LIB and HOGWASH_VEROVIO_DATA." >&2
    echo "  HOGWASH_VEROVIO_LIB   path to libverovio.so / .dll / .dylib" >&2
    echo "  HOGWASH_VEROVIO_DATA  path to verovio's data/ directory" >&2
    exit 1
fi

cargo build --quiet --package hogwash-engraving-verovio --example engraved-ids

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
    echo "S0 Q3: verovio's identifiers are random unless resetXmlIdSeed is called"
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

echo "ok  the same score engraved identically in two separate processes"
echo "ok  the engraving placed notes, so the comparison is meaningful"
echo
echo "Engraving determinism check passed."
