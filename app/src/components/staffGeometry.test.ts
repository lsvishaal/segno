import { describe, expect, it } from "vitest";

import { diatonicIndex, nearestSpelling, verticalPositionFor } from "./staffGeometry";

describe("diatonicIndex", () => {
  it("counts adjacent letters as one step apart", () => {
    expect(diatonicIndex({ step: "D", octave: 4 }) - diatonicIndex({ step: "C", octave: 4 })).toBe(
      1,
    );
  });

  it("counts an octave as seven steps", () => {
    expect(diatonicIndex({ step: "C", octave: 5 }) - diatonicIndex({ step: "C", octave: 4 })).toBe(
      7,
    );
  });

  // The octave turns over between B and C, not at any other letter.
  it("carries the octave across the B-to-C boundary", () => {
    expect(diatonicIndex({ step: "C", octave: 4 }) - diatonicIndex({ step: "B", octave: 3 })).toBe(
      1,
    );
  });
});

describe("nearestSpelling", () => {
  // A pitch matching the reference exactly needs no accidental at all.
  it("prefers the reference's own letter when it matches exactly", () => {
    expect(nearestSpelling(60, { step: "C", octave: 4 })).toEqual({ step: "C", octave: 4 });
  });

  // 61 is C-sharp-4 *or* D-flat-4. Next to a C4 reference, C-sharp (no
  // staff movement at all) is the smaller jump than D-flat (one step away).
  it("picks the enharmonic spelling that moves least on the staff", () => {
    expect(nearestSpelling(61, { step: "C", octave: 4 })).toEqual({ step: "C", octave: 4 });
  });

  // The same pitch, next to a D4 reference, reads more naturally as
  // D-flat (zero steps from D) than C-sharp (one step away).
  it("changes its answer with the reference, for the same pitch", () => {
    expect(nearestSpelling(61, { step: "D", octave: 4 })).toEqual({ step: "D", octave: 4 });
  });
});

describe("verticalPositionFor", () => {
  const C4 = { step: "C", octave: 4 } as const;
  const G4 = { step: "G", octave: 4 } as const;
  const D4 = { step: "D", octave: 4 } as const;

  it("extrapolates linearly from two real measured points", () => {
    // C4 at y=100, G4 (4 steps higher) at y=70: 7.5px per step, downward
    // as pitch rises, matching how screen y decreases upward.
    const calibration = [
      { spelling: C4, y: 100 },
      { spelling: G4, y: 70 },
    ];

    expect(verticalPositionFor(D4, calibration)).toBeCloseTo(92.5);
    expect(verticalPositionFor(C4, calibration)).toBeCloseTo(100);
    expect(verticalPositionFor(G4, calibration)).toBeCloseTo(70);
  });

  it("has nothing to extrapolate from with fewer than two points", () => {
    expect(verticalPositionFor(D4, [])).toBeNull();
    expect(verticalPositionFor(D4, [{ spelling: C4, y: 100 }])).toBeNull();
  });

  it("has nothing to extrapolate from when every point is the same step", () => {
    const calibration = [
      { spelling: C4, y: 100 },
      { spelling: C4, y: 100 },
    ];

    expect(verticalPositionFor(D4, calibration)).toBeNull();
  });

  it("picks the widest pair, when given more than two points", () => {
    // A close, noisy pair (C4/D4) sits alongside a wide, stable one
    // (C4/G4); the wide pair should win.
    const calibration = [
      { spelling: C4, y: 100 },
      { spelling: D4, y: 95 },
      { spelling: G4, y: 70 },
    ];

    expect(verticalPositionFor(D4, calibration)).toBeCloseTo(92.5);
  });
});
