import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  DEFAULT_SCALE,
  MAX_SCALE,
  MIN_SCALE,
  SCALE_STEP,
  SCORE_MAX_WIDTH,
  clampScale,
  loadZoom,
  saveZoom,
  scoreWidth,
  stepScale,
} from "./zoom";

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

describe("the score's width", () => {
  it("is the room available, up to a readable maximum", () => {
    expect(scoreWidth(900.4)).toBe(900);
    expect(scoreWidth(4000)).toBe(SCORE_MAX_WIDTH);
  });
});

describe("remembered zoom", () => {
  beforeEach(() => {
    const stored = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => stored.get(key) ?? null,
      setItem: (key: string, value: string) => void stored.set(key, value),
    });
  });
  afterEach(() => vi.unstubAllGlobals());

  it("starts at the default", () => {
    expect(loadZoom()).toBe(DEFAULT_SCALE);
  });

  it("falls back to the default when storage is unavailable", () => {
    vi.stubGlobal("localStorage", undefined);
    saveZoom(150);
    expect(loadZoom()).toBe(DEFAULT_SCALE);
  });

  it("comes back as it was saved, within range", () => {
    saveZoom(150);
    expect(loadZoom()).toBe(150);
    saveZoom(9000);
    expect(loadZoom()).toBe(MAX_SCALE);
  });
});
