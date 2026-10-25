# Hogwash — decisions

Why things are the way they are. Three parts: the **status** of every numbered
decision (kept current), the **D-list as first written** (options and reasoning,
kept as written), and the **log** (append-only: date · decision · why · what it
replaced). A new decision is a log line, plus a status row if it has a D-number.

## Status (updated 2026-10-25)

| ID | Topic | Status |
|---|---|---|
| D1 | Ship dates | **Open.** Working to (b): v0.1 preview 8 Nov, v1.0 13 Dec; not yet confirmed |
| D2 | Licence | Recommended: Apache-2.0 until MIDI import is next; not yet confirmed |
| D3 | Platforms | **Decided 25 Oct:** Linux only for v1.0; Windows (and Windows CI) return with Next 6 |
| D4 | Prelude in C source | **Decided 25 Oct:** swap to the OpenWTC edition (M0.3, on the v0.1 checklist) |
| D5 | Store or recompute per-bar stats | **Decided by implementation 24 Oct:** recompute from raw events |
| D6 | Practice log | **Open.** Recommended (b): per-piece log, dates and minutes, no streaks |
| D7 | Computer-keyboard fallback | Recommended: yes, v1.0 (M3.12), labelled "for trying Hogwash" |
| D8 | The name | **Open.** Must be decided for v0.1 |
| D9 | Velocity | **Decided by implementation 20 Oct:** stored, not analysed until after v1.0 |
| D10 | What "weak" means over time | **Decided 25 Oct:** see the log; replaces "average of the last 3 sessions of the piece" and the quartile ranking in the code |
| D11 | Clean pass in a loop | **Decided by implementation 22 Oct:** every position right, no wrong notes |
| D12 | Bundled library for v1.0 | Recommended: 12–15 pieces across levels (M3.18) |
| D13 | Where Listen's sound comes from | **Open** (reopened 24 Oct). "Built-in synth" is no longer on the Never list |

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
