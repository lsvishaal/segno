// M1.5: click a bar to start there; Restart returns to it; a click mid-attempt
// keeps the attempt and jumps; clicks wait while the keyboard is lost; leaving
// puts the start back at bar 1.
import fs from "node:fs";
import * as w from "../wd.mjs";
import { DIR, check, log, sessionFiles, openPiece, chooseKeyboard, clickButton, waitForButton, cursorX, kb, shot } from "../lib.mjs";

// The Minuet's expected notes with their bars, in played order.
const expected = JSON.parse(fs.readFileSync(`${DIR}/minuet.expected.json`, "utf8"));
/** The notes of bar `n`'s first occurrence. */
const firstPass = (n) => {
  const start = expected.findIndex((e) => e.bar === n);
  const end = expected.findIndex((e, i) => i > start && e.bar !== n);
  return expected.slice(start, end).map((e) => e.pitch);
};
// Bar 9 is step 36 of the Minuet in played order (`just replay ... --from-bar 9`).
const BAR_9 = 36;

const WRAPPER = "document.querySelector('.hw-page').parentElement.parentElement";

/** Bar `n`'s box on screen, scrolled into view. */
const barBox = (n) =>
  w.run(
    `const m = document.querySelector('.hw-page g.measure[data-bar="' + arguments[0] + '"]');
     if (!m) return null;
     m.scrollIntoView({ block: 'center' });
     const r = m.getBoundingClientRect();
     return { left: r.left, right: r.right, top: r.top, bottom: r.bottom };`,
    n,
  );

/** The hover hint over bar `n`, as the pointer shows it. */
async function hintOver(n) {
  const box = await barBox(n);
  await w.run(
    `${WRAPPER}.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientX: arguments[0], clientY: arguments[1] }))`,
    box.right - 6,
    box.bottom - 6,
  );
  await w.sleep(300);
  return w.run(`return ${WRAPPER}.getAttribute('title')`);
}

async function clickBar(n) {
  await barBox(n);
  await w.sleep(400);
  const box = await barBox(n);
  // Blank paper near the bar's lower right: a click anywhere in the bar counts.
  await w.clickAt(box.right - 6, box.bottom - 6);
  await w.sleep(700);
}

const cursorIn = async (n) => {
  const box = await barBox(n);
  const x = await cursorX();
  return box && x !== null && x >= box.left - 2 && x <= box.right + 2;
};
const marker = () => w.run("const m = document.querySelector('[data-start-marker]'); return m ? m.innerText.trim() : null");
const attempts = () => sessionFiles().flatMap((s) => s.attempts);
async function play(notes) {
  for (const n of notes) kb(`on ${n} 80`, "wait 40", `off ${n}`);
  await w.sleep(45 * notes.length + 500);
}

export default async function startFromBar() {
  await openPiece("Minuet in G Major");

  const hint = await hintOver(9);
  check("hovering bar 9 hints \"Start here (bar 9)\"", hint === "Start here (bar 9)", String(hint));
  check("at the beginning there is no start marker", (await marker()) === null);

  await clickBar(9);
  await shot("start-from-bar-9");
  check("clicking bar 9 before Start marks the start there", (await marker()) === "Start");
  check("and rests the cursor in bar 9", await cursorIn(9));
  check("nothing is saved by moving the start", attempts().length === 0);

  await chooseKeyboard();
  await clickButton("Start");
  await waitForButton("Stop");
  check("Start begins at bar 9", await cursorIn(9));
  const x0 = await cursorX();
  await play(firstPass(9));
  check("playing bar 9's notes moves the cursor on", (await cursorX()) !== x0 && (await cursorIn(10)));

  // Mid-attempt: the attempt so far is kept, a new one starts at bar 3,
  // whose first occurrence is in the first pass of the repeat.
  await clickBar(3);
  check("clicking bar 3 mid-attempt jumps the cursor there", await cursorIn(3));
  check("still practising, with no summary", (await w.bodyText()).includes("Stop") && !(await w.bodyText()).includes("Practice again"));
  const kept = attempts();
  log("    after the jump:", JSON.stringify(kept.map(({ from, reached, ended, notes }) => ({ from, reached, ended, notes: notes.length }))));
  check(
    "the attempt from bar 9 is kept with from = step 36 (not bar 9) and ended = jump",
    kept.length === 1 && kept[0].from === BAR_9 && kept[0].ended === "jump" && kept[0].notes.length === firstPass(9).length,
  );

  await play(firstPass(3).slice(0, 2));
  await w.press("r");
  await w.sleep(800);
  check("R goes back to bar 3, the chosen start", await cursorIn(3));
  check("and keeps the attempt it ended (ended = restart)", attempts().length === 2 && attempts()[1].ended === "restart");

  await clickBar(5);
  await clickBar(7);
  await clickBar(5);
  check("jumping again and again without playing saves nothing", attempts().length === 2, String(attempts().length));
  check("the cursor follows the last click (bar 5)", await cursorIn(5));

  // Keyboard lost: a click must change nothing.
  kb("close");
  await w.until("return [...document.querySelectorAll('p')].some(p=>/stopped reporting/.test(p.textContent))", "the lost banner", 8000);
  const before = await cursorX();
  await clickBar(12);
  check("while the keyboard is lost, a click on a bar changes nothing", (await cursorX()) === before && (await cursorIn(5)));
  check("and saves nothing", attempts().length === 2);
  kb("open");
  await w.until("return !([...document.querySelectorAll('p')].some(p=>/stopped reporting/.test(p.textContent)))", "the banner to go", 12000);

  await play(firstPass(5).slice(0, 1));
  await clickButton("Stop");
  await w.until("return document.body.innerText.includes('Practice again')", "the summary", 10000);
  check("Stop keeps the attempt (ended = stop)", attempts().at(-1)?.ended === "stop" && attempts().length === 3);

  await clickButton("← Library");
  await w.sleep(800);
  await openPiece("Minuet in G Major");
  check("after leaving and reopening, the start is bar 1 again", (await marker()) === null);
  await chooseKeyboard();
  await clickButton("Start");
  await waitForButton("Stop");
  check("and Start begins at bar 1", await cursorIn(1));
}
