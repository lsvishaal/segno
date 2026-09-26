import type { Mark, Step } from "../../bridge";
import { readPage } from "../scoreGeometry";
import minuet8vaNotes from "./minuet-in-g-8va.page1.notes.json";
import minuet8va from "./minuet-in-g-8va.page1.svg?raw";
import minuetNotes from "./minuet-in-g.page1.notes.json";
import minuet from "./minuet-in-g.page1.svg?raw";
import preludeNotes from "./prelude-in-c.page1.notes.json";
import prelude from "./prelude-in-c.page1.svg?raw";

/**
 * Page 1 of real bundled pieces, as Verovio rendered them
 * (`scripts/score-fixtures.py`), with each note's sounding pitch.
 */
export const PIECES = {
  "minuet-in-g": { svg: minuet, notes: minuetNotes },
  "prelude-in-c": { svg: prelude, notes: preludeNotes },
  // The Minuet with an 8va line over bars 1-2 of the right hand.
  "minuet-in-g-8va": { svg: minuet8va, notes: minuet8vaNotes },
} as const;

export type Piece = keyof typeof PIECES;

export function pageOf(piece: Piece): Element {
  return new DOMParser().parseFromString(PIECES[piece].svg, "image/svg+xml").documentElement;
}

/**
 * Positions for a page, the way the app gets them from Rust: here, the
 * page's note columns left to right, system by system. Close enough to the
 * performed order to put the cursor on any note of the page.
 */
export function marksOf(piece: Piece): Mark[] {
  const notes: Record<string, { step: string; octave: number; pitch: number }> = PIECES[piece].notes;
  const geometry = readPage(pageOf(piece));
  const heads = [...geometry.noteheads.values()].sort((a, b) => a.system - b.system || a.x - b.x);
  const marks: { measure: number; position: number; page: number; notes: Mark["notes"][number][] }[] = [];
  let columnX = -Infinity;
  let columnSystem = -1;

  for (const head of heads) {
    const spacing = geometry.systems[head.system]!.staves[0]!.spacing;
    if (head.system !== columnSystem || head.x - columnX > spacing) {
      marks.push({ measure: 1, position: marks.length, page: 0, notes: [] });
      columnX = head.x;
      columnSystem = head.system;
    }
    const note = notes[head.id]!;
    marks[marks.length - 1]!.notes.push({ elementId: head.id, step: note.step as Step, octave: note.octave, pitch: note.pitch });
  }

  return marks;
}
