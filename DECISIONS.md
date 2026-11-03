# Segno — decisions

Why things are the way they are. Three parts: the **status** of every numbered
decision (kept current), the **D-list as first written** (options and reasoning,
kept as written), and the **log** (append-only: date · decision · why · what it
replaced). A new decision is a log line, plus a status row if it has a D-number.

## Status (updated 2026-11-03)

| ID | Topic | Status |
|---|---|---|
| D1 | Ship dates | **Decided 2 Nov:** no fixed dates; a milestone ends when its gates pass, reviewed weekly in chat (ROADMAP.md) |
| D2 | Licence | Recommended: Apache-2.0 until MIDI import is next; not yet confirmed |
| D3 | Platforms | **Decided 2 Nov:** Linux and Windows from the first tester release (M-Test); unsigned Windows builds for testers, signing a Public gate; macOS later |
| D4 | Prelude in C source | **Decided 25 Oct:** swap to the OpenWTC edition (M0.3, on the v0.1 checklist) |
| D5 | Store or recompute per-bar stats | **Decided by implementation 24 Oct:** recompute from raw events |
| D6 | Practice log | **Open.** Recommended (b): per-piece log, dates and minutes, no streaks |
| D7 | Computer-keyboard fallback | Recommended: yes, v1.0 (M3.12), labelled "for trying Segno" |
| D8 | The name | **Decided 2 Nov (evening):** "Segno"; identifier `com.segno.practice` (rename in M-Base) |
| D9 | Velocity | **Decided by implementation 20 Oct:** stored, not analysed until after v1.0 |
| D10 | What "weak" means over time | **Decided 25 Oct**, thresholds 26 Oct (provisional), built 26 Oct (M2.9): see the log; replaces "average of the last 3 sessions of the piece" and the quartile ranking in the code; thresholds stay provisional until dogfood data exists |
| D11 | Clean pass in a loop | **Decided by implementation 22 Oct:** every position right, no wrong notes |
| D12 | Bundled library for v1.0 | Recommended: 12–15 pieces across levels (M3.18) |
| D13 | Where Listen's sound comes from | **Open** (reopened 24 Oct). "Built-in synth" is no longer on the Never list |
| D14 | Updates | **Decided 3 Nov:** opt-in. First run asks once; yes = a daily check of the latest version number on GitHub and a one-click install (Tauri updater); "Check for updates" always in About; never silent. An M-Test gate. The first-run question's wording isn't decided: the owner approves it when it's built |

## Proposals (to discuss)

The owner's proposals, 3 Nov evening, recorded as given. **Status of every entry:
PROPOSED: not decided, not scheduled, do not build.** "Touches" names an existing
decision or rule a proposal would change; it is not resolved here.

**P1 · MIDI import.** Status: PROPOSED.
- Need: many pieces adult learners want exist only as MIDI; MusicXML-only import limits reach.
- Direction: convert MIDI → MusicXML locally with webmscore (proven early Oct, `docs/archive/MIDI-IMPORT.md`); the result is flagged in the UI "converted from MIDI; bar numbers may not match a printed copy".
- Touches: D2 (webmscore makes the app GPL-3.0); "MIDI import deferred until after v1.0" (6 Oct) and PRODUCT pitfall 10; local-first (conversion stays offline); bar-accurate advice needs correct bars and rhythms.
- Open questions: the GPL-3.0 switch (D2); bundle size; how hands are split onto staves; a converted piece later imported as real MusicXML (the Q3 flow).
- Size: L. Depends on: a grading spike of 15 real MIDIs (classical, pop/film, game, Synthesia-style), each A playable / B minor fixes / C wrong rhythm or bars / D unusable; go only if ≥ 10 are A or B.
- Schedule when: spike after `stable-base-1`; build only if the spike passes and D2 is decided.

**P2 · Wayland Alt-Tab icon.** Status: PROPOSED.
- Need: the generic Wayland icon shows in Alt-Tab and the task bar (KDE Plasma Wayland).
- Direction: make the window's app_id match a desktop entry with the Segno icon; decide how the AppImage provides that entry.
- Touches: writing a `.desktop` file into `~/.local/share/applications` changes the user's system: needs the owner's OK and an opt-in.
- Open questions: root cause (app_id vs StartupWMClass vs missing entry); GNOME Wayland and X11.
- Size: S. Depends on: nothing. Schedule when: first task of M-Design.

