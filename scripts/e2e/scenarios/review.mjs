// M1.4: after Stop, the review: marks left on the score, bars not reached
// faded, one column per bar, one line, and a suggestion that loops the
// trouble spot.
import fs from "node:fs";
import * as w from "../wd.mjs";
import { DIR, check, log, openPiece, chooseKeyboard, clickButton, waitForButton, kb, shot, flashed } from "../lib.mjs";

const expected = JSON.parse(fs.readFileSync(`${DIR}/minuet.expected.json`, "utf8"));
/** Bars 1-8 the first time through, each note with its bar. */
const opening = expected.slice(0, expected.findIndex((e) => e.bar === 9));

const columns = () =>
  w.run(
    `return [...document.querySelectorAll('[data-bar-strip] [role=listitem]')].map(b => ({ state: b.dataset.state, line: b.title }))`,
  );
const count = (selector) => w.run(`return document.querySelectorAll('${selector}').length`);
const text = () => w.bodyText();
const loopButton = () =>
  w.run("const b=[...document.querySelectorAll('button')].find(b=>b.textContent.startsWith('⟲')); return b ? b.textContent.trim() : null");

export default async function review() {
  await openPiece("Minuet in G Major");
  await chooseKeyboard();
  await clickButton("Start");
  await waitForButton("Stop");

  // C#5, nowhere in bar 5's first step, before it; four seconds' pause
  // before bar 6's first note.
  const lines = [];
  let total = 0;
  for (const [index, note] of opening.entries()) {
    const first = index === opening.findIndex((e) => e.bar === note.bar);
    if (first && note.bar === 5) lines.push("on 73 80", "wait 40", "off 73");
    if (first && note.bar === 6) {
      lines.push("wait 4000");
      total += 4000;
    }
    lines.push(`on ${note.pitch} 80`, "wait 40", `off ${note.pitch}`);
    total += 45;
  }
  kb(...lines);
  await w.sleep(total + 1500);

  await clickButton("Stop");
  await w.until("return document.body.innerText.includes('Practice again')", "the review", 10000);
  await w.sleep(600);
  await shot("review");

  const body = await text();
  const line = body.split("\n").find((l) => l.includes("most trouble")) ?? "";
  log("    headline:", line);
  check("the line names the trouble: going into bar 6", line.includes("going into bar 6"), line);
  const strip = await columns();
  check("one column per bar, bars 1 to 32", strip.length === 32, String(strip.length));
  check("bars 1-8 played, bars 10 on not reached", strip.slice(0, 8).every((c) => c.state === "played") && strip.slice(9).every((c) => c.state === "notReached"));
  check("bar 6's column says how long the pause was", /Bar 6 · \d+\.\d s pause/.test(strip[5]?.line ?? ""), strip[5]?.line);
  check("bar 5's column counts the wrong note", (strip[4]?.line ?? "").includes("1 wrong"), strip[4]?.line);
  check("the wrong note is crossed on the score", (await count("[data-wrong-mark]")) >= 1);
  check("right notes stay green", (await flashed()) > 0, String(await flashed()));
  check("bars not reached are faded", (await count("[data-not-reached]")) >= 1);
  check("every note's detail waits behind a disclosure", !body.includes("Never played"));
  check("the suggestion: Loop bars 5–6", await w.run("return [...document.querySelectorAll('button')].some(b => b.textContent.trim() === 'Loop bars 5–6')"));

  await clickButton("Loop bars 5–6");
  await waitForButton("Stop");
  await w.sleep(600);
  check("it loops bars 5–6 and starts playing", (await loopButton()) === "⟲ Loop 5–6", String(await loopButton()));
  check("the review's marks are gone", (await count("[data-wrong-mark]")) === 0 && (await count("[data-not-reached]")) === 0);

  await w.press("Escape");
  await w.sleep(400);
  await clickButton("Stop");
  await w.until("return document.body.innerText.includes('Practice again')", "the review", 10000);
  check("stopping with nothing played says so, with no strip", (await text()).includes("Nothing played.") && (await columns()).length === 0);
}
