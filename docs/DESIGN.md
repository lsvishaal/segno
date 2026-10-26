# Hogwash — design

How Hogwash looks, behaves and speaks: the UX, the visual system, each screen, the words, and the budgets it must meet.

---

## 15. UX blueprint: making it frictionless

### 15.1 The friction principle

> **From launching Hogwash to hearing your first note judged: at most
> 2 clicks. From finishing a pass to starting the next: 0 clicks.**

Every flow below has a click budget. A change that adds a click needs a reason
written in DECISIONS.md.

### 15.2 Friction audit (25 Oct)

Launch → play the Minuet, as the code works now:

| Step | Today | Target |
|---|---|---|
| Open app | Library | Library |
| Open piece | 1 click (or Continue) | 1 click |
| Choose instrument | 1 click, every time | 0: remembered / auto-detected (M1.3) |
| Press Start | 1 click | 0: the first correct note starts |
| Practise bar 9 | 1 click on bar 9 | 1 click |
| Loop bars 9–12 | Click + Shift-click, or L | done |
| Try again | **R** | done |
| Back to library | 1 click; the attempt is saved | done |
| See where you struggled | Results after the run (coach cards) | + the Progress page (M2.10) |

### 15.3 Core flows and click budgets

**Flow A — First launch ever (budget: 3 clicks to playing)**
1. Library opens with bundled pieces and a first-run card.
2. If no keyboard: onboarding strip (connect → it's detected automatically →
   ready). If one keyboard: header shows it, nothing to do.
3. Click "Minuet in G" → the score opens, cursor on the first note, a quiet
   line: *"Play the first note to begin."*
4. Play. That's it.

**Flow B — Daily practice (budget: 1 click)**
- Library shows a **"Continue"** card at the top: last piece, the bar you
  stopped at, and its weakest bars. One click → practice screen, same loop
  and hands as last time.
- Option (decide in M3): a setting "Open the last piece on launch" (default off).

**Flow C — Fix a hard bar (budget: 2 clicks)**
- On the score, click the darkest bar → bar card → **Loop this bar**.
- Or toolbar → **Practise weak bars** (1 click, Hogwash chooses).

**Flow D — Import a piece (budget: 2 clicks, or a drag)**
- Library → **Add a piece** → file dialog → it opens straight into practice.
- Or drag the file onto the window.
- Duplicate → *"Already in your library"* and opens it (exists today).

**Flow E — Keyboard unplugged mid-practice (budget: 0 clicks)**
- Banner: *"FP-10 disconnected. Plug it back in to keep going — your
  progress is kept."* Reconnect → banner fades, cursor still where it was.
  (The grace-period logic exists; the copy and the "keep going" behaviour
  are the work.)

**Flow F — End of a session (budget: 0–1 clicks)**
- Leaving or stopping always saves. The summary appears inline above the
  score; it never blocks. **Again** is the default (Enter).

### 15.4 Practice screen states

The practice screen is a small state machine. Every state has one obvious
next action.

