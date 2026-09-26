import { describe, expect, it } from "vitest";

import { afterImport } from "./App";
import type { PieceSummary } from "./bridge";

const CANON: PieceSummary = { id: "23762273abf1d6bd", title: "Canon in D", composer: "Johann Pachelbel", bars: 57, notes: 1604 };
const MINUET: PieceSummary = { id: "minuet", title: "Minuet in G Major", composer: "Christian Petzold", bars: 32, notes: 209 };

describe("the shelf after an import", () => {
  it("gains a new piece", () => {
    expect(afterImport([MINUET], { piece: CANON, lines: [], already: false })).toEqual([MINUET, CANON]);
  });

  it("is unchanged when the file was already there, however often it is imported", () => {
    let shelf = afterImport([MINUET], { piece: CANON, lines: [], already: false });
    shelf = afterImport(shelf, { piece: CANON, lines: [], already: true });
    shelf = afterImport(shelf, { piece: CANON, lines: [], already: true });

    expect(shelf).toEqual([MINUET, CANON]);
  });

  it("never lists a piece twice, even if told it is new", () => {
    expect(afterImport([MINUET, CANON], { piece: CANON, lines: [], already: false })).toEqual([MINUET, CANON]);
  });
});
