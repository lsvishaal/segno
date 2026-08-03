# Hogwash — where the work stands

**Last updated:** 2026-08-03, end of the Tauri interface session.
**Safe to pause here:** yes. Both repositories are clean, every gate is green,
and nothing is half-finished.

This file is the handoff. It says what exists, what does not, and what a
person picking this up next needs to know. It is deliberately blunt about
gaps — a status document that reads as if everything is fine is worse than no
status document.

---

## 1. How to run it

**The single most important line in this file**, because it is not obvious and
it wasted real time:

```bash
just dev          # starts Vite, then the app
```

A Tauri **debug** build loads the frontend from the Vite dev server at
`localhost:1420`. Only a **release** build uses the bundled files. So running
`./target/debug/hogwash-app` directly shows *"Could not connect to localhost:
Connection refused"* and always will. That is not a bug; it is how Tauri
works.

Verovio must be findable. Either put `libverovio.so` and a `verovio-data/`
directory beside the executable, or set both of:

```bash
export HOGWASH_VEROVIO_LIB=../verovio-build/libverovio.so
export HOGWASH_VEROVIO_DATA=../verovio/data
```

Build the engine with `just verovio-build ../verovio` if it is missing.

### Every command

| Command | What it does |
|---|---|
| `just ci` | The gate. fmt, clippy `-D warnings`, tests, cargo-deny, arch guard, native-licence guard, determinism, frontend typecheck and tests. |
| `just dev` | Runs the application with live reload. |
| `just engraving-test` | The Verovio adapter against a real engine. Needs the env vars. |
| `just engraving-determinism` | Same score, two processes, identical output. |
| `just provided-test` | The three shipped pieces actually engrave. Needs the env vars. |
| `just verify-licence-gate` | Proves cargo-deny still refuses a copyleft crate. |
| `just ui` | Frontend typecheck + tests alone. |

---

## 2. What exists

### The Rust core — **complete**

209 tests. Every one of `004 §8`'s components is built, and each is a crate.

| Crate | Responsibility | State |
|---|---|---|
| `hogwash-domain` | Pitch, spelling, Location, PieceId. Zero dependencies, by rule. | done |
| `hogwash-notation` | Score, Measure, Note, ReferencePerformance | done |
| `hogwash-capture` | ObservedPerformance, and the `MidiSource` port | done |
| `hogwash-practice` | PracticeSession, Attempt, **the wait-for-me Cursor** | done |
| `hogwash-analysis` | Comparison, findings, the attempt summary | done |
| `hogwash-feedback` | Findings turned into English | done |
| `hogwash-engraving` | The `Engraver` port (pure; no engine) | done |
| `hogwash-engraving-verovio` | Verovio behind that port. **The only `unsafe` crate.** | done |
| `hogwash-library` | The Library and Piece. Never empty, by type. | done |
| `hogwash-acquisition` | Format sniffing, validation, explained refusals | done |
| `hogwash-midi` | `midir` behind the port | done |
| `hogwash-records` | Practice history, one JSON file per session | done |
| `hogwash-cli` | Determinism harness. Not a product surface. | done |

### The interface — **about a third**

`app/` holds a Tauri v2 application: React 19 + TypeScript + Vite 7,
Tailwind v4, Motion (installed, not yet used).

| Stage | What | State |
|---|---|---|
| A | Purge egui, restructure to `app/` + `src-tauri` | **done** |
| B | Window, startup, library screen | **done — seen working** |
| C | Score in the DOM, CSS highlighting | **done — 8 tests** |
| D | The practice loop: MIDI → cursor → live marks | **not started** |
| E | Summary, history, import screens | **not started** |
| F | Animation and visual polish | **not started** |
| G | ADR-0012, IF-6, release-gate walk-through | **not started** |

**Verified visually**: the library screen renders with all three provided
pieces, their composers read from the MusicXML, and bar/note counts computed
from the real engraved Score. The whole chain works — Verovio → Score →
Piece → Library → Tauri command → React.

---

## 3. What does NOT exist yet

Stated plainly so nothing is assumed working.

