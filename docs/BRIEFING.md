# Segno — briefing for Claude chat

> Generated 2026-11-02 (after stable base part A) from `README.md`, `ROADMAP.md`, `DECISIONS.md`, `CLAUDE.md`
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
1. Segno is a free, local-first Linux desktop app: practise piano from real sheet
   music with a MIDI keyboard. The cursor **waits** for the right notes.
2. **The one sentence:** open a piece, play it, and Segno finds the bars you
   struggle with and helps you fix them.
3. **Built with:** a Rust workspace (13 crates, pure engine crates) and a Tauri v2
   app (React 19, TypeScript, Tailwind v4). Verovio draws the score at run time.
4. Every played note is kept with its timing and velocity. Analysis is recomputed
   from those, so it improves without losing old sessions.
5. **Working today:**
   - a library with shelves and Continue
   - wait-mode practice; start from any bar; loop bars
   - Results after each run with up to 3 coach cards
   - a per-piece **Progress page**: what to practise next, a word for every bar, graphs
6. **Now:** a feature freeze until the stable base is tagged (2 Nov): storage fixes
   (done), a demo history, a UI tour, a base check. Then the v0.1 launch tasks.
7. **v0.1 preview:** Sun 8 Nov 2026. **v1.0:** Sun 13 Dec 2026. The active milestone
   is the v0.1 checklist below.
8. **Deadline guard** (Progress page by Sun 1 Nov): met on 31 Oct; the on-score tint
   is gone.
9. **One developer**, who is also the main user: a self-taught adult pianist.
10. **Open decisions:** D1 dates, D6 practice log, D13 Listen's sound. D8 is
    decided: the name is "Segno".

## Who it's for (`docs/PRODUCT.md`)
- **Primary:** the self-taught adult with a digital piano who learns pieces they
  care about. They don't know what to practise and play start to finish every
  time, so the hard bars stay hard.
- **Secondary:** a returning player. **Tertiary:** a student between lessons ("the 3
  bars you struggled with this week").
- **Not for:** complete beginners, fans of falling notes, people without a MIDI
  keyboard, other instruments.

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
- **Never:** accounts, sync or telemetry; XP or streaks; accuracy as the headline;
  drills or lessons; falling notes; PDF scanning; locked features; copyrighted
  arrangements; an in-app store; AI-generated advice. A built-in synth is *not* on
  the list: D13 is open.

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
  weakest bar …" or "Not practised yet", and an always-visible "Progress" link (also
  beside Continue)

**Practice:**
- the cursor waits (chords, ties, repeats, grace notes)
- click a bar to start there; Shift-click or L to loop, with a clean-passes counter
  and "Join it back up"
- R restarts; zoom (Ctrl+wheel); note names; MIDI Thru; remembers the keyboard,
  zoom and marks in `settings.json` beside the history
- a 60 s grace period when the keyboard disconnects
- auto-finish 1.5 s after the last note
- **marks while playing:** each right key's note turns green, a wrong key is a red head
  with a ring where it would be written; they stay for the attempt (clear on a new
  attempt and each loop pass); Settings › Marks while playing (off = a brief flash).
  Key → mark 15 ms median on the Prelude

**Results** (they replace the score after a run, Monkeytype-style):
- a top line "Bars 1–8 · 6 s" with accuracy, tempo and steadiness
- **up to 3 coach cards**, each with a cut-out of your bars, what happened, why, how
  to practise it, and one button
- Went well
- Details (big numbers, a per-bar chart)
- Keys: Enter = first card, R = again, Esc = the score
- "Progress →" opens the piece's Progress page

**The Progress page** (`docs/DESIGN.md` §17.5; built 31 Oct, made readable 1 Nov):
- header: "Practised 3 times · 4 min in total · last practised today" + what it's for
- **Practise next:** up to 3 passages (judged bars at level ≥ 1, worst first,
  neighbours merged), e.g. "Bar 6 — about 3.1 s of hesitation each time through"
  with "⟲ Practise bars 5–6"; none: "Every bar is secure in your recent sessions."
- **The piece, bar by bar:** 8 boxes to a row, each its number and a word:
  Secure · Small slips · Uneven · Shaky · Needs work; "Not enough yet" = played
  once; blank = not played; a legend underneath
- **Bar N over time** (hardest first): "Now: … In your first sessions: …", Practise,
  and line graphs of hesitation and wrong notes per time through, a dot per session
- never played: "Play this piece once and your progress shows here." + Practise
- from the cards, Continue, Results and the practice header; Practise from the
  library starts at once on the remembered keyboard (no keyboard: the loop is set
  and the picker asks)

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

