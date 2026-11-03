# Segno — briefing for Claude chat

> Generated 2026-11-03 (checkpoint `stable-base-1`) from `README.md`, `ROADMAP.md`,
> `DECISIONS.md`, `CLAUDE.md` and `docs/`. It makes no claim those files don't.
> Regenerated at the end of every milestone or phase, and before a release.

**How we work:**
- **You (Claude chat) and the owner** decide *what* to build, and what will and won't
  work.
- **Claude Code** (the worker, with the repo) records those decisions, plans against
  the code, asks about anything ambiguous, then builds one phase at a time with tests.
- Don't design code-level details. Focus on product, UX and feasibility with the data
  that exists.
- End with a `DECISIONS — …` block (format at the bottom).

## In ten lines
1. Segno (SEH-nyoh) is a free, local-first desktop app: practise piano from real sheet
   music with a MIDI keyboard. The cursor **waits** for the right notes. Linux today;
   Windows joins for the first tester release (D3).
2. **The one sentence:** open a piece, play it, and Segno finds the bars you struggle
   with and helps you fix them.
3. **Built with:** a Rust workspace (13 crates, pure engine crates) and a Tauri v2 app
   (React 19, TypeScript, Tailwind v4). Verovio draws the score at run time.
4. Every played note is kept with its timing and velocity, and each session records
   the exact version of the score it was played on. Analysis is recomputed from those
   (D5), so past results never change when a score is edited.
5. **Working today:** a library with shelves and Continue; wait-mode practice; start
   from any bar; loop bars; Results with up to 3 coach cards; a per-piece Progress page.
6. **Now:** checkpoint `stable-base-1` (M-Base) is reached. Next, two spikes (Verovio on
   Windows; MIDI grading, P1, proposed), then the milestones below.
7. **No fixed dates (D1):** a milestone ends when its gates pass, reviewed weekly in chat.
   Milestones: M-Base → M-Win → M-Test (tester release, was v0.1) → M-Public (was v1.0).
8. **Fresh start (3 Nov):** history from before the rename isn't carried over; the owner
   re-imports their own pieces by hand.
9. **One developer**, who is also the main user: a self-taught adult pianist.
10. **Open:** D2 licence, D6 practice log, D13 Listen's sound, and proposals P1–P8.

## Who it's for (`docs/PRODUCT.md`)
- **Primary:** the self-taught adult with a digital piano who learns pieces they care
  about. They don't know what to practise and play start to finish every time, so the
  hard bars stay hard.
- **Secondary:** a returning player. **Tertiary:** a student between lessons.
- **Not for:** complete beginners, fans of falling notes, people without a MIDI
  keyboard, other instruments.

## Product rules (`docs/PRODUCT.md`)
- **Three screens:** Library, Practice, and a per-piece Progress page. Settings and help
  are panels. Results stays first after a run; Progress never opens by itself.
- **Pieces, not drills.** No flashcards, lessons, exams or games.
- **No gamification:** no XP, ranks, streaks, points or badges.
- **Factual feedback:** where and what, never praise or judgement. Accuracy % and tempo
  may appear, never as the headline; never the combined difficulty number.
- **Local-first:** no accounts, no telemetry, no network except the opt-in update check
  (D14, not built yet). **Everything free.** **Only lawfully redistributable** bundled
  scores.
- **Deterministic analysis:** same performance + same score = identical results.
- **Never:** accounts, sync or telemetry; XP or streaks; accuracy as the headline; drills
  or lessons; falling notes; PDF scanning; locked features; copyrighted arrangements;
  an in-app store; AI-generated advice.

## Competitors (`docs/PRODUCT.md`)
- **MidiSlayer** (closest; paid; drills first): take the one-row toolbar, the position
  band, sheet-music-cover cards, levels, a device indicator; avoid tabs, PRO badges, raw
  error toasts, ranks and streaks.
- **AnyScore** validates "weakest measures". Others: playsheet, Synthesia/Neothesia,
  midipiano.app, Soundslice.
- **The gap:** other apps grade; none diagnose what kind of problem it is and how to
  practise it.

## What exists today (`README.md`, `CHANGELOG.md`, `docs/DESIGN.md` §17)
**Library:**
- Continue (the piece practised last: when, and its weakest bars)
- Your pieces: MusicXML / MXL imports, rename and delete. A file identical to one already
  there opens it ("Already in your library"). A file with the same title and composer as
  one of your imports asks "Update <title> (keeps your history)" or "Add as a new piece".
