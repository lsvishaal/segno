# MIDI import: research notes and the deferred decision

Written 2026-10-06 so none of this has to be researched again.

**Status: deferred.** MIDI import is not on the v1 path. v1 ships with the
existing message that points people at MuseScore. Revisit after v1, when
there's evidence people want it.

## What we looked at

- `webmscore-pwa` (cloned beside this repo) is **only a Svelte UI**, about 1,500
  lines with no music code. It calls `WebMscore.load('midi', bytes)` and then
  `saveXml()` / `saveMxl()`. Nothing in it can be extracted.
- The engine is **`LibreScore/webmscore`**: MuseScore's `libmscore` compiled to
  WebAssembly. npm package `webmscore` 1.2.1, licence **GPL**, last release
  January 2023. It converts locally in the browser; nothing is uploaded.
- MuseScore's MIDI importer is `importmidi_*`: about 50 files, ~650 KB of C++
  (quantising, tuplet detection, voice separation, left/right-hand split,
  beat/tempo/meter/key/clef/tie handling). It is written against MuseScore's own
  score model, and the MusicXML writer uses the same model.

## Proof that it works (run 2026-10-06, headless Firefox, webmscore 1.2.1)

- **Metro_2033_Main_Menu_Theme.mid** (a real piano MIDI):
  - the MusicXML is byte-identical to what webmscore-pwa.librescore.org produced;
  - one part, two staves, 25 bars, 4/4;
  - Hogwash's own score reader (`hogwash-cli expected`) accepts it.
- **Minuet in G → MIDI → MusicXML** (known notes, so fidelity can be judged):
  - all 408 pitches come back, in the same order;
  - bar 1 is identical, including hands and voices;
  - 58 of 64 bars are identical to the original's bars played through with its
    repeats; the other 6 differ only in how a rest is written, not in any note;
  - the MIDI contains the repeats played out (64 bars, not 32); passing
    "expand repeats = false" to `saveMidi` made no difference.
- **Cost:** about 14 MB is needed at run time (`webmscore.lib.wasm` 9.2 MB,
  `webmscore.lib.data` 4.1 MB, JS 0.26 MB); about 2.2 s to load and convert.
  The npm package is 24 MB, with extras that aren't needed.
- **Node:** the package's Node entry point fails on Node 26 (it assigns
  `globalThis.navigator`, which is now read-only). Use the browser build.
- **Caveat:** these MIDIs were clean and quantised. A MIDI recorded from a human
  performance will convert rougher; that is inherent to MIDI-to-notation.

## Options judged

| Option | Verdict |
|---|---|
| Port the algorithms to a Rust crate | Feasible with a rewrite and a diff against webmscore as the oracle (an AI can translate quickly; verifying is the hard part). Scope: about 5–6k lines of Rust for piano only, plus a small score model and a fresh MusicXML writer. **Does not remove the licence problem**, because a port of GPL code is a derivative work. Needs a new crate (approval). |
| Compile MuseScore's C++ into the Rust build | Rejected: Qt and framework weight, same GPL issue. |
| Call the user's own MuseScore as a program | No GPL code shipped; but the user must install MuseScore. |
| webmscore (browser build) in the Tauri webview | Works and is the same converter as the website. About 14 MB; GPL code in the app. Needs the `webmscore` npm dependency (approval) and a check that Tauri's webview allows the wasm. |

## The licence problem

Hogwash is **Apache-2.0**. webmscore and MuseScore are **GPL**. Bundling
webmscore, or porting its code, makes the distributed app GPL. A paraphrased
port doesn't escape this; only a clean-room rewrite would, and that has to be
done by someone who has never read the C++. Check the exact terms of both
packages before shipping; this is not legal advice.

## The decision, when you come back to it

1. **GPL-3.0 and bundle webmscore.** One decision, simplest, and as the sole
   author you can relicense your own code later. Recommended if MIDI is wanted.
2. **Stay Apache-2.0 with a fresh, independent, piano-only importer**, designed
   from MIDI and MusicXML alone, without copying MuseScore. Rougher scores,
   no GPL. This is the "free rewrite" route.
3. **Use the user's own MuseScore** if installed. Small, no bundling.

## Design that keeps all of these open

One function, `midiToMusicXml(bytes) → string`, in `app/src/midiImport/`.
- Loaded lazily (dynamic `import()`), only when a `.mid`, `.midi` or `.kar`
  file is picked.
- Its output goes into the existing `import_musicxml` path, so the damaged-file
  check, duplicate detection, filename titles, rename and delete all apply.
- `import_musicxml` today opens its own file dialog, so it needs a way to take
  bytes already produced by the frontend.
- If more than two staves or several parts come back, say so clearly and don't
  pick silently.
- Whichever option wins later, only the body of that one function changes.

## How to re-run the proof

1. `npm i webmscore@1.2.1 playwright-core@1.62.0` in a scratch folder.
2. Serve the folder (for example `python3 -m http.server`) with a page that does
   `import WebMscore from './node_modules/webmscore/webmscore.mjs'`, awaits
   `WebMscore.ready`, then `WebMscore.load('midi', bytes, [], false)` and
   `score.saveXml()`.
3. Drive it with headless Firefox (`PLAYWRIGHT_BROWSERS_PATH=~/.cache/ms-playwright`).
4. Read the result with `hogwash-cli expected <file.musicxml>`, with
   `HOGWASH_VEROVIO_LIB` and `HOGWASH_VEROVIO_DATA` set as in the `justfile`.
5. For fidelity, make a MIDI from a bundled piece with `saveMidi`, convert it
   back, and compare pitch order and bars. Use only the Minuet in G or the
   pieces in `app/src-tauri/library/`, never invented notes.
