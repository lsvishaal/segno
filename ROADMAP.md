# Hogwash — Roadmap, Product Plan and Design Blueprint

> **The one sentence:** _Open a piece, play it, and Hogwash finds the bars you
> struggle with and helps you fix them. Free, open source, local-first._
>
> Every task in this file serves that sentence. If something doesn't, it goes
> in the **Parking lot** at the bottom, not into the active milestone.

| | |
|---|---|
| Last rewritten | 6 October 2026 |
| Based on | A full scan of the codebase (export of 6 Oct), a hands-on look at MidiSlayer, the MIDI-import research in `MIDI-IMPORT.md`, and the project history since August |
| Active milestone | **M0 — Housekeeping** |
| Next release | **v0.1 preview — Sunday 8 November** |
| Public launch | **v1.0 — Sunday 13 December** (pending decision D1) |

---

## Contents

1. How to use this file
2. Who Hogwash is for
3. Product rules (and why each exists)
4. The landscape: competitors and what we take from each
5. Where things stand (code inventory, 6 Oct)
6. Technical facts that shape the work
7. Decisions — made, and still open
8. Timeline at a glance
9. M0 — Housekeeping
10. M1 — The practice loop actually loops
11. M2 — The differentiator: weak bars
12. Self-use week and the cut-check
13. M4 — Preview release v0.1
14. M3 — Polish to v1.0
15. UX blueprint: making it frictionless
16. Visual design system
17. Screen-by-screen specifications
18. Words: microcopy and feedback language
19. Errors, edge cases and empty states
20. Accessibility
21. Performance budgets
22. Testing and quality
23. Content and licensing
24. Release and launch playbook
25. After v1.0 — the ordered plan
26. Full feature catalogue (everything ever discussed)
27. Known limits
28. Never — and why
29. Mistakes to avoid
30. Motivation and sustainability
31. Working with Claude Code
32. Decision log
33. Parking lot

---

---

## 1. How to use this file

### The rules

1. **Exactly one milestone is active.** Work top to bottom inside it. Don't
   start the next milestone until the current one's **Done when** is true.
2. **New idea mid-task?** One dated line in the **Parking lot**, then back to
   work. Don't open a branch for it, don't research it, don't prototype it.
3. **Every task has a size.**
   - **S** = under half a day
   - **M** = 1–2 days
   - **L** = 3–5 days
   - Anything bigger than L gets split before it starts.
4. **Every task has an ID** (`M1.5`) so commits, Claude Code prompts and
   `annoyances.md` lines can point at it.
5. **This file is a plan, not a spec.** It says *what* and *why* and *how
   you'll know it's done*. It does not freeze implementation details. If a
   task needs more than a short paragraph of explanation, it isn't understood
   yet: do a spike first, write down what you learned, then size it.
6. **Cut from the bottom.** On a cut-check date, cut the lowest unfinished
   items in the active milestone. Items marked **never cut** stay.
7. **Review this file only between milestones.** Mid-milestone, you only tick
   boxes and add Parking lot lines.

### Status markers

- `[ ]` not started
- `[~]` in progress (only one at a time)
- `[x]` done (all acceptance checks pass, `just ci` green, pushed)
- `[-]` cut (leave the line, add the date and why)

### Tags used in the feature catalogue

- **v0.1** — in the 8 November preview
- **v1.0** — in the 13 December public launch
- **Next** — first things after v1.0, in order
- **Later** — worth doing, not scheduled
- **Never** — decided against, with a reason (see §28)

---

## 5. Where things stand (code inventory, 6 Oct)

### What works today

| Area | State | Where |
|---|---|---|
| Library | Bundled pieces + imported MusicXML/MXL; rename, delete, duplicate detection by fingerprint; "already in your library" notice; history count per piece | `app/src/screens/Library.tsx`, `components/PieceCard.tsx`, `crates/hogwash-library`, `crates/hogwash-acquisition` |
| Import | Explained refusals for bad files; damaged-container check; `.mxl` support | `crates/hogwash-acquisition` |
| Engraving | Verovio via runtime `.so`, reflow to window width, zoom (Ctrl+scroll, remembered) | `crates/hogwash-engraving-verovio`, `screens/zoom.ts` |
| Cursor | Waits for correct notes; chords (all notes, rolled OK); ties; repeats in played order; grace notes and ornaments tolerated; 8va | `crates/hogwash-practice/src/cursor.rs` |
| Live marks | Correct/wrong notes drawn on the right staff, with ledger lines and accidental | `components/Score.tsx`, `staffGeometry.ts` |
| MIDI | Input selection, hot-unplug grace period with countdown, MIDI Thru to an output port | `crates/hogwash-midi`, `app/src-tauri/src/practice.rs` |
| Records | One file per session, written atomically | `crates/hogwash-records` |
| Settings panel | Score surface (paper/dark), note names, show geometry (debug), MIDI in/out, zoom, export MusicXML | `screens/PracticeSettings.tsx` |
| Summary | Counts (correct / incorrect / omitted / unexpected) + one line per finding | `Practice.tsx` → `SummaryPanel`, `crates/hogwash-feedback` |
| Tooling | `just replay`, `just expected`, determinism check across processes, architecture guard, licence gates, CI on Ubuntu + Windows | `justfile`, `scripts/`, `.github/workflows/ci.yml` |

**Size:** about 14.6k lines of Rust, 4.4k lines of TypeScript/TSX.

### What's missing for the one sentence

- Timestamps (and velocity) on played notes
- Any per-bar aggregation of results
- A weak-bar view
- Starting from a bar; looping a range
- Hands separately
- A history view
- A release build anyone else can install

### Rough edges found in the scan

