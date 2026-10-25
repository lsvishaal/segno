# Hogwash — product

Who Hogwash is for, the rules it keeps and why, and what it will never be.

---

## 2. Who Hogwash is for

Being specific about who it's for is what stops the app from becoming
everything. When a feature idea comes up, ask: *which of these people needs
it, today?*

### Primary: the self-taught adult with a digital piano (you)

- Owns a MIDI-capable digital piano (Roland FP-10 class), plays at home.
- Can sort of read notation (landmark notes) and wants to get properly fluent.
- Learns specific pieces they care about, not graded exercises.
- Practises 15–45 minutes, irregularly, usually in the evening.
- Frustrations: doesn't know *what* to practise; plays the whole piece
  start-to-finish every time; the hard bars stay hard; apps either teach
  key-pressing (Synthesia) or put everything behind a paywall.
- Uses Linux or is comfortable installing software.

### Secondary: the returning player

- Had lessons as a kid, stopped, came back as an adult.
- Reads reasonably well; rusty.
- Wants a structured way to bring old pieces back and find weak spots.
- Values a calm, serious tool over a game.

### Tertiary: the student between lessons

- Has a teacher once a week, practises alone in between.
- Benefits from "here are the 3 bars you struggled with this week," which
  they can show their teacher.
- Not a teacher-facing product in v1 (no assignments, no accounts).

### Not for (and that's fine)

- Complete beginners who can't find middle C: they need lessons and
  flashcards (MidiSlayer, Simply Piano, books).
- People who want falling notes: Synthesia and Neothesia exist.
- People without a MIDI keyboard: the computer-keyboard fallback is for
  trying the app, not for practising.
- Non-piano instruments.

---

## 3. Product rules (and why each exists)

These are settled. Reopen one only with new evidence, and log it in §32.

| Rule | Why |
|---|---|
| **Two screens: Library and Practice.** Settings, history and help are panels, not tabs. | MidiSlayer has eight top-level tabs and its identity got lost in them. A practice tool should open straight into practice. |
| **Pieces, not drills.** No flashcards, interval trainers, theory lessons, exams or games. | That's MidiSlayer's lane; they're far ahead and charge for it. Depth in one lane beats breadth in five. |
| **No gamification.** No XP, ranks, levels, streaks, points or badges. | It optimises for opening the app, not for getting better. Motivation should come from watching a bar improve. |
| **Factual feedback.** Location + measurement, never praise or judgment. | "Bar 9: stalled 4 s, 2 wrong notes" is useful. "Great job!" is noise, and "You struggled" stings. |
| **The cursor waits.** Wait mode is the core experience. | It's what makes reading practice possible at your own speed. Tempo comes later, as a graduation step. |
| **Local-first.** No accounts, no network, no telemetry. | Your practice history is personal. It also means nothing to host, nothing to break. |
| **Everything is free.** No locked features, ever. | MidiSlayer charges $50 to play a public-domain Bach prelude. Being the free version is the point. |
| **Users bring their own music.** Hogwash bundles only lawfully redistributable scores. | Shipping copyrighted arrangements gets a repo taken down. |
| **Integration over invention.** Verovio engraves, midir talks MIDI, MusicXML is the source of truth. | Every hour spent rebuilding a solved problem is an hour not spent on weak bars. |
| **Deterministic analysis.** Same performance + same score = byte-identical results. | It's what makes the engine testable without a piano, and it's already enforced in CI. Don't lose it. |

---

## 4. The landscape: competitors and what we take from each

### MidiSlayer (closest competitor) — studied hands-on, 6 Oct

**What it is:** a desktop sight-reading trainer by Monoglyph Ltd (UK).
$50 one-time, proprietary. Linux AppImage/.deb/.rpm, Windows, macOS.
69 public-domain pieces (4 playable free), MusicXML import, nine drill modes,
seven reading lessons, a daily game, ranks/XP/streaks, analytics.

**Its centre is drills, not pieces.** The home screen's main button is
"Begin: Note Flashcards." Practising a piece is one tab of eight. Its
analytics (note weakness map, slowest notes, exam score, first-try accuracy)
are all about reading single notes, not about bars of a piece.

**Adopt (take the idea, build it our way):**
- One-row practice toolbar: title + position counter ("1 / 561"), hands
  selector, Wait/Tempo toggle, tempo stepper, Listen, Loop, zoom, library.
- Current position highlighted as a soft vertical band through both staves.
- Library cards styled like sheet-music covers: cream paper, serif italic
  title, composer in small caps, a one-line musical hook
  ("G major · 3/4 · two hands").
