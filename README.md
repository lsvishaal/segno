# Hogwash

**A desktop piano practice companion, built around standard sheet music.**

You open a score, play it on a MIDI keyboard, and see immediately whether
each note is right — the cursor waiting for you, correctness marked live on
the staff, and the session recorded when you're done.

---

## Status

Working, not finished. The Rust core is complete: 14 crates, 215 tests,
every component `004_System_Architecture.md §8` calls for is built and
`just ci`-green. The Tauri interface has a real practice loop — pick a piece
from the shelf, choose an instrument, play, and watch the cursor advance only
when you do, with wrong notes and correct notes both marked as they happen.

MusicXML import is live (Library screen, `import_musicxml`), and a MIDI
Thru/output route now sits beside the keyboard input path. A visual rehaul
is underway — single accent color, a real settings panel with a proper
device dropdown, more breathing room, a slower "breathing" cursor, larger
default notation — but it hasn't been checked side-by-side against its own
design reference yet.

What's still ahead: a history screen (the data is already recorded; there's
no screen to browse it yet), finishing the visual rehaul, an ADR recording
the move to Tauri (superseding `0011-egui...`), and a real release build —
today the application only runs via `just run`. `STATUS.md` is a dated
handoff snapshot (predates the above); `ROADMAP.md` tracks current status.

The specification is written and frozen first; the code follows it. The
authoritative documents live in `docs/`, a separate repository:

| Document | Owns |
|---|---|
| `001_system_charter.md` | Purpose, principles, scope, governance |
| `002_Requirements_Specification.md` | Observable behaviour (FR / NFR / C) |
| `002_Amendment_A_Capability_Gaps.md` | Capability gaps the MVP left open, proposed |
| `003_Domain_Model.md` | The vocabulary and what it means |
| `004_System_Architecture.md` | Components, boundaries, state ownership |
| `architecture/adr/` | Decisions, one file each, never edited after acceptance |
| `Implementation_Findings.md` | What building it taught us, fed back to the spec |
| `Backlog_From_Reference.md` | Ideas seen elsewhere and deliberately not built, with why |

Implementation may not invent a domain concept (`004 §3`) or a component
boundary (`004 §14`). Where code and documents disagree, the documents win.

## Philosophy

A few decisions run through the whole project, not just one corner of it —
worth stating plainly, because they explain choices that would otherwise
look like missing features.

**Local, and only local.** No accounts, no telemetry, no network calls.
Everything — your music, your history, your progress — stays on your
machine, by rule rather than by default (`001 §7`).

**Feedback describes; it never grades.** There are no difficulty ratings, no
scores, no gamification (`001 §11`). `hogwash-feedback` is documented as
"descriptive only — never advisory," and a correctness judgment always names
what was *written*, never what was *played* (`ADR-0004`) — so it can tell you
a G was expected without ever implying your B was a *bad* note, only a
different one. The line between describing and grading is the one this
project is most careful not to cross.

**The cursor waits for you.** It marks the note expected next and does not
advance until you play it. That is not merely the simplest practice mode —
it removes a whole class of failure. A tool that keeps advancing after it
has lost your place produces confident, plausible nonsense on exactly the
passages you cannot yet play, which is precisely where you needed the
truth. Waiting makes that structurally impossible rather than something to
defend against. It has a real cost: if the system waits for you, the tempo
*is* your speed, so timing cannot be assessed in Version 1. Wait-for-me and
timing assessment are mutually exclusive — a derivation, not a preference
(`MVP §5`, `003 §7.4`).

**A judgment is reproducible, or it isn't trusted.** The same performance
against the same score must produce byte-identical output, checked by CI
running the same input twice in *separate processes* — the failures that
matters (hash-iteration order, anything address-derived) are invisible
inside a single run. A practice tool that can't reproduce its own verdict
isn't one you can rely on.

**One owner per resource that can't be shared safely.** A MIDI connection and
the Verovio engraving engine each get exactly one thread and a channel to
reach them (`ADR-0009`) — nothing else is allowed to touch them, so a
Tauri command arriving on its own thread pool can never race a live
practice session.

**The interface originates no judgment of its own.** Every sentence about a
performance is composed in Rust and handed over verbatim (`004 §8.8`); the
live marks you see while playing are computed from the exact same rule the
after-the-fact analysis uses (`Cursor::expecting()`), so the two agree by
construction, not by coincidence.

**Integration over invention.** Verovio engraves the notation, `midir`
talks to MIDI hardware, a real MusicXML file is the source of truth. None of
that is reimplemented — the effort goes into the practice experience itself,
not into re-solving problems mature open-source tools already solved.