1. Ending early prints **one line per unplayed note** (517 lines for the Prelude).
2. Leaving the practice screen **doesn't end the attempt**; it keeps running.
3. The instrument must be **picked from a list every time** before Start.
4. No device indicator outside the practice picker.
5. `CLAUDE.md` rule 7 ("clean and readable, not polished, no redesigns")
   contradicts the new goal.
6. Five `.mxl` files in `app/src-tauri/library/` are tracked by git but not
   bundled and not in `PROVENANCE.md`:
   `Canon_in_D.mxl`, `Erik_Satie_-_Gymnopedie_No.1.mxl`, `Fur_Elise.mxl`,
   `Fur_Elise_Easy.mxl`, `Ode_to_Joy_Easy_variation.mxl`.
   `Fur_Elise.mxl` has a third-party rights line (`pianolessenassen.nl`,
   "arr. Verona"). **None of these may be in a public repo.**
7. Many code comments still cite deleted specs (`003 §7.4`, `ADR-0006`,
   `FR-501`, `MVP §5`).
8. `prelude-in-c.musicxml` has no rights statement in the file itself.

---

## 6. Technical facts that shape the work

These come straight from the code and decide how big several tasks are.

- **`ObservedPerformance` stores only pitches**, in order
  (`crates/hogwash-capture/src/observed.rs`). No time, no velocity. Both
  MIDI callbacks ignore midir's timestamp (`hogwash-midi/src/keyboard.rs`,
  `_timestamp`). → M1.1 changes the core data type; records format changes
  with it.
- **The `Cursor` can only start at the beginning** (`Cursor::at_the_beginning`).
  There is no seek. → Start-from-bar and looping need a new constructor or
  method that places the cursor at the first position of a given bar,
  respecting repeats (a bar can appear twice in played order).
- **Analysis has no per-bar rollup.** `finding.rs` says so explicitly: results
  carry a location, but aggregation per measure was deferred. → M2.1 adds it.
- **`AttemptRecord` is `{ played, reached }`.** Per-bar stats can be
  *recomputed* from (score, played events) at any time because analysis is
  pure. → Decide in M2.1 whether to store stats or recompute (recommendation:
  recompute, cache in memory; store only raw events + timestamps).
- **`SessionRecord.started` is a caller-supplied string**, never read from a
  clock inside the domain. Keep that discipline for timestamps: the MIDI
  adapter supplies time as data.
- **Feedback lines are built per finding** (`hogwash-feedback`), which is why
  an early stop produces hundreds of lines. → M1.4 groups them.
- **Marks are addressed by measure + position + page** (`Mark` in
  `bridge/types.ts`). The heatmap can tint per measure using the same SVG
  geometry the cursor already uses.
- **Verovio is a runtime `.so` from a sibling checkout.** The release build
  must bundle it (LGPL, separate library). This is the single biggest
  release risk.
- **Engine crates must stay pure** (`scripts/check-arch.sh`). New logic for
  looping, hands and per-bar stats goes in `hogwash-practice` and
  `hogwash-analysis`, never in the Tauri layer.

---

## 8. Timeline at a glance

| Week | Dates | Work | Checkpoint |
|---|---|---|---|
| 1 | Tue 6 – Sun 11 Oct | M0 complete; M1.1–M1.4 | Repo pushed and clean |
| 2 | 12 – 18 Oct | M1.5–M1.9 (start from bar, looping, hands) | Can loop bars 9–12 one-handed |
| 3 | 19 – 25 Oct | M2.1–M2.4 (per-bar stats, heatmap, bar detail) | Heatmap visible on the Minuet |
| 4 | 26 Oct – 1 Nov | M2.5–M2.8; **self-use week starts Mon 26 Oct** | **Cut-check Sun 1 Nov** |
| 5 | 2 – 8 Nov | M4 (Linux build, README, first outside user) | **v0.1 preview Sun 8 Nov** |
| 6–7 | 9 – 22 Nov | M3 part 1: Practice screen, toolbar, keyboard shortcuts | Annoyances from first users triaged |
| 8–9 | 23 Nov – 6 Dec | M3 part 2: Library, empty states, onboarding, fallback keyboard, a11y | Feature freeze Sun 6 Dec |
| 10 | 7 – 13 Dec | Bundled library, demo video, launch posts, bug fixes only | **v1.0 launch Sun 13 Dec** |

Rule: if a week slips, the slip comes out of the *next* week's lowest items,
never out of the release dates.

---

## 9. M0 — Housekeeping

**Goal:** the repo is safe, honest and ready to build on.
**Time:** 1–2 days. **Starts:** now.

- [x] **M0.1 · S · Push to the remote.** From now on: commit after every
      task, push at the end of every working session.
      *Done when:* `git status` clean and `git log origin/main..main` empty.
- [ ] **M0.2 · S · Remove unlicensed scores from the repo.**
      0. The Gymnopédie No. 1 `.mxl` is used by the M1.5 tied-note test
         (`hogwash-app`, ignored test): move that test's file with the rest.
      1. `mkdir -p ../hogwash-test-scores`, copy the five `.mxl` files there,
         then `git rm` them from `app/src-tauri/library/`.
      2. Add a `test-scores/` line to `.gitignore` in case you keep a local
         copy inside the repo later.
      3. If the repo will ever be public (it will), purge them from history
         with `git filter-repo --path <file> --invert-paths` for each file,
         then force-push. Back up the repo folder first.
      Do the same for any earlier copies of the MuseTrainer/MuseScore `.mxl`
      downloads if they were ever committed (the 23 Sep baseline mentioned them).
      *Done when:* `git log --all --name-only | grep -i '\.mxl'` shows no file
      whose licence isn't recorded in `PROVENANCE.md`.
      **Deferred, 17 Oct:** the repo is private, so the five files stay (the
      tests read them). **Blocker before the repo goes public or `just release`
      is built:** do steps 1-3 above, and change `tauri.conf.json`
      `resources` from `library/*` to the bundled pieces only, or the build
      ships the five files.