- Pieces grouped by level ("Beginner — first tunes", "Intermediate — two
  hands, Grades 1–4").
- Device indicator always visible, top right: green dot + device name.
- Computer-keyboard fallback (A–K plays C4–C5) with an on-screen keyboard.
- Keyboard range: "From A0 to C8 · 88 notes", with a Detect button.
- Empty states that tell you what to do next ("Complete a session to see
  your weak spots").
- Onboarding strip when no device: 1 Connect your keyboard → 2 Select it →
  3 You're ready.
- Engraving with fingering numbers, tempo marking, expression text visible.

**Adapt (good idea, wrong framing for us):**
- *Consistency calendar* → a factual practice log per piece (dates and
  minutes), without streak counts. See decision D6.
- *Focus note / weak spots* → focus **bar**, computed from real pieces.
- *Graded library* → grade labels as plain description, not a progression
  system you "unlock."
- *Tip of the day* → contextual one-line hints in empty states only.

**Avoid (what we saw go wrong):**
- Eight tabs; Progress and Analytics overlapping.
- "PRO" badges on 8 of 9 cards: the free tier feels like a shop window.
- A raw `UnhandledPromise: Failed to start the audio device` error toast on
  every screen. Errors must never leak internals.
- Built-in software synth that fails on Linux (PipeWire/ALSA): we send MIDI
  out to the user's piano instead.
- Ranks ("Novice"), points per minute, XP levels, practice streaks.

### Other tools

| Tool | What it is | Take | Leave |
|---|---|---|---|
| **midipiano.app** | Free browser app; marks notes on the score as you play; loop, slow tempo, hands separately, wait mode; MusicXML/MXL/MEI import | Proof that free + sheet music + wait mode is wanted | Browser-only, no history across sessions |
| **Synthesia** | Paid, falling notes + sheet view from MIDI; last release Dec 2022 | Loop and hands-separate conventions users already know | Falling notes as the primary view |
| **Neothesia** | Open-source Rust falling-notes visualiser | Nothing directly; clean-room anyway (GPL) | Its scoring code that nobody reads; per-frame MIDI polling |
| **Soundslice** | Paid web player + OMR scanner + editor | Proof that PDF scanning is a hard, funded, multi-year problem | Trying to compete on scanning |
| **playsheet** | Free web; sheet + MIDI; rhythm-game scoring | Session review idea | Proprietary service, data not local, game scoring |
| **AnyScore** | iPad/iPhone; wait-for-me; weakest measures across pieces | **Validation of the weak-bars idea** | Apple-only, closed |
| **MuseScore Studio** | Free notation editor; MIDI import; MusicXML export | Where users get MusicXML; its MIDI importer via webmscore (post-v1) | — |

### Lessons from projects that walked this road

- **MuseScore** (Werner Schweer, 2002): built for himself, nearly lost
  interest after v1 worked, paused, came back, and outlasted Finale.
  Its co-founder's lesson: they *couldn't win on price*, only on product
  quality. **For us: "free" gets people to try it; "it actually tells me
  which bar to practise" is what keeps them.**
- **Lichess** (Thibault Duplessis, 2010): a throwaway project kept alive by
  one stranger who used it and complained. **For us: motivation comes from
  the first real user, not from more planning.** Get one other pianist using
  Hogwash as early as M4.

---

## 23. Content and licensing

### 23.1 Rules

1. A public-domain **composition** is not a public-domain **file**. Every
   bundled file needs its own licence recorded in `PROVENANCE.md`.
2. Never commit a score you can't redistribute, even temporarily (git
   history is forever). Test scores live outside the repo.
3. Never download from MuseScore.com with tools that bypass its paywall.
4. Arrangements of copyrighted songs are never bundled. Users import their own.

### 23.2 Good sources

| Source | What | Licence |
|---|---|---|
| Open Well-Tempered Clavier (MuseScore + Kimiko Ishizaka) | Bach WTC Book I | Public domain |
| Open Goldberg Variations | Bach Goldberg | Public domain |
| OpenScore (MuseScore) | Lieder, string quartets, some piano | CC0 |
| Mutopia Project | Classical, typeset in LilyPond (MusicXML via conversion) | PD / CC per piece |
| MuseScore.com with licence filter | User uploads marked Public Domain / CC0 / CC-BY | As marked (attribute CC-BY) |
| Hand-encoded by you | Simple pieces, exercises | Apache-2.0 (yours) |
| IMSLP | Mostly PDFs (not usable directly) | Varies |

### 23.3 Target bundled library for v1.0 (D12)

Each needs a sourced, licence-checked file before it's added.

**Beginner — single lines and first two-hand pieces**
- Lightly Row (have) · Ode to Joy (have) · A Little Chord Study (have)
- Für Elise, simplified opening (have, hand-arranged)
- Schumann — Melody, Op. 68 No. 1
- Schumann — Soldier's March, Op. 68 No. 2

**Intermediate — around Grades 1–4**
- Petzold — Minuet in G, BWV Anh. 114 (have)
- Petzold — Minuet in G minor, BWV Anh. 115
- Anon. — Musette in D, BWV Anh. 126
- C.P.E. Bach (attr.) — March in D, BWV Anh. 122
- Burgmüller — Arabesque, Op. 100 No. 2
- Clementi — Sonatina in C, Op. 36 No. 1 (first movement)

**Further — Grades 5+**
- Bach — Prelude in C, BWV 846 (have; swap to OpenWTC)
- Satie — Gymnopédie No. 1
- Chopin — Prelude in E minor, Op. 28 No. 4

Each row in `PROVENANCE.md` also gets the **musical hook line** used on the
card ("D major · 2/4 · the left hand drones").

---

## 27. Known limits (fix when one actually bites)

Carried over from the previous roadmap; each one is real but rare.

- **Upward-resolving trill** (F trilled with G, then G) advances the cursor
  on its first G; the rest of the trill counts as wrong. The ornament
  tolerance covers only the trilled step and the next one.
- **Tied-to notes and grace notes get no note-name label** (they aren't marks).
- **At a repeat**, the cursor slides back across the page and nothing shows
  which pass (1st/2nd time) you're on.
- **A clef change mid-bar** takes effect from the next bar in score geometry
  (no bundled piece has one).
- **Wrong-note mark's accidental** ignores accidentals earlier in the same bar.
- **Re-importing an edited file** creates a new piece (new fingerprint); the
  old history stays with the old version. Want: "replace file, keep history."
- **Closing the window mid-attempt** loses the attempt (leaving the practice
  screen now saves it; the window close does not).
- **A zip damaged inside an entry** (not cut short) could still reach Verovio
  and crash it; only the container's structure is checked.
- **A real Für Elise with a pickup bar** hasn't been verified; test bar
  numbering with a public-domain source (don't commit the test file).
- Promoted out of this list into milestones: the 517-line early stop (M1.4),
  the attempt that keeps running (M1.2), the unreachable-keys wait (M3.13),
  the octave-shift buttons (M3.14).

---

## 28. Never — and why

| Never | Why |
|---|---|
| Accounts, cloud sync, telemetry | Breaks local-first; adds servers to run and data to protect |
| XP, levels, ranks, streaks, points, badges | Optimises for opening the app, not for playing better |
| Accuracy percentages as the headline | One number hides *where*; bars are the unit of practice |
| Flashcards, theory drills, lessons, exams, games | MidiSlayer's lane; we win on depth with pieces |
| Falling notes as a mode | Synthesia/Neothesia's lane; Hogwash is about reading real notation |
| PDF / photo scanning (OMR) | A funded multi-year problem (Soundslice); point to Audiveris |
| A built-in software synth (reopened, D13) | Linux audio is where MidiSlayer broke; the user's piano has speakers |
| Locked or paid features | The point is that it's free |
| Bundling copyrighted arrangements | Legal risk to the whole repo |
| An in-app store or content marketplace | Not a business; not the product |
| AI-generated practice advice | Factual stats are explainable and deterministic; an LLM is neither (and needs a network) |

---

## 29. Mistakes to avoid

From this project's own history, and from what we saw in other projects.
Ordered by how likely you are to make each one.

1. **Polishing before the loop exists.** A beautiful screen around a missing
   core. M1 and M2 come before M3 for this reason.
2. **Copying a competitor's breadth.** MidiSlayer looks complete because it
   does many things. You win by doing one thing better.
3. **Researching when motivation dips.** You already researched the wrong
   thing once ("can I render notation?" instead of "does the content exist?").
   When stuck: practise with the app for 15 minutes and write annoyances.
4. **Rewriting the stack.** It already happened once (spec-first → Tauri).
   Rust core + Tauri + Verovio works. No framework changes before v1.0.
5. **Unpushed work.** Six weeks once lived only on one disk. Push daily.
6. **Committing files you can't redistribute.** Git history is forever.
7. **Letting this file become a spec.** Lists, sizes, "done when." No ADRs,
   no FR-numbers, no governance.
8. **Letting Claude Code redesign freely.** One screen per task; plan →
   approve → build; manual checklist at the end.
9. **Adding gamification to fix your own motivation.** Streaks help the
   builder feel good about metrics; they don't help the player.
10. **Starting MIDI import early because it's exciting.** It's post-v1.0
    and needs a licence decision. The weak-bars feature is the product.
11. **Moving dates instead of cutting features.** Cut from the bottom.
12. **Judging success by stars or downloads.** See §30.
13. **Building for imaginary users.** Every feature should trace back to
    `annoyances.md`, a real user's hesitation, or the one sentence.
14. **Big-bang changes.** Each task should leave the app working. If a task
    breaks the app for more than a day, it was too big.

---

## 30. Motivation and sustainability

### Success, defined in advance

Hogwash v1.0 is a success if all three are true, regardless of stars:
1. You practise with it **5+ days a week** for a month after launch.
2. **At least one stranger** opens an issue, sends a message, or says it helped.
3. You can explain **every design decision** in your own words.

### On low-motivation days

- **The minimum day:** practise piano with Hogwash for 15 minutes and add
  one line to `annoyances.md`. That counts as a project day.
- **The smallest task:** pick an S task from the active milestone. Finish it.
  Push. Stop.
- **Don't** research competitors, read about new frameworks, or redesign
  this file on low days.

### Doubts

- New doubts about the project go in a `## Doubts` section of
  `annoyances.md` with a date. They're reviewed at the end of each milestone,
  not acted on mid-milestone.

### Remember why

- MuseScore's founder nearly lost interest once his first version worked;
  he came back and it outlasted Finale.
- Lichess survived because one stranger used it and complained about bugs.
- MidiSlayer charges $50 to practise a public-domain Bach prelude. You're
  building the version anyone can use.
