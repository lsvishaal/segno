# Segno — roadmap

> **The one sentence:** _Open a piece, play it, and Segno finds the bars you
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
| Active milestone | **M-Base** (checkpoint `stable-base-1`) |
| Dates | None (D1): a milestone ends when its gates pass |

**Gates, not dates (D1, 2 Nov).** Progress is reviewed weekly in chat. A milestone
with no gate progress at 2 weekly reviews in a row means cut scope, in the cut
order below.

**Cut order (priority list; cut from the bottom):**
1. Never cut: M1.1 timestamps, M1.5 start from bar, M1.6 looping, M2.1 per-bar
   stats, weak bars over time (the Progress page), M4.1 release build.
2. Then the gates in milestone order (M-Base, M-Win, M-Test, M-Public).
3. Then M-Public's scope list, bottom first.

---

## Milestones

### M-Base — checkpoint `stable-base-1`
- [x] Named Segno everywhere (D8): code, app, docs, CI, the GitHub repo — 3 Nov
- [x] Docs match code; the dead-reference grep is empty (absorbs M4.5) — 3 Nov
- [x] All test tiers green (`just check-all`) — 3 Nov
- [x] `just e2e` green 3× in a row — 3 Nov (329, 199, 164 s; 165 checks; none skipped)
- [x] A fresh clone builds and passes `just ci` — 3 Nov (231 s from cold)
- [x] A clean first run works (empty data folder → library → a starter piece) — 3 Nov (library in 1.1–1.3 s)
- [x] Tag `stable-base-1` pushed (not a release) — 3 Nov, on 499450f
- [x] CI green on the tagged commit — 3 Nov, https://github.com/lsvishaal/segno/actions/runs/41669595757
- [x] **Phase 2.5 · M** Durable identity and history (DECISIONS 3 Nov): permanent piece IDs, sessions keep their score version, replay per version — built and tested 3 Nov (commits 1–6); Q3 re-import choice and CLI `bars` per version done 3 Nov (7a, 7b); Q4 moved to Phase 2.6
- [x] **Phase 2.6 · M** Pickups and irregular bars: the bar numbers shown are the printed ones; a pickup is "Pickup" (DECISIONS 3 Nov). Includes the Für Elise (WoO 59) pickup test (Q4) — done 3 Nov: a CC0 OpenScore fixture, the `pickup` e2e scenario, the owner's Für Elise locally
- [-] Stable base B (demo history) and C (UI tour) — 3 Nov, deferred until after `stable-base-1`; B comes before the M-Test README and screenshots (M4.6, M3.20)

### M-Win — Windows parity (D3)
First task after the checkpoint: the Verovio DLL spike (a `windows-latest` CI job builds
`verovio.dll` at the pinned commit and runs the engraving tests; ~½ day).

- [ ] CI builds and runs the Rust and UI tests on Windows
- [ ] Verovio builds for Windows in CI and loads from the installed bundle
- [ ] The data folder is in the proper Windows location (`%APPDATA%\com.segno.practice`)
- [ ] MIDI in and Thru work with a real keyboard
- [ ] Engraving and the determinism check pass on Windows
- [ ] A written manual checklist for what the Linux-only e2e kit can't cover
- [ ] Checked by the owner on their dual-boot Windows install with the FP-10 (a VM only for installer smoke tests, never MIDI sign-off)

### M-Test — tester release, Linux + Windows (was v0.1)
- [ ] **M4.1 · L** CI builds the AppImage and a Windows installer from a tag, with checksums (never cut).
      `just release` works locally since 18 Oct. Still open: an older base system, the CI job, Windows.
- [ ] **M4.2 · S** Clean-machine test on both: Windows on the owner's second laptop, kept free of dev tools; Linux on a fresh VM or live USB
- [ ] **M4.3 · S** First piece playable in ≤ 3 clicks
- [ ] **M1.3 · S** A keyboard plugged in after launch is found (every 3 s, plus Rescan); one-click
      start; visible device indicator. Remembering the keyboard: done 31 Oct
