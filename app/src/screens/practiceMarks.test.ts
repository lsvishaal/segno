import { describe, expect, it } from "vitest";

import type { PracticeEvent } from "../bridge";
import { markFor } from "./practiceMarks";

/**
 * `markFor` is where a `PracticeEvent` becomes a position on the score.
 * `correct`/`wrong` themselves are not decided here — they arrive already
 * decided from `Cursor::expecting()` on the practice thread (`003 §7.16`) —
 * so these tests only check that the position and the state Rust sent are
 * carried through unchanged, and that nothing accumulates across events.
 */
describe("markFor", () => {
  it("marks the position a note was played at, correct or not", () => {
    const correct: PracticeEvent = { kind: "played", position: 3, correct: true };
    const wrong: PracticeEvent = { kind: "played", position: 3, correct: false };

    expect(markFor(correct)).toEqual(new Map([[3, "correct"]]));
    expect(markFor(wrong)).toEqual(new Map([[3, "wrong"]]));
  });

  // `ADR-0002` — an unexpected note is marked without moving the Cursor.
  // `markFor` reflects that: a `played` event never returns a `"cursor"` mark.
  it("never marks a played note as the cursor", () => {
    const event: PracticeEvent = { kind: "played", position: 0, correct: true };

    expect(markFor(event)?.get(0)).not.toBe("cursor");
  });

  it("moves the cursor mark, and only the cursor mark, on advancing", () => {
    const event: PracticeEvent = { kind: "advanced", position: 5 };

    expect(markFor(event)).toEqual(new Map([[5, "cursor"]]));
  });

  // Every other kind of event says something about the attempt, not about a
  // position on the score — `undefined` means "leave the marks alone".
  it("marks nothing for events that are not about a position", () => {
    const events: PracticeEvent[] = [
      { kind: "reachedEnd" },
      { kind: "instrumentLost", instrument: "Digital Piano", secondsRemaining: 60 },
      { kind: "instrumentRestored" },
      {
        kind: "ended",
        summary: {
          lines: [],
          correct: 0,
          incorrect: 0,
          omitted: 0,
          unexpected: 0,
          reachedTheEnd: true,
          autoEnded: false,
          reason: null,
        },
      },
    ];

    for (const event of events) {
      expect(markFor(event)).toBeUndefined();
    }
  });
});
