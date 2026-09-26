import type { DrawnNote, Mark, Step } from "../bridge";
import { cursorLine } from "./scoreGeometry";
import type { Key, PageGeometry } from "./scoreGeometry";

/**
 * Where a wrong note would be written.
 *
 * A keyboard reports a sound, never a spelling, and nothing was drawn for a
 * wrong note. So it is placed relative to something that *was* drawn on the
 * same staff: an expected note, a nearby note, or failing both the clef's own
 * line. Heights come from where Verovio drew that reference, in the page's
 * own units, so an 8va line or a clef change moves the mark exactly as it
 * moved the reference. The spelling made here is for drawing only; nothing
 * that judges a performance ever sees it.
 */

const STEPS: readonly Step[] = ["C", "D", "E", "F", "G", "A", "B"];
const NATURAL: Record<Step, number> = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 };
const SHARPS: readonly Step[] = ["F", "C", "G", "D", "A", "E", "B"];
const FLATS: readonly Step[] = ["B", "E", "A", "D", "G", "C", "F"];
/** A black notehead's width, in staff spaces (SMuFL). */
const NOTEHEAD_WIDTH = 1.18;
/** How far a ledger line reaches beyond its notehead, each side, in spaces. */
const LEDGER_OVERHANG = 0.27;

/** A letter, octave and accidental (-2..2 semitones). */
export interface Spelling {
  readonly step: Step;
  readonly octave: number;
  readonly alter?: number;
}

/** Staff position in diatonic steps: adjacent letters are one apart, an octave seven. */
export function diatonicIndex(spelling: Spelling): number {
  return spelling.octave * STEPS.length + STEPS.indexOf(spelling.step);
}

/** The alteration a key signature gives a letter. */
function keyAlter(step: Step, key: Key): number {
  if (key.accidental === "sharp") return SHARPS.slice(0, key.count).includes(step) ? 1 : 0;
  if (key.accidental === "flat") return FLATS.slice(0, key.count).includes(step) ? -1 : 0;
  return 0;
}

/**
 * How a MIDI pitch would be spelled in `key`: a white key natural, a black
 * key with a flat in a flat key and a sharp otherwise.
 */
export function spellingFor(pitch: number, key: Key): Spelling & { alter: number } {
  const octave = Math.floor(pitch / 12) - 1;
  const pitchClass = pitch % 12;
  const natural = STEPS.find((step) => NATURAL[step] === pitchClass);
  if (natural) return { step: natural, octave, alter: 0 };

  // ponytail: no key signature spells black keys sharp; spell from context if that reads wrong.
  const flat = key.accidental === "flat";
  const step = STEPS.find((candidate) => NATURAL[candidate] === pitchClass + (flat ? 1 : -1))!;
  return { step, octave, alter: flat ? -1 : 1 };
}

/** Everything needed to draw a wrong note, in the page's drawing units. */
export interface WrongNoteMark {
  readonly system: number;
  readonly staff: number;
  /** The notehead's centre: the cursor's x, and the staff position's y. */
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly spacing: number;
  /** The height of each ledger line to draw, between the staff and the note. */
  readonly ledgers: readonly number[];
  readonly ledgerWidth: number;
  readonly accidental: "sharp" | "flat" | "natural" | null;
  /** The key that was pressed, e.g. "C♯5". */
  readonly label: string;
}

/**
 * The wrong-note mark for `pitch`, played while the cursor is on
 * `expected`. `marks` is every position in the piece, for finding a nearby
 * note on a staff the cursor has nothing on. Null if the cursor's notes are
 * not on this page.
 */
export function wrongNoteMark(
  geometry: PageGeometry,
  expected: readonly DrawnNote[],
  marks: readonly Mark[],
  pitch: number,
): WrongNoteMark | null {
  const cursor = cursorLine(
    geometry,
    expected.map((note) => note.elementId),
  );
  if (!cursor) return null;

  const system = geometry.systems.find((candidate) => candidate.index === cursor.system)!;
  const drawn = (notes: readonly DrawnNote[]) =>
    notes.flatMap((note) => {
      const head = geometry.noteheads.get(note.elementId);
      return head && head.system === system.index ? [{ note, head }] : [];
    });
  const here = drawn(expected);
  const bar = geometry.bars.find((candidate) => candidate.id === here[0]!.head.bar)!;
  const byDistance = (a: { note: DrawnNote }, b: { note: DrawnNote }) =>
    Math.abs(a.note.pitch - pitch) - Math.abs(b.note.pitch - pitch);

  // Which staff: that of the closest expected note within an octave, else
  // treble from middle C up and bass below.
  const closest = [...here].sort(byDistance)[0]!;
  const staff =
    Math.abs(closest.note.pitch - pitch) <= 12
      ? closest.head.staff
      : staffForRegister(bar.clefs.map((clef) => clef?.sign ?? null), pitch);
  const lines = system.staves[staff]!.lines;
  const spacing = system.staves[staff]!.spacing;

  // What to measure from: an expected note on that staff, else the nearest
  // drawn note on it in this system, else the clef's own line.
  const onStaff = here.filter(({ head }) => head.staff === staff).sort(byDistance);
  const nearby = drawn(marks.flatMap((mark) => mark.notes))
    .filter(({ head }) => head.staff === staff)
    .sort((a, b) => Math.abs(a.head.x - cursor.x) - Math.abs(b.head.x - cursor.x));
  const clef = bar.clefs[staff];
  const reference = onStaff[0] ?? nearby[0];
  const from = reference
    ? { y: reference.head.y, spelling: reference.note as Spelling }
    : clef
      ? { y: lines[5 - clef.line]!, spelling: clef.reference }
      : { y: lines[2]!, spelling: { step: "B", octave: 4 } as Spelling };

  const key = bar.keys[staff] ?? { accidental: null, count: 0 };
  const spelling = spellingFor(pitch, key);
  const y = from.y - ((diatonicIndex(spelling) - diatonicIndex(from.spelling)) * spacing) / 2;
  const width = NOTEHEAD_WIDTH * spacing;

  return {
    system: system.index,
    staff,
    x: cursor.x,
    y,
    width,
    spacing,
    ledgers: ledgersFor(y, lines[0]!, lines[4]!, spacing),
    ledgerWidth: width + 2 * LEDGER_OVERHANG * spacing,
    accidental: accidentalFor(spelling, key),
    label: `${spelling.step}${spelling.alter > 0 ? "♯" : spelling.alter < 0 ? "♭" : ""}${spelling.octave}`,
  };
}

function staffForRegister(clefs: readonly (string | null)[], pitch: number): number {
  const wanted = clefs.indexOf(pitch >= 60 ? "G" : "F");
  if (wanted >= 0) return wanted;
  return pitch >= 60 ? 0 : clefs.length - 1;
}

/** Ledger lines from the staff out to a note at `y`, one per line position passed. */
export function ledgersFor(y: number, top: number, bottom: number, spacing: number): number[] {
  const ledgers: number[] = [];
  const reach = spacing / 4; // tolerance, so rounding never drops a ledger
  for (let line = top - spacing; line >= y - reach; line -= spacing) ledgers.push(line);
  for (let line = bottom + spacing; line <= y + reach; line += spacing) ledgers.push(line);
  return ledgers;
}

// ponytail: ignores accidentals earlier in the bar; read them from the page's g.accid if that misleads.
function accidentalFor(spelling: Spelling & { alter: number }, key: Key): WrongNoteMark["accidental"] {
  if (spelling.alter === keyAlter(spelling.step, key)) return null;
  return spelling.alter > 0 ? "sharp" : spelling.alter < 0 ? "flat" : "natural";
}