- **You cannot practise.** No MIDI is wired to the interface. Opening a piece
  shows the score with a cursor resting on the first note, and it never moves.
- **No summary, no history screen, no import button.** The Rust behind all
  three is finished and tested; none of it is reachable from the window.
- **No animation beyond the cursor pulse and card hover.** Motion is installed
  and unused.
- **The visual design is a first pass.** Correct and plain, not finished.
- **Windows and macOS are untested.** Everything so far is Linux only.
- **No release build has ever been made.** `just dev` only.

---

## 4. Decisions already made — do not relitigate these

Twelve ADRs in `docs/architecture/adr/`. The ones that constrain the work
still to come:

| ADR | Decision | Why it matters to Stages D–G |
|---|---|---|
| 0001 | Correctness compares **sounding pitch**, not written spelling | Nothing may reconstruct a spelling from a played note |
| 0002 | The Cursor **ignores** unexpected notes | A wrong note does not reset chord progress |
| 0004 | Feedback names **what was written**, never what was played | The summary screen must not name a played note |
| 0005 | Verovio, loaded at run time behind a port | Startup has to *find* it and explain if it cannot |
| 0009 | **One channel, one owner** for MIDI | Stage D's whole shape |
| 0010 | History is one JSON file per session, written by rename | Persistence is done; just call it |
| 0011 | egui — **superseded**, needs marking (Stage G) | Historical |

**ADR-0012 is not written yet.** It should supersede 0011 and record why the
interface moved to Tauri. Stage G.

---

## 5. The two rules the interface must not break

Both are easy to violate by accident and neither has an automated guard yet.

**1. The frontend judges nothing.** `004 §8.8` — Presentation "never
originates a finding of its own". Every sentence about a performance must
arrive from Rust already written, via `Feedback::lines()`. If TypeScript ever
composes a sentence about how someone played, the guards that keep feedback
non-prescriptive (FR-503) and stop it naming the played note (ADR-0001) have
been routed around and nothing will notice.

**2. Live correctness is computed in Rust, using the Cursor.** `MVP §6.6`
wants notes marked as they are played, but `003 §7.16` says the Cursor renders
no verdict — `Held` means "wrong note" *or* "chord not finished yet". So the
live signal must be derived in Rust from `Cursor::expecting()`, the same rule
`Analysis::of` uses. That is `004 §6.5`: the marks on screen and the summary
afterwards then agree **by construction** rather than by coincidence.

---

## 6. Stage D, in detail — the next thing to do

The only remaining stage with real design content. Everything after it is
wiring and taste.

**Rust** (`app/src-tauri/src/practice.rs`, new):

1. A dedicated thread owns the `Attempt` and its `Cursor`. This is ADR-0009's
   "one owner", relocated: a webview has no frame loop, so a thread takes that
   role. Do **not** advance the cursor from a Tauri command handler — commands
   run on a pool, and that would be several owners.
2. It drains `Keyboard::played()`, records each pitch into the
   `ObservedPerformance`, advances the `Cursor`, and emits a Tauri event.
3. The event carries: the new cursor position, which pitches of the current
   position are already down (`Cursor::has_been_played`), and whether the note
   just played matched — computed from `Cursor::expecting()`, per §5 rule 2.
4. Commands needed: `instruments()`, `start_attempt(pieceId, instrument)`,
   `end_attempt() -> summary`.
5. On ending: `Analysis::of` → `Feedback::of` → `Records::keep`. All three
   crates are finished; this is a call sequence, not new logic.

**Frontend** (`app/src/screens/Practice.tsx`):

6. `listen()` for the event, keep a `Marked` map in state, hand it to the
   existing `<Score>`. The component already handles everything else.
7. An instrument picker. `NoInstrument::describe()` already writes the words
   for every failure case, including a keyboard that disconnects mid-attempt.

**Watch for:** `Attempt::end` consumes the Attempt (typestate — `003 §7.3`
says an ended Attempt does not change). The thread must own it in a way that
allows moving it out, so an `Option<Attempt>` or a channel-driven state
machine rather than a borrow.

---

## 7. Open questions — genuinely unresolved

Not oversights. Each is recorded and waiting.

