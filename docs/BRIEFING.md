# Hogwash — briefing for Claude chat

> Generated 2026-10-25 from `README.md`, `ROADMAP.md`, `DECISIONS.md`, `CLAUDE.md`
> and `docs/`. It makes no claim those files don't. Regenerated at the end of every
> milestone or phase, and before a release.

**How we work:**
- **You (Claude chat) and the owner** decide *what* to build, and what will and won't
  work.
- **Claude Code** (the worker, with the repo) records those decisions, plans
  against the code, asks about anything ambiguous, then builds one phase at a time
  with tests.
- Don't design code-level details. Focus on product, UX and feasibility with the
  data that exists.
- End with a `DECISIONS — …` block (format at the bottom).

## In ten lines
1. Hogwash is a free, local-first Linux desktop app: practise piano from real sheet
   music with a MIDI keyboard. The cursor **waits** for the right notes.
2. **The one sentence:** open a piece, play it, and Hogwash finds the bars you
   struggle with and helps you fix them.
3. **Built with:** a Rust workspace (13 crates, pure engine crates) and a Tauri v2
   app (React 19, TypeScript, Tailwind v4). Verovio draws the score at run time.
4. Every played note is kept with its timing and velocity. Analysis is recomputed
   from those, so it improves without losing old sessions.
5. **Working today:**
   - a library with shelves and Continue
   - wait-mode practice; start from any bar; loop bars
   - Results after each run with up to 3 coach cards
   - weak bars over time
6. **Next:** absolute weak-bar levels (D10), a per-piece **Progress page**, and live
   marks that stay for the whole attempt (M2.9–M2.11).
7. **v0.1 preview:** Sun 8 Nov 2026. **v1.0:** Sun 13 Dec 2026. The active milestone
   is the v0.1 checklist below.
8. **Deadline guard:** if the Progress page isn't done by Sun 1 Nov, ship v0.1 with
   the on-score tint and do launch tasks.
9. **One developer**, who is also the main user: a self-taught adult pianist.
10. **Open decisions:** D1 dates, D6 practice log, D8 the name (needed for v0.1), D13
    Listen's sound.

## Who it's for (`docs/PRODUCT.md`)
- **Primary:** the self-taught adult with a digital piano who learns pieces they
  care about. They don't know what to practise and play start to finish every
  time, so the hard bars stay hard.
