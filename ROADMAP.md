# Hogwash — roadmap

> **The one sentence:** _Open a piece, play it, and Hogwash finds the bars you
> struggle with and helps you fix them. Free, open source, local-first._

This file is the plan only. Why things are the way they are: `DECISIONS.md`.
What the product is and isn't: `docs/PRODUCT.md`. How it looks: `docs/DESIGN.md`.
What used to be here (as-built notes, the 6 Oct inventory, the feature catalogue):
`docs/archive/ROADMAP-2026-10-25.md` and git history.

**How to use it**
- One milestone is active; work top to bottom inside it.
- Every task has an ID (`M1.5`) and a size: **S** under half a day, **M** 1–2 days,
  **L** 3–5 days. Split anything bigger before starting.
- Statuses: `[ ]` open, `[x]` done (date), `[-]` replaced or dropped (date, by what),
  `[~]` partly done.
- An idea mid-task is one dated line in the **Parking lot**, then back to work.

| | |
|---|---|
| Active milestone | **v0.1 preview** (the checklist below) |
| v0.1 preview | **Sunday 8 November 2026** |
| v1.0 public launch | **Sunday 13 December 2026** (D1) |

---

## v0.1 checklist (Sun 8 Nov)

- [~] **M0.2** Purge the five unlicensed `.mxl` scores and narrow `resources`.
      `resources` already lists only Verovio, and the AppImage holds no scores. Still
      to do: the history purge (`git filter-repo` + force-push), in its own session.
- [ ] **M0.3** Swap the Prelude for the OpenWTC edition (D4).
- [ ] **M2.9 + M2.10 + M2.11** D10 absolute weak levels, the per-piece Progress
      page, and live marks while playing (see M2 below).
- [ ] **M1.3** Remember the keyboard; auto-detect one plugged in later (every 3 s,
      plus Rescan).
- [ ] **D8** The name decided (keep "Hogwash" or rename).
- [ ] **M4.1** Release build, built by CI from a version tag.
- [ ] **M4.2** Test the AppImage on a clean system.
- [ ] **M4.3** First run playable within 3 clicks.
- [ ] **M4.6** README for a stranger.
- [ ] **M4.9** One outside pianist tries it.
- [ ] **M4.8** Tag `v0.1.0`; GitHub release with the AppImage and checksums.

**Deadline guard:** if M2.10 (Progress page) isn't done by **Sun 1 Nov**, stop. Keep
the on-score tint for v0.1 and ship the launch tasks instead.

---

## Timeline

| Week | Dates | Work | Checkpoint |
|---|---|---|---|
| 1 | 6–11 Oct | M0 housekeeping | — |
| 2 | 12–18 Oct | M1 practice loop | Loop bars 9–12 (done 22 Oct) |
| 3 | 19–25 Oct | M1.4 results + coach, M2.1–M2.3, M2.8, library | Weak bars on the Minuet (done 25 Oct) |
| 4 | 26 Oct – 1 Nov | Docs cleanup, CI, M2.9–M2.11, M1.3; **self-use week** | **Cut-check / deadline guard Sun 1 Nov** |
| 5 | 2–8 Nov | M4.1–M4.3, M4.6, M4.8, M4.9, D8, M0.2 purge, M0.3 | **v0.1 preview Sun 8 Nov** |
| 6–10 | 9 Nov – 13 Dec | M3 polish and the v1.0 list | **v1.0 Sun 13 Dec** |

**Self-use week (26 Oct – 1 Nov):** practise with Hogwash every day for at least
15 minutes, on the Minuet and one piece you want to learn. Note every friction in
`annoyances.md` (`date · where · what`). Day 7: did you reach for it on your own?
Did the weak bars match your sense of the hard bars? Which three annoyances first?

**Never cut:** M1.1 timestamps, M1.5 start from bar, M1.6 looping, M2.1 per-bar
stats, weak bars over time (the Progress page), M4.1 release build.

---

## M0 — Housekeeping

- [x] M0.1 Push to the remote — tracking fixed 25 Oct (`origin` over HTTPS, upstream set)
- [~] M0.2 Remove unlicensed scores — see the v0.1 checklist
- [ ] M0.3 Swap the Prelude for the OpenWTC edition (D4)
- [x] M0.4 Update `CLAUDE.md`
- [x] M0.5 `docs/` gone (re-created 25 Oct under the docs protocol: what is and what is decided, no specs)
- [x] M0.6 Start `annoyances.md`
- [ ] M0.7 Decide D1, D6, D8 (all three open; D8 is on the v0.1 checklist)

