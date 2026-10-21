import type { PracticeEvent } from "../bridge";

/** Where the cursor bar sits — a position in the piece, nothing else. */
export interface CursorState {
  readonly position: number;
}

/**
 * One flash of feedback on a played note.
 *
 * `pitch` is carried through so the wrong-note case can be placed on the
 * staff (`staffGeometry.ts`) — the correct case never needs it, since a
 * correct note already has a real, drawn position to appear at.
 */
export interface Dot {
  readonly kind: "correct" | "wrong";
  readonly position: number;
  readonly pitch: number;
}

/** The Cursor's resting position before an attempt has begun (`003 §7.16`). */
export const BEFORE_STARTING: CursorState = { position: 0 };

/**
 * Where the cursor bar moves to, given `event`: on to the next position, or
 * back to a loop's start when a pass wraps or the loop ends. Unchanged for
 * anything else.
 *
 * Never derived from a `Played` event: `003 §7.16` says the Cursor moves
 * only when a position is satisfied, and `Advanced` is Rust's own signal
 * for exactly that, computed by the same `Cursor` `Analysis` replays
 * afterwards (`004 §6.5`) — this function only carries that decision to
 * the screen, it does not make one of its own.
 */
export function cursorFor(event: PracticeEvent, current: CursorState): CursorState {
  switch (event.kind) {
    case "advanced":
    case "looped":
    case "loopDone":
      return { position: event.position };
    default:
      return current;
  }
}

/**
 * The dot a `Played` event asks for, or `null` for any other event.
 *
 * A `null` return does not mean "clear the dot on screen" — it means this
 * event says nothing about one. A dot's own lifetime (appearing, then
 * fading out) is the caller's concern, on a timer, independent of whatever
 * event happens to arrive next.
 */
export function dotFor(event: PracticeEvent): Dot | null {
  return event.kind === "played"
    ? { kind: event.correct ? "correct" : "wrong", position: event.position, pitch: event.pitch }
    : null;
}
