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
- [x] 0.5.2 Pitch -> staff position from measured staff lines and clefs
- [x] 0.5.3 Cursor inside the SVG: a line through the notes, both staves
- [x] 0.5.4 Wrong-note mark on the right staff, with ledger lines, accidental
      and label; notes under an 8va line expected at the pitch they sound
- [x] 0.5.5 Zoom that actually changes size and reflows to the window

## Order to v1

1. A real Für Elise (with its pickup bar) through MuseScore Studio and
   imported; fix bar numbering if it is refused
2. Timestamps on every played note
3. "Slowest bars" readout after a session (a plain list)
4. Seven practice sessions (Minuet + Für Elise), writing down every annoyance
5. Fix the top annoyances from that list
6. Publish (below)

## Before public release

- Licences of bundled pieces: every file clearly public domain or open;
  swap unclear ones for OpenScore / Open Well-Tempered Clavier files
- README for a stranger: what it does, a screenshot, installing on Linux
- Remove the references to deleted specs (§7.4, ADR-, FR-)
- Helpful messages for MIDI and PDF files (point to MuseScore Studio and
  Audiveris)
- Keyboard-range note (fits / doesn't fit 61 keys)
- A 60-second screen recording

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
- A piece that needs keys the connected keyboard doesn't have can never be
  finished (the cursor waits forever): warn when it is opened, "This piece
  uses keys your keyboard can't reach."
- A keyboard's octave up/down buttons shift every note it sends, so
  everything reads as wrong: detect a whole-performance octave offset and say so.
- The wrong-note mark's accidental ignores accidentals earlier in the same bar.
- "Replace a piece's file": re-importing an edited file is a new piece
  today (new fingerprint), and the old practice history stays with the old
  version. Offer to replace the file and carry the history over.
- Leaving the practice screen does not end the attempt; it keeps running in
  the background until another command ends it (issue 4: session handling).
- A zip damaged *inside* an entry (not cut short) could still reach Verovio
  and crash it; only the container's structure is checked before loading.
