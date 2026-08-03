import type { PracticeEvent } from "../bridge";
import type { Marked } from "../components/Score";

/** The Cursor's resting position before an attempt has begun (`003 §7.16`). */
export const BEFORE_STARTING: Marked = new Map([[0, "cursor"]]);

/**
 * What `event` does to the marks on the score, if anything.
 *
 * A pure function, deliberately: it is the one place a `PracticeEvent`
 * becomes a `Marked` map, and the mechanism `MVP §6.6` rests on — a wrong
 * note flashes at the position the Cursor is waiting at rather than moving
 * it (`ADR-0002`), and `correct`/`wrong` themselves are never decided here.
 * They arrive already decided, from `Cursor::expecting()` on the practice
 * thread (`003 §7.16`); this function only turns that decision into a
 * position on the score.
 *
 * Returns `undefined` for an event that marks nothing (an instrument
 * appearing or disappearing, or the piece having been fully reached), so a
 * caller can tell "no change" from "cleared".
 */
export function markFor(event: PracticeEvent): Marked | undefined {
  switch (event.kind) {
    case "played":
      return new Map([[event.position, event.correct ? "correct" : "wrong"]]);
    case "advanced":
      return new Map([[event.position, "cursor"]]);
    case "reachedEnd":
    case "instrumentLost":
    case "instrumentRestored":
    case "ended":
      return undefined;
  }
}