- [ ] **D14 · M** Opt-in updates: asked once on first run; daily version check, one-click install (Tauri updater); "Check for updates" in About (plugins approved 3 Nov, Q6)
- [x] Bundled pieces keep a fixed ID across edits (Phase 2.5) — 3 Nov
- [x] Pickup bars + printed bar numbers (Phase 2.6) — 3 Nov
- [ ] **M4.10 · S** No raw error text anywhere
- [ ] **M4.6 · M** README for strangers (outline in `docs/RELEASE.md`)
- [ ] **M4.11 · S** A short "please try this" card
- [ ] **M4.12 · M** Send feedback + Copy diagnostics
- [ ] **M0.3 · S** Swap the Prelude for the OpenWTC edition (D4); required before any distribution
- [ ] **M4.13 · S** `just e2e` within its 180 s budget (196 s on 2 Nov; 164–199 s on 3 Nov with 13 scenarios)
- [ ] **Dogfood gate:** the owner used the app ≥ 10 sessions over ≥ 2 weeks, and every
      `annoyances.md` entry is fixed, parked or rejected
- [ ] **M4.9 · S** One outside pianist per OS tried it while being watched; their blockers fixed
- [ ] **M4.8 · S** Tag `v0.1.0`; GitHub release with both builds, checksums and notes

### M-Public — public release (was v1.0)
- [ ] ≥ 3 testers practised a week without a blocker
- [ ] Windows signing decided
- [ ] Site and trailer
- [ ] M0.2 part 2 · S History purge (`git filter-repo`) + force-push, in its own session (blocks going public)
- [ ] D6 practice log decided

Scope (moved from v0.1 on 25 Oct, and the M3 polish):
- [ ] M1.7 · M Hands separately (Both / Right / Left; the other staff dimmed and not waited for)
- [ ] M1.9 · S Remember per-piece state (zoom, hands, loop, last bar)
- [ ] M1.10 · M Chords held together (all notes down at the same moment)
- [ ] M1.11 · M Learn / Play modes (Learn: the 3rd wrong note in a bar restarts the
      bar; suggest note names). Later: bar-gated Learn, where you can't move on until
      the bar is clean.
- [ ] M2.5 · M Practise my weak bars, rotating between passages (interleaved, not one looped until clean)
- [ ] M3.1 · L The toolbar (one row)
- [ ] M3.2 · M Focus mode while playing (the header collapses)
- [ ] M3.3 · M Auto-scroll that follows the cursor smoothly
- [ ] M3.4 · M Current-position band through both staves
- [ ] M3.6 · M Keyboard shortcuts and a `?` overlay
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

## Done so far (history)

Open tasks from these lists now live under the milestones above.

## M0 — Housekeeping

- [x] M0.1 Push to the remote — tracking fixed 25 Oct (`origin` over HTTPS, upstream set)
- [x] M0.2 part 1 Personal `.mxl` scores untracked, kept in `test-scores/` — 26 Oct
- M0.2 part 2 → M-Public
- M0.3 → M-Test
- [x] M0.4 Update `CLAUDE.md`
- [x] M0.5 `docs/` gone (re-created 25 Oct under the docs protocol: what is and what is decided, no specs)
- [x] M0.6 Start `annoyances.md`
- [~] M0.7 Decide D1, D6, D8 — D1 and D8 decided 2 Nov; D6 → M-Public

## M1 — The practice loop actually loops

- [x] M1.1 Timestamps and velocity on every played note — 20 Oct
- [x] M1.2 Leaving the practice screen ends and saves the attempt — 18 Oct
- M1.3 → M-Test
- [x] M1.4 Results after a run, with the coach's cards — 24 Oct
- [x] M1.5 Start from any bar — 22 Oct
- [x] M1.6 Loop a range of bars — 22 Oct
- [x] M1.8 Restart without leaving (R) — 18 Oct
- Moved to v1.0 (25 Oct): M1.7 hands separately, M1.9 per-piece state, M1.10 chords
  held together, M1.11 Learn / Play modes (now M-Public scope)

## M2 — The differentiator: weak bars

The definitions (pace, pause, into-bar pause, break, wrong-note cap, gave up,
passes, difficulty, levels) are owned by the doc comments in
`crates/segno-analysis/src/bars.rs` and `coach.rs`. They are not restated here.

- [x] M2.1 Per-bar stats in analysis — 24 Oct
- [x] M2.2 Per-piece history across sessions — 25 Oct
- [-] M2.3 Weak-bar tint on the score — 25 Oct, replaced by the Progress page (M2.10);
      the tint and the W key are removed in M2.10
