# Segno

Segno (SEH-nyoh) is the musical sign that sends you back to repeat a passage.

**A free, local-first desktop app for practising piano from real sheet music
with a MIDI keyboard.**

Open a piece and the cursor waits until you play the right notes. Segno keeps
every note you play, tells you after each run what went wrong, where and how to
practise it, and remembers which bars give you trouble over time.

## Status

**A working preview, not yet released.** It runs on Linux. The first tester
release (Linux and Windows) comes when its gates pass; there are no fixed dates
(see `ROADMAP.md`). There is no installer
yet. Build it from source (below), or run `just release` to make an AppImage
for your own machine.

What works today:
- **Library:**
  - six bundled pieces, on Beginner and Intermediate shelves
  - your own MusicXML / MXL imports (with clear refusals for MIDI, PDF and
    damaged files)
  - rename and delete
  - a Continue card for the piece you played last
  - "last practised" and the weakest bars on each card
- **Practice:**
  - the cursor waits for the right notes (chords, ties, repeats, grace notes)
  - start from any bar (click it)
  - loop a range of bars (Shift-click or L), with a clean-passes counter
  - restart (R)
  - zoom
  - note names
  - MIDI Thru to send what you play to a piano or synth
  - a keyboard that's unplugged gets a minute to come back
- **After each run, a Results view:**
  - accuracy, tempo and steadiness
  - up to three "work on this" cards: the bars cut out of your score, what
    happened, why, how to practise it, and a button that loops those bars
  - what went well
  - a per-bar chart
- **History:**
  - every attempt is saved locally with timings, so analysis improves without
    losing old sessions
  - a Progress page per piece: what to work on next (with a loop button),
    every session bar by bar, and how each bar has changed

## Principles

- **Local only.** No accounts, no network calls, no telemetry. Your music and
  history stay on your machine (`~/.local/share/com.segno.practice`: your
  imported scores, practice history, `settings.json`, and `segno.log` for
  warnings and errors).
- **No gamification.** No XP, streaks or badges. Feedback says where and what,
  never how good.
- **The cursor waits for you.** Pauses are judged against your own pace and the
  written rhythm, not a metronome.
- **Reproducible.** The same performance against the same score always gives the
  same analysis, checked in CI.
- **Integration over invention.** Verovio draws the notation, `midir` talks to the
  keyboard, and MusicXML is the source of truth.

More: `docs/PRODUCT.md`.

## Getting music

Segno reads **MusicXML** (`.musicxml`, or compressed `.mxl`). Export it from
[MuseScore Studio](https://musescore.org) or download public-domain scores from
[OpenScore](https://openscore.cc). MIDI files and PDFs aren't scores and can't be
imported (see `docs/archive/MIDI-IMPORT.md`).

## Build from source

You need:
- the Rust toolchain pinned in `rust-toolchain.toml`
- [`just`](https://just.systems), `cargo-deny` and Node
- the Tauri v2 Linux prerequisites (WebKitGTK 4.1 and friends)
- a build of [Verovio](https://github.com/rism-digital/verovio) as a shared
  library, in a sibling checkout

```sh
just verovio-build ../verovio   # once: builds ../verovio-build
(cd app && npm install)         # once
just run                        # the app, with live reload
just ci                         # the full gate: fmt, clippy, tests, licences, architecture, determinism, UI
just release                    # an AppImage with Verovio bundled beside it
just                            # every command
```

How it's put together:
- a Rust workspace of 13 crates in `crates/`
- pure engine crates with no I/O or clock, guarded by `scripts/check-arch.sh`
- a Tauri v2 app in `app/` (React 19, TypeScript, Tailwind v4)

Tests: `docs/TESTING.md`. Working rules for contributors and Claude Code:
`CLAUDE.md`.

## Licence

**Apache-2.0** (`LICENSE`). Verovio is **LGPL-3.0-or-later** and is never vendored
or linked into the binary: it's a separate shared library opened at run time, so
you can replace it with your own build. See `THIRD_PARTY.md`.

Where each bundled piece came from: `app/src-tauri/library/PROVENANCE.md`.
