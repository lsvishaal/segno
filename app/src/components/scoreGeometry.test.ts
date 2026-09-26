import { describe, expect, it } from "vitest";

import minuet from "./__fixtures__/minuet-in-g.page1.svg?raw";
import prelude from "./__fixtures__/prelude-in-c.page1.svg?raw";
import { cursorBand, readPage } from "./scoreGeometry";

const FIXTURES: Record<string, string> = { "minuet-in-g": minuet, "prelude-in-c": prelude };

/** Page 1 of a real bundled piece, as Verovio rendered it (see scripts/score-fixtures.py). */
function page(piece: string): Element {
  return new DOMParser().parseFromString(FIXTURES[piece]!, "image/svg+xml").documentElement;
}

describe.each(["minuet-in-g", "prelude-in-c"])("geometry of %s, page 1", (piece) => {
  const root = page(piece);
  const geometry = readPage(root);

  it("finds every system, each a grand staff of five evenly spaced lines", () => {
    expect(geometry.systems.length).toBe(root.querySelectorAll("g.system").length);

    for (const system of geometry.systems) {
      expect(system.staves).toHaveLength(2);
      for (const staff of system.staves) {
        expect(staff.lines).toHaveLength(5);
        const gaps = staff.lines.slice(1).map((y, i) => y - staff.lines[i]!);
        for (const gap of gaps) expect(gap).toBeCloseTo(staff.spacing, 5);
        expect(staff.spacing).toBeGreaterThan(0);
      }
      expect(system.staves[0]!.lines[4]!).toBeLessThan(system.staves[1]!.lines[0]!);
    }
  });

  it("reads a treble clef on the second line up and a bass clef on the second line down", () => {
    for (const bar of geometry.bars) {
      expect(bar.clefs[0]).toMatchObject({ sign: "G", line: 2, reference: { step: "G", octave: 4 } });
      expect(bar.clefs[1]).toMatchObject({ sign: "F", line: 4, reference: { step: "F", octave: 3 } });
    }
  });

  it("has one bar box per drawn bar, inside its system", () => {
    expect(geometry.bars.length).toBe(root.querySelectorAll("g.measure").length);

    for (const bar of geometry.bars) {
      const system = geometry.systems[bar.system]!;
      expect(bar.top).toBe(system.top);
      expect(bar.bottom).toBe(system.bottom);
      expect(bar.x1).toBeGreaterThan(bar.x0);
    }
  });

  it("has a notehead for every note, sitting on a line or a space of its own staff", () => {
    expect(geometry.noteheads.size).toBe(root.querySelectorAll("g.note").length);

    for (const head of geometry.noteheads.values()) {
      const staff = geometry.systems[head.system]!.staves[head.staff]!;
      const halfSteps = (head.y - staff.lines[0]!) / (staff.spacing / 2);

      expect(halfSteps).toBeCloseTo(Math.round(halfSteps), 5);
      expect(head.width).toBeGreaterThan(staff.spacing);
      expect(head.height).toBe(staff.spacing);
    }
  });
});

describe("key signatures", () => {
  it("reads the Minuet's one sharp (G major) and the Prelude's none (C major)", () => {
    for (const bar of readPage(page("minuet-in-g")).bars) {
      expect(bar.keys).toEqual([
        { accidental: "sharp", count: 1 },
        { accidental: "sharp", count: 1 },
      ]);
    }
    for (const bar of readPage(page("prelude-in-c")).bars) {
      expect(bar.keys.every((key) => key.count === 0)).toBe(true);
    }
  });
});

describe("the cursor band, on the Minuet's first bar", () => {
  const geometry = readPage(page("minuet-in-g"));
  // Bar 1: D5 in the right hand over G3 in the left; then the right hand's G4
  // while the left hand is still held.
  const D5 = "b37hn12";
  const G3 = "j14qhuvu";
  const G4 = "d1eyoooy";

  it("covers both staves even where only the right hand starts a note", () => {
    const band = cursorBand(geometry, [G4])!;
    const system = geometry.systems[band.system]!;

    expect(band.y).toBeLessThan(system.top);
    expect(band.y + band.height).toBeGreaterThan(system.bottom);
  });

  it("is centred on its notehead", () => {
    const band = cursorBand(geometry, [G4])!;
    const head = geometry.noteheads.get(G4)!;

    expect(band.x + band.width / 2).toBeCloseTo(head.x + head.width / 2, 5);
  });

  it("covers the noteheads of both hands", () => {
    const band = cursorBand(geometry, [D5, G3])!;

    for (const id of [D5, G3]) {
      const head = geometry.noteheads.get(id)!;
      expect(head.x).toBeGreaterThanOrEqual(band.x);
      expect(head.x + head.width).toBeLessThanOrEqual(band.x + band.width);
      expect(head.y).toBeGreaterThan(band.y);
      expect(head.y).toBeLessThan(band.y + band.height);
    }
  });

  it("is nothing for notes not on the page", () => {
    expect(cursorBand(geometry, ["not-a-note"])).toBeNull();
    expect(cursorBand(geometry, [])).toBeNull();
  });
});