- [-] M2.4 Bar detail popover — 25 Oct, replaced by the Progress page's bar detail
- M2.5 → M-Public scope
- [-] M2.6 History panel — 25 Oct, replaced by the Progress page
- [-] M2.7 Per-bar trend — 25 Oct, replaced by the Progress page's bar detail
- [x] M2.8 Library card shows the weakest bars and when last practised — 25 Oct
- [x] M2.9 D10 absolute levels (0.5 s / 2 / 5 / 10, provisional) from each bar's own last 3 sessions; ≥ 2 passes for judgements; per-session cells; library cards and the tint use it — 26 Oct
- [x] M2.10 The per-piece Progress page: header, Work on next (up to 3, looped from the bar before), every session bar by bar with a Now row of heights, bar detail (1 Nov: redone as Practise next, a piece map with a word per bar, and a bar's line graphs); from the cards, Continue, Results and the practice header; Practise from the library starts at once on the remembered keyboard; the on-score tint, legend and W removed — 31 Oct
- [x] M2.11 Live marks while playing (per key in a chord, wrong keys ringed, at most 3 at one place), cleared on a new attempt and each loop pass, kept after Results until the next Start; setting "Marks while playing"; key → mark 15 ms median — 26 Oct

## M4 — Release tasks

- M4.1 → M-Test
- M4.2 → M-Test
- M4.3 → M-Test
- [x] M4.4 Helpful refusals for `.mid`, `.kar`, `.pdf` and other files
- M4.5 (dead spec references; `[~]` 25 Oct) → M-Base, "docs match code"
- M4.6 → M-Test
- [x] M4.7 Version number and About
- M4.8 → M-Test
- M4.9 → M-Test

---

## Next (after M-Public, in order)

Don't start these before M-Public. Details of each are in the archive snapshot.

- **Next 1** · Tempo mode and a metronome (timing judged early/late)
- **Next 2** · Listen (sound source: D13)
- **Next 3** · MIDI file import (research: `docs/archive/MIDI-IMPORT.md`; licence: D2)
- **Next 4** · `.mscz` import
- **Next 5** · A daily practice plan across pieces
- **Next 6** · macOS (Windows moved to M-Win, D3)
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
- 2026-10-26 · Windows/macOS builds and an optional portable mode (data next to the app); macOS needs signing. Windows → M-Win (D3, 2 Nov); macOS and portable mode later.
- 2026-10-26 · Re-check level thresholds (0.5 s / 2 / 5 / 10) against the self-use week's data at the 1 Nov cut-check · 2 Nov: provisional until dogfood data exists (D10)
- 2026-11-01 · Play on the computer keyboard when no MIDI keyboard is connected ("x is middle C"; a PianoBooster idea).
- 2026-11-02 · Accent green #16a34a is close to correct-note green #2f6f4e; the oxblood change fixes it.
- 2026-11-03 · P1 MIDI import → DECISIONS Proposals (to discuss)
- 2026-11-03 · P2 Wayland Alt-Tab icon → DECISIONS Proposals (to discuss)
- 2026-11-03 · P3 Launch experience (splash screen idea) → DECISIONS Proposals (to discuss)
- 2026-11-03 · P4 Export PDF and MIDI → DECISIONS Proposals (to discuss)
- 2026-11-03 · P5 Smooth cursor → DECISIONS Proposals (to discuss)
- 2026-11-03 · P6 Design refresh (oxblood, modern UI), milestone M-Design → DECISIONS Proposals (to discuss)
- 2026-11-03 · P7 Library UX → DECISIONS Proposals (to discuss)
- 2026-11-03 · P8 Bundled starter repertoire → DECISIONS Proposals (to discuss)
- 2026-11-03 · Lettered bar numbers ("12a", MuseScore's "X1"): refused for now (E3); review before M-Test: 3 of 80 sampled OpenScore files use them
- 2026-11-03 · A pickup joined with a short last bar across a repeat (E4); revisit with a real file (OpenScore Lieder's Kinkel "Gondellied", Op. 8 No. 3, is one)
- 2026-11-03 · Cadenzas and free (unmetred) bars (E4); revisit with a real file
- 2026-11-03 · A short bar mid-piece already keeps its printed number (checked 3 Nov on a probe); a test when a real file appears
- 2026-11-03 · Für Elise (the owner's file) repeats back into its pickup (0, 1–8, 0, 1–7, 9): the E4 case; with it
- 2026-11-03 · Playing the CC0 Schumann fixture exactly as written scores 3 notes wrong (bars 16 and 28, voice + piano); look into it (not pickup-related)
- 2026-11-03 · Pickup numbered ≥ 1 (implicit / metcon="false"): handle when a real file needs it
- 2026-11-03 · e2e can't run in GitHub CI (it needs a display and ALSA); it runs locally
