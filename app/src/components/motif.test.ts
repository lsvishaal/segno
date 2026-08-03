import { describe, expect, it } from "vitest";

import { motifFor } from "./motif";

describe("motifFor", () => {
  it("is deterministic for the same id", () => {
    expect(motifFor("abc123")).toBe(motifFor("abc123"));
  });

  it("returns a usable CSS color value", () => {
    expect(motifFor("abc123")).toMatch(/^var\(--color-/);
  });

  // Never a difficulty signal (`001 §11`) — same id in, same colour out,
  // regardless of how the piece is described.
  it("does not depend on anything but the id", () => {
    expect(motifFor("deadbeef")).toBe(motifFor("deadbeef"));
    expect(motifFor("cafef00d")).not.toBe(undefined);
  });
});
