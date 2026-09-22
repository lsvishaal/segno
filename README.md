# Hogwash

**A local-first desktop app for practising piano from standard sheet music
with a MIDI keyboard.**

Open a score, play it on your keyboard, and the cursor waits for the right
notes. Correct and wrong notes are marked on the staff as you play, and each
session is kept so you can see where you struggled.

## Status

Working, not finished. You can pick a piece from the library or import your
own MusicXML, choose a MIDI input (and optionally route it to a synth), and
practise it with live marks and a summary at the end. Practice history is
recorded but there is no screen to browse it yet, and there is no packaged
release build: the app runs via `just run`. `ROADMAP.md` tracks what's next.

## Principles

- **Local only.** No accounts, no telemetry, no network calls. Your music and
  your history stay on your machine.
- **No gamification.** No scores, combos, streaks or difficulty badges.
  Feedback is factual and specific: which bar, which note.
- **The cursor waits for you.** It does not advance until you play what is
  written. The tempo is your speed, so timing is not assessed.
- **Reproducible results.** The same performance against the same score
  always gives byte-identical analysis, checked in CI across separate
  processes.
- **Integration over invention.** Verovio engraves the notation, `midir`
  talks to MIDI hardware, and MusicXML is the source of truth.

Not a falling-notes game, and not a reader of MIDI files as scores: MIDI is
only used for live input from your instrument.

## How it's put together

One crate per component. The engine crates are pure: no I/O, no MIDI, no GUI,
no async, no clock. `scripts/check-arch.sh` fails the build if one of them
picks up a dependency on the outside world.

| Crate | Purpose |
|---|---|
| `hogwash-domain` | Shared vocabulary (pitch, measure, location). No dependencies. |
| `hogwash-notation` | Written music: Score, Measure, Note, and the Reference Performance the cursor follows. |
| `hogwash-capture` | What the performer actually played. |
| `hogwash-practice` | Practice sessions, attempts, and the cursor that waits. |
| `hogwash-analysis` | Compares what was played against what was written. |
| `hogwash-feedback` | Turns an analysis into plain, located statements. |
| `hogwash-engraving` | The interface an engraver must provide. |
| `hogwash-engraving-verovio` | The Verovio engraver. The only crate that calls C (and the only one allowed `unsafe`). |
| `hogwash-library` | The pieces available to practise. Never empty. |
| `hogwash-acquisition` | Turns an offered file into a playable piece, or explains why not. |
| `hogwash-midi` | The MIDI keyboard, plus MIDI Thru to a synth or hardware output. |
| `hogwash-records` | Practice history on disk, one file per session, written atomically. |
| `hogwash-cli` | Determinism harness, and `replay`: a MusicXML file plus a JSON file of played notes, run through the real practice path without a keyboard (`just replay <score> <events>`). |
| `hogwash-app` (`app/src-tauri`) | The Tauri shell: commands over the Rust core. |

The interface (`app/`) is React 19, TypeScript, Vite, Tailwind v4 and Motion
on Tauri v2. The score is real SVG in the DOM.

## Getting started

```sh
just        # list every command
just run    # launch the app with live reload
just ci     # the full gate: fmt, clippy, tests, licences, architecture, determinism, frontend
```

Verovio is loaded at run time from a sibling checkout (`../verovio`, built
into `../verovio-build`). If it isn't built yet:

```sh
just verovio-build ../verovio
```

You need the Rust toolchain pinned in `rust-toolchain.toml`, `cargo-deny`, and
Node (run `npm install` in `app/` once).

## Licence

**Apache-2.0** (`LICENSE`). Verovio is **LGPL-3.0-or-later** and is never
vendored or linked into the binary: it is a separate shared library opened at
run time, so you can replace it with your own build. See `THIRD_PARTY.md`.

The bundled pieces and their provenance are listed in
`app/src-tauri/library/PROVENANCE.md`.
