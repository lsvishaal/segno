/**
 * A sound for every note played.
 *
 * Plain Web Audio — an oscillator per note, shaped by a short envelope —
 * rather than a sampled instrument. Hogwash is offline-first (`NFR-400`,
 * `C-300`), so nothing here is fetched from anywhere, and nothing is bundled
 * either: shipping a realistic piano means shipping licensed samples, which
 * is a real provenance question in the same spirit `PROVENANCE.md` already
 * takes seriously for notation, not a decision to make in passing. That
 * upgrade is recorded in `Backlog_From_Reference.md` rather than built here.
 */

let context: AudioContext | null = null;

/**
 * Unlocks audio playback, if it is not already.
 *
 * Browsers refuse to run an `AudioContext` before a user gesture. Call this
 * from inside a click handler that is already there for another reason
 * (starting an attempt) — never as a dedicated "enable sound" step of its
 * own, which would be a permission the performer didn't come here to grant.
 */
export function unlock(): void {
  context ??= new AudioContext();
  if (context.state === "suspended") void context.resume();
}

/** The frequency, in Hz, of MIDI note `pitch` — A4 (69) is concert pitch. */
function frequencyOf(pitch: number): number {
  return 440 * 2 ** ((pitch - 69) / 12);
}

/**
 * Plays `pitch` once, briefly.
 *
 * Two detuned oscillators (a plain tone reads as an electronic beep, not a
 * struck string) through a short attack and a longer decay — an envelope
 * shape borrowed from how a hammer actually excites a string, not a
 * research-grade physical model of one.
 */
export function play(pitch: number): void {
  if (!context) return; // not unlocked yet — silently does nothing rather than throwing

  const now = context.currentTime;
  const frequency = frequencyOf(pitch);
  const gain = context.createGain();

  gain.gain.setValueAtTime(0, now);
  gain.gain.linearRampToValueAtTime(0.22, now + 0.008);
  gain.gain.exponentialRampToValueAtTime(0.0001, now + 1.1);
  gain.connect(context.destination);

  for (const detune of [0, 4]) {
    const oscillator = context.createOscillator();
    oscillator.type = "triangle";
    oscillator.frequency.value = frequency;
    oscillator.detune.value = detune;
    oscillator.connect(gain);
    oscillator.start(now);
    oscillator.stop(now + 1.15);
  }
}