**Weak bars over time (D10, built 26 Oct):**
- Each bar gets a level 0–4 on one fixed scale for every piece:
  - 0 = no wrong notes, no stops, pause under 0.5 s per pass
  - then difficulty per pass (seconds of pause + 1 per wrong note + 2 per stop)
    under 2 / 5 / 10 → 1 / 2 / 3, else 4
  - provisional, re-checked at the 1 Nov cut-check
- **Now** is a bar's own last 3 sessions in which it was played, judged only with ≥ 2
  passes. Per-session cells show their level from 1 pass.
- Shown on the library cards ("weakest bars …", level 3+) and on the Progress page.
  Nothing about past sessions is drawn on the score.

## What history holds (`docs/DESIGN.md` §17.5)
- **Per session:** start time, attempts, how each ended, bars covered, time played.
- **Per attempt:** every note (time, key, velocity), start and end, loop or not,
  accuracy, tempo, steadiness, the coach's findings.
- **Per bar:** passes, pause (total, going into the bar, longest), breaks, wrong
  notes, stops, tempo, time played.
- **Not yet known:** which hand a note is in; key signatures in the steps.

**Data folder** (`~/.local/share/com.segno.practice`): imported scores (copied in,
named by a hash of their bytes), `history/` (one JSON file per session, versioned),
`settings.json` (versioned), `segno.log` (warnings, errors, panics; ~2 MB at most).
No data folder: Segno says so rather than saving elsewhere.

## Decided 25 Oct – 2 Nov (`DECISIONS.md`)
- **D10:** difficulty stays a total per pass, not per note; a stop counts only in a
  bar you'd begun.
- **One shared rule:** every "Practise bars …" loop starts one bar before the trouble,
  including "You keep stopping" (N−1–N).
- **Live marks:** after Results, bars not reached stay faded with the marks until the
  next Start.
- **Checking (31 Oct):** Claude Code checks visuals itself from screenshots of the
  real AppImage driven by the e2e kit (`just shots`); the owner's checklist covers
  only how it feels on the piano.
- **Progress page (1–2 Nov):** the piece map with a word per bar and line graphs
  replaced the bars × sessions grid, which wasn't readable to a pianist.
- **2 Nov:** D8 decided (keep the old name; changed the same evening to "Segno", `com.segno.practice`); feature freeze until
  the stable base is tagged; a demo history from real pieces played through the
  virtual keyboard, dated by a clock hook (`SEGNO_CLOCK_FILE`), in
  `target/demo-data`, opened with `just demo`; settings and a log file in the data
  folder.
- **Moved to v1.0:** hands separately, per-piece state, chords held together,
  Learn/Play modes, "Practise my weak bars" (which should rotate between passages).

## v0.1 checklist (`ROADMAP.md`)
- **M0.2 part 1 (done):** the five personal scores are untracked, kept in the
  ignored `test-scores/`. Part 2, the history purge, blocks going public, not v0.1.
- **M0.3:** swap the Prelude for the OpenWTC edition.
- **M2.9, M2.10, M2.11 (done):** D10 absolute levels, the Progress page, live marks.
- **M1.3:** auto-detect a keyboard plugged in later (remembering it is done).
- **D8 (decided 2 Nov):** the name is "Segno".
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
- **Measured 25–26 Oct:**
  - Release is fast: opening the Prelude takes 430 ms, key press → mark 23 ms.
  - Debug builds now optimise dependencies, so `just run` is close to release.
  - Screens cross-fade in 120 ms (was ~0.4 s of fades, the likeliest felt lag).
  - Over budget: the AppImage is 115 MB against a 60 MB target; most of it is
    WebKit and host codecs, to revisit with the CI build (M4.1).

## Open decisions (`DECISIONS.md`)
| ID | Topic | Status |
|---|---|---|
| D1 | Ship dates | open; working to v0.1 8 Nov + v1.0 13 Dec |
| D2 | Licence | recommended Apache-2.0 until MIDI import is next |
| D6 | Practice log | open; recommended per-piece dates and minutes, no streaks |
| D7 | Computer-keyboard fallback | recommended for v1.0 |
| D12 | Bundled library | recommended 12–15 pieces for v1.0 |
| D13 | Listen's sound | open |

Decided: D3 Linux only (Windows CI returns with Next 6), D4 Prelude swap, D5
recompute, D8 "Segno", D9 velocity stored, D10, D11 clean pass.

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
