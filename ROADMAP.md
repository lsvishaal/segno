# Hogwash — Roadmap

v1 goal (ship by 8 November): the practice tool that remembers which bars you
struggle with and helps you fix them.

## Phase 0 — clean up, and make Hogwash handle real pieces

- [x] 0.1 Cleanup: drop the no-advice rule and its test, delete STATUS.md,
      reset this roadmap, trim the README
- [x] 0.2 `hogwash-cli replay`: run a MusicXML file and a JSON file of played
      notes through the real practice + analysis path, step by step
- [x] 0.3 Real-music verification on Prelude in C and Minuet in G: grand
      staff, ties, repeats, grace notes and ornaments; bundle both pieces
- [x] 0.4 Report: what worked, what was fixed, what is still broken in the UI,
      and a manual test checklist

## Phase 0.5 — score display correctness

- [x] 0.5.1 Geometry extraction from Verovio's SVG + "Show geometry" debug view
- [ ] 0.5.2 Pitch -> staff position from measured staff lines and clefs
- [ ] 0.5.3 Overlays inside the SVG: cursor on noteheads, soft played tint
- [ ] 0.5.4 Wrong-note ghost on the right staff, with ledger lines and label
- [ ] 0.5.5 Zoom that actually changes size and reflows to the window

## Phase 1

## Phase 2

## Phase 3

## Later
- An upward-resolving trill (e.g. F trilled with G, then G) advances the
  cursor early on its first G, and the rest of the trill then counts as wrong:
  the ornament tolerance only covers the trilled step and the next one.
- Stopping early lists every note never played, one line each (517 lines for
  the Prelude): summarise the untouched tail as a bar range instead.
- Tied-to notes and grace notes get no note-name label (they are not marks).
- At a repeat the cursor slides back across the page, and nothing shows
  which pass (1st / 2nd time) the performer is on.
- A clef change in the middle of a bar takes effect from the next bar in the
  score geometry (neither bundled piece has one).