- [ ] **M0.3 · S · Swap the Prelude for the OpenWTC edition** (D4).
      Download the public-domain MuseScore/OpenWTC Book I Prelude 1,
      export MusicXML, replace `prelude-in-c.musicxml`, update the
      `PROVENANCE.md` row (source URL, "public domain — OpenWTC").
      Re-run `just replay` with `prelude-in-c.perfect.json` and
      `prelude-in-c.retie.json`; regenerate them if positions moved.
      *Done when:* tests green, provenance row has a source URL and a licence.
- [x] **M0.4 · S · Update `CLAUDE.md`.**
      - Replace "ship by 8 November" with "v0.1 preview 8 Nov, v1.0 13 Dec."
      - Replace rule 7 with: *"Visual work follows §16–§17 of ROADMAP.md.
        One screen per task. Show a plan with the layout described in words
        before changing styles."*
      - Add: *"Task IDs come from ROADMAP.md (e.g. M1.5). Reference them in
        commit messages."*
- [x] **M0.5 · S · Make sure `docs/` is gone** from the working tree and
      from git (the export still contained the folder).
- [x] **M0.6 · S · Start `annoyances.md`** in the repo root (gitignored or
      not, your choice) with a one-line header: `date · where · what`.
- [ ] **M0.7 · S · Decide D1, D6, D8** (dates, practice log, name). Write
      the answers into §32.

**Done when:** pushed, no unlicensed files anywhere in history, `CLAUDE.md`
matches this file, `just ci` green.

---

## 10. M1 — The practice loop actually loops

**Goal:** practising a piece feels like practising, not like taking a test.
**Time:** ~1.5 weeks.
**Done when:** you can open the Minuet, jump to bar 9, loop bars 9–12 with
your right hand only, see "3 clean in a row," and go back to the library —
touching nothing but the score, the keyboard and at most two buttons.

### M1.1 · M · Timestamps and velocity on every played note (never cut) — done 20 Oct

- **As built:**
  - Session schema 2 stores, per attempt, `began_ms` and `notes`. Each note
    has `key`, `at_us`, `velocity` and `released_us`. The sustain pedal's
    changes are kept too.
  - Times are taken as an `Instant` when each message arrives, not from
    midir's timestamp, which restarts at 0 on an ALSA reconnect.
  - Schema 1 files still load, with their notes untimed.
  - Golden files in `crates/hogwash-records/tests/fixtures/` pin the
    format.
  - Replay is unchanged: it keeps no record, so timing it would show
    nothing.
  - Verified with `just e2e` (`scripts/e2e/`), the real-app test kit. It
    drives the packaged AppImage with a virtual MIDI keyboard; run it after
    any change in behaviour.
- **Why:** stall time per bar (M2) is impossible without them.
- **What:** midir's microsecond timestamp and the note-on velocity travel
  from the MIDI callback → practice thread → `ObservedPerformance` →
  `AttemptRecord` on disk.
- **How (outline):**
  - `hogwash-capture`: `ObservedPerformance` records `(pitch, micros, velocity)`
    instead of `pitch`. Keep `played()` returning pitches so existing
    analysis keeps working unchanged.
  - `hogwash-midi/src/keyboard.rs`: stop ignoring `_timestamp`; read velocity
    from the note-on message.
  - Records: add the fields with defaults so **old session files still load**
    (time and velocity unknown).
  - Engine stays pure: time is data supplied by the adapter, never read from
    a clock in the domain.
  - `just replay` event files: add optional `"t"` and `"v"` fields; files
    without them still work.
- **Tests:** an old record loads; a new record round-trips; replay with and
  without times gives identical findings; determinism check still passes.
- **Manual check:** play 8 notes, quit, open the newest session file, see
  increasing timestamps and non-zero velocities.

### M1.2 · S · Leaving the practice screen ends and saves the attempt — done 18 Oct

- **Why:** today the attempt keeps running in the background.
- **What:** "← Library", Esc, and opening another piece all call
  `end_attempt` first. The attempt is saved like any other.
- **Edge:** if nothing was played, don't save an empty session.
- **Manual check:** play 5 notes, press ← Library, reopen; history count +1.

### M1.3 · S · Remember the instrument; one-click start; visible device

- **Why:** picking the same keyboard every time is pure friction.
- **What:**
  - Remember the last MIDI input name. If it's connected, select it silently.
  - If exactly one device is connected, use it. Only show the picker when
    there are several, or when the remembered one is missing.
  - A device indicator in the header of both screens: green dot +
    short device name; grey dot + "No keyboard" when none. Clicking it opens
    the input/output settings.
  - **Auto-start:** with a device selected, the first correct note starts the
    attempt. No Start button needed. (Keep a visible "Start" for clarity on
    first use only; see §15.)
  - **Auto-detect (added 24 Oct):** while no keyboard is connected, look again
    every 3 seconds, on the library and the practice screen. When one appears,
    select it (the remembered one first) and show it at once: no going back,
    no re-opening the piece. While still empty, the "No MIDI keyboard" panel
    has a **Rescan** button. Unplugging mid-practice stays as it is (the
    one-minute reconnect window).
- **Manual check:** unplug/replug the FP-10 between launches; the app picks
  it up with zero clicks. Open the Minuet with the piano off, switch it on:
  within ~3 s it is picked up, with zero clicks.

### M1.4 · M · A summary you can read in five seconds — done 24 Oct

