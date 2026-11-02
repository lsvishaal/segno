#!/usr/bin/env bash
#
# The licence check that cargo-deny structurally cannot perform.
#
# deny.toml polices every crate in the Cargo graph and prints "licenses ok".
# That is reassuring and incomplete: Verovio is not a Cargo dependency. It is
# a shared library opened by name at run time, so it never appears
# in the graph, and the single most licence-significant component in the
# product — LGPL-3.0 — is invisible to the gate that exists to catch exactly
# that.
#
# A gate that cannot see the thing it is trusted to check is worse than no
# gate. This is the other half.
#
# WHAT IT ASSERTS
#
#   1. Segno declares a licence, and the file backing it exists.
#   2. THIRD_PARTY.md exists and records every native component below, with
#      its licence named.
#   3. No native component has been added to the code without being recorded
#      here — found by looking for the library names the code actually opens.
#
# It cannot verify that the obligations are *discharged* at distribution time;
# shipping the notice files is a release gate, not a build one.

set -euo pipefail

cd "$(dirname "$0")/.."

# Native components, and the licence THIRD_PARTY.md must name for each.
# A component added to the code without a line here fails check 3.
NATIVE=(
    "Verovio:LGPL-3.0"
)

failed=0

# --- 1. Segno's own licence is declared and present -----------------------
declared=$(grep -m1 '^license = ' Cargo.toml | cut -d'"' -f2 || true)

if [ -z "$declared" ]; then
    echo "LICENCE VIOLATION: the workspace declares no license."
    echo "  Segno is open source, under one chosen licence."
    failed=1
elif [ ! -s LICENSE ]; then
    echo "LICENCE VIOLATION: Cargo.toml declares $declared but LICENSE is missing or empty."
    failed=1
else
    echo "ok  segno is $declared, and LICENSE carries its text"
fi

# --- 2. every native component is recorded ---------------------------------
if [ ! -s THIRD_PARTY.md ]; then
    echo "LICENCE VIOLATION: THIRD_PARTY.md is missing or empty."
    echo "  It is where components outside the Cargo graph are declared."
    failed=1
else
    for entry in "${NATIVE[@]}"; do
        component="${entry%%:*}"
        licence="${entry##*:}"

        if ! grep -q "$component" THIRD_PARTY.md; then
            echo "LICENCE VIOLATION: THIRD_PARTY.md does not mention $component."
            failed=1
        elif ! grep -q "$licence" THIRD_PARTY.md; then
            echo "LICENCE VIOLATION: THIRD_PARTY.md mentions $component but not $licence."
            echo "  Naming the component without its licence records nothing useful."
            failed=1
        else
            echo "ok  $component is recorded as $licence"
        fi
    done
fi

# --- 3. nothing opens a library that is not recorded ------------------------
#
# Every native component reaches the process through a dlopen, and every
# dlopen in this workspace goes through libloading's `Library::new`. So the
# crates that call it are exactly the crates with a native component, and
# there should be one.
#
# Both `crates/*/src` and the application shell are searched. The shell lives
# outside `crates/`, and a glob that missed it would have left the
# one crate most likely to reach for a system library unchecked.
opening=$(grep -rl 'Library::new' crates/*/src app/src-tauri/src 2>/dev/null \
    | sed -e 's|^crates/||' -e 's|^app/src-tauri/src.*|segno-app|' \
    | cut -d/ -f1 | sort -u || true)
expected="segno-engraving-verovio"

if [ "$opening" != "$expected" ]; then
    echo "LICENCE VIOLATION: the set of crates opening a native library changed."
    echo "  expected: $expected"
    echo "  found:    ${opening:-(none)}"
    echo
    echo "  A new native dependency carries licence obligations that cargo-deny"
    echo "  will not report. Record it in THIRD_PARTY.md and in NATIVE above."
    failed=1
else
    echo "ok  only $expected opens a native library"
fi

if [ "$failed" -ne 0 ]; then
    echo
    echo "Native licence obligations are not a formality: the choice of"
    echo "Apache-2.0 for Segno depends on Verovio staying a separate,"
    echo "replaceable shared library. Changing how it is linked changes what"
    echo "this project may be licensed as."
    exit 1
fi

echo
echo "Native licence check passed."