## M1 — The practice loop actually loops

- [x] M1.1 Timestamps and velocity on every played note — 20 Oct
- [x] M1.2 Leaving the practice screen ends and saves the attempt — 18 Oct
- [ ] **M1.3 · S** Remember the keyboard; one-click start; visible device indicator;
      auto-detect a keyboard plugged in later (every 3 s, plus a Rescan button)
- [x] M1.4 Results after a run, with the coach's cards — 24 Oct
- [x] M1.5 Start from any bar — 22 Oct
- [x] M1.6 Loop a range of bars — 22 Oct
- [x] M1.8 Restart without leaving (R) — 18 Oct
- Moved to v1.0 (25 Oct): M1.7 hands separately, M1.9 per-piece state, M1.10 chords
  held together, M1.11 Learn / Play modes (see "v1.0" below)

## M2 — The differentiator: weak bars

The definitions (pace, pause, into-bar pause, break, wrong-note cap, gave up,
passes, difficulty, levels) are owned by the doc comments in
`crates/hogwash-analysis/src/bars.rs` and `coach.rs`. They are not restated here.

- [x] M2.1 Per-bar stats in analysis — 24 Oct
- [x] M2.2 Per-piece history across sessions — 25 Oct
- [-] M2.3 Weak-bar tint on the score — 25 Oct, replaced by the Progress page (M2.10);
      the tint and the W key are removed in M2.10
- [-] M2.4 Bar detail popover — 25 Oct, replaced by the Progress page's bar detail
- [ ] M2.5 "Practise my weak bars" — moved to v1.0 (it should rotate between the weak
      passages, interleaved, not loop one until clean)
- [-] M2.6 History panel — 25 Oct, replaced by the Progress page
- [-] M2.7 Per-bar trend — 25 Oct, replaced by the Progress page's bar detail
- [x] M2.8 Library card shows the weakest bars and when last practised — 25 Oct
- [ ] **M2.9 · M** D10: a bar's "Now" level comes from the last 3 sessions in which
      that bar was played. Absolute thresholds, the same in every piece, calibrated on
      real history and approved before commit. At least 2 passes, or "not enough yet".
      Also gives difficulty per session × per bar. Library cards use it.
- [ ] **M2.10 · L** The per-piece Progress page.
      - Contents: header, "Work on next" (up to 3 passages), the session × bar grid,
        and bar detail on a column click.
      - Reached from: the library card, Continue, Results ("Progress →") and the
        practice screen. Results stays first after a run.
      - Every "Practise bars …" loop, on coach cards and here, starts one bar before
        the trouble (one shared function).
      - Removes the on-score tint, its legend and W.
- [ ] **M2.11 · M** Live marks while playing: correct notes stay green and wrong notes
      stay red with a ring, for the whole attempt. Cleared on a new attempt and on each
      loop pass. Esc after Results shows them until the next Start. Setting "Marks while
      playing" (default on). Key press → mark under 30 ms.

## M4 — Preview release v0.1

- [ ] **M4.1 · L** Linux release build (never cut).
      - AppImage (and `.deb`), with Verovio bundled as a separate library.
      - Built by CI from a version tag, not locally.
      - `just release` works locally since 18 Oct.
      - Still open: building on an older base system, `.deb`, and the CI job.
- [ ] **M4.2 · S** Test the AppImage on a clean system (a fresh user or VM).
- [ ] **M4.3 · S** First run: playable within 3 clicks.
- [x] M4.4 Helpful refusals for `.mid`, `.kar`, `.pdf` and other files
- [~] M4.5 Remove dead spec references from code comments — finished in the 25 Oct
      docs cleanup
- [ ] **M4.6 · M** README for a stranger (outline in `docs/RELEASE.md`).
- [x] M4.7 Version number and About
- [ ] **M4.8 · S** Tag `v0.1.0`; GitHub release with the AppImage, checksums and notes.
- [ ] **M4.9 · S** One outside pianist: send it to one person and watch them use it.

## v1.0 — Polish to the public launch (9 Nov – 13 Dec)