- **As built (24 Oct, third pass — the coach):** after your test ("graphs, but
  no real feedback") and research (Duke et al. 2009; r/piano, r/pianolearning;
  Chang; Morsi et al. SMC 2024; feedback-after-practice findings), the results
  lead with **at most three cards**: the bars cut out of your score (wrong
  notes ringed, pauses marked), what happened, why, 2–3 steps to fix it, and a
  button that starts the drill. Diagnoses (`hogwash-analysis/src/coach.rs`):
  going into a bar, a jump, fast notes (as a chord shape or a run), wrong
  notes (misread vs landed a key off), long notes cut short, bars that slow
  you down, stopping in the same bar, a piece that is a big step. One fumble
  is not a card; the same mistake every time through is. "Went well" chips;
  numbers and chart under Details. The run finishes by itself 1.5 s after the
  last note. Next: the long-term view of a piece (step 3), then key
  signatures and hands in the score data (step 4) for the reading and
  hand-coordination diagnoses.
- **As built (24 Oct, second pass after your test):** stopping swaps the score
  for a **results view** in the middle of the screen (Monkeytype-style; Esc
  shows the score with the run's marks, Enter takes the suggestion, R plays
  again). Big numbers: accuracy and tempo (♩/min); a chart of tempo per bar
  (per pass in a loop) with ✕ for wrong notes and a grey band for bars not
  reached; small numbers: notes right/wrong/not reached, steadiness, longest
  pause and its bar, time, restarts. It covers the whole **run** (Start to
  stopping: restarts, jumps, loop passes). Nothing sits above the score while
  practising: messages go to one status line in the header. Scores like
  accuracy % are allowed now (your call). e2e scenario `review`.
- **Why:** 517 lines is not feedback.
- **When:** only after playing (Stop, the end, or leaving), never during.
- **What:** ~80% picture, ~20% text, like Monkeytype's results screen:
  1. **On the score:** bars not reached greyed; every right note a green
     dot, every wrong one a red ✕, kept (not faded) until the next attempt.
  2. **A strip under the score:** one column per bar played, in played
     order. Column height = pause (M2.1), red dots = wrong notes, grey = not
     reached. Hover/click a column → that bar on the score.
  3. **One line of text:** *"Bars 1–18 · 6 min 40 s · most trouble going into bar 9."*
  4. **One button:** Loop the weakest spot (with the bar before it when the
     trouble is going into the bar) · plus Again · Library.
- Detailed per-note lines move behind a "Show every note" disclosure, closed
  by default.
- **Where:** numbers from `Bars` (M2.1) and grouping in `hogwash-feedback`
  (pure, tested); React only draws.

### M1.5 · M · Start from any bar (never cut) — done 22 Oct

- **As built:**
  - Session schema 3 adds, per attempt, `from` (the step it started at) and
    `ended` (finished, restart, jump, stop, left, lost). Older files load;
    a file with an unknown ending is skipped and logged.
  - A click goes to the bar's first occurrence in played order; a bar with
    nothing to strike starts at the next one. A note tied into the start
    bar is waited for.
  - The start point shows as a green "Start" marker, and resets to bar 1 on
    leaving the piece. Mid-attempt, a click keeps the attempt (`jump`).

- **Why:** you never want to play bars 1–8 again to practise bar 9.
- **What:** click a bar on the score → the cursor jumps to the first
  position of that bar and waits there.
- **How:** a new `Cursor` constructor/method that places it at the first
  position of a measure *in played order*. With repeats, a bar appears twice;
  pick the first occurrence after the current cursor, or the first overall if
  not practising.
- **UX:** hovering a bar shows a faint outline and a tooltip "Start here (bar 9)".
  The click target is the bar's whole area, not just the notes.
- **Tests:** replay from bar 9 of the Minuet; from bar 1 of the second half
  (repeat-aware).

### M1.6 · L · Loop a range of bars (never cut) — done 22 Oct

- **As built:**
  - Shift+click loops from the start bar to the bar clicked; Loop / L loops
    the cursor's bar; Esc, Loop again, a plain click or leaving ends it.
  - Each pass is its own attempt: schema 4 stores `"to"` (the loop's end,
    a step) and `"ended": "pass"`. A loop over a repeat sign is the first
    place its bars are played in a row.
  - "N clean in a row" (D11); a wrong note shows 0 at once. "stop at" 1–10
    or never, default 3, not yet remembered (M1.9). At the target: "3 clean
    passes · bars 9–12", the band goes, the cursor waits at the loop start.
  - Known limit: the key-up of a pass's last note lands in the next pass.

- **Why:** the single most effective practice technique, and the feature
  every competitor has.
- **What:**
  - Select: click a bar, Shift+click another → bars between are selected
    (a tinted band across both staves). Or use the toolbar's Loop button,
    which loops the bar the cursor is in.
  - While looping, reaching the end of the range wraps the cursor to the
    range start.
  - Counter in the toolbar: **"2 clean in a row"** (D11). A wrong or
    unexpected note resets it to 0 for that pass.
  - Optional target: "Stop after 3 clean passes" (default on, value
    editable: 1–10). When reached: a calm confirmation, loop ends, the cursor
    stays.
  - Esc or the Loop button clears the loop.
- **Where:** loop logic in `hogwash-practice` (a range on the attempt, wrap
  rule, clean-pass counter), fully unit-tested; UI only draws it.
- **Records:** each attempt stores its loop range, so stats know which bars
  were drilled.
- **Edge cases:** a range that crosses a repeat sign; a one-bar loop; a range
  ending on a tied note that continues into the next bar (the tie must not
  be waited for again on wrap).

### M1.7 · M · Hands separately (cut first)

- **What:** toolbar selector Both / Right / Left. Right = staff 1, Left =
  staff 2. The other staff is dimmed to ~35% opacity and its notes are not
  waited for. Notes played from the unselected hand are neither correct nor
  wrong (tolerated).
- **Where:** `hogwash-practice` filters expected notes by staff. Analysis
  must know which hand was practised so it doesn't count the other staff as
  "never played."
