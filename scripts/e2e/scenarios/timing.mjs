// M1.1: the session file records when each note was played, how hard, when
// it was released, and the sustain pedal; times run on across a reconnect.
import * as w from "../wd.mjs";
import { check, log, sessionFiles, minuet, startPractising, clickButton, play, kb } from "../lib.mjs";

const GAP = 300; // ms from one press to the next
const LONG = 5; // this note is held 1.5 s
const TOLERANCE = 15; // ms

export default async function timing() {
  const startedAt = Date.now();
  await startPractising();

  // The Minuet's first 8 notes, at velocities 40, 50, ... 110. Pedal down
  // before the second note, up after the long one.
  const notes = minuet.slice(0, 8);
  const velocities = notes.map((_, i) => 40 + 10 * i);
  const holds = notes.map((_, i) => (i === LONG ? 1500 : 100));
  for (const [i, n] of notes.entries()) {
    if (i === 1) kb("cc 64 127");
    kb(`on ${n} ${velocities[i]}`, `wait ${holds[i]}`, `off ${n}`);
    if (i === LONG) kb("cc 64 0");
    kb(`wait ${GAP - 100}`);
  }
  await w.sleep(holds.reduce((a, b) => a + b, 0) + 8 * (GAP - 100) + 500);

  // Unplug and replug, then two more notes.
  kb("close");
  await w.until("return [...document.querySelectorAll('p')].some(p=>/stopped reporting/.test(p.textContent))", "the lost banner", 8000);
  kb("open");
  await w.until("return !([...document.querySelectorAll('p')].some(p=>/stopped reporting/.test(p.textContent)))", "the banner to go", 12000);
  await play(minuet.slice(8, 10), 100);

  await clickButton("Stop");
  await w.until("return document.body.innerText.includes('Practice again')", "the summary", 10000);

  const files = sessionFiles();
  const session = files.at(-1);
  const attempt = session?.attempts.at(-1);
  log("    attempt:", JSON.stringify({ began_ms: attempt?.began_ms, notes: attempt?.notes?.length, pedal: attempt?.pedal }));
  check("the session file is schema 4", session?.schema === 4, String(session?.schema));
  check("every note played is in it", attempt?.notes?.length === 10, String(attempt?.notes?.length));
  if (!attempt?.notes) return;

  const at = attempt.notes.map((n) => n.at_us / 1000);
  const firstGaps = at.slice(1, 8).map((t, i) => t - at[i]);
  const wantedGaps = notes.slice(1).map((_, i) => holds[i] + GAP - 100);
  log("    gaps (ms):", firstGaps.map((g) => g.toFixed(1)).join(" "), " wanted:", wantedGaps.join(" "));
  check("press times increase across the whole attempt, reconnect included", at.every((t, i) => i === 0 || t > at[i - 1]));
  check(`gaps between presses are as played, within ${TOLERANCE} ms`, firstGaps.every((g, i) => Math.abs(g - wantedGaps[i]) <= TOLERANCE));
  check("velocities are exactly as sent", attempt.notes.slice(0, 8).every((n, i) => n.velocity === velocities[i]), JSON.stringify(attempt.notes.slice(0, 8).map((n) => n.velocity)));

  const held = attempt.notes.slice(0, 8).map((n) => (n.released_us - n.at_us) / 1000);
  log("    held (ms):", held.map((h) => h.toFixed(1)).join(" "));
  check(`each release is recorded, as long as it was held within ${TOLERANCE} ms`, held.every((h, i) => Math.abs(h - holds[i]) <= TOLERANCE));

  const pedal = attempt.pedal ?? [];
  const pedalHeld = pedal.length === 2 ? (pedal[1].at_us - pedal[0].at_us) / 1000 : NaN;
  const pedalWanted = at[LONG] + holds[LONG] - at[1];
  check("the pedal went down and came up, at the times it did", pedal.length === 2 && pedal[0].down && !pedal[1].down && Math.abs(pedalHeld - pedalWanted) <= 2 * TOLERANCE, `${pedalHeld.toFixed?.(1)} vs ${pedalWanted.toFixed(1)} ms`);
  check("the attempt's start is recorded on the wall clock", attempt.began_ms >= startedAt && attempt.began_ms <= Date.now(), String(attempt.began_ms));
}
