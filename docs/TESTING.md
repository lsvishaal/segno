# Hogwash — testing

How Hogwash is tested.

---

## 22. Testing and quality

### 22.1 Automated (every task)

- `just ci` green: fmt, clippy `-D warnings`, tests, cargo-deny, native
  licences, architecture guard, determinism (two processes, byte-diff),
  frontend tests.
- Every engine feature (seek, loop, hands, per-bar stats) gets:
  - unit tests in its crate;
  - at least one `just replay` fixture under `crates/hogwash-cli/examples/`
    named `<piece>.<behaviour>.json` (e.g. `minuet-in-g.loop-9-12.json`).
- Old record formats keep loading: a test with a pre-M1.1 session file.

### 22.1a Test tiers (26 Oct)

**Where a new test goes:** at the cheapest tier that fails when the behaviour
breaks. e2e only for what nothing cheaper can see: real MIDI, real engraving and
layout, real files written by the running app, and timing.

| Tier | What belongs there | Where |
|---|---|---|
| Rust unit | Domain logic, analysis, words, formats: pure, no engine, no I/O | `#[cfg(test)]` in the crate, or its `tests/` |
| Rust real engine | Anything that needs real Verovio: the bundled pieces, importing, the practice thread and history on real scores | `#[ignore = "… just provided-test"]` in `hogwash-app`; `… just engraving-test` in the engraving and analysis crates |
| Rust ALSA | Real MIDI ports: timestamps, unplugging, Thru | `#[ignore]` in `hogwash-midi/tests/` |
| UI | What a screen shows and asks for, given bridge answers (mocked) | `*.test.ts(x)` next to the component (vitest, jsdom) |
| e2e | The real AppImage with a virtual keyboard: wiring across Rust, the webview and the disk; layout; timing | `scripts/e2e/scenarios/*.mjs` |

**Recipes:**

| Command | What it runs | Needs |
|---|---|---|
| `just ci` | fmt, clippy, all Rust unit tests, cargo-deny, licence gates, architecture guard, determinism, frontend lint and UI tests (vitest) | nothing extra. `cargo-deny` writes to `~/.cargo`, so run it outside a sandbox |
| `just engraving-test` | ignored tests in `hogwash-engraving-verovio` and `hogwash-analysis` (coach on real scores) | a built Verovio |
| `just provided-test` | ignored `hogwash-app` tests: the bundled pieces through the real engine, the practice thread, history | a built Verovio |
| `just e2e [--rebuild] [scenario …]` | builds the AppImage (or reuses it, saying `reusing AppImage <hash>`, when nothing it's built from changed; `--rebuild` forces it), then drives the real app through `scripts/e2e/scenarios/*.mjs` | `tauri-driver`, ALSA |
| `just midi-test` | ignored `hogwash-midi` tests: notes stamped in order, an unplug noticed, Thru forwarding to an output | ALSA (Linux) |
| `just shots <scenario …>` | only those scenarios, into a fresh `target/e2e/shots/`, and lists the PNGs (for checking the screens by eye) | as `e2e` |
| `just check-all` | every row above, `e2e` last | all of the above |

**Time budgets** (this machine; measured 26 Oct):

| Recipe | Budget | Measured |
|---|---|---|
| `just ci` | ≤ 30 s | 11–16 s |
| `just provided-test` | — | 6 s |
| `just engraving-test` | — | 1 s |
| `just midi-test` | — | 2 s |
| `just e2e` | ≤ 180 s | 162 s reusing the AppImage (31 Oct, with `progress`: three launches); 158 s after a front-end-only rebuild; up to 290 s when Rust changed (the release build is ~170 s of it) |

**e2e conventions:**
- Each scenario starts from a clean app and data folder; none depends on another.
- History is made by playing, never by writing session files: a scenario that needs
  several sessions plays them and calls `relaunch()` (one launch is one session),
  as `progress.mjs` does for the Progress page.
- Wait for a condition, not a guessed time: `w.until(...)`, `w.settle()` (the page
  has stopped changing), `eventually(...)` (a file the app writes). A fixed sleep
  only where the time *is* the test (press gaps, flash spacing, how long marks stay).
- Measure layout with `barBox`, which waits for the bar to stop moving.

**Real pieces only.** Tests use the Minuet in G, the Prelude in C (both in
`app/src-tauri/library/`), the fixtures in `app/src/components/__fixtures__`
(made by `scripts/score-fixtures.py`), or pieces you supplied. Never invented
notes. Your own scores aren't redistributable: they live in the ignored
`test-scores/` (or `HOGWASH_TEST_SCORES`), and the tests that read them are
skipped, with a note, when it's absent.

**The e2e kit** (`scripts/e2e/`):
- `run.mjs` starts a virtual MIDI keyboard (`hogwash-midi` example `vkbd`) and
  `tauri-driver`.
- Each scenario runs with its own data folder, `target/e2e/data` via `XDG_DATA_HOME`.
  The kit waits for the previous scenario's app to exit before clearing it.
  The real folder is never written; scenarios marked `mine` start from a copy of it.
- Screenshots go to `target/e2e/shots/`. Claude Code opens them and checks the
  screens against the change's checklist (`just shots progress marks` for the
  Progress page and the live marks).
- `just expected` writes the Minuet's notes in cursor order for scenarios to play.

### 22.2 Manual checklist per behaviour change

Claude Code writes it; you run it with the FP-10 before ticking the box.
It covers only how it feels on the piano (timing, loops, marks keeping up): Claude
Code checks the visuals itself from real-app screenshots (`just shots`).
Format: *open X → play Y → expect Z.* Store the latest ones at the bottom of
`annoyances.md` or in the commit message.

### 22.3 Regression pieces

Before each release, play through these by hand:
1. Lightly Row (single line)
2. A Little Chord Study (chords)
3. Minuet in G (repeats, two hands) — with a loop across the repeat
4. Prelude in C (68 ties) — start from bar 20
5. One imported piece with a pickup bar

### 22.4 The real test

Seven days of self-use (ROADMAP.md, self-use week), then one outside pianist (M4.9), then three
before v1.0. Their hesitations are the backlog.