- **Edge:** pieces with one staff (Ode to Joy) hide the selector.

### M1.8 · S · Restart without leaving — done 18 Oct (bar 1); from the loop start since M1.6

- **What:** **R** restarts the attempt from the current loop start, or from
  bar 1 if no loop. Saves the current attempt first.

### M1.10 · M · Chords held together (next, after you test the results view)

- A chord step counts only when all its notes are **down at the same moment**;
  releasing one before the last arrives means pressing it again (not a mistake).
- Exempt: notes tied into it, grace notes, written arpeggios. The pedal isn't holding.
- The cursor needs releases (already recorded); analysis replays presses and
  releases in time order so screen and results agree. Old records replay leniently.
- Both modes.

### M1.11 · M · Learn / Play modes (after M1.10)

- Header toggle **Learn | Play**, remembered.
- **Learn:** the 3rd wrong note in the current bar sends the cursor back to the
  bar's start ("↺ bar 9" in the status line; tied-in notes struck, as when
  starting there). 4+ wrong in one bar in a run, note names off → once per run:
  "Bar 9 keeps tripping you — turn on note names? [Yes]". Inside a loop the
  restart goes to the bar's start and the pass isn't clean. Results show
  restarts per bar.
- **Play:** today's relaxed wait-for-me (with M1.10's chords).
- Bar restarts are recorded as attempts ending "restart" inside the run.

### M1.9 · S · Remember per-piece state

- **What:** per piece, remember last zoom, hands setting, loop range and the
  bar you stopped at. Reopening the piece restores them, with a quiet hint:
  *"Picking up at bar 9 · Start from bar 1"* (link).
- **Where:** a small per-piece settings file next to the piece record.

---

## 11. M2 — The differentiator: weak bars

**Goal:** Hogwash can tell you which bar to practise today, and you agree.
**Time:** ~1.5 weeks.

### Fixed definitions

Change these deliberately (log in §32), never by accident.

Built in M2.1 (`hogwash-analysis/src/bars.rs`, `Bars::of`):

- **Pace** = the attempt's median time per written quarter note. A slow day
  is not a hesitant one.
- **Pause** at a step = the time beyond 1.5 × what the written rhythm asks at
  that pace. A long note or rest is not a pause. The first step of an attempt
  or loop pass has none. Without a written rhythm every step counts as a quarter.
- **Into-bar pause** = the pause on a bar's first step: the change from the
  bar before (suggest looping the two together).
- **Break** = a gap over 20 s and over 10× what the music asks. Counted, not a pause.
- **Bar wrong notes** = wrong notes while the cursor waited in the bar, **at
  most 3 per step** (hunting for one note is one problem). Grace notes and
  ornaments excluded. Brushed neighbour keys count (your call, 24 Oct).
- **Gave up** = the attempt ended by Restart or Stop while in this bar.
- **Passes** = times through the bar; a repeat's bars count once per time through.
- **Bar difficulty** (`Bar::difficulty`, ranking only, never shown) = per
  pass: pause seconds + wrong notes × 1 + gave up × 2. Tune later.
- **Baseline** = average over the last 3 sessions of that piece in which the
  bar was played (D10).
- A bar needs at least **2 passes** of data before it gets a colour; until
  then it's "not enough data yet." Bars never reached have no data, not "easy".

### Tasks

- [x] **M2.1 · M · Per-bar rollup in analysis (never cut).** — done 24 Oct.
      `Bars::of(observed, reference, steps, gave_up)` → per-bar stats as
      defined above, recomputed from raw events (D5); `Bars::add` sums
      attempts and sessions. The written rhythm now comes from Verovio's
      timemap (each position's beat, each bar's length). Check real data
      with `hogwash-cli bars <score> <data folder> <piece id>`.
- [x] **M2.2 · M · Per-piece history across sessions.**
      Load all sessions of a piece, compute per-bar stats per session, then
      the baseline. Fast enough to run on open (budget: < 100 ms for 50
      sessions of the Prelude).
- [x] **M2.3 · L · Heatmap on the score (never cut).**
      - Each bar's background tinted by its difficulty relative to the
        piece's other bars (quantiles: 5 steps).
      - **Single-hue scale** (warm amber, light → deep), not red/green, so
        it works for colour-blind players and doesn't read as "failing."
      - Off while playing (it would distract); on in the idle state and in
        the end summary. Toggle with **W** ("weak bars").
      - Amber is "hard for you over time"; M1.4's green/red is "this attempt
        only". The two never show at once.
      - Legend in the corner: "Lighter = easier for you · Deeper = harder."
- [ ] **M2.4 · M · Bar detail popover.**
      Click a tinted bar (when not playing) → a small card:
      *"Bar 9 · avg pause 4.2 s · 2 wrong notes per pass · last 3 sessions"*
      plus buttons **Loop this bar** and **Start here**.
- [ ] **M2.5 · M · "Practise my weak bars."**
      One button in the toolbar (and in the summary). Picks the 3 hardest
      bars, loops the hardest until 3 clean passes, moves to the next, then
      ends with a short report: *"Bars 9, 14, 21 · 11 passes · 7 min."*
      Built entirely on M1.6 looping; no new engine.
- [ ] **M2.6 · M · History panel (inside Practice, not a new screen).**
      Opened from the toolbar (clock icon) or **Y**. Lists sessions newest
      first: date, duration, bars reached, weakest bar, loops practised.
      Clicking a session shows its heatmap on the score. Includes the
      per-piece practice log (D6).
- [ ] **M2.7 · S · Per-bar trend (cut second).**
      In the bar detail card: *"Avg pause 14.1 s → 3.2 s over 6 sessions."*
      A tiny sparkline at most; no charts page.
- [x] **M2.8 · S · Library card shows weakest bar and last practised.**
      *"Last practised 2 days ago · weakest: bar 9."*