Moved from v0.1 (25 Oct):
- [ ] M1.7 · M Hands separately (Both / Right / Left; the other staff dimmed and not waited for)
- [ ] M1.9 · S Remember per-piece state (zoom, hands, loop, last bar)
- [ ] M1.10 · M Chords held together (all notes down at the same moment)
- [ ] M1.11 · M Learn / Play modes (Learn: the 3rd wrong note in a bar restarts the
      bar; suggest note names). Later: bar-gated Learn, where you can't move on until
      the bar is clean.
- [ ] M2.5 · M Practise my weak bars, rotating between passages

M3:
- [ ] M3.1 · L The toolbar (one row)
- [ ] M3.2 · M Focus mode while playing (the header collapses)
- [ ] M3.3 · M Auto-scroll that follows the cursor smoothly
- [ ] M3.4 · M Current-position band through both staves
- [-] M3.5 End-of-attempt panel around the heatmap — 25 Oct, replaced by Results (M1.4)
      and the Progress page (M2.10)
- [ ] M3.6 · M Keyboard shortcuts and a `?` overlay
- [x] M3.7 Gentle motion pass (120–200 ms, ease-out) — 25 Oct
- [~] M3.8 Library cards as sheet-music covers — shelves, small-caps composer and serif
      title done 25 Oct; cream paper and the one-line musical hook still open
- [ ] M3.9 · S Search and sort in the library
- [ ] M3.10 · M Drag-and-drop import
- [ ] M3.11 · M Onboarding strip (no keyboard) and first-run card
- [ ] M3.12 · M Computer-keyboard fallback with an on-screen keyboard (D7)
- [ ] M3.13 · S Keyboard range (detect lowest and highest key)
- [ ] M3.14 · S Octave-shift detection
- [ ] M3.15 · S Dark chrome
- [ ] M3.16 · M Accessibility pass (`docs/DESIGN.md`)
- [ ] M3.17 · S Settings panel tidy (Keyboard · Sound · Display · About)
- [ ] M3.18 · M Bundled library to 12–15 pieces (D12)
- [ ] M3.19 · S Demo video
- [ ] M3.20 · S Screenshots for the README and posts
- [ ] M3.21 · S Launch posts (`docs/RELEASE.md`)

---

## Next (after v1.0, in order)

Don't start these before v1.0 ships. Details of each are in the archive snapshot.

- **Next 1** · Tempo mode and a metronome (timing judged early/late)
- **Next 2** · Listen (sound source: D13)
- **Next 3** · MIDI file import (research: `docs/archive/MIDI-IMPORT.md`; licence: D2)
- **Next 4** · `.mscz` import
- **Next 5** · A daily practice plan across pieces
- **Next 6** · Windows build, then macOS; Windows CI returns here
- **Next 7** · Dynamics feedback (velocity is already stored, D9)
- **Next 8** · Practice notes per bar
- **Next 9** · Fingering
- **Next 10** · Session report export (PDF or Markdown, for a teacher)

**Later (unordered):**
- a one-page website
- more bundled pieces (OpenScore, OpenWTC Book I)
- accompaniment for one hand
- page-turn mode
- duet / two keyboards
- translations
- Flatpak / AUR
- practice reminders (opt-in, local)
- watch a folder of MusicXML
- simplify a piece (only if it can be done honestly)
- replace a piece's file and keep its history
- a repeat pass indicator (1st/2nd time)

---

## Parking lot

_One dated line per idea. Reviewed between milestones; survivors move into a milestone._

- 2026-10-18 · Click a note (not just a bar) to start there
- 2026-10-18 · Highlight a held note for its real written length
- 2026-10-18 · Layout modes (page / continuous / single line)
- 2026-10-18 · Drag to select a loop range, snapping to notes or bars
- 2026-10-25 · Screen-switch fades (~0.4 s) may be the felt lag; consider shortening · done 26 Oct: a 120 ms cross-fade
- 2026-10-25 · AppImage is 115 MB against a 60 MB budget (`docs/DESIGN.md`); look at what's in it · 26 Oct: WebKit 95 MB, JavaScriptCore 40 MB, ICU data 32 MB, plus AV1/JPEG-XL codecs (libaom, SvtAv1, rav1e, jxl, openraw, dav1d) pulled from this rolling distro; revisit with the CI build on Ubuntu (M4.1)
