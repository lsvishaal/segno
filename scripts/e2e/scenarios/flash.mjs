// With "Marks while playing" off: a correct note's green flash, noteheads
// too, and it always fades, even at tempo.
import * as w from "../wd.mjs";
import { check, minuet, startPractising, flashed, kb, shot } from "../lib.mjs";

export default async function flash() {
  // Read when the practice screen opens.
  await w.run("localStorage.setItem('hogwash.marks', 'off'); return 1");
  await startPractising();

  for (const n of minuet.slice(0, 4)) kb(`on ${n} 80`, "wait 15", `off ${n}`);
  await w.sleep(250);
  const fills = await w.run(
    "return [...document.querySelectorAll('g.note')].filter(n=>n.style.color && n.querySelector('g.notehead use')).map(n=>getComputedStyle(n.querySelector('g.notehead use')).fill)",
  );
  await shot("flash-heads");
  const green = fills.filter((f) => {
    const [r, g] = f.match(/\d+/g)?.map(Number) ?? [];
    return g > r + 30;
  });
  check("noteheads turn green during the flash", fills.length > 0 && green.length === fills.length, JSON.stringify(fills));
  await w.sleep(1200);

  for (const n of minuet.slice(4, 8)) {
    kb(`on ${n} 80`, "wait 20", `off ${n}`);
    await w.sleep(700);
  }
  await w.sleep(800);
  check("700 ms apart: no note is left green", (await flashed()) === 0);

  for (const n of minuet.slice(8, 21)) kb(`on ${n} 80`, "wait 20", `off ${n}`, "wait 230");
  await w.sleep(13 * 250 + 1500);
  check("250 ms apart (about 120 bpm): no note is left green", (await flashed()) === 0, `stuck=${await flashed()}`);
}