**Done when:** after a week of real use, opening the Minuet shows a heatmap
whose darkest bars match the bars you know are hardest.

---

## 12. Self-use week and the cut-check

### The self-use week — Mon 26 Oct to Sun 1 Nov

The real gate. Building can continue in parallel.

- Practise with Hogwash **every day**, at least 15 minutes, on two pieces:
  the Minuet and one piece you actually want to learn.
- Every friction, confusion, bug or wish → one line in `annoyances.md`:
  `2026-10-27 · practice · had to scroll to see bar 30 while looping`.
- Don't fix annoyances during the week unless they block practice. Collect.
- Day 7, answer honestly in `annoyances.md`:
  1. Did I reach for Hogwash on my own, or force myself?
  2. Did the heatmap match my own sense of the hard bars?
  3. Which three annoyances would I fix first?
- Those three become the first tasks of M4/M3.

### Cut-check — Sun 1 Nov

If behind, cut in this order: M1.7 hands separately → M2.7 trend →
M2.6 history panel → M1.9 per-piece state → M2.8 library card stats.
**Never cut:** M1.1 timestamps, M1.5 start from bar, M1.6 looping,
M2.1 per-bar stats, M2.3 heatmap, M4.1 release build.

---

## 13. M4 — Preview release v0.1 (Sun 8 November)

M4 comes before M3 on purpose: **ship the working thing, then polish it.**
v0.1 is for you and a handful of pianists, not a public launch.

