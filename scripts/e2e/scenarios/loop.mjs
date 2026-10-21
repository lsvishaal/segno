// M1.6: Shift+click loops bars; each pass is kept; a wrong note drops the
// count at once; the target ends the loop with the cursor at its start; L
// and Esc; a loop over the repeat sign.
import fs from "node:fs";
import * as w from "../wd.mjs";
import { DIR, check, log, sessionFiles, openPiece, chooseKeyboard, clickButton, waitForButton, cursorX, kb, shot } from "../lib.mjs";

const expected = JSON.parse(fs.readFileSync(`${DIR}/minuet.expected.json`, "utf8"));
/** The notes of bar `n`'s first occurrence. */
const firstPass = (n) => {
  const start = expected.findIndex((e) => e.bar === n);
  const end = expected.findIndex((e, i) => i > start && e.bar !== n);
  return expected.slice(start, end).map((e) => e.pitch);
};
const PASS = [...firstPass(9), ...firstPass(10)];
// Bars 9-10 of the Minuet are steps 36 to 43 (`just replay ... --loop 9-10`).
const FROM = 36;
const TO = 44;

const barBox = (n) =>
  w.run(
    `const m = document.querySelector('.hw-page g.measure[data-bar="' + arguments[0] + '"]');
     if (!m) return null;
     m.scrollIntoView({ block: 'center' });
     const r = m.getBoundingClientRect();
     return { left: r.left, right: r.right, top: r.top, bottom: r.bottom };`,
    n,
  );
async function clickBar(n, shift = false) {
  await barBox(n);
  await w.sleep(400);
  const box = await barBox(n);
  await w.clickAt(box.right - 6, box.bottom - 6, { shift });
  await w.sleep(700);
}
const cursorIn = async (n) => {
  const box = await barBox(n);
  const x = await cursorX();
  return box && x !== null && x >= box.left - 2 && x <= box.right + 2;
};
const bands = () => w.run("return document.querySelectorAll('[data-loop-band]').length");
const loopButton = () =>
  w.run("const b=[...document.querySelectorAll('button')].find(b=>b.textContent.startsWith('⟲')); return b ? b.textContent.trim() : null");
const counter = () => w.run("return [...document.querySelectorAll('[role=status]')].map(e=>e.textContent.trim()).join(' | ')");
const passes = () => sessionFiles().flatMap((s) => s.attempts).filter((a) => a.ended === "pass");
async function play(notes) {
  for (const n of notes) kb(`on ${n} 80`, "wait 40", `off ${n}`);
  await w.sleep(45 * notes.length + 600);
}

export default async function loop() {
  await openPiece("Minuet in G Major");
  await chooseKeyboard();
  await clickButton("Start");
  await waitForButton("Stop");

  await clickBar(9);
  await clickBar(10, true);
  await shot("loop-9-10");
  check("Shift+click loops bars 9-10: the button says so", (await loopButton()) === "⟲ Loop 9–10", String(await loopButton()));
  check("a band lies behind both bars", (await bands()) >= 2, String(await bands()));
  check("the cursor waits at bar 9", await cursorIn(9));
  check("the count starts at 0", (await counter()).includes("0 clean in a row"), await counter());

  await play(PASS);
  check("a clean pass: 1 clean in a row, back at bar 9", (await counter()).includes("1 clean in a row") && (await cursorIn(9)), await counter());

  // C#5 is nowhere in bar 9's first step.
  await play([PASS[0], 73]);
  check("a wrong note drops the count to 0 at once", (await counter()).includes("0 clean in a row"), await counter());
  await play(PASS.slice(1));
  check("and that pass does not count", (await counter()).includes("0 clean in a row") && (await cursorIn(9)), await counter());

  await play(PASS);
  await play(PASS);
  check("two clean passes: 2 clean in a row", (await counter()).includes("2 clean in a row"), await counter());
  await play(PASS);
  check("the third ends the loop: \"3 clean passes · bars 9–10\"", (await counter()).includes("3 clean passes · bars 9–10"), await counter());
  check("the band is gone", (await bands()) === 0);
  check("the cursor waits at bar 9", await cursorIn(9));

  const kept = passes();
  log("    passes kept:", JSON.stringify(kept.map(({ from, to, reached, notes }) => ({ from, to, reached, notes: notes.length }))));
  check("all five passes are kept, ended = pass", kept.length === 5, String(kept.length));
  check(
    "each from step 36 to 44 (positions, not bars), reaching its end",
    kept.every((a) => a.from === FROM && a.to === TO && a.reached === TO),
  );
  check("the spoiled pass kept its wrong note", kept[1]?.notes.some((n) => n.key === 73));

  await w.press("l");
  await w.sleep(700);
  check("L loops the bar the cursor is in (bar 9)", (await loopButton()) === "⟲ Loop 9", String(await loopButton()));
  await w.press("Escape");
  await w.sleep(500);
  check("Esc stops looping", (await loopButton()) === "⟲ Loop" && (await bands()) === 0, String(await loopButton()));

  // Bars 15-18 cross the repeat sign at the end of bar 16.
  await clickBar(15);
  await clickBar(18, true);
  check("a loop over the repeat sign: bars 15–18", (await loopButton()) === "⟲ Loop 15–18", String(await loopButton()));
  check("starting in bar 15", await cursorIn(15));
  await play(firstPass(15));
  check("playing bar 15 moves on into bar 16", await cursorIn(16));

  await clickBar(3);
  check("a plain click ends the loop", (await loopButton()) === "⟲ Loop" && (await bands()) === 0);
  await clickButton("Stop");
  await w.until("return document.body.innerText.includes('Practice again')", "the summary", 10000);
}