1. **`002` Amendment A is Proposed, not accepted.** Twelve requirements
   covering the gaps `MVP §11` left (score display, live feedback, MIDI
   capture, the provided library, import guidance). Drafted in
   `docs/002_Amendment_A_Capability_Gaps.md` as a separate file, because
   `002 §9` makes adding a requirement a deliberate act. **Accepting it is
   your call and nobody else's.** Not blocking; the code is built to it.
2. **NFR-202's 100 ms** is the one number in that amendment that is a
   judgement rather than a restatement. It says so and invites argument.
3. **`MVP §10`: does the wrong-place problem fully disappear?** The docs say
   it is best answered by real playing — which needs Stage D.
4. **Windows build friction** for Verovio is untested (`S0`, ADR-0005).
5. **The provided library is three short pieces.** Enough for a first run;
   `MVP §10` asks how much repertoire is enough and does not answer it.

Findings handed back to the specification live in
`docs/Implementation_Findings.md` — IF-1 through IF-5, all open, none
blocking. **IF-6 is not written yet** (the usvg finding, Stage G).

---

## 8. Bugs found and fixed — worth not re-introducing

Both were silent. Both now have tests that were checked to fail before being
trusted.

- **usvg reports `<use>` geometry wrongly.** For Verovio's
  `<use transform="translate(6758,1440) scale(0.72,0.72)"/>`, it reported the
  glyph at `t + s·t` — x≈1334 on a page whose music ends at x≈915. Every
  notehead is drawn that way, so every highlight would have landed off the
  music. Rendering was correct; only the reported box was wrong. This is what
  killed the egui approach, and it is *absent* in a webview. Record as IF-6.
- **Verovio searches only the first 600 characters for a root element.**
  *Ode to Joy* had a long provenance comment, so its `<score-partwise>` sat at
  offset 751 and was rejected as unknown XML. `Hogwash::start` skips a piece
  that will not engrave — deliberately — so the shelf looked fine with a third
  missing. Guarded by `app/src-tauri/tests/provided.rs`.

---

## 9. Which model to use from here

An honest read rather than a flattering one.

**Sonnet 5 is a good fit for Stages E, F and G.** They are wiring against
finished, tested Rust and an established pattern: a command in
`commands.rs`, a typed wrapper in `bridge/`, a screen in `screens/`. The
decisions are made and written down. Stage F is taste and iteration, where
fast turnaround matters more than deep reasoning.

**Stage D is the one place where the extra reasoning earns its keep.** It has
concurrency (a thread owning a typestate value that must be *moved* out to
end it), and the two rules in §5 that are easy to violate invisibly. Sonnet 5
can do it given §6 above as the brief — that section exists precisely so it
can — but if you are rationing Opus, spend it here rather than on the
screens.

**What Opus was actually useful for in this session**, so you can judge the
pattern: noticing from measurements that a geometry library was lying rather
than assuming the code was wrong; catching the self-referential lifetime
problem by reading type signatures *before* writing a screen against them;
and re-deriving the licence exemptions instead of carrying stale ones
forward. Diagnosis and architecture — not code volume.

**Suggested split:** Stage D on Opus if available, E–G on Sonnet. If you would
rather not switch mid-stream, Sonnet 5 throughout is a reasonable call — the
plan and this document carry most of what was hard-won.

---

## 10. Good places to stop

- **Now.** Everything is committed, `just ci` is green, and the application
  demonstrably runs. Nothing is mid-thought.
- **After Stage D**, which is the next natural boundary: at that point you can
  actually practise a piece end to end, and the product exists rather than
  merely compiling.

## 11. Repository state

Two independent git repositories, and **neither has been pushed**. The docs
remote is yours to push.

```
hogwash/          code       HEAD = "Stage C: the score in the DOM, and it renders"
hogwash/docs/     specs      HEAD = "ADR-0011: egui, and a score…"  (gitignored by the code repo)
```

Verovio lives at `../verovio` (a reference clone, never vendored, never
committed) and is built into `../verovio-build`.

The live plan is at `~/.claude/plans/analyze-indepth-the-serialized-wilkinson.md`
and still describes Stages D–G accurately.
