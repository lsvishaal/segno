#!/usr/bin/env bash
#
# Determinism, proven across processes.
#
# FR-406, NFR-300 and NFR-301 require identical results for identical inputs,
# and MVP §9.2 lists it as a release-readiness criterion. It is checked here
# rather than asserted in a document.
#
# WHY TWO PROCESSES, and not an assertion inside one test:
# the failures this guards against are invisible within a single run.
# Rust seeds HashMap's hasher randomly per process, so iterating one yields a
# different order on each execution while looking perfectly stable inside any
# one of them. The same goes for anything derived from a pointer address or an
# allocation order. Comparing two runs of one process would pass while the
# product silently violated its own release criterion.
#
# scripts/check-arch.sh bans hash-ordered containers from the pure crates.
# This is the other half: that guard is static, this one is empirical.

set -euo pipefail

cd "$(dirname "$0")/.."

# Fixed inputs. This performance is deliberately messy — a wrong note, a
# recovery, a chord, and a note played after the piece ends — so that every
# Finding variant appears in the output being compared. A clean run would
# exercise only one of the four.
PERFORMANCE=(60 62 64 66 65 67 69 71 60 64 67 62)

# A different performance, to prove the comparison is not vacuously true.
# If the harness ever printed nothing, or a constant, the check above would
# still pass — this is what notices.
OTHER=(60 62)

cargo build --quiet --package hogwash-cli

binary="target/debug/hogwash-cli"
if [ ! -x "$binary" ]; then
    echo "check-determinism: $binary was not built" >&2
    exit 1
fi

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT

# Two separate process invocations. Not a loop inside one.
"$binary" "${PERFORMANCE[@]}" > "$workdir/first"
"$binary" "${PERFORMANCE[@]}" > "$workdir/second"
"$binary" "${OTHER[@]}"       > "$workdir/other"

if ! cmp -s "$workdir/first" "$workdir/second"; then
    echo "DETERMINISM VIOLATION: the same performance analysed twice differed."
    echo
    diff "$workdir/first" "$workdir/second" || true
    echo
    echo "FR-406 and NFR-301 require identical results across executions."
    echo "MVP §9.2 makes this release-blocking."
    exit 1
fi

if cmp -s "$workdir/first" "$workdir/other"; then
    echo "HARNESS VACUOUS: two different performances produced identical output."
    echo "The determinism check above therefore proves nothing. Fix the harness."
    exit 1
fi

echo "ok  identical inputs produced byte-identical output in separate processes"
echo "ok  different inputs produced different output, so the check is meaningful"
echo
echo "Determinism check passed."
