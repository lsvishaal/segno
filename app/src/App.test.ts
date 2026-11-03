import { describe, expect, it } from "vitest";

import { afterImport } from "./App";
import type { PieceSummary } from "./bridge";

const CANON: PieceSummary = { id: "23762273abf1d6bd", title: "Canon in D", composer: "Johann Pachelbel", bars: 57, notes: 1604, fileName: "Canon_in_D.mxl", imported: true, level: null, lastPractisedMs: null, weakest: [] };
const MINUET: PieceSummary = { id: "minuet", title: "Minuet in G Major", composer: "Christian Petzold", bars: 32, notes: 209, fileName: null, imported: false, level: "intermediate", lastPractisedMs: null, weakest: [] };

describe("the shelf after an import", () => {
  it("gains a new piece", () => {
    expect(afterImport([MINUET], { piece: CANON, lines: [], already: false, asks: false })).toEqual([MINUET, CANON]);
  });

  it("is unchanged when the file was already there, however often it is imported", () => {
    let shelf = afterImport([MINUET], { piece: CANON, lines: [], already: false, asks: false });
    shelf = afterImport(shelf, { piece: CANON, lines: [], already: true, asks: false });
    shelf = afterImport(shelf, { piece: CANON, lines: [], already: true, asks: false });

    expect(shelf).toEqual([MINUET, CANON]);
  });

  it("never lists a piece twice, even if told it is new", () => {
    expect(afterImport([MINUET, CANON], { piece: CANON, lines: [], already: false, asks: false })).toEqual([MINUET, CANON]);
  });

  it("is unchanged while an import waits for Update or Add as a new piece", () => {
    expect(afterImport([MINUET, CANON], { piece: CANON, lines: [], already: false, asks: true })).toEqual([MINUET, CANON]);
  });

  it("replaces an updated piece where it was", () => {
    const updated = { ...CANON, notes: 1610 };
    expect(afterImport([CANON, MINUET], { piece: updated, lines: [], already: false, asks: false })).toEqual([updated, MINUET]);
  });
});
