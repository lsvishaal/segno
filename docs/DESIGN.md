# Hogwash — design

How Hogwash looks, behaves and speaks: the UX, the visual system, each screen, the words, and the budgets it must meet.

---

## 15. UX blueprint: making it frictionless

### 15.1 The friction principle

> **From launching Hogwash to hearing your first note judged: at most
> 2 clicks. From finishing a pass to starting the next: 0 clicks.**

Every flow below has a click budget. A change that adds a click needs a reason
written in §32.

### 15.2 Today's friction audit (6 Oct)

Launch → play the Minuet, as the code works now:

| Step | Today | Target |
|---|---|---|
| Open app | Library | Library (or the last piece, see 15.3) |
| Open piece | 1 click | 1 click |
| Choose instrument | 1 click, every time | 0 (remembered / auto) |
| Press Start | 1 click | 0 (first correct note starts) |
| Practise bar 9 | Play bars 1–8 first | 1 click on bar 9 |
| Loop bars 9–12 | Not possible | Click + Shift-click |
| Try again | 1 click "Again" in summary | 0 (loop wraps) or **R** |
| Back to library | 1 click (attempt keeps running) | 1 click or Esc (attempt saved) |
| See where you struggled | Read a list of lines | Heatmap on the score |

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
| **Ready** | Score, heatmap on, cursor at start/last bar, "Play the first note to begin" | Play a note |
| **Playing** | Heatmap off, toolbar dimmed, cursor + position band, live marks | Keep playing |
| **Looping** | Range band, "2 clean in a row" counter in the toolbar | Keep playing; Esc to stop looping |
| **Loop done** | Short confirmation: "3 clean passes · bars 9–12" | Next weak bar / keep looping / stop |
| **Device lost** | Banner with countdown; cursor frozen | Plug the keyboard back in |
| **Ended** | Inline summary above the score, heatmap back on | Again (Enter) |

### 15.5 Keyboard shortcuts

Shortcuts must never fire while typing in a text field (rename, search).

| Key | Action |
|---|---|
| Space | Start / stop the attempt |
| R | Restart from loop start (or bar 1) |
| L | Loop the current bar; with a selection, loop the selection; again to stop |
| ← / → | Move the cursor one bar back / forward (when not mid-chord) |
| Home | Go to bar 1 |
| H | Cycle hands: Both → Right → Left |
| W | Toggle weak-bar heatmap |
| Y | History panel |
| N | Toggle note names |
| + / − / 0 | Zoom in / out / reset |
| Esc | Clear selection → stop loop → close panel → back to library (in that order) |
| ? | Shortcut overlay |
| Ctrl+O | Add a piece |
| Ctrl+F | Search the library |

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
| `wrong` | `#dc2626` | `#f87171` | Wrong-note marks (fade after 1.5 s) |
| `cursor` | accent at 70% | accent at 70% | Cursor line |
| `position-band` | accent at 8% | accent at 12% | Current position band |
| `loop-band` | `#2563eb` at 8% | at 14% | Selected loop range |
| `heat-1…5` | `#fef3c7` → `#f59e0b` → `#b45309` | same hues, lower lightness | Weak-bar heatmap |

Rules:
- **Never encode meaning in colour alone.** Wrong notes also get an ✕ shape;
  heatmap bars also get a number in the bar card; correct notes are a filled
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

- Durations: 120 ms (hover, press), 160 ms (panels), 200 ms (screens).
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

### 17.1 Library

```
┌───────────────────────────────────────────────────────────────────────┐
│ Hogwash                                        ● Roland FP-10    ⚙    │
├───────────────────────────────────────────────────────────────────────┤
│  CONTINUE                                                             │
│  ┌──────────────────────────────────────────────────┐                 │
│  │ Minuet in G · bar 9 · weakest: 9, 14, 21  [Continue]│              │
│  └──────────────────────────────────────────────────┘                 │
│                                                                       │
│  YOUR PIECES                                   [Search] [+ Add a piece]│
│  ┌────────┐ ┌────────┐ ┌────────┐                                     │
│  │ COMPOSER│ │        │ │        │   cream "sheet-music cover" cards  │
│  │ Title   │ │        │ │        │                                    │
│  │ hook    │ │        │ │        │                                    │
│  │ last ·  │ │        │ │        │                                    │
│  └────────┘ └────────┘ └────────┘                                     │
│                                                                       │
│  BUNDLED · BEGINNER — single lines and first two-hand pieces          │
│  ...                                                                  │
│  BUNDLED · INTERMEDIATE — around Grades 1–4                           │
│  ...                                                                  │
└───────────────────────────────────────────────────────────────────────┘
```

- **Card content:** composer (small caps), title (serif italic), one
  musical hook line (key · time · one distinguishing feature, written into
  `PROVENANCE.md` for bundled pieces; for imports: key · time · bars),
  footer: "Last practised 2 days ago · weakest bar 9" or "Not practised yet."
- **Card actions:** click opens; a ⋯ menu for Rename, Export MusicXML,
  Delete (imports only).
- **Empty "Your pieces":** *"Pieces you add appear here. Hogwash reads
  MusicXML — export it from MuseScore Studio or download from OpenScore."*
  plus the Add button.
