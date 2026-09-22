/**
 * The pure arithmetic behind zoom (`BFR-4`) — kept separate from the wheel
 * handler and the slider so it can be tested without mounting anything or
 * mocking `invoke`.
 */

export const MIN_SCALE = 50;
export const MAX_SCALE = 400;
export const SCALE_STEP = 10;

/** Keeps a staff size within what the slider (and the backend) allow. */
export function clampScale(value: number): number {
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, value));
}

/**
 * The next scale after a wheel tick. `deltaY` is negative when scrolling up
 * (away from the performer), which should zoom in — hence the sign flip.
 */
export function stepScale(current: number, deltaY: number): number {
  return clampScale(current - Math.sign(deltaY) * SCALE_STEP);
}