**P3 · Launch experience (splash screen idea).** Status: PROPOSED.
- Need: launch should feel immediate and branded.
- Direction: measure cold start first (AppImage mount, WebKit, Verovio load, library scan). Preferred: the main window at once with a branded loading state; a separate splash window only if a long wait proves unavoidable.
- Touches: the performance budgets in `docs/DESIGN.md`; reduced motion.
- Open questions: targets (proposed: window visible ≤ 300 ms, library usable ≤ 1.5 s cold); Windows cold start (WebView2) once M-Win exists.
- Size: S. Depends on: the measurement. Schedule when: M-Design.

**P4 · Export PDF and MIDI (alongside MusicXML).** Status: PROPOSED.
- Need: not yet stated by the owner.
- Direction: undecided. Two features: (a) the score as PDF (Verovio SVG → PDF) or MIDI (Verovio's MIDI output); (b) the owner's performance (recorded notes, timing, velocity) as MIDI.
- Touches: none known; bundled pieces' licences allow export.
- Open questions: (a), (b) or both, and for what use (printing, a teacher, a DAW)?
- Size: S–M. Depends on: the owner's answer. Schedule when: after the answer.

**P5 · Smooth cursor.** Status: PROPOSED.
- Need: the cursor jumps from note to note; it should glide.
- Direction: animate between positions (≈ 80–120 ms, ease-out); a note arriving mid-glide jumps to the newest target at once; across a line or page break, fade or jump; reduced motion = instant.
- Touches: the motion rules in `docs/DESIGN.md` (120–200 ms, no bounce); the 30 ms key → mark budget must hold for the start of movement.
- Open questions: glide duration; whether live marks animate too.
- Size: S–M. Depends on: nothing. Schedule when: M-Design.

**P6 · Design refresh (oxblood, modern UI): a new milestone M-Design.** Status: PROPOSED.
- Need: the UI is black and white with a green hover; it looks dated and unpolished.
- Direction: a design system on a branch (e.g. `design/oxblood`): oxblood brand colour, refined type, spacing and surfaces, light and dark. The score stays paper-white; semantic colours stay (correct green, wrong red, weak amber, loop blue); the brand colour is never feedback. For review: the tokens plus `just shots` of every screen, reviewed in chat before anything merges.
- Touches: oxblood (provisional, 2 Nov); "never encode meaning in colour alone"; WCAG AA; WebKitGTK and WebView2 performance (translucency and blur can be slow).
- Open questions: what "liquid" means (Apple-style glass vs fluid motion and colour); dark mode in or out of the first pass.
- Size: M–L. Depends on: P2, P3, P5 folded in. Schedule when: after the two spikes that follow `stable-base-1`; before Windows parity, so Windows is checked once against the final UI.

**P7 · Library UX.** Status: PROPOSED.
- Need: the library feels like a wall of cards.
- Direction: Continue as the hero; Your pieces; starter repertoire by level; search and sort; clear empty states; sheet-music-cover cards; keeps "first piece playable in ≤ 3 clicks" (M4.3).
- Touches: the three-screens rule; the friction rule.
- Open questions: grouping by level vs composer vs recently played; how many pieces before search appears.
- Size: M. Depends on: P6 (same branch), P8 (content). Schedule when: M-Design.

**P8 · Bundled starter repertoire.** Status: PROPOSED.
- Need: testers need good pieces on first launch; the current starters (written by Claude) are placeholders.
- Direction: bundle 15–25 commonly played PD/CC0 pieces, beginner to intermediate, from sources with verifiable licences (e.g. OpenScore CC0, Mutopia), each with source URL, licence and editor in PROVENANCE and a permanent tag; some with pickups and repeats; piano solo pieces (voice + piano scores expect the voice line; known limit). Replaces the current starters.
- Touches: absorbs M0.3 (the Prelude swap, D4) and D12; Never "an in-app store" and local-first (bundled, not downloaded); "only lawfully redistributable scores"; Q1 (3 Nov: keep the current starters until M3.18).
- Open questions: the final list (Claude researches candidates with licences; the owner picks); AppImage size.
- Size: M. Depends on: Phase 2.6 (pickups). Schedule when: in parallel with M-Design; required before M-Test.

**Proposed order (PROPOSED):** `stable-base-1` → two spikes (Verovio on Windows; P1 MIDI grading) → M-Design (P2, P3, P5, P6, P7) with P8 in parallel → M-Win → P1 if the spike passes → the rest of M-Test. Touches: ROADMAP's milestone order (M-Win before M-Test today; no M-Design).

---

## The D-list as first written (6–24 Oct)

### Made (don't reopen without new evidence)

| ID | Decision | Date |
|---|---|---|
| — | Product is pieces + weak bars, not drills | 6 Oct |
| — | No gamification | Aug, reconfirmed 6 Oct |
| — | MIDI import deferred until after v1.0 (`MIDI-IMPORT.md`) | 6 Oct |
| — | No software synth; "Listen" will send MIDI to the user's piano | 6 Oct |
| — | Grace notes and ornaments tolerated, never counted wrong | Sep |
| — | Bundled scores must each have a provenance row | Sep |

### Open — your call (my recommendation in bold)

| ID | Question | Options | Recommendation |
|---|---|---|---|
| **D1** | Ship date structure | (a) One release on 8 Nov, polish cut to the bone. (b) **v0.1 preview 8 Nov + v1.0 public 13 Dec.** (c) Move everything to December. | **(b)** — the goal grew from "readable" to "polished", which is a real scope change. Two named dates keep "never move the date" honest. |
| **D2** | Licence | **Stay Apache-2.0 until MIDI import is actually next**; then decide GPL-3.0 + webmscore vs. user's own MuseScore. | No GPL code in the app yet. |
| **D3** | Windows/macOS in v1.0 | **Linux only.** Windows in "Next." | Bundling Verovio is the release risk; do it once. |
| **D4** | Prelude in C source | **Swap to Open Well-Tempered Clavier edition** (public domain). | Removes the missing rights statement. |
| **D5** | Store per-bar stats or recompute them | **Recompute from raw events** (analysis is pure and deterministic); cache in memory. | One source of truth; changing the difficulty formula later updates all history for free. |
| **D6** | A practice log / calendar | (a) None. (b) **Per-piece log: dates and minutes, no streaks, no counts of days in a row.** (c) MidiSlayer-style consistency heatmap. | **(b)** — factual, useful to show a teacher, no streak pressure. |
| **D7** | Computer-keyboard fallback | **Yes, v1.0**, clearly labelled "for trying Hogwash without a piano." | Lets anyone try it from the README in 10 seconds. |
| **D8** | The name "Hogwash" | Keep, or rename before launch. | **Your call.** Honest flag: "hogwash" means *nonsense* in English. It's memorable and self-deprecating, but a search for it won't find a piano app, and some people will read it as a joke. If you rename, do it before v0.1, when it costs nothing. |
| **D9** | Velocity (how hard keys are pressed) | **Capture and store it in M1.1, don't analyse it** until after v1.0. | Costs one byte per note now; dynamics feedback becomes possible later without losing old history. |
| **D10** | Heatmap baseline | **Average of the last 3 sessions** of that piece. | Smooths one bad day; still responsive. |
| **D11** | What counts as a "clean pass" in a loop | **Every position in the range satisfied with zero wrong or unexpected notes.** Stall time doesn't matter in wait mode. | Simple, explainable, matches wait mode. |
| **D12** | Bundled library size for v1.0 | **12–15 pieces across three levels**, every one with a provenance row. | Enough to try every feature; small enough to verify licences properly. |
| **D13** | Where Listen's sound comes from (reopened 24 Oct) | (a) Your piano over MIDI only. (b) A built-in piano sound only. (c) **Piano over MIDI when connected, a built-in piano sound when not.** | Your lean is a built-in engine. Recommendation (c): MIDI first has no audio bugs; add the built-in sound as the fallback, choosing a small, permissively licensed soundfont player and checking Linux audio (PipeWire) before committing. Decide when Listen is built. |

---

## Log

_Date · decision · why · what it replaced._

- 2026-09-23 · Spec-first process removed · it blocked features · the 5k-line `docs/`
- 2026-09-23 · "Never advisory" rule removed · telling the learner what to practise is the product
- 2026-09-23 · Grace notes and ornaments tolerated · stats depend on clean counts
- 2026-10-06 · MIDI import deferred to after v1.0 · licence decision needed; weak bars first
- 2026-10-06 · Product lane is pieces + weak bars, not drills · MidiSlayer owns drills
- 2026-10-06 · "Listen" will use MIDI out to the user's piano · avoids software synth/audio bugs
- 2026-10-06 · Goal widened from "readable" to "polished and frictionless" · v1.0 public launch
- 2026-10-17 · Five `.mxl` test scores stay in the private repo; move them out, purge history and narrow `resources` before going public · tests read them, no exposure while private · M0.2 as written
- 2026-10-18 · Listen stays piano-only (MIDI out), no built-in synth · reconfirmed · avoids audio-engine bugs
- 2026-10-24 · Written rhythm read into the notation (beats, bar lengths), tempo still not · so long notes aren't mistaken for pauses · "no timing in the domain"
- 2026-10-24 · Stall replaced by pause beyond the rhythm at the performer's own pace; breaks, a cap of 3 wrong per step, restarts/stops count · real playing isn't metronomic; real data had 245 wrong notes in one bar from hunting · raw stall seconds
- 2026-10-24 · Review after playing is mostly visual (score marks + per-bar strip), little text · your call · M1.4's text list
- 2026-10-24 · Auto-detect a keyboard plugged in later (every 3 s, plus Rescan) added to M1.3 · re-opening the piece to find it is friction · M1.3 checked only on opening
- 2026-10-24 · Built-in sound for Listen reopened as D13 · you'd like it · was "piano-only, no synth" (6 and 18 Oct)
- 2026-10-24 · Results after playing replace the score (Monkeytype-style), scores like accuracy % allowed, nothing above the score while practising · your test and call · the review card above the score; "counts only, no %"
- 2026-10-25 · A per-piece **Progress page** replaces M2.4 (bar popover), M2.6 (history panel) and M2.7 (trend). The product has three screens: Library, Practice, and Progress (reached from the library card, Continue, Results and the practice screen). Results stays first after a run; Progress never opens by itself · history needs room to show change across sessions · "two screens; history is a panel"
- 2026-10-25 · The on-score weak-bar tint (M2.3), its legend and the W key are removed; weak bars over time live only on the Progress page. Never-cut "heatmap" becomes "weak bars over time (Progress page)" · you disliked history drawn on the score · M2.3
- 2026-10-25 · The score shows this attempt only: correct notes stay green and wrong notes stay red with a ring for the whole attempt, cleared on a new attempt and on each loop pass; setting "Marks while playing", default on (M2.11) · marks that vanish can't be reviewed · "wrong-note marks fade after 1.5 s" (docs/DESIGN.md colour tokens, M3.4)
- 2026-10-25 · D10: a bar's "Now" level comes from the last 3 sessions in which that bar was played (per bar, not by calendar), with absolute thresholds that are the same in every piece and session, and at least 2 passes (M2.9) · a quartile ranking always calls some bar "deepest", even in a clean piece · the quartile ranking over all history
- 2026-10-25 · Every "Practise bars …" loop (coach cards and Progress) starts one bar before the trouble, clamped at bar 1, including "You keep stopping at bar N" (N−1) · practise from a bar before the error (Duke, Simmons & Cash 2009); stopping at N usually means the way into N · loops of exactly the trouble bars
- 2026-10-25 · Show facts (pause, wrong notes, stops, passes, levels), never the combined difficulty number · one number hides why · —
- 2026-10-25 · D13 stays open; "built-in synth" removed from the Never list (docs/PRODUCT.md) · the Never list contradicted D13 · "Never: a built-in software synth"
- 2026-10-25 · Releases are built by CI from a version tag, not locally (M4.1) · reproducible, and anyone can check how it was built · a local `just release`
- 2026-10-25 · Moved to v1.0: M1.7 hands separately, M1.9 per-piece state, M1.10 chords held together, M1.11 Learn/Play, M2.5 practise weak bars (which should rotate between weak passages, interleaved, not loop one until clean) · v0.1 scope · all were v0.1
- 2026-10-25 · Deadline guard: if the Progress page isn't done by Sun 1 Nov, keep the on-score tint for v0.1 and ship launch tasks · v0.1 date holds · —
- 2026-10-25 · Linux-only CI (D3); the Windows CI job is dropped and returns with Next 6 · the Windows job only tested a platform we don't ship · CI on Ubuntu + Windows
- 2026-10-25 · `docs/` is allowed again, under the docs protocol in CLAUDE.md: docs describe what is and what is decided, never speculative designs · stale, scattered docs caused wrong builds · "the docs/ folder is deleted; no spec documents"
- 2026-10-25 · Phase 0 measurements (release vs `just run`) recorded in docs/DESIGN.md (performance budgets); release is fast, `just run` is not much slower; the felt lag is unexplained (Parking lot: screen-switch fades) · measure before optimising · unmeasured budgets
- 2026-10-26 · Debug builds optimise dependencies and the two engine crates (`opt-level = 2` in `[profile.dev]`, root `Cargo.toml`): in a debug build, engraving the Prelude 155 → 97 ms, startup 991 → 709 ms, library summaries 2.8 → 1.4 ms; an incremental rebuild is unchanged (~2.3 s), one full debug rebuild ~2 min · `just run` should feel like the release · unoptimised debug dependencies
- 2026-10-26 · Release builds strip symbols (`strip = true`): the binary 13.9 → 9.2 MB, the AppImage 120.4 → 120.0 MB (its compression already squeezed them). LTO measured and left off: fat 7.3 MB but a release rebuild 17 → 83 s, thin 9.3 MB; no run-time gain from either · smaller install, no build-time cost · default release profile
- 2026-10-26 · Screens cross-fade in 120 ms (old out while new in) instead of a 200 ms fade out then a 200 ms fade in · the ~0.4 s switch was the likeliest felt lag · docs/DESIGN.md motion "200 ms (screens)"
- 2026-10-26 · Personal test scores live in the ignored test-scores/ folder, kept on disk, untracked · they're mine, not redistributable · tracked in app/src-tauri/library/
- 2026-10-26 · D10 thresholds, provisional: level 0 = no wrong notes, no stops, pause under 0.5 s per pass; then difficulty per pass under 2 → 1, under 5 → 2, under 10 → 3, 10 or more → 4; the same scale for every piece and session · calibrated on the history so far (30 bar-sessions; re-checked at the 1 Nov cut-check) · the quartile ranking (under 0.5 s → 0, the rest by quarter within the piece)
- 2026-10-26 · Difficulty stays a total per pass, not normalised per note · levels answer "where to practise", and a dense bar with several slips does need the work; teachers judge bars as secure or not; facts read as "3 wrong notes per pass", not a percentage; nothing compares pieces against each other, and progress compares a bar with itself · — (per-note normalisation was the alternative)
- 2026-10-26 · Per-session grid cells show their level even with 1 pass (a cell is a factual record of that session); cells with no passes are blank. The ≥ 2 passes rule applies only to judgements: the Now level, weakest bars (library card, Continue) and Work on next · a cell describes, a judgement decides · ≥ 2 passes everywhere, and a hairline outline for cells under 2 passes (the 25 Oct plan)
- 2026-10-26 · Stop attribution stays: a stop counts only in a bar you'd begun · a stop at a bar line is ambiguous (a hard bar, or done playing) · —
- 2026-10-26 · Work on next (Progress page) lists judged bars at level ≥ 1, worst first (level, then difficulty), neighbours merged, up to 3. If none: 'Every bar is clean in your recent sessions.' + one 'Play it through' button. The library card / Continue keep level ≥ 3 for 'weakest bars'. Reason: the card shouldn't nag about small slips; the Progress page should always offer the next target until clean.
- 2026-10-26 · "You keep stopping at bar N" loops bars N−1–N ("Practise bars N−1–N"), like every other card · one rule, and a loop is what fixes a bar you keep stopping in · "Start at bar N", which played on with no loop
- 2026-10-26 · After a run, bars not reached stay faded as part of this attempt, and clear on the next Start like the marks. Only the review ✕ counts and the green review notes are removed (live marks replace them); correct-note marks are per pitch, so unplayed notes of a chord stay ink · the fade says where the attempt stopped · the review drawing (green notes, ✕ counts, fade)
- 2026-10-26 · While an attempt runs, the practice thread reads the keyboard every 5 ms instead of every 25 ms: key → mark 25–30 ms → 14–15 ms median on the Prelude; idle stays at 500 ms · the 30 ms key → mark budget (M2.11) · a 25 ms poll
- 2026-10-31 · Visual checks are done by Claude Code from screenshots of the real AppImage driven by the e2e kit (`just shots`), not from a browser or mock mode · a plain browser has no Tauri commands, Verovio or MIDI; a mock mode would be a second app that drifts; Chromium is not WebKitGTK · the owner running every visual check
- 2026-10-31 · The owner's manual checklist covers only how it feels on the piano (timing, loops, marks keeping up) · visuals are checked from screenshots · a checklist covering every visual detail
- 2026-10-31 · "⟲ Practise bars …" on a Progress page opened from the library starts at once with the loop set, like a coach-card button, on the keyboard last started on (remembered across restarts); if that one isn't connected, the first connected keyboard; with none, the usual "No MIDI keyboard" prompt with the loop set · one click from "what to practise" to practising · opening the piece and pressing Start
- 2026-10-31 · The library card's "Progress" link is always visible, not only on hover · it is the way in to the product's main page · hover-only card actions
- 2026-11-01 · Progress page: a map of the piece (8 bars to a row, each box its number and a word) and the selected bar's two line graphs over time (hesitation and wrong notes, each time through) replace the bars × sessions grid and the Now heights; the page says what it's for, and every fact is a plain sentence ("about 3.1 s of hesitation each time through", "a wrong note 1 time in 3") · the owner found the grid and "avg pause … per pass" unreadable to a pianist · the 25–26 Oct grid, its caption and "Show all"
- 2026-11-01 · Level words, 0–4: Secure · Small slips · Uneven · Shaky · Needs work; Secure is a pale green, the rest the amber that deepens; played once is "Not enough yet" · a pianist reads a word, not a level number · levels shown only as heights and cell shades
- 2026-11-01 · "Work on next" is called "Practise next"; its button keeps "⟲" like the coach cards · it names the action · "Work on next"
- 2026-11-02 · Progress page details, recorded after the 1 Nov build: the header line is plain sans ("Practised N times · … in total"); "Every bar is secure" replaces "clean"; the hardest bar is selected on open, and with none "Click a bar to see how it has gone over time."; graph dates gain the time when two sessions share a day; the top tick rounds up to a tenth · plain words, and a graph never reads "0.0" or three identical dates · the mono "N sessions" line, "clean", no bar selected on open
- 2026-11-02 · D8 closed: the name stays "Hogwash" and the identifier `com.hogwash.practice` · the owner's call · D8 open
- 2026-11-02 · Feature freeze until the stable base is tagged: storage fixes, a demo history, a UI tour and a base check first · a stable base before anything new · building features as they come
- 2026-11-02 · A demo history for viewing and testing: real pieces played through the virtual keyboard with deliberate mistakes, dated by a clock hook, kept in `target/demo-data` (never the real folder), opened with `just demo` · something realistic to look at without touching real practice · three Minuet sessions on one day
- 2026-11-02 · Settings (marks, keyboard, zoom) move from the webview's localStorage to `settings.json` (with a schema number) in the app data folder, read from localStorage once; warnings, errors and panics go to `hogwash.log` there (1 MB, one older file kept); with no app data folder the app says so instead of using the current folder · the 2 Nov storage audit: on Windows localStorage is in a different folder, there was no log, and the fallback to "." could put records anywhere · localStorage, stderr only, "."
- 2026-11-02 · The demo clock: `HOGWASH_CLOCK_FILE` names a file holding an offset in milliseconds added to the shell's session and attempt start stamps and to the "now" the screens count from; unset, nothing changes. Note timings still come from ALSA. The demo history is deterministic in its sessions, dates (to the minute), mistakes and the word on every bar, not in exact milliseconds; `just demo` opens a fresh copy of it · dates without hand-written files, and no invented timestamps · a cargo feature (a second AppImage build)
- 2026-11-02 · D8 changed: the name is "Segno" (SEH-nyoh; the 𝄋 repeat sign), identifier `com.segno.practice`; `HOGWASH_*` environment variables become `SEGNO_*` with no fallback, and the CLI becomes `segno-cli` · "hogwash" means nonsense, which works against a tool that asks to be trusted; Segno is musical, short, and unused as a practice-app name in a web check · "keep Hogwash" (earlier on 2 Nov)
- 2026-11-02 · Brand colour: oxblood, provisional. Not applied during the freeze; applied only after screenshots of Library, Practice and Results show it doesn't read as wrong-note red. No purple or violet · the accent green is too close to correct-note green · the green accent
- 2026-11-02 · D1 changed: no fixed dates; quality over speed. A milestone ends when its gates pass; progress is reviewed weekly in chat; a milestone with no gate progress at 2 weekly reviews in a row means cut scope. The cut order stays as a priority list · quality over speed · v0.1 on 8 Nov, v1.0 on 13 Dec, the deadline guard
- 2026-11-02 · D3 changed: Windows joins Linux as a supported platform from the first tester release; macOS later. Unsigned Windows builds are fine for testers; signing is a Public gate · the owner's call (2 Nov evening) · Linux only for v1.0, Windows with Next 6
- 2026-11-02 · Milestones replace v0.1 and v1.0: M-Base (checkpoint `stable-base-1`), M-Win (Windows parity), M-Test (tester release, Linux + Windows; was v0.1), M-Public (was v1.0); each with gates, listed in ROADMAP.md · gates instead of dates · the dated v0.1 checklist, timeline and v1.0 section
- 2026-11-02 · Stable base B (demo history) and C (UI tour): the owner decides after the Phase 1 audit · they're not M-Base gates · both planned as part of the freeze
- 2026-11-02 · Level thresholds (D10) stay provisional until dogfood data exists · the self-use week produced no sessions · a re-check at the 1 Nov cut-check
- 2026-11-03 · Stable base B (demo history) and C (UI tour) are deferred until after `stable-base-1`; B comes before the M-Test README and screenshots (M4.6, M3.20) · neither is an M-Base gate, and B adds a shell hook during the freeze · "the owner decides after the Phase 1 audit"
- 2026-11-03 · The old localStorage keys `hogwash.zoom`, `hogwash.marks` and `hogwash.keyboard` keep their names through the rename · the one-time settings migration reads them by name; renaming them would silently lose the owner's settings · renaming every "hogwash"
- 2026-11-03 · The owner's data starts fresh: only the owner's own imported MusicXML files matter, and Claude re-imports them (Canon in D, Metro 2033 Main Menu Theme, "Untitled"; not the generated "Ode to Joy Level 1") after Phase 2.5. The owner parks the old folder as a backup. No code reads pre-Segno history, settings or webview storage · the owner is the only user so far · keeping old history readable through aliases and old-format readers (the first Phase 2.5 draft)
- 2026-11-03 · The one-time localStorage → `settings.json` migration and the three `hogwash.*` keys are removed; with no `settings.json`, the defaults are used and saved · fresh start · the 3 Nov line keeping the old keys
- 2026-11-03 · A Little Chord Study is no longer bundled; files Claude writes (scores, fixtures) are marked as Claude's in PROVENANCE or a header comment · Claude wrote it for testing; the owner keeps only pieces they chose · bundling it
- 2026-11-03 · Phase 2.5, durable identity: every piece has a permanent ID (bundled: a fixed tag in a manifest, e.g. `minuet-in-g`; imported: an ID given once at import). The content hash stays, only to spot duplicate imports and to name a version. Every session records the piece ID and the score version (content hash) it was played on; that version is kept in the data folder's content-addressed score store (bundled ones copied in on first play); analysis replays each session against its own version, so editing a score never changes past results. Progress combines sessions per bar number across versions and says so plainly when a new version changes the bar count or structure · any edit to a score orphaned its history, and under D5 old attempts would be re-judged against edited notes · piece ID = SHA-256 of the file (first 16 hex)
- 2026-11-03 · D14 decided: updates are opt-in. First run asks once whether to check for new versions (only the latest version number is fetched from GitHub; nothing else is sent). Yes = a daily check offering a one-click install via the Tauri updater; "Check for updates" is always in About. No silent updates. The product rule "no network" becomes "no network except the opt-in update check"; an M-Test gate · testers need fixes without hunting for downloads · no update mechanism
- 2026-11-03 · Q1: Lightly Row, Ode to Joy and the simplified Für Elise stay as starters until real editions replace them (M3.18) · something to play on first run · removing every piece Claude wrote
- 2026-11-03 · Q2: when a new version of a piece changes its bar structure, Progress shows the newest version only, with one line: "This piece was updated on <date>; bar-by-bar progress starts from then." Older sessions still count in the header totals. With the same structure, bars are combined by number across versions · bars that don't match are never mixed · showing old and new separately (Claude's proposal)
- 2026-11-03 · Q3: re-importing a file whose title and composer match a piece in the library asks once: "Update <title> (keeps your history)" (default) or "Add as a new piece"; otherwise it is added as new · an edited file is usually a new version · always a new piece
- 2026-11-03 · Q4: the pickup-bar test uses a public-domain Für Elise (WoO 59) MusicXML with a verifiable PD/CC0 licence, recorded in PROVENANCE; tracked only if the licence allows, else `test-scores/`; with no clear licence, stop and ask · a real pickup piece · none
- 2026-11-03 · Q5: analysis-rule changes show nothing in the UI; each session stores a rules number internally · D5 stays as it is for the performer · a "judged with newer rules" note
- 2026-11-03 · Q6: the D14 plugins (`tauri-plugin-updater`, `tauri-plugin-process`) are approved in principle, added when the updater is built in M-Test; the owner keeps the signing key in a password manager plus an offline backup · open
- 2026-11-03 · Q7 and the re-import: the owner re-imports their own pieces by hand after the checkpoint; Claude imports nothing into the real data folder · the owner's call · Claude re-importing three pieces after Phase 2.5
- 2026-11-03 · The library e2e seeds its own import (the bundled public-domain Minuet, through `segno-cli import`) instead of copying the owner's folder, so its checks run everywhere; startup no longer deletes duplicate imports (duplicates are caught at import) · tests must not depend on the owner's data; deleting files at startup risks losing them · copying the owner's folder; `collapse` at startup
- 2026-11-03 · Pickup bars and irregular bar numbers are fixed before the checkpoint (Phase 2.6, M-Base), not in M-Test. Rule: the bar numbers Segno shows always equal the numbers printed on the score (its own measure numbers), never Segno's count. A pickup is shown as "Pickup" (in sentences such as "Going into bar 1 from the pickup", on the Progress map, in loop and start-from labels), never "bar 0" or "bar 1"; it is its own entry for stats and levels; "loop from one bar before" when the trouble is in bar 1 starts at the pickup · pickups are common (Für Elise, many folk tunes and hymns), and a refusal or a shifted number points every coach card, loop and Progress entry at the wrong bar · refusing a bar numbered 0
- 2026-11-03 · CLI `bars` replays each session against its own version from `versions/`, like the app; the score given is only a fallback (it doesn't keep versions with other bars apart) · one judgement everywhere · replaying every session against the score given
- 2026-11-03 · E1: the pickup fixture is a real CC0 OpenScore piece (Schumann, "Schlusslied des Narren", Op. 127 No. 5, unpacked only), tracked under `app/src-tauri/tests/fixtures` with a PROVENANCE row; the owner's `test-scores/Fur_Elise.mxl` also runs locally and skips visibly when absent; no converted files (python-ly) · a converted file would partly test the converter · Für Elise only
- 2026-11-03 · E2: wording with the pickup: labels and cut-outs "Pickup–bar 2"; the button "⟲ Practise pickup–bar 2"; sentences "from the pickup to bar 2"; alone "the pickup". Never "bar 0" · plain words · open
- 2026-11-03 · E3: a lettered bar number ("12a") is refused: "This score numbers a bar 12a; Segno can't open scores with lettered bar numbers yet." Parking lot · no real file has one yet · open
- 2026-11-03 · E4: a pickup joined with a short last bar across a repeat, and cadenza or free bars, are out of Phase 2.6 (Parking lot) until a real file appears; a short bar mid-piece already keeps its printed number · real files first · open
- 2026-11-03 · Pickup detection and order: a first bar marked incomplete (Verovio `metcon="false"`, MusicXML `implicit="yes"`) is the pickup whatever its number; if a file numbers its pickup 1, Segno stops and reports rather than shift the printed numbers. Bar numbers shown are the printed ones; "previous" and "next" bar, merging neighbours and "one bar before" use written order. Bar 0 must count as a bar in the UI (no truthiness checks) · the printed score is what the performer looks at · number − 1 arithmetic and bar 1 defaults
- 2026-11-03 (evening) · The Pickup is a first bar numbered 0. An incomplete first bar numbered 1 or higher is treated as a normal bar for now (Parking lot) · matches the code; every pickup in the real files sampled is numbered 0 · "a first bar marked incomplete is the pickup whatever its number" (3 Nov)
- 2026-11-03 (evening) · `segno-cli bars` keeps its argument order: `<score.musicxml> <data folder> <piece id>` · the code; docs follow it · `bars <data folder> <piece id> [score]` (the Phase 2.5 plan)
- 2026-11-03 (evening) · Test machines (decided in chat 3 Nov): Windows MIDI sign-off on the owner's dual-boot Windows install with the FP-10; a VM only for installer smoke tests, never MIDI sign-off; the Windows clean-machine test (M4.2) on the owner's second laptop, kept free of dev tools; the Linux clean-machine test on a fresh VM or live USB · real hardware for real MIDI timing; a machine that never had dev tools is closest to a tester's · "a real Windows PC", "a fresh user or VM"
- 2026-11-03 (evening) · D14: the first-run question's wording is not decided; it's user-visible copy the owner approves when the updater is built · open