- **No grades as rewards.** Levels are descriptions only.

### 17.2 Practice

```
┌──────────────────────────────────────────────────────────────────────────┐
│ ← Library   Minuet in G · Petzold                  ● FP-10   ⏱  ⚙   ?    │
├──────────────────────────────────────────────────────────────────────────┤
│ Bar 9 / 32 │ Both ▾ │ ⟲ Loop 9–12 · 2 clean │ ◎ Weak bars │ −  100%  + │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│                    (the score, full width, cream paper)                  │
│                                                                          │
│   ░░ position band   ▒▒ loop band   heat tint on bars when idle          │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│ (only when needed) Play the first note to begin · or click a bar         │
└──────────────────────────────────────────────────────────────────────────┘
```

- **Header:** back, title + composer, device indicator, history, settings,
  help. Collapses while playing (M3.2).
- **Toolbar (left → right):** position "Bar 9 / 32" · hands · loop (state +
  clean counter) · weak bars (toggle + "Practise weak bars" in its menu) ·
  zoom. After v1.0: Wait/Tempo toggle and tempo stepper, Listen.
- **Status line** at the bottom, only when it has something to say.
- **Summary** appears as an inline panel above the score, never a modal.

### 17.3 Panels (not screens)

| Panel | Opens from | Contents |
|---|---|---|
| Settings | ⚙ / device indicator | Keyboard (input, output/thru, range, detect) · Display (theme, note names, bar numbers) · About (version, licence, credits). Developer: show geometry. |
| History | ⏱ / Y | Session list, practice log (D6), per-session heatmap preview |
| Bar card | Click a bar when idle | Bar stats, trend, Loop this bar, Start here |
| Shortcuts | ? | The table from §15.5 |

### 17.4 Onboarding and first run

- **No device connected** (library and practice):
  `① Connect your keyboard (USB) → ② Hogwash finds it automatically → ③ Open a piece and play.`
  plus *"No keyboard? Try it with your computer keyboard."* (D7)
- **First time on the practice screen:** three one-line hints, shown once,
  each dismissible: *"Play the first note to begin."* · *"Click any bar to
  start there."* · *"Shift-click a second bar to loop between them."*
- No tour, no modal walkthrough, no lessons.

### 17.5 The end-of-attempt panel

```
Bars 1–18 played · 3 bars to work on · 6 min 40 s
[ Bar 9 · 3 wrong · pause 6.2 s ] [ Bar 14 · 2 wrong ] [ Bar 17 · pause 4.0 s ]
Bars 19–32 not reached.
                                   [Again ⏎]  [Practise these bars]  [Library]
▸ Show every note
```

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
| Very long piece (300+ bars) | Engraving and history stay within budgets (§21) |
| Mid-bar clef change | Known limit (§27) |
| Played an octave off everywhere | Octave-shift hint (M3.14) |
| Piece needs keys the keyboard lacks | Warn on open (M3.13) |
| Session file corrupted on disk | Skip it, keep the rest, note it in Details |
| Disk full / no write permission | "Couldn't save this session (disk full?). You can keep playing." |
| App closed mid-attempt | Attempt saved on window close (extend M1.2) |
| Computer sleeps mid-attempt | On wake, treat like a device reconnect |

### 19.3 Empty states (all need copy and one action)

- No pieces imported yet (library section)
- Piece never practised (heatmap toggle, history panel)
- Not enough data for a bar (bar card: "Play this bar twice to see stats.")
- No keyboard connected (header + practice status line)
- History panel with one session ("Practise again to see changes over time.")

---

## 20. Accessibility

Not a polish item — a correctness item. Minimum bar for v1.0:

- Every control reachable by keyboard, in a sensible order; visible focus
  ring (accent, 2 px, offset 2 px — already the pattern).
- Every icon button has an `aria-label`; toggles use `aria-pressed`;
  the hands selector is a radio group (the instrument picker already is).
- Live region (`role="status"`) for: attempt started, loop pass counter,
  loop done, device lost/restored. Throttled so it doesn't chatter.
- Colour never the only signal (§16.2).
- Text scales with the OS setting; layout survives 150% UI zoom.
- Reduced motion honoured everywhere.
- Hit targets ≥ 32 px for toolbar controls.

---

## 21. Performance budgets

Targets to **measure** (write the numbers in §32 when measured), not claims.

| What | Budget | How to measure |
|---|---|---|
| Key press → mark on screen | < 30 ms on your machine | Log MIDI timestamp vs. paint (devtools performance), 50 presses |
| Opening the Prelude (engrave + show) | < 800 ms | Timer around `open_piece` + first paint |
| Reflow after zoom/resize | < 300 ms | Timer around `engrave_piece` |
| Loading 50 sessions + heatmap | < 100 ms | Unit benchmark in `hogwash-analysis` |
| Cold start to library | < 1.5 s | Stopwatch on the AppImage |
| Idle CPU | ~0% | `top` with the app open, no input |
| AppImage size | < 60 MB | `ls -lh` |

If a budget is blown, fix it before adding features in that area.
