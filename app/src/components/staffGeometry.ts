import type { Step } from "../bridge";

/**
 * Placing a note that was never written.
 *
 * A wrong note has no position of its own — a MIDI keyboard reports a
 * struck key's sound, never its spelling (`ADR-0001`), and nothing was
 * drawn for it. Everything in this module answers one question only:
 * *where would that sound sit on the staff, if it had been written?* —
 * for the wrong-note dot to appear in a sensible place, and nowhere else.
 * A spelling produced here (`nearestSpelling`) is a display artefact. It
 * must never reach `Analysis`, `Feedback`, or the `Cursor` — those already
 * work from the chromatic pitch alone, exactly as `ADR-0001` requires, and
 * nothing in this module changes that.
 *
 * Pure, and deliberately ignorant of the DOM: every position here comes
 * from real, already-measured points (an actually-drawn note's spelling
 * and its on-screen `y`), never from assumed Verovio internals or
 * hard-coded staff geometry. Two real points are enough to know how many
 * pixels one diatonic step is worth, on whatever staff was actually drawn.
 */

const STEPS: readonly Step[] = ["C", "D", "E", "F", "G", "A", "B"];

const SEMITONES_ABOVE_C: Record<Step, number> = {
  C: 0,
  D: 2,
  E: 4,
  F: 5,
  G: 7,
  A: 9,
  B: 11,
};

/** A written position: letter name and octave, nothing else. */
export interface Spelling {
  readonly step: Step;
  readonly octave: number;
}

/** A real, measured point: a drawn note's spelling and its on-screen `y`. */
export interface Calibration {
  readonly spelling: Spelling;
  readonly y: number;
}

/**
 * Where `spelling` sits on a staff, counted in diatonic steps rather than
 * pixels or semitones — two adjacent letters are one step apart, one
 * octave is seven. This is staff position, not sound: C and C-sharp are
 * the same step, which is exactly why an accidental never moves a note.
 */
export function diatonicIndex(spelling: Spelling): number {
  return spelling.octave * STEPS.length + STEPS.indexOf(spelling.step);
}

/** The chromatic (MIDI) pitch of a natural letter in `octave`, no accidental. */
function naturalPitch(step: Step, octave: number): number {
  return (octave + 1) * 12 + SEMITONES_ABOVE_C[step];
}

/**
 * The spelling closest to `near` that sounds as `midiPitch` — a guess,
 * made only so a wrong note's dot has *some* sensible vertical position.
 *
 * Tries every letter within a step of `near`'s octave, keeping only those
 * reachable with a double-flat-to-double-sharp accidental (any real
 * pitch always has one), and prefers the smallest jump from `near` on the
 * staff, breaking ties toward the smaller accidental — the reading a
 * performer would find least surprising next to the note they were
 * expecting.
 */
export function nearestSpelling(midiPitch: number, near: Spelling): Spelling {
  let best: { spelling: Spelling; distance: number; alterMagnitude: number } | null = null;

  for (let octaveOffset = -1; octaveOffset <= 1; octaveOffset += 1) {
    for (const step of STEPS) {
      const octave = near.octave + octaveOffset;
      const alter = midiPitch - naturalPitch(step, octave);
      if (alter < -2 || alter > 2) continue;

      const distance = Math.abs(diatonicIndex({ step, octave }) - diatonicIndex(near));
      const alterMagnitude = Math.abs(alter);

      if (
        !best ||
        distance < best.distance ||
        (distance === best.distance && alterMagnitude < best.alterMagnitude)
      ) {
        best = { spelling: { step, octave }, distance, alterMagnitude };
      }
    }
  }

  // Unreachable in practice — every real pitch has a spelling within an
  // octave of any reference — but `near` itself is an honest fallback.
  return best?.spelling ?? near;
}

/**
 * The pair of calibration points furthest apart on the staff, for the
 * least noise-sensitive estimate of pixels-per-step.
 */
function widestPair(
  calibration: readonly Calibration[],
): readonly [Calibration, Calibration] | null {
  let best: readonly [Calibration, Calibration] | null = null;
  let widest = 0;

  for (let i = 0; i < calibration.length; i += 1) {
    for (let j = i + 1; j < calibration.length; j += 1) {
      const a = calibration[i]!;
      const b = calibration[j]!;
      const spread = Math.abs(diatonicIndex(a.spelling) - diatonicIndex(b.spelling));

      if (spread > widest) {
        widest = spread;
        best = [a, b];
      }
    }
  }

  return best;
}

/**
 * The pixel `y` a written `target` would sit at, extrapolated from real
 * measured points on the same staff.
 *
 * Returns `null` when there is nothing to extrapolate from — fewer than
 * two calibration points, or points that do not differ in staff position
 * (a single repeated pitch, or a chord with no other note to compare
 * against). A caller sees that as "nothing to calibrate against" and
 * falls back to a fixed position — an honest degradation, not a defect.
 */
export function verticalPositionFor(
  target: Spelling,
  calibration: readonly Calibration[],
): number | null {
  const pair = widestPair(calibration);
  if (!pair) return null;

  const [a, b] = pair;
  const stepSpread = diatonicIndex(b.spelling) - diatonicIndex(a.spelling);
  if (stepSpread === 0) return null;

  const pixelsPerStep = (b.y - a.y) / stepSpread;
  if (!Number.isFinite(pixelsPerStep)) return null;

  return a.y + pixelsPerStep * (diatonicIndex(target) - diatonicIndex(a.spelling));
}
