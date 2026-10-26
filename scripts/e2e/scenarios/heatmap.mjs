// M2.3: bars that have been hard over time are tinted amber on the score
// while not playing, by absolute level (D10); W hides them.
import fs from "node:fs";
import * as w from "../wd.mjs";
import { DIR, check, log, openPiece, chooseKeyboard, clickButton, waitForButton, kb, shot } from "../lib.mjs";

const expected = JSON.parse(fs.readFileSync(`${DIR}/minuet.expected.json`, "utf8"));
const opening = expected.slice(0, expected.findIndex((e) => e.bar === 9));

/** Bars 1–8, with `pause` ms before bar 6's first note. */
async function play(pause) {
  const lines = [];
  for (const [index, note] of opening.entries()) {
    if (note.bar === 6 && opening[index - 1]?.bar === 5) lines.push(`wait ${pause}`);
    lines.push(`on ${note.pitch} 80`, "wait 40", `off ${note.pitch}`);
  }
  kb(...lines);
  await w.sleep(opening.length * 45 + pause + 1500);
}

const tints = () =>
  w.run(
    "const bars = new Map([...document.querySelectorAll('g.measure')].map(m => [m.getBoundingClientRect(), m.dataset.bar])); return [...document.querySelectorAll('[data-weak-bar]')].map(t => { const r = t.getBoundingClientRect(); let bar = null; for (const [b, n] of bars) if (Math.abs(b.left - r.left) < 2 && Math.abs(b.top - r.top) < 2) bar = n; return { bar, level: Number(t.dataset.weakBar) }; })",
  );
const legend = () => w.run("const l = document.querySelector('[data-weak-legend]'); return l ? l.textContent : null");
const press = (key) => w.run(`window.dispatchEvent(new KeyboardEvent('keydown', { key: '${key}' }))`);
const untilResults = () => w.until("return !!document.querySelector('section[aria-label=Results]')", "the results", 10000);

export default async function heatmap() {
  await openPiece("Minuet in G Major");
  await chooseKeyboard();
  await clickButton("Start");
  await waitForButton("Stop");

  for (let run = 0; run < 2; run++) {
    await play(4000);
    await clickButton("Stop");
    await untilResults();
    await press("r");
    await waitForButton("Stop");
  }
  await w.sleep(800);
  await shot("heatmap");

  const shown = await tints();
  log("    tints:", JSON.stringify(shown));
  // 4 s before bar 6 in each of two runs: 4 s of pause per pass is level 2.
  check("only bar 6 is tinted, at level 2", shown.length === 1 && shown[0].bar === "6" && shown[0].level === 2, JSON.stringify(shown));
  check("the legend says what the amber means", /Weak bars from \d+ runs · lighter = easier for you, deeper = harder/.test((await legend()) ?? ""), await legend());

  await press("w");
  await w.sleep(200);
  check("W hides them", (await tints()).length === 0 && (await legend()) === null);
  await press("w");
  await w.sleep(200);
  check("W shows them again", (await tints()).length === shown.length);

  kb("on 67 80", "wait 40", "off 67");
  await w.sleep(400);
  check("off while playing", (await tints()).length === 0 && (await legend()) === null);
  await clickButton("Stop");
}
