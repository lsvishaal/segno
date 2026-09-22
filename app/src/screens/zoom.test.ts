import { describe, expect, it } from "vitest";

import { MAX_SCALE, MIN_SCALE, SCALE_STEP, clampScale, stepScale } from "./zoom";

describe("clampScale", () => {
  it("passes a usable value through unchanged", () => {
    expect(clampScale(150)).toBe(150);
  });

  it("clamps below the minimum", () => {
    expect(clampScale(0)).toBe(MIN_SCALE);
  });

  it("clamps above the maximum", () => {
    expect(clampScale(10_000)).toBe(MAX_SCALE);
  });
});

describe("stepScale", () => {
  it("zooms in when scrolling up (negative deltaY)", () => {
    expect(stepScale(150, -1)).toBe(150 + SCALE_STEP);
  });

  it("zooms out when scrolling down (positive deltaY)", () => {
    expect(stepScale(150, 1)).toBe(150 - SCALE_STEP);
  });

  it("does not step past the maximum", () => {
    expect(stepScale(MAX_SCALE, -1)).toBe(MAX_SCALE);
  });

  it("does not step past the minimum", () => {
    expect(stepScale(MIN_SCALE, 1)).toBe(MIN_SCALE);
  });
});
