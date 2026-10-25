# Hogwash — decisions

Open decisions, and an append-only log of decisions made.

---

## 7. Decisions — made, and still open

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

## 32. Decision log

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
- _(D1–D12 answers go here)_
