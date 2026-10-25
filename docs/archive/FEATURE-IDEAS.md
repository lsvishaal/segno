> Archived 2026-10-25: historical, not maintained. See ROADMAP.md (Later, Parking lot).
>
> Triaged 2026-10-25. Built: 1 (dark score surface removed), 2 (M1.8), 4 (green
> notes; persistent marks are M2.11), 5 (M1.1, M1.4), 9 (Results + coach, M1.4).
> Scheduled: 3 (bars: M1.5 done; notes: Parking lot), 6 (M3.3), 7 (M3.17),
> 8 (M3.12), 10 (Next 2). Parking lot: 11, 12, 13.

# Feature ideas, sorted

Written 2026-10-18 from a list of requests, so none of them is forgotten.
Nothing here is scheduled yet except where a ROADMAP ID is shown. To promote
an item, move it into a milestone in `ROADMAP.md` **between milestones** (its
own rule 7), keep the number here, and tick it off.

Sizes follow the ROADMAP: **S** under half a day, **M** 1-2 days, **L** 3-5 days.

## Facts that shape these

- **Duration and tempo are not in the domain** (`hogwash-notation` leaves them
  out on purpose). But Hogwash already loads Verovio's `renderToTimemap`, and
  `libverovio.so` exports `renderToMIDI`. The score's note on/off times and
  BPM can come from Verovio, so Listen (10) and held notes (11) need no new
  domain model.
- **Each note's staff (its hand) is already read from the page**
  (`app/src/components/staffGeometry.ts`), which the keyboard (8) needs.
- **Listen is piano only.** Decided 2026-10-06, reconfirmed 2026-10-18: Listen
  sends MIDI out to the user's piano. No built-in synth. A built-in sound
  could be revisited after v1.0.
- **A tension to keep in mind:** Monkeytype shows scores (WPM). Hogwash has
  "no gamification: no scores". The report (9) can be just as visual if it
  shows facts (pauses, wrong notes, steadiness over time), not points.

## Tier 1: core to practising, small, do first (target v0.1)

1. **Remove the dark score surface** (S).
   Paper only. Touches the surface switch in `PracticeSettings.tsx`, the
   `data-surface="dark"` rules in `score.css`, the `surface` prop and its
   tests. A removal, not a redesign.

2. **Restart and play from the start** (S). ROADMAP **M1.8**.
   A Restart control (and R), plus Stop. Restart saves the interrupted attempt;
   an attempt with no notes is never saved. Same as the session-lifecycle issue
   (auto-start on the first note, 10 s pause, 10 min auto-restart).

3. **Click to seek** (M). ROADMAP **M1.5**, extended to notes.
   Click a **bar** and the cursor goes to its first note; click a **note** and
   it goes exactly there, like seeking in a video. One new `Cursor` seek method
   serves both, repeat-aware (a bar can appear twice in played order). A seek is
   not a pass and does not score.

4. **A green "played correctly" note** (S-M).
   When the right note is played, a green note appears at its place on the
   staff, with its name when note names are on. The counterpart of the red
   wrong-note mark (same placement code). The held-for-its-length part is item 11.

5. **Feedback foundation** (M). ROADMAP **M1.1** and **M1.4**.
   Timestamps and velocity on every played note, and a summary that is readable
   (today an early stop prints one line per unplayed note). Prerequisite for 9.

## Tier 2: polish and the differentiator (target v1.0)

6. **Smooth scrolling** (M). ROADMAP **M3.3**.
   Ease the auto-scroll with a small `requestAnimationFrame` tween, about 20
   lines, no library, no work when idle, reduced-motion respected. Wheel scrolling
   stays native, so no Lenis-style hijacking. Measure before adding more.

7. **Cleaner settings panel** (M). ROADMAP **M3** (screen spec in section 17).
   Minimal, monochrome, small infographics instead of text-only rows.

8. **On-screen keyboard** (L). ROADMAP **D7** (computer-keyboard fallback shares it).
   On by default, optional. Left hand **orange**, right hand **blue**, with a
   gradient and soft glow so it looks magical, not flat. Highlights the keys
   the cursor is waiting for and the keys pressed. Uses the per-note staff for
   the hand. Rule 7: describe the layout in words and get approval before styling.

9. **Post-session report** (L, the biggest one).
   Shown only after an **End** button. A timeline of the attempt, per-bar
   pauses and wrong notes, repeated mistakes, steadiness, comparison with
   earlier sessions, and a suggestion of what to practise. Monkeytype-style
   visuals; the reference image is to be shown when this starts.
   Builds on ROADMAP **M2.1** (per-bar stats), **M2.3** (heatmap), **M2.6**.
   Analysis ideas to grow into:
   - hesitation bars (long pauses before a note);
   - which wrong notes recur;
   - rhythm steadiness against the score's tempo (from the timemap);
   - left/right hand imbalance;
   - early versus late in the attempt, fatigue;
   - progress against the last 3 sessions (D10 baseline).

## Tier 3: after v1.0 ("Next")

10. **Listen / play button** (L). ROADMAP **Next 2**.
    Play a bar, a loop range, or from where you clicked, **through MIDI out to
    the piano**; never scored. Uses Verovio's `renderToMIDI` and timemap, so it
    follows note lengths, chords and the score's BPM, with a speed factor. The
    cursor follows playback; Space stops.

11. **Held-note highlight for a note's real length** (M).
    The green note (4) stays lit as long as the note lasts: a whole note longer
    than a half note. Needs key-release capture plus timemap durations.

12. **Layout modes** (L).
    Vertical scroll (today) or one horizontal infinite line. Verovio can lay a
    piece out as a single endless system; geometry, cursor and auto-scroll need
    a horizontal variant. Zoom and fit change too.

13. **Drag to select a range, snapping to notes or bars** (M). Nice to have, later.
    Extends the loop selection (ROADMAP **M1.6**: click, Shift+click). Dragging
    from bar 2 to bar 3 snaps to the notes as landmarks.

## Order and dependencies

- 5 comes before 9.
- The Verovio timemap serves 10, 11 and the steadiness analysis in 9.
- 3 comes before 13.
- 8 needs only the per-note staff, so it can move earlier if wanted.
- The ROADMAP timeline (M0 to M4) is unchanged by this list.
