import { describe, expect, it } from "vitest";

import { marksOf, pageOf } from "./__fixtures__/pages";
import type { Piece } from "./__fixtures__/pages";
import { cursorLine, readPage } from "./scoreGeometry";
import { ledgersFor, spellingFor, wrongNoteMark } from "./staffGeometry";

const G_MAJOR = { accidental: "sharp", count: 1 } as const;
const F_MAJOR = { accidental: "flat", count: 1 } as const;
const C_MAJOR = { accidental: null, count: 0 } as const;

describe("spelling a pressed key", () => {
  it.each([
    [60, C_MAJOR, { step: "C", octave: 4, alter: 0 }],
    [66, G_MAJOR, { step: "F", octave: 4, alter: 1 }],
    [70, F_MAJOR, { step: "B", octave: 4, alter: -1 }],
    [61, C_MAJOR, { step: "C", octave: 4, alter: 1 }],
    [71, C_MAJOR, { step: "B", octave: 4, alter: 0 }],
    [72, C_MAJOR, { step: "C", octave: 5, alter: 0 }],
  ])("MIDI %i is %o", (pitch, key, spelling) => {
    expect(spellingFor(pitch, key)).toEqual(spelling);
  });
});

describe("ledger lines", () => {
  // Treble lines F5 (0) to E4 (4), 180 apart.
  const [top, bottom, space] = [0, 720, 180];

  it.each([
    ["G5, in the space above", -90, 0],
    ["A5, on the first ledger", -180, 1],
    ["B5, above the first", -270, 1],
    ["C6", -360, 2],
    ["C4, below", 900, 1],
    ["a note on the staff", 360, 0],
  ])("%s needs the right number", (_, y, count) => {
    expect(ledgersFor(y, top, bottom, space)).toHaveLength(count);
  });
});

/** The mark for `pitch` with the cursor on the `step`th column of the page. */
function markFor(piece: Piece, step: number, pitch: number) {
  const geometry = readPage(pageOf(piece));
  const marks = marksOf(piece);
  const mark = wrongNoteMark(geometry, marks[step]!.notes, marks, pitch)!;
  return { geometry, marks, mark, expected: marks[step]!.notes };
}

function headOf(piece: Piece, step: number, pitch: number) {
  const { geometry, expected } = markFor(piece, step, 60);
  return geometry.noteheads.get(expected.find((note) => note.pitch === pitch)!.elementId)!;
}

// Bar 1, beat 1 of the Minuet: D5 over a G3-B3-D4 chord.
describe("a wrong note on the Minuet's first beat", () => {
  const D5 = headOf("minuet-in-g", 0, 74);
  const half = D5.height / 2;

  it("is on the cursor's column, one step above D5 for E5", () => {
    const { geometry, mark, expected } = markFor("minuet-in-g", 0, 76);
    const cursor = cursorLine(geometry, expected.map((note) => note.elementId))!;

    expect(mark.x).toBe(cursor.x);
    expect(mark.staff).toBe(0);
    expect(mark.y).toBeCloseTo(D5.y - half, 5);
    expect(mark).toMatchObject({ ledgers: [], accidental: null, label: "E5" });
  });

  it("is one step below D5 for C5", () => {
    expect(markFor("minuet-in-g", 0, 72).mark.y).toBeCloseTo(D5.y + half, 5);
  });

  it("gives C sharp the C's position and a sharp", () => {
    const { mark } = markFor("minuet-in-g", 0, 73);
    expect(mark.y).toBeCloseTo(D5.y + half, 5);
    expect(mark).toMatchObject({ accidental: "sharp", label: "C♯5" });
  });

  it("marks F natural, which the key makes sharp", () => {
    expect(markFor("minuet-in-g", 0, 77).mark.accidental).toBe("natural");
    expect(markFor("minuet-in-g", 0, 78).mark.accidental).toBeNull();
  });

  it("puts A3 on the bass staff, one step above the G3", () => {
    const G3 = headOf("minuet-in-g", 0, 55);
    const { mark } = markFor("minuet-in-g", 0, 57);

    expect(mark.staff).toBe(1);
    expect(mark.y).toBeCloseTo(G3.y - half, 5);
  });

  it("puts C6 two ledger lines above the treble staff", () => {
    const { geometry, mark } = markFor("minuet-in-g", 0, 84);
    const top = geometry.systems[0]!.staves[0]!.lines[0]!;

    expect(mark.staff).toBe(0);
    expect(mark.ledgers).toHaveLength(2);
    expect(mark.y).toBeCloseTo(top - 2 * mark.spacing, 5);
  });

  it("puts C2 below the bass staff, with ledger lines", () => {
    const { geometry, mark } = markFor("minuet-in-g", 0, 36);
    const bottom = geometry.systems[0]!.staves[1]!.lines[4]!;

    expect(mark.staff).toBe(1);
    expect(mark.y).toBeCloseTo(bottom + 2 * mark.spacing, 5);
    expect(mark.ledgers).toHaveLength(2);
  });

  it("puts middle C by the bass chord's D4, one ledger line above the bass staff", () => {
    const { geometry, mark } = markFor("minuet-in-g", 0, 60);
    const top = geometry.systems[0]!.staves[1]!.lines[0]!;

    expect(mark.staff).toBe(1);
    expect(mark.y).toBeCloseTo(top - mark.spacing, 5);
    expect(mark.ledgers).toHaveLength(1);
  });
});

// The next beat: only the right hand's G4 starts; the left hand is held.
describe("a wrong note while only the right hand starts a note", () => {
  it("puts middle C one ledger line below the treble staff", () => {
    const { geometry, mark } = markFor("minuet-in-g", 1, 60);
    const bottom = geometry.systems[0]!.staves[0]!.lines[4]!;

    expect(mark.staff).toBe(0);
    expect(mark.y).toBeCloseTo(bottom + mark.spacing, 5);
    expect(mark.ledgers).toHaveLength(1);
  });

  it("still puts a low note on the bass staff, measured from a bass note nearby", () => {
    const { geometry, mark } = markFor("minuet-in-g", 1, 48);
    const bass = geometry.systems[0]!.staves[1]!;

    // C3 in the bass clef: the space between B2 and D3, the second from the bottom.
    expect(mark.staff).toBe(1);
    expect(mark.y).toBeCloseTo(bass.lines[3]! - bass.spacing / 2, 5);
  });
});

// The same beat under an 8va line: D5 is drawn where D4 would be.
describe("a wrong note under an 8va line", () => {
  it("is drawn a step above the drawn D, not an octave away from it", () => {
    const D5 = headOf("minuet-in-g-8va", 0, 74);
    const { mark } = markFor("minuet-in-g-8va", 0, 76);

    expect(mark.y).toBeCloseTo(D5.y - D5.height / 2, 5);
    expect(mark.label).toBe("E5");
  });
});

describe("the whole Minuet page", () => {
  // Pressing exactly the expected key must land on that key's notehead,
  // for every note on the page: the placement agrees with Verovio's own.
  it("places every expected pitch on its own notehead", () => {
    const geometry = readPage(pageOf("minuet-in-g"));
    const marks = marksOf("minuet-in-g");

    for (const mark of marks) {
      for (const note of mark.notes) {
        const placed = wrongNoteMark(geometry, [note], marks, note.pitch)!;
        expect(placed.y).toBeCloseTo(geometry.noteheads.get(note.elementId)!.y, 5);
      }
    }
  });
});