## What it is, and isn't

Not a falling-notes game. Standard notation is the centre of the product,
and falling-notes presentation is a permanent exclusion (`MVP §7`), not a
deferral.

Not a reader of MIDI files as scores. A MIDI file records a performance, not
written music — no clefs, no key signature, no spelling, no articulation as
written. A score reconstructed from one is a guess you would have to correct
by hand (`001 §11`). MIDI is used for live input from your instrument, which
is a completely different role.

Not a grading tool. There is no score, no difficulty badge, no streak to
protect — only a description of what happened, so practicing something hard
never feels like failing a test.

Not a cloud service. No accounts, no telemetry, no network calls. Everything
is local (`001 §7`).

## How it's put together

One crate per architectural component. The engine crates — `notation`,
`capture`, `practice`, `analysis`, `feedback`, `engraving`, `library`,
`acquisition` — are pure: no I/O, no MIDI, no GUI, no async, no wall-clock
time anywhere. `hogwash-domain` has no dependencies at all. That's enforced
rather than intended: `scripts/check-arch.sh` fails the build the moment any
of them acquires a dependency on the outside world. Everything the world
actually touches — MIDI hardware, the Verovio C library, the filesystem, the
window — lives in one of four adapter crates instead.

| Crate | Purpose |
|---|---|
| `hogwash-domain` | Shared domain vocabulary. No dependencies, by rule. |
| `hogwash-notation` | Written music: Score, Measure, Note, and the Reference Performance derived from a Score. |
| `hogwash-capture` | What the performer actually played. The MIDI adapter that feeds it lives elsewhere. |
| `hogwash-practice` | Wait-for-me practice: Practice Session, Attempt, and the Cursor that never gets ahead of the performer. |
| `hogwash-analysis` | Comparison of what was played against what was expected. Reproducible by definition. |
| `hogwash-feedback` | An Analysis in terms a performer can understand. Descriptive only — never advisory. |
| `hogwash-engraving` | The port between written music and something drawn. Defines what an engraver must provide, and implements none. |
| `hogwash-engraving-verovio` | The Verovio engraver. The only crate in the workspace that knows C exists. |
| `hogwash-library` | The Pieces available to practise. Never empty, by type. |
| `hogwash-acquisition` | Resolving offered material into a practisable Piece or an explained refusal. |
| `hogwash-midi` | The performer's keyboard, and (via `output.rs`) an independent Thru route to a real synth/hardware output. The only crate that talks to MIDI hardware. |
| `hogwash-records` | What happened, kept. One file per session, written so an interruption can't damage an older one. |
| `hogwash-cli` | Determinism harness. Not a product surface — exists so determinism can be tested rather than asserted. |
| `hogwash-app` (`app/src-tauri`) | The application shell: Tauri commands over the Rust core, and the window the performer sees. |

The interface (`app/`) is React 19, TypeScript, Vite 7, Tailwind v4, and
Motion, over Tauri v2. The score is real DOM/SVG, not a rasterised picture —
a note's on-screen position comes from the browser's own layout, which is
what makes marking a note correct or wrong trustworthy at all.

## Getting started

```sh
just
```

lists every command and does nothing else. The one that matters most:

```sh
just run
```

Launches the application with live reload. It finds Verovio automatically
if you have the sibling checkout this project was built against
(`../verovio`, built into `../verovio-build`) — if not:

```sh
just verovio-build ../verovio
just run
```

Before anything is considered done:

```sh
just ci
```

The full gate — formatting, clippy with warnings denied, every Rust test,
licence/advisory checks (`cargo-deny` and a separate check for Verovio,
which isn't a Cargo dependency), the architecture guard, determinism, and
the frontend's own typecheck and tests. If it passes locally, it passes in
CI.

Requires the Rust toolchain pinned in `rust-toolchain.toml` (rustup fetches
it automatically), `cargo-deny`, and Node for the `app/` frontend.

## Licence

**Apache-2.0** (`LICENSE`). Verovio — the engraving engine — is
**LGPL-3.0-or-later** and is never vendored or linked into the binary: it's
a separate shared library, opened at run time (`ADR-0005`, `ADR-0007`), so
you can replace it with your own build. The full accounting of what that
requires and why is in `THIRD_PARTY.md`.

The pieces that ship with Hogwash are hand-encoded for this project
specifically, rather than sourced from a third party whose licence and
provenance couldn't be verified — see
`app/src-tauri/library/PROVENANCE.md`.