| State | What you see | Obvious next action |
|---|---|---|
| **Ready** | Score, cursor at the start point; the instrument picker until M1.3 | Start, or play the first note (planned) |
| **Playing** | Cursor; correct notes green, wrong notes as a red head where they'd be written (today they fade after 1.5 s; from M2.11 they stay for the attempt) | Keep playing |
| **Looping** | Loop band behind the bars, "2 clean in a row" in the status line | Keep playing; Esc or L to stop |
| **Loop done** | "3 clean passes · bars 9–12", and "Join it back up · loop bars 8–13" | Widen the loop, or stop |
| **Device lost** | Status line with a 60 s countdown; cursor frozen | Plug the keyboard back in |
| **Ended** | Results replace the score (Monkeytype-style); auto-finish 1.5 s after the last note | Enter (first card's drill), R (again), Esc (the score) |

### 15.5 Keyboard shortcuts

Shortcuts must never fire while typing in a text field (rename, search).

Built:

| Key | Action |
|---|---|
| Click a bar / Shift-click | Start there / loop to it |
| L | Loop on / off |
| R | Practise again (from the loop start, or the start point) |
| Esc | Stop looping; on Results, show the score and back |
| Enter | On Results: the first card's button (or practise again) |
| Ctrl+wheel | Zoom |
| W | Weak-bar tint on/off (removed with M2.10) |

Planned (M3.6 with a `?` overlay): Space start/stop, ← / → move a bar, Home bar 1,
H hands, N note names, + / − / 0 zoom, Ctrl+O add a piece, Ctrl+F search.

### 15.6 Small conveniences worth having (all v1.0 unless marked)

- The window remembers its size and position.
- The last-used zoom per piece (M1.9).
- Hover on any bar shows its number (helpful in long pieces).
- Bar numbers on every system start (Verovio option), not just every 5.
- Clicking the device indicator opens MIDI settings directly.
- Rename a piece by double-clicking its title (library).
- Undo for delete: a 6-second "Deleted · Undo" toast instead of a
  confirmation dialog.
- "Copy summary" button in the end panel (plain text, for sending to a teacher).
- **Next:** export a session report as PDF.

---

## 16. Visual design system

### 16.1 Direction

- **A quiet music room, not a game.** Paper, ink, one accent. The score is
  the hero; chrome recedes while you play.
- **Calm over clever.** No gradients, no glassmorphism, no glow. Hairline
  borders and soft shadows only.
- **Typography carries the personality:** a serif for titles (like a printed
  edition), a clean sans for UI, monospace for numbers and bar counts.

### 16.2 Colour tokens (extend what `index.css` already defines)

| Token | Light | Dark | Use |
|---|---|---|---|
| `paper` | `#eef0f3` | `#16171b` | App background |
| `paper-raised` | `#ffffff` | `#1f2026` | Panels, cards |
| `score-paper` | `#fbf8f1` (warm cream) | `#1b1c21` | The score surface |
| `ink` | `#191a1e` | `#e8e9ee` | Text, notation |
| `ink-muted` | `#6b6d76` | `#9a9caa` | Secondary text |
| `rule` / `rule-strong` | `#dde1e7` / `#c7ccd6` | `#2c2e36` / `#3a3d47` | Hairlines |
| `accent` | `#16a34a` | `#22c55e` | Focus, primary action, selected |
| `correct` | `#2f6f4e` | `#4ade80` | Correct-note marks |
| `wrong` | `#dc2626` | `#f87171` | Wrong-note marks (today they fade after 1.5 s; from M2.11 they stay for the attempt, with a ring) |
| `cursor` | accent at 70% | accent at 70% | Cursor line |
| `position-band` | accent at 8% | accent at 12% | Current position band |
| `loop-band` | `#2563eb` at 8% | at 14% | Selected loop range |
| `heat-1…5` | `#fef3c7` → `#f59e0b` → `#b45309` | same hues, lower lightness | Weak bars over time (Progress page, M2.10); `--color-pause` amber is also the pause mark on Results |

Rules:
- **Never encode meaning in colour alone.** Wrong notes also get an ✕ shape;
  weak-bar levels on the Progress page are also drawn as bar heights and listed as facts; correct notes are a filled
  dot, wrong an outlined cross.
- Heat colours are a **single amber hue**, so the score never looks covered
  in failure-red.
- Contrast: body text ≥ 4.5:1, large text and icons ≥ 3:1, in both themes.

### 16.3 Type

| Role | Font | Size token |
|---|---|---|
| Display (library heading) | Serif | `display` 4.25 rem |
| Heading | Serif | `heading` 1.875 rem |
| Piece title (practice header) | Serif italic | `title` 1.25 rem |
| Card title | Serif italic | 1.125 rem |
| UI body | Sans | 0.875–1 rem |
| Labels, small caps | Sans, uppercase, +0.06em tracking | 0.6875 rem |
| Numbers, bar counts, timers | Mono, tabular figures | as context |

Only the scale in `index.css` is allowed. No one-off sizes.

### 16.4 Spacing, shape, elevation

- Spacing on a 4 px grid: 4, 8, 12, 16, 24, 32, 48.
- Radius: 8 px controls, 12 px cards, 16 px panels, full for pills.
- Elevation: two levels only (raised panel, popover). Shadows soft and low.

### 16.5 Motion

- Durations: 120 ms (hover, press, screens), 160 ms (panels). Screens cross-fade: the old
  one fades out while the new one fades in.
- One easing: ease-out. No spring bounce, no stagger longer than 50 ms.
- Score never animates position on reflow (it's hidden until fitted, as now).
- Reduced motion: all transitions become fades or instant.

### 16.6 Icons

- One icon set, 16/20 px, 1.5–2 px stroke, inline SVG (no icon dependency
  unless it's tiny and tree-shakeable, e.g. Lucide).
- Needed: gear, library, loop, hands, clock (history), weak bars
  (a simple target), keyboard, play/stop, restart, zoom ±,
  question mark.

---

## 17. Screen-by-screen specifications

### 17.1 Library (as built, 25 Oct)

```
Hogwash

CONTINUE
┌────────────────────────────────────────────────────────────┐
│ CHRISTIAN PETZOLD                                          │
│ Minuet in G Major                                [Continue]│
│ Practised today · weakest bars 3, 7                        │
└────────────────────────────────────────────────────────────┘
YOUR PIECES                                     [+ Add a piece]
  cards: COMPOSER · Title · 32 bars · 274 notes · "Last practised 2 days ago · weakest bar 9"
STARTER · BEGINNER — single lines and first two-hand pieces
STARTER · INTERMEDIATE — around Grades 1–4
```

- **Continue** appears once anything has been practised: the piece practised last.
- **Your pieces:** imports, most recently practised first. Empty: *"Pieces you
  add appear here. Hogwash reads MusicXML — export it from MuseScore Studio or
  download from OpenScore."*
- **Cards:** composer in small caps, title in serif italic, size in mono, footer
  "Last practised … · weakest bar(s) …" or "Not practised yet". Rename and Delete
  (imports only) appear on hover or focus.
- **Planned:**
  - a visible "Progress" link on every card and next to Continue (M2.10)
  - cream "sheet-music cover" paper and a one-line musical hook (M3.8)
  - search (M3.9)
  - a device indicator (M1.3)
- **No grades as rewards.** Levels are descriptions only.

### 17.2 Practice (as built, 25 Oct)

- **Header:**
  - ← Library, title + composer
  - Loop, Restart and Stop/Finish while playing
  - the settings gear
  - a status line that appears only when it has something to say (keyboard lost,
    loop counts, "Join it back up")
- **The score:** full width; the cursor; correct and wrong marks; the loop band
  behind looped bars; the amber weak-bar tint when idle (removed with M2.10).
- **After a run, Results replace the score:**
  - a top line ("Bars 1–8 · 6 s") with accuracy, tempo and steadiness in small mono
  - up to 3 coach cards (each: a cut-out of the bars, what happened, why, how to fix
    it, one button)
  - "Went well" chips
  - Details: big numbers, a per-bar chart, small numbers
  - Practise again, Show on the score, Library
- **The coach's cards** (rules in `crates/hogwash-analysis/src/coach.rs`, which
  owns the thresholds; at most 3 cards, judged per pass so one fumble isn't a card):
  - **Going into bar N:** the pause sits just before the bar's first note.
  - **Jump in bar N:** a hesitation or wrong key after a leap of more than an
    octave.
  - **Fast notes / broken chord:** slips or pauses in notes of an eighth or
    faster.
  - **Wrong notes in bar N:** a misread versus "landed a key off".
  - **Rushed the long notes:** half notes or longer cut short.
  - **Bars N–M slow you down:** well under your usual tempo.
  - **You keep stopping at bar N:** stopped or restarted there twice or more.
  - **This piece is a big step:** trouble in most bars and low accuracy.
  - **Clean run** or **Play a few bars more** when nothing qualifies.
  - Pauses are judged against your own pace and the written rhythm, and hunting
    for one note counts as at most 3 wrong notes (`bars.rs`).
- **Planned:**
  - a "Progress" button in the header (M2.10)
  - the one-row toolbar (M3.1)
  - focus mode while playing (M3.2)
  - the position band (M3.4)

### 17.3 Panels (not screens)

| Panel | Opens from | Contents |
|---|---|---|
| Settings | ⚙ | Appearance (note names, show geometry), Sound (MIDI input and output / Thru), View & export (zoom, export MusicXML), About (version, licence, credits) |
| Shortcuts (planned, M3.6) | ? | The table in 15.5 |

History and the bar card are no longer panels. They live on the Progress page (M2.10, planned): a session × bar grid, "Work on next", and bar detail on a column click.

### 17.4 Onboarding and first run

- **No device connected** (library and practice):
  `① Connect your keyboard (USB) → ② Hogwash finds it automatically → ③ Open a piece and play.`
  plus *"No keyboard? Try it with your computer keyboard."* (D7)
- **First time on the practice screen:** three one-line hints, shown once,
  each dismissible: *"Play the first note to begin."* · *"Click any bar to
  start there."* · *"Shift-click a second bar to loop between them."*
- No tour, no modal walkthrough, no lessons.

### 17.5 Results (as built) and the Progress page (planned)

Results are described in 17.2.

The **Progress page** (M2.10) will have:
- a header: composer, title, "N sessions · total time · last practised …"
- Work on next: up to 3 passages, each with "⟲ Practise bars …" starting one bar
  before the trouble
- the grid: bars × sessions, newest on top, a "Now" row of bar heights, amber cells
  by absolute level
- bar detail on a column click (now vs the earliest sessions)

It is reached from the library card, Continue, Results and the practice screen,
and never opens by itself.

**What history holds** (all of it recomputed from the raw notes, D5):
- **Per session:** start time, attempts, how each ended, bars covered, time
  played.
- **Per attempt:** every note with its time, key and velocity; start and end;
  loop or not; accuracy, tempo and steadiness; the coach's findings.
- **Per bar:** passes, pause (total, going into the bar, longest), breaks,
  wrong notes, stops, tempo and time played.
- **Not yet known:** the hand a note belongs to, and key signatures in the steps.

---

## 18. Words: microcopy and feedback language

### 18.1 Voice

- Plain, calm, specific. Like a good teacher's pencil note in the margin.
- Second person, present tense. Short sentences.
- Say *where* and *what*, never *how good*.
- British/US spelling: pick one (currently British: "practise" as verb,
  "practice" as noun). Keep it consistent.

### 18.2 Do / don't

| Situation | Don't | Do |
|---|---|---|
| Wrong note | "Oops! Wrong note!" | (no text; the mark on the staff says it) |
| End of attempt | "Great job! 87% accuracy!" | "Bars 1–18 played · 3 bars to work on" |
| Weak bar | "You're bad at bar 9" | "Bar 9 · average pause 4.2 s · 2 wrong notes per pass" |
| Improvement | "Amazing progress!!" | "Bar 9: average pause 14.1 s → 3.2 s over 6 sessions" |
| No device | "ERROR: No MIDI device found" | "No keyboard connected. Plug it in — Hogwash will find it." |
| Bad file | "Parse error at line 412" | "This file couldn't be read as sheet music. If it came from MuseScore, try exporting it again as MusicXML." |
| Loop done | "🎉 Level up!" | "3 clean passes · bars 9–12" |

### 18.3 Glossary (use these words, consistently)

- **piece** (not song, not score, in UI), **bar** (not measure, in UI),
  **attempt** (one play-through), **session** (one sitting with a piece),
  **loop**, **clean pass**, **weak bars**, **pause** (user-facing word for
  stall), **keyboard** (the MIDI instrument), **library**.

---

## 19. Errors, edge cases and empty states

### 19.1 Error rules

1. Every error says **what happened** and **what to do next**, in plain words.
2. Internals (`UnhandledPromise`, stack traces, paths) never reach the main
   UI. Put them behind a "Details" disclosure, copyable for bug reports.
3. A failure in one feature never takes down the screen. (Today, any thrown
   command error replaces the whole window with the Unavailable screen —
   keep that only for "Hogwash can't start at all.")
4. Errors that fix themselves (device back) disappear on their own.

### 19.2 Edge cases to handle explicitly

| Case | Behaviour |
|---|---|
| Two MIDI keyboards connected | Use the remembered one; picker in settings |
| Keyboard reconnects with a different port name | Match by name prefix; ask only if ambiguous |
| Sustain pedal (CC64) | Ignored for judging; never counts as a wrong note |
| Notes held from the previous position | Not re-judged (ties logic already covers written ties) |
| Piece with > 2 staves or several parts | Refuse clearly on import for v1.0: "Hogwash handles one piano part (two staves)." |
| Piece with a pickup (anacrusis) bar | Bar numbering must match the printed score (test with a PD Für Elise) |
| Very long piece (300+ bars) | Engraving and history stay within budgets (section 21) |
| Mid-bar clef change | Known limit (`docs/PRODUCT.md`, Known limits) |
| Played an octave off everywhere | Octave-shift hint (M3.14) |
| Piece needs keys the keyboard lacks | Warn on open (M3.13) |
| Session file corrupted on disk | Skip it, keep the rest, note it in Details |
| Disk full / no write permission | "Couldn't save this session (disk full?). You can keep playing." |
| App closed mid-attempt | Attempt saved on window close (extend M1.2) |
| Computer sleeps mid-attempt | On wake, treat like a device reconnect |

### 19.3 Empty states (all need copy and one action)

- No pieces imported yet (library section)
- Piece never practised (Progress page: one line and a Practise button)
- Not enough data for a bar (bar card: "Play this bar twice to see stats.")
- No keyboard connected (header + practice status line)
- Progress page with one session: the grid with one row, no trend text

---

## 20. Accessibility

Not a polish item — a correctness item. Minimum bar for v1.0:

- Every control reachable by keyboard, in a sensible order; visible focus
  ring (accent, 2 px, offset 2 px — already the pattern).
- Every icon button has an `aria-label`; toggles use `aria-pressed`;
  the hands selector is a radio group (the instrument picker already is).
- Live region (`role="status"`) for: attempt started, loop pass counter,
  loop done, device lost/restored. Throttled so it doesn't chatter.
- Colour never the only signal (section 16.2).
- Text scales with the OS setting; layout survives 150% UI zoom.
- Reduced motion honoured everywhere.
- Hit targets ≥ 32 px for toolbar controls.

---

## 21. Performance budgets

Targets to **measure** (measured numbers are dated below), not claims.

| What | Budget | How to measure |
|---|---|---|
| Key press → mark on screen | < 30 ms on your machine | Log MIDI timestamp vs. paint (devtools performance), 50 presses |
| Opening the Prelude (engrave + show) | < 800 ms | Timer around `open_piece` + first paint |
| Reflow after zoom/resize | < 300 ms | Timer around `engrave_piece` |
| Loading 50 sessions + weak bars | < 100 ms | Unit benchmark in `hogwash-analysis` |
| Cold start to library | < 1.5 s | Stopwatch on the AppImage |
| Idle CPU | ~0% | `top` with the app open, no input |
| AppImage size | < 60 MB | `ls -lh` |

If a budget is blown, fix it before adding features in that area.

### 21.1 Measured, 25 Oct 2026 (this machine; median of 5, click → painted)

Measured with a WebDriver script and the virtual keyboard, on a copy of real data.
"Debug" is debug Rust with the built frontend. `just run` is debug Rust plus the
Vite dev server.

| What | Release | Debug | `just run` | Budget |
|---|---|---|---|---|
| Cold start → library | 1.2–1.5 s | 1.8 s | 1.6 s | < 1.5 s (met in release) |
| Open the Prelude (engrave + show) | 430 ms | 508 ms | 585 ms | < 800 ms ✓ |
| Key press → mark painted | 23 ms (max 34) | — | 24 ms (max 36) | < 30 ms ✓ (median) |
| Zoom step → re-engraved (incl. 250 ms debounce) | 486 ms | 585 ms | 606 ms | < 300 ms for the reflow itself; not separated |
| Start → playing | 12 ms | 12 ms | 12 ms | — |
| Stop → Results | 69 ms | 68 ms | 72 ms | — |
| ← Library → library (incl. ~0.4 s of fades) | 294 ms | 292 ms | 300 ms | — |
| `library` command, 10 pieces, 6 sessions | 3 ms | 46 ms | 46 ms | — |
| 50 Minuet sessions replayed | < 100 ms (test) | ~104 ms | — | < 100 ms ✓ (release) |
| AppImage size | 115 MB | — | — | **< 60 MB ✗ over budget** |

26 Oct, after the build settings (`Cargo.toml` profiles) and the 120 ms cross-fade,
measured in-process (median of 5, the same data copy):

| What | Debug before | Debug after | Release |
|---|---|---|---|
| Engrave the Prelude | 155 ms | 97 ms | 91 ms |
| Startup (engrave 6 pieces, read history) | 991 ms | 709 ms | 660 ms |
| Library summaries, 10 pieces | 2.8 ms | 1.4 ms | 0.5 ms |
| Screen switch fades | 400 ms | 120 ms | 120 ms |

The `library` row above (46 ms in debug) is mostly the trip across the webview
boundary, not the work. Release binary: 9.2 MB stripped (was 13.9 MB).

Not measured yet: idle CPU.
