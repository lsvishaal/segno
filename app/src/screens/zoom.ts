/**
 * The arithmetic behind zoom, kept apart from the wheel handler and the
 * slider so it can be tested without mounting anything.
 *
 * Zoom reflows the music to the score's width (Rust's `Layout::fitting`):
 * 100% is Verovio's natural size, and a bigger zoom fits fewer bars per line.
 */
import { remember, setting } from "../settings";

export const MIN_SCALE = 50;
export const MAX_SCALE = 200;
export const SCALE_STEP = 10;
export const DEFAULT_SCALE = 100;
/** The widest the score is drawn; past this, lines get too long to read. */
export const SCORE_MAX_WIDTH = 1600;
/** A width change smaller than this (a scrollbar appearing) does not re-engrave. */
export const REFLOW_THRESHOLD = 20;


/** The width the score is engraved for, given the room it has. */
export function scoreWidth(available: number): number {
  return Math.round(Math.min(SCORE_MAX_WIDTH, available));
}

/** The zoom last chosen, or the default. */
export function loadZoom(): number {
  return clampScale(setting("zoom") || DEFAULT_SCALE);
}

/** Remembers the zoom for every piece, across restarts. */
export function saveZoom(scale: number): void {
  remember({ zoom: scale });
}

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