- [ ] **M4.1 · L · Linux release build (never cut).** AppImage + .deb via
      `tauri build`, with `libverovio.so` and Verovio's `data/` bundled as
      resources and found relative to the executable. LGPL: keep Verovio as
      a separate shared library beside the app, list it in `THIRD_PARTY.md`,
      include its licence text. Add `just release`.
      **Alpha slice done 18 Oct:** `just release` builds a 127 MB AppImage with
      Verovio and its licences in `usr/lib/Hogwash`; it starts with no
      environment variables. Still open: a build on an older base (the
      library needs this machine's libstdc++), `.deb`, M4.2.
      **Time-box: 2 days.** If it isn't working by then, stop, write down
      exactly what failed, and decide between (a) static linking with LGPL
      relinking instructions, (b) WASM Verovio in the webview. Don't spiral.
- [ ] **M4.2 · S · Test the AppImage on a clean system** (a fresh user
      account, or a CachyOS/Ubuntu live USB/VM): no Rust, no Node, no
      sibling Verovio. It must launch and play the Minuet.
- [ ] **M4.3 · S · First run: playable within 3 clicks.** Library opens with
      bundled pieces; a gentle first-run card: *"Connect your keyboard, open
      a piece, play the first note."*
- [x] **M4.4 · S · Helpful refusals for `.mid`, `.midi`, `.kar`, `.pdf`,
      `.mscz`.** Each says what it is and what to do:
      MIDI → *"Hogwash reads sheet music, not MIDI recordings. Open the file
      in MuseScore Studio (free) and export MusicXML."*
      PDF → *"PDFs are pictures of music. Audiveris (free) can try to convert
      them; results vary."*
      `.mscz` → *"Export it from MuseScore Studio as MusicXML (File → Export)."*
      _The refusals existed (format, reason, route); 18 Oct: MIDI and PDF routes now name MuseScore Studio and Audiveris._
- [x] **M4.5 · S · Remove dead spec references** (`§`, `ADR-`, `FR-`, `MVP`)
      from user-visible strings everywhere, and from comments in files touched
      during M1–M2.
      _18 Oct: none left in user-visible strings; only comments, removed as files are touched._
- [ ] **M4.6 · M · README for a stranger** (outline in §24).
- [x] **M4.7 · S · Version number and "About"** (version, licence, links,
      Verovio credit) reachable from the settings panel. _Done 18 Oct, without
      links (Hogwash makes no network calls); add the repository URL as plain
      text once the repo is public._
- [ ] **M4.8 · S · GitHub release** with the AppImage, .deb, checksums, and
      short release notes.
- [ ] **M4.9 · S · One outside pianist.** Send it to one person. Watch them
      use it (in person or screen share). Don't explain anything. Write down
      every moment they hesitate.

**Done when:** a pianist on Linux downloads one file and practises the
Minuet without asking you anything.

---

## 14. M3 — Polish to v1.0 (Mon 9 Nov – Sun 13 Dec)

**Rule for M3: polish only what exists.** No new engine features. If an
idea needs new engine logic, it goes to the Parking lot.

Order matters: the screen you use most gets polished first.

### Part 1 — Practice screen (9–22 Nov)

- [ ] **M3.1 · L · The toolbar.** One row, as specified in §17.2. Every
      control has a keyboard shortcut and a tooltip that shows it.
- [ ] **M3.2 · M · Focus mode while playing.** After the first note, the
      toolbar dims to 40% and the header collapses; moving the mouse or
      pressing Esc brings them back. The score gets every pixel.
- [ ] **M3.3 · M · Auto-scroll that follows the cursor.** The current system
      stays in the upper third of the view; scrolling is smooth (respecting
      reduced motion); during a loop, the whole range stays visible if it
      fits.
- [ ] **M3.4 · M · Current-position band.** A soft vertical band through both
      staves (MidiSlayer-style), in addition to the existing cursor line.
      Wrong-note marks fade out after ~1.5 s so the page doesn't fill with red.
- [ ] **M3.5 · M · End-of-attempt panel redesign** around the heatmap:
      headline, the worst bars as clickable chips, three actions.
- [ ] **M3.6 · M · Keyboard shortcuts + `?` overlay** (map in §15.5).
- [ ] **M3.7 · S · Gentle motion pass.** Every transition 120–200 ms, one
      easing curve, nothing bounces. Honour `prefers-reduced-motion`
      (already wired through `MotionConfig`).

### Part 2 — Library, onboarding and the rest (23 Nov – 6 Dec)

- [ ] **M3.8 · M · Library cards** as sheet-music covers (§17.1): serif
      italic title, composer small caps, musical hook line, last practised,
      weakest bar. Grouped by level for bundled pieces, "Your pieces" for
      imports.
- [ ] **M3.9 · S · Search and sort** in the library (title/composer;
      sort by last practised, title, level). Only if there are > 12 pieces.
- [ ] **M3.10 · M · Drag-and-drop import:** drop a MusicXML/MXL file
      anywhere on the window.
- [ ] **M3.11 · M · Onboarding strip** (no device) and first-run card, per
      §17.4. Dismissible, never returns once a session exists.
- [ ] **M3.12 · M · Computer-keyboard fallback** (D7): A–K = C4–C5, W/E/T/Y/U
      = black keys, Z/X shift octave, plus a clickable on-screen keyboard
      drawer. Banner: *"Using your computer keyboard — connect a MIDI keyboard
      for real practice."* Sessions recorded this way are marked as such.
- [ ] **M3.13 · S · Keyboard range** (from the "Later" list): detect lowest/
      highest key by asking the user to press both ends; warn on opening a
      piece that needs keys outside it.
- [ ] **M3.14 · S · Octave-shift detection:** if every note of the last 8 is
      exactly 12 semitones off, show *"Your keyboard seems shifted an octave
      — check its octave buttons."*
- [ ] **M3.15 · S · Dark chrome** following the score surface toggle; one
      setting controls both. Optionally follow the system theme.
- [ ] **M3.16 · M · Accessibility pass** (§20).
- [ ] **M3.17 · S · Settings panel tidy:** group into Keyboard · Sound ·
      Display · About. Hide "Show geometry" behind a developer toggle.

### Part 3 — Launch week (7–13 Dec) — bug fixes only

- [ ] **M3.18 · M · Bundled library to 12–15 pieces** (§23.3).
- [ ] **M3.19 · S · Demo video** (shot list in §24).
- [ ] **M3.20 · S · Screenshots** for README and posts: library, practice
      with heatmap, loop in progress, summary.
- [ ] **M3.21 · S · Launch posts** (§24).

**Done when:** you'd put a screenshot of Hogwash next to MidiSlayer's
and be proud of it, and three people other than you have practised with it.

---

## 25. After v1.0 — the ordered plan

Don't start any of these before v1.0 ships. Order may change based on what
users ask for; log the change in §32.

### Next 1 · Tempo mode and a metronome
- A bar that's clean in wait mode can **graduate** to tempo practice: the
  cursor moves at a set tempo; notes are judged early/late/on time.
- Metronome click: send it to the piano as MIDI (a rim-click or woodblock
  note on channel 10), not through a software audio stack.
- Tempo ladder for a loop: start at 60%, +5% after each clean pass, back
  5% after a failed one.
- Timing stats join the bar card ("rushes the third beat").

### Next 2 · Listen
- Play the score (or the loop range, or one hand) so you can hear how it
  goes: the cursor follows and each note lights up as it sounds; Space stops.
- Sound: **see D13** (your piano over MIDI, a built-in piano sound, or both).

### Next 3 · MIDI file import
- Via webmscore in the webview, behind `midiToMusicXml(bytes)` (design in
  `MIDI-IMPORT.md`). Requires the licence decision (D2): GPL-3.0 + bundle
  is the simplest route.
- Before building: run the 15-file conversion test (sequenced, performance-
  captured and full-band MIDIs) and grade A/B/C. Go only if ≥ 10/15 are A or B.
- Clear messaging: "Converted from MIDI — notation may need cleanup."

### Next 4 · `.mscz` import
- Same webmscore path; no conversion loss. Covers people's own MuseScore.com
  downloads.

### Next 5 · A daily practice plan across pieces
- "Today, 15 min: Minuet bars 9–12 (5 min) · Prelude bars 21–24 (5 min) ·
  play the Minuet through once (5 min)."
- Spaced review: bars that were weak a week ago come back.
- Still no streaks, no scores.

### Next 6 · Windows build (then macOS)
- CI already runs on Windows. Bundling Verovio as a `.dll` is the work.
- macOS needs signing/notarisation; only if there's demand.

### Next 7 · Dynamics feedback
- Velocity is already captured (D9). Compare against written dynamics
  (p, f, crescendo) per bar: "Bar 12 marked p · you played mf."

### Next 8 · Practice notes per bar
- A short text note attached to a bar ("thumb under on the F#"), shown in
  the bar card.

### Next 9 · Fingering
- Show fingering from MusicXML (Verovio already draws it); toggle on/off.

### Next 10 · Session report export
- PDF or Markdown of a week's practice, for a teacher.

### Later (unordered)
- One-page website (GitHub Pages) with the video.
- More bundled pieces (OpenScore, OpenWTC Book I in full).
- Accompaniment for one hand while you play the other (MIDI out).
- Page-turn mode (no scrolling; full page flips) for people who prefer it.
- Multiple keyboards / duet mode.
- Translations.
- Flatpak / AUR package.
- Practice-time reminders (local notification, opt-in).
- Import from a folder (watch a directory of MusicXML).
- 2026-10-18 · Feature ideas (click to seek, play button, on-screen keyboard, smooth scroll, report, layouts, ...): sorted in `FEATURE-IDEAS.md`.
- Simplify a piece (melody only / reduce chords) — only if it can be done
  honestly.

---

## 26. Full feature catalogue (everything ever discussed)

| Area | Feature | Tag | Where |
|---|---|---|---|
| Practice | Cursor waits for correct notes (chords, ties, repeats, grace, ornaments) | done | — |
| Practice | Live correct/wrong marks on the staff | done | — |
| Practice | Timestamps + velocity on notes | v0.1 | M1.1 |
| Practice | Leaving ends and saves the attempt | v0.1 | M1.2 |
| Practice | Remember instrument; first note starts | v0.1 | M1.3 |
| Practice | Readable summary (bar-level, ≤ 8 lines) | v0.1 | M1.4 |
| Practice | Start from any bar | v0.1 | M1.5 |
| Practice | Loop a bar range + clean-pass counter | v0.1 | M1.6 |
| Practice | Hands separately | v0.1 (cut first) | M1.7 |
| Practice | Restart (R) | v0.1 | M1.8 |
| Practice | Per-piece state restored | v0.1 | M1.9 |
| Practice | Focus mode while playing | v1.0 | M3.2 |
| Practice | Auto-scroll following the cursor | v1.0 | M3.3 |
| Practice | Position band | v1.0 | M3.4 |
| Practice | Keyboard shortcuts + overlay | v1.0 | M3.6 |
| Practice | Computer-keyboard fallback | v1.0 | M3.12 |
| Practice | Tempo mode, metronome, tempo ladder | Next 1 | §25 |
| Practice | Listen (MIDI out to piano) | Next 2 | §25 |
| Practice | Daily plan across pieces | Next 5 | §25 |
| Practice | Accompaniment for the other hand | Later | §25 |
| Feedback | Per-bar stats | v0.1 | M2.1 |
| Feedback | Weak-bar heatmap on the score | v0.1 | M2.3 |
| Feedback | Bar card | v0.1 | M2.4 |
| Feedback | Practise weak bars | v0.1 | M2.5 |
| Feedback | History panel + practice log | v0.1 | M2.6 |
| Feedback | Per-bar trend | v0.1 (cut second) | M2.7 |
| Feedback | Copy summary as text | v1.0 | §15.6 |
| Feedback | Dynamics feedback | Next 7 | §25 |
| Feedback | Timing (early/late) | Next 1 | §25 |
| Feedback | Session report export | Next 10 | §25 |
| Feedback | Scores, accuracy %, XP, streaks | **Never** | §28 |
| Library | Bundled + imported MusicXML/MXL, rename, delete, duplicates | done | — |
| Library | Continue card | v1.0 | §15.3 |
| Library | Sheet-music cover cards, levels | v1.0 | M3.8 |
| Library | Search and sort | v1.0 | M3.9 |
| Library | Drag-and-drop import | v1.0 | M3.10 |
| Library | Undo delete | v1.0 | §15.6 |
| Library | 12–15 bundled pieces | v1.0 | M3.18 |
| Library | Replace a piece's file, keep history | Later | §27 |
| Library | Watch a folder | Later | §25 |
| Import | Helpful refusals for MIDI/PDF/mscz | v0.1 | M4.4 |
| Import | MIDI import (webmscore) | Next 3 | §25 |
| Import | `.mscz` import | Next 4 | §25 |
| Import | PDF / OMR scanning | **Never** | §28 |
| Notation | Zoom + reflow | done | — |
| Notation | Paper / dark score | done | — |
| Notation | Note names toggle | done | — |
| Notation | Bar numbers on every system | v1.0 | §15.6 |
| Notation | Fingering toggle | Next 9 | §25 |
| Notation | Repeat pass indicator (1st/2nd time) | Later | §27 |
| Notation | Page-turn mode | Later | §25 |
| Notation | Practice notes per bar | Next 8 | §25 |
| Devices | Hot-unplug grace period | done | — |
| Devices | MIDI Thru to output | done | — |
| Devices | Device indicator in header | v0.1 | M1.3 |
| Devices | Keyboard range + warning | v1.0 | M3.13 |
| Devices | Octave-shift detection | v1.0 | M3.14 |
| Devices | Duet / two keyboards | Later | §25 |
| Look & feel | Design system (§16) applied | v1.0 | M3 |
| Look & feel | Dark chrome | v1.0 | M3.15 |
| Look & feel | Onboarding strip, first-run hints | v1.0 | M3.11 |
| Look & feel | Accessibility pass | v1.0 | M3.16 |
| Platform | Linux AppImage + .deb | v0.1 | M4.1 |
| Platform | Windows | Next 6 | §25 |
| Platform | macOS | Later | §25 |
| Platform | Flatpak / AUR | Later | §25 |
| Platform | Accounts, sync, telemetry | **Never** | §28 |
| Community | README, video, launch posts | v1.0 | §24 |
| Community | Issue templates | v1.0 | §24.4 |
| Community | Website | Later | §25 |
| Community | Translations | Later | §25 |
| Learning | Flashcards, drills, lessons, games | **Never** | §28 |

---

## 31. Working with Claude Code

### Prompt template for a task

```text
Task M1.6 from ROADMAP.md: loop a range of bars.

Read ROADMAP.md §10 (M1.6), §6 (technical facts) and §15.4 (practice states).
Then read the relevant code and give me a short plan: files to touch,
approach, tests and replay fixtures you'll add, risks. Wait for my approval.

Constraints: engine logic in hogwash-practice (pure, tested), UI only draws
it; just ci green; commit with "M1.6:" prefix; end with a manual test
checklist for the FP-10.
```

### Rules to keep in `CLAUDE.md`

- One task ID per session. Stop and report when it's done.
- Plan → approval → build → `just ci` → commit → checklist.
- No new crates, docs, or features outside the task; new ideas → one line in
  the Parking lot.
- Visual work follows §16–§17; one screen per task; describe the layout in
  words before changing styles.
- Never commit score files without a `PROVENANCE.md` row.

---

## 33. Parking lot

_One dated line per idea. Reviewed only between milestones. Ideas that
survive review move into §25 or §26._

- 