- **Secondary:** a returning player. **Tertiary:** a student between lessons ("the 3
  bars you struggled with this week").
- **Not for:**
  - complete beginners
  - fans of falling notes
  - people without a MIDI keyboard
  - other instruments

## Product rules (`docs/PRODUCT.md`)
- **Three screens:** Library, Practice, and a per-piece Progress page. Settings and
  help are panels. Results stays first after a run; Progress never opens by itself.
- **Pieces, not drills.** No flashcards, lessons, exams or games.
- **No gamification:** no XP, ranks, streaks, points or badges.
- **Factual feedback:** where and what, never praise or judgement. Accuracy % and
  tempo may appear, never as the headline. Show facts (pause, wrong notes, stops,
  passes, levels), never the combined difficulty number.
- **The cursor waits.** **Local-first:** no accounts, network or telemetry.
  **Everything free.** **Only lawfully redistributable** bundled scores.
- **Deterministic analysis:** same performance + same score = identical results.
- **Never:**
  - accounts, sync or telemetry
  - XP or streaks
  - accuracy as the headline
  - drills or lessons
  - falling notes
  - PDF scanning
  - locked features
  - copyrighted arrangements
  - an in-app store
  - AI-generated advice
  - A built-in synth is *not* on the list: D13 is open.

## Competitors (`docs/PRODUCT.md`)
- **MidiSlayer** (closest; paid; drills first):
  - Take: the one-row toolbar, the position band, sheet-music-cover cards, levels,
    a device indicator, empty states that say what to do.
  - Avoid: eight tabs, PRO badges, raw error toasts, a synth that broke on Linux,
    ranks and streaks.
- **AnyScore:** validates "weakest measures".
- **Others:** playsheet, Synthesia/Neothesia, midipiano.app, Soundslice.
- **The gap:** other apps grade; none diagnose what kind of problem it is and how to
  practise it.

## What exists today (`README.md`, `CHANGELOG.md`, `docs/DESIGN.md` §17)
**Library:**
- Continue (the piece practised last: when, and its weakest bars)
- Your pieces (imports, most recent first; "+ Add a piece")
- Starter · Beginner (Lightly Row, Ode to Joy, A Little Chord Study, Für Elise
  simplified)
- Starter · Intermediate (Minuet in G, Prelude in C)
- Cards: composer in small caps, title in serif italic, size, "Last practised … ·
  weakest bar …" or "Not practised yet"

**Practice:**
- the cursor waits (chords, ties, repeats, grace notes)
- click a bar to start there; Shift-click or L to loop, with a clean-passes counter
  and "Join it back up"
- R restarts; zoom (Ctrl+wheel); note names; MIDI Thru
- a 60 s grace period when the keyboard disconnects
- auto-finish 1.5 s after the last note
- wrong notes are drawn as a red head where they'd be written; today they fade after
  1.5 s

**Results** (they replace the score after a run, Monkeytype-style):
- a top line "Bars 1–8 · 6 s" with accuracy, tempo and steadiness
- **up to 3 coach cards**, each with a cut-out of your bars, what happened, why, how
  to practise it, and one button
- Went well
- Details (big numbers, a per-bar chart)
- Keys: Enter = first card, R = again, Esc = the score

**Coach cards:**
- Going into bar N
- Jump in bar N
- Fast notes / broken chord
- Wrong notes in bar N (misread vs a key off)
- Rushed the long notes
- Bars N–M slow you down
- You keep stopping at bar N
- This piece is a big step
- Clean run / Play a few bars more

Thresholds live in `coach.rs`. Pauses are judged against your own pace and the
written rhythm, and hunting for one note counts as at most 3 wrong notes.

**Weak bars over time:** per-bar difficulty over all attempts, shown as an amber
tint on the idle score (W toggles). It is to be replaced by the Progress page.

## What history holds (`docs/DESIGN.md` §17.5)
- **Per session:** start time, attempts, how each ended, bars covered, time played.
- **Per attempt:** every note (time, key, velocity), start and end, loop or not,
  accuracy, tempo, steadiness, the coach's findings.
- **Per bar:** passes, pause (total, going into the bar, longest), breaks, wrong
  notes, stops, tempo, time played.
- **Not yet known:** which hand a note is in; key signatures in the steps.

## Decided on 25 Oct and about to be built (`DECISIONS.md`, `ROADMAP.md` M2)
- **M2.9, D10:**
  - A bar's "Now" level comes from the last 3 sessions in which that bar was played.
  - Absolute thresholds, the same in every piece, calibrated on real history.
  - At least 2 passes, or "not enough yet".
- **M2.10, the Progress page:**
  - Contents:
    - a header ("N sessions · total time · last practised …")
    - Work on next (up to 3 passages)
    - a session × bar grid with a "Now" row of bar heights
    - bar detail ("avg pause 14.1 s → 3.2 s over 6 sessions")
  - Reached from: the library card, Continue, Results and the practice screen.
  - It removes the on-score tint and W.
- **One shared rule:** every "Practise bars …" loop starts one bar before the trouble,
  including "You keep stopping".
- **M2.11, live marks:**
  - Correct notes stay green and wrong notes stay red with a ring, for the whole
    attempt.
  - Cleared on a new attempt and on each loop pass.
  - Setting "Marks while playing" (default on). Under 30 ms.
- **Moved to v1.0:**
  - hands separately
  - per-piece state
  - chords held together
  - Learn/Play modes
  - "Practise my weak bars" (which should rotate between passages)

## v0.1 checklist (`ROADMAP.md`)
- **M0.2:** purge the unlicensed scores. `resources` is done; the history purge is
  pending.
- **M0.3:** swap the Prelude for the OpenWTC edition.
- **M2.9–M2.11:** D10, the Progress page, live marks.
- **M1.3:** remember the keyboard and auto-detect it.
- **D8:** the name.
- **M4.1:** a release build made by CI from a tag.
- **M4.2:** a clean-system test.
- **M4.3:** playable in 3 clicks.
- **M4.6:** a README for strangers.
- **M4.9:** one outside pianist.
- **M4.8:** tag v0.1.0.
- **Self-use week:** 26 Oct – 1 Nov, logged in `annoyances.md`.
- **Never cut:** timestamps, start from bar, looping, per-bar stats, weak bars over
  time, release build.

## Design rules (`docs/DESIGN.md`)
- **A quiet music room, not a game:**
  - paper, ink, one green accent
  - amber for "slow/weak", red only for wrong notes
  - no gradients or glow
- **Type:** serif titles; sans UI; mono for numbers and bar counts.
- **Spacing:** a 4 px grid. Radius: 8 px controls, 12 px cards, 16 px panels.
- **Motion:** 120–200 ms, ease-out, no bounce; reduced motion = fades.
- **Colour is never the only signal.** British spelling.
- **Glossary:** piece, bar, attempt, session, loop, clean pass, weak bars, pause,
  keyboard, library.
- **Measured 25 Oct:**
  - Release is fast: opening the Prelude takes 430 ms, key press → mark 23 ms.
  - `just run` is barely slower.
  - Over budget: the AppImage is 115 MB against a 60 MB target.
  - A felt lag is unexplained (Parking lot: the ~0.4 s screen-switch fades).

## Open decisions (`DECISIONS.md`)
| ID | Topic | Status |
|---|---|---|
| D1 | Ship dates | open; working to v0.1 8 Nov + v1.0 13 Dec |
| D2 | Licence | recommended Apache-2.0 until MIDI import is next |
| D6 | Practice log | open; recommended per-piece dates and minutes, no streaks |
| D7 | Computer-keyboard fallback | recommended for v1.0 |
| D8 | The name | open; needed for v0.1 |
| D12 | Bundled library | recommended 12–15 pieces for v1.0 |
| D13 | Listen's sound | open |

Decided: D3 Linux only (Windows CI returns with Next 6), D4 Prelude swap, D5
recompute, D9 velocity stored, D10, D11 clean pass.

## How the worker works (`CLAUDE.md`)
- Plan first and wait for approval; one phase at a time; stop and report.
- `just ci` green before every commit. Tests on real pieces only.
- Never touch the real data folder. Ask on any unspecified user-visible choice.
- **The docs protocol:**
  - docs change in the same commit as the behaviour they describe
  - every commit has a `Docs:` trailer
  - decisions from chat are recorded in `DECISIONS.md` first, then planned

## Hand decisions back in this format
```
DECISIONS — <topic>
1. <decision> · why · what it replaces
2. ...
Scope for v0.1: ...   Later: ...
Questions for the worker to answer from the code: ...
```
