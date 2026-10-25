import { describe, expect, it } from "vitest";

import type { PracticeEvent } from "../bridge";
import { cursorFor, dotFor } from "./overlayEvents";

describe("cursorFor", () => {
  it("moves to an advanced position", () => {
    const event: PracticeEvent = { kind: "advanced", position: 5 };

    expect(cursorFor(event, { position: 0 })).toEqual({ position: 5 });
  });

  // the Cursor moves only when a position is satisfied. A
  // `Played` event, correct or not, is not that signal on its own.
  it("does not move on a played event, even a correct one", () => {
    const event: PracticeEvent = { kind: "played", position: 3, correct: true, pitch: 60 };

    expect(cursorFor(event, { position: 3 })).toEqual({ position: 3 });
  });

  it("is unaffected by events that say nothing about position", () => {
    const events: PracticeEvent[] = [
      { kind: "reachedEnd" },
      { kind: "instrumentLost", instrument: "Digital Piano", secondsRemaining: 60 },
      { kind: "instrumentRestored" },
    ];

    for (const event of events) {
      expect(cursorFor(event, { position: 7 })).toEqual({ position: 7 });
    }
  });
});

describe("dotFor", () => {
  it("asks for a correct dot on a correctly played note", () => {
    const event: PracticeEvent = { kind: "played", position: 2, correct: true, pitch: 60 };

    expect(dotFor(event)).toEqual({ kind: "correct", position: 2, pitch: 60 });
  });

  it("asks for a wrong dot, carrying the pitch actually played", () => {
    const event: PracticeEvent = { kind: "played", position: 2, correct: false, pitch: 62 };

    expect(dotFor(event)).toEqual({ kind: "wrong", position: 2, pitch: 62 });
  });

  it("asks for nothing on events that are not about a played note", () => {
    const events: PracticeEvent[] = [
      { kind: "advanced", position: 3 },
      { kind: "reachedEnd" },
      { kind: "instrumentLost", instrument: "Digital Piano", secondsRemaining: 60 },
      { kind: "instrumentRestored" },
    ];

    for (const event of events) {
      expect(dotFor(event)).toBeNull();
    }
  });
});
