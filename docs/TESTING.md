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

### 22.1a Test levels as built (25 Oct)

| Command | What it runs | Needs |
|---|---|---|
| `just ci` | fmt, clippy, all Rust unit tests, cargo-deny, licence gates, architecture guard, determinism, frontend lint and UI tests (vitest) | nothing extra. `cargo-deny` writes to `~/.cargo`, so run it outside a sandbox |
| `just engraving-test` | ignored tests in `hogwash-engraving-verovio` and `hogwash-analysis` (coach on real scores) | a built Verovio |
| `just provided-test` | ignored `hogwash-app` tests: the bundled pieces through the real engine, the practice thread, history | a built Verovio |
| `just e2e [scenario …]` | builds the AppImage, then drives the real app through `scripts/e2e/scenarios/*.mjs` | `tauri-driver`, ALSA |
| `just midi-test` | ignored `hogwash-midi` tests: notes stamped in order, an unplug noticed, Thru forwarding to an output | ALSA (Linux) |
| `just check-all` | every row above, `e2e` last | all of the above |

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
- Screenshots go to `target/e2e/shots/`.
- `just expected` writes the Minuet's notes in cursor order for scenarios to play.

### 22.2 Manual checklist per behaviour change

Claude Code writes it; you run it with the FP-10 before ticking the box.
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