- Starters, five, all bundled: Beginner (Lightly Row, Ode to Joy, Für Elise simplified;
  written by Claude, kept until real editions, Q1) and Intermediate (Minuet in G, Prelude
  in C)
- Cards: composer, title, size, "Last practised … · weakest bar …", a "Progress" link

**Practice:**
- the cursor waits (chords, ties, repeats, grace notes, a pickup before bar 1)
- **bar numbers are the printed ones.** A pickup is "the pickup" / "Pickup–bar 2", never
  "bar 0"; "the bar before" is the one printed before (bar 1's is the pickup). Lettered
  numbers ("12a") are refused, saying so.
- click a bar to start there; Shift-click or L to loop, with a clean-passes counter
- R restarts; zoom; note names; MIDI Thru; remembers the keyboard, zoom and marks
- a 60 s grace period when the keyboard disconnects; auto-finish 1.5 s after the last note
- marks while playing (green right keys, red ringed wrong keys; or a brief flash)

**Results** (they replace the score after a run): a top line with accuracy, tempo and
steadiness; **up to 3 coach cards** (a cut-out of your bars, what happened, why, how to
practise it, one button); Went well; Details (a per-bar chart); "Progress →".

**Coach cards:** Going into bar N · The jump in bar N · Fast notes / broken chord · Wrong
notes in bar N · Long notes · Bars N–M slow you down · Stopping at bar N · This piece is
a big step · Clean run. Thresholds live in `coach.rs`.

**The Progress page** (`docs/DESIGN.md` §17.5):
- header: "Practised N times · … in total · last practised …" (every session counts)
- **Practise next:** up to 3 passages, e.g. "Bar 6 — about 3.1 s of hesitation each time
  through" with "⟲ Practise bars 5–6"
- **The piece, bar by bar:** a box per printed bar (a pickup first, "Pickup"), each with a
  word: Secure · Small slips · Uneven · Shaky · Needs work; "Not enough yet"; blank
- **Bar N over time:** "Now: … In your first sessions: …", Practise, two line graphs
- after an update that changes a score's bars: "This piece was updated on <date>;
  bar-by-bar progress starts from then." Same bars (an edited note): combined, no line.

**Weak bars over time (D10):** a level 0–4 per bar on one fixed scale (difficulty per
pass: seconds of pause + 1 per wrong note + 2 per stop). Thresholds are provisional until
dogfood data exists. **Now** = a bar's own last 3 sessions with it, judged with ≥ 2 passes.

## What the data folder holds (`~/.local/share/com.segno.practice`)
- `versions/<fingerprint>.musicxml`: every version of every score played or imported,
  written once, read-only (bundled ones kept at startup).
- `scores/<id>.json`: an import's record (permanent ID `imp-…`, title, current version).
  Bundled pieces have fixed tags (`minuet-in-g`, …) as their IDs.
- `history/`: one JSON file per session (schema 5: piece ID, score version, rules number,
  every attempt's notes with timing and velocity). Older schemas are refused.
- `settings.json` (versioned; defaults saved on first run); `segno.log` (warnings, errors,
  panics, ~2 MB at most). No data folder: Segno says so rather than saving elsewhere.
- **Not yet known:** which hand a note is in; voice parts are expected like piano staves.

## Decided 25 Oct – 3 Nov (`DECISIONS.md`)
- **D1 (2 Nov):** no fixed dates; gates, reviewed weekly; cut order kept as a priority list.
- **D3 (2 Nov):** Linux and Windows from the tester release; unsigned Windows builds for
  testers; signing a Public gate; macOS later.
- **D8 (2 Nov):** the name is Segno, `com.segno.practice`. Repo renamed 3 Nov.
- **D14 (3 Nov):** updates are opt-in: asked once on first run; a daily check of the
  latest version number only; one-click install; "Check for updates" in About; never
  silent. An M-Test gate; the updater plugins are approved for when it's built (Q6).
- **Fresh start (3 Nov):** no migration of pre-Segno history or settings.
- **Phase 2.5 (3 Nov):** permanent piece IDs; each session records its score version and
  is judged against it; Progress combines versions with the same bars and says plainly
  when the bars changed (Q2); re-import asks Update / Add (Q3, only for your own imports);
  analysis-rule changes show nothing (Q5, a rules number is stored).
- **Phase 2.6 (3 Nov):** pickups and printed bar numbers (E1–E4 above); a pickup joined
  across a repeat, cadenzas and lettered numbers are parked until real files need them.
- **Files Claude writes are marked as Claude's** (A Little Chord Study removed, 3 Nov).
- **Brand colour:** oxblood, provisional, not applied during the freeze (2 Nov).
- **Stable base B (demo history) and C (UI tour):** deferred until after
  `stable-base-1`; B before the M-Test README and screenshots.

## Milestones (`ROADMAP.md`)
- **M-Base (reached 3 Nov):** named Segno; docs match code; all tiers green; e2e green 3×;
  a fresh clone passes `just ci`; a clean first run works; tag `stable-base-1`.
- **M-Win:** first the Verovio DLL spike; then Windows CI, Verovio on Windows, the data
  folder, MIDI in and Thru, engraving and determinism, a manual checklist, the owner's
  check on a real Windows PC with the FP-10.
- **M-Test (tester release, Linux + Windows):** M4.1 CI release builds with checksums;
  M4.2 clean machines; M4.3 playable in ≤ 3 clicks; M1.3 keyboard hot-plug; D14 updates;
  M4.10 no raw error text; M4.6 a README for strangers; M4.11 "please try this" card;
  M4.12 feedback + diagnostics; M0.3 the Prelude swap; M4.13 e2e within 180 s; the
  dogfood gate (≥ 10 sessions over ≥ 2 weeks); M4.9 one outside pianist per OS; M4.8 tag.
- **M-Public:** ≥ 3 testers for a week without a blocker; Windows signing; site and
  trailer; the history purge; D6; then the scope list (hands separately, per-piece state,
  chords held together, Learn/Play, …).

## Design rules (`docs/DESIGN.md`)
- **A quiet music room, not a game:** paper, ink, one accent; amber for slow/weak, red
  only for wrong notes; no gradients or glow.
- **Type:** serif titles; sans UI; mono for numbers. **Spacing:** a 4 px grid.
- **Motion:** 120–200 ms, ease-out, no bounce; reduced motion = fades.
- **Colour is never the only signal.** British spelling.
- **Glossary:** piece, bar, attempt, session, loop, clean pass, weak bars, pause,
  keyboard, library; the pickup.
- **Measured:** the library appears 1.3 s after a cold first launch (3 Nov); opening an
  older score version for Progress costs ~110 ms once, then ~1.5 ms; e2e takes ~200 s
  against a 180 s budget (M4.13); the AppImage is ~120 MB against a 60 MB target.

## Testing (`docs/TESTING.md`)
- Tiers: `just ci` (Rust, UI, lint, licences, determinism) → `provided-test` /
  `engraving-test` (real Verovio) → `midi-test` (ALSA) → `just e2e` (the real AppImage,
  a virtual keyboard, 13 scenarios, 165 checks, none skipped; never the owner's data).
- **Real pieces only:** the Minuet, the Prelude, a CC0 OpenScore song with a pickup
  (Schumann, Op. 127 No. 5), the owner's `test-scores/` locally.
- **The owner's manual checklist** covers only how it feels on the piano.

## Open decisions (`DECISIONS.md`)
| ID | Topic | Status |
|---|---|---|
| D2 | Licence | recommended Apache-2.0 until MIDI import is next |
| D6 | Practice log | open; recommended per-piece dates and minutes, no streaks |
| D7 | Computer-keyboard fallback | recommended, labelled "for trying Segno" |
| D12 | Bundled library | recommended 12–15 pieces (P8 proposes 15–25) |
| D13 | Listen's sound | open |

**Proposals (to discuss; PROPOSED, not scheduled):** P1 MIDI import · P2 Wayland
Alt-Tab icon · P3 launch experience · P4 export PDF and MIDI · P5 smooth cursor · P6
design refresh, milestone M-Design · P7 library UX · P8 bundled starter repertoire.

Decided: D1, D3, D4, D5, D8, D9, D10 (thresholds provisional), D11, D14.

## How the worker works (`CLAUDE.md`)
- Plan first and wait for approval; one phase at a time; stop and report.
- `just ci` green before every commit. Tests on real pieces only; never weaken a test.
- Never touch the real data folder. Ask on any unspecified user-visible choice.
- **The docs protocol:** docs change in the same commit as the behaviour; every commit
  has a `Docs:` trailer; decisions from chat go into `DECISIONS.md` first.

## Hand decisions back in this format
```
DECISIONS — <topic>
1. <decision> · why · what it replaces
2. ...
Scope: ...   Later: ...
Questions for the worker to answer from the code: ...
```
