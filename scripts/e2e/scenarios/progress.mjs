// M2.10: the Progress page, from history made by playing the real Minuet
// on the virtual keyboard over three launches of the app (three sessions),
// with mistakes made on purpose. Then Practise from it, and a piece never
// played.
import fs from "node:fs";
import * as w from "../wd.mjs";
import { DIR, check, log, openPiece, chooseKeyboard, clickButton, waitForButton, kb, shot, relaunch, cursorX } from "../lib.mjs";

const expected = JSON.parse(fs.readFileSync(`${DIR}/minuet.expected.json`, "utf8"));
const nine = expected.findIndex((e) => e.bar === 9);
const opening = expected.slice(0, nine);

/**
 * Bars 1–8 (twice: the repeat), `pause` ms before each bar 6, a wrong key
 * (a semitone up) before bar 3's first note if `wrong`, and the first note
 * of bar 9 if `into9`.
 */
async function session({ pause, wrong = false, into9 = false }) {
  await openPiece("Minuet in G Major");
  await chooseKeyboard();
  await clickButton("Start");
  await waitForButton("Stop");
  const lines = [];
  let waited = 0;
  for (const [index, note] of opening.entries()) {
    const before = opening[index - 1]?.bar;
    if (note.bar === 6 && before === 5) {
      lines.push(`wait ${pause}`);
      waited += pause;
    }
    if (wrong && note.bar === 3 && before === 2 && index < opening.length / 2) {
      lines.push(`on ${note.pitch + 1} 80`, "wait 40", `off ${note.pitch + 1}`, "wait 200");
    }
    lines.push(`on ${note.pitch} 80`, "wait 40", `off ${note.pitch}`, "wait 160");
  }
  const notes = into9 ? [...opening, expected[nine]] : opening;
  if (into9) lines.push(`on ${expected[nine].pitch} 80`, "wait 40", `off ${expected[nine].pitch}`);
  kb(...lines);
  await w.sleep(notes.length * 200 + waited + 1500);
  await clickButton("Stop");
  await w.until("return !!document.querySelector('section[aria-label=Results]')", "the results", 10000);
}

/** The Progress link on `title`'s shelf card, as it is drawn now. */
const cardLink = (title) =>
  w.run(
    `const b=[...document.querySelectorAll('li button')].find(b=>b.getAttribute('aria-label')==='Progress for '+arguments[0]);
     if(!b) return null;
     let opacity=1; for(let e=b;e;e=e.parentElement) opacity*=Number(getComputedStyle(e).opacity);
     const r=b.getBoundingClientRect(); return { opacity, width: r.width, height: r.height }`,
    title,
  );
const openProgress = async (title) => {
  await w.run(
    "[...document.querySelectorAll('li button')].find(b=>b.getAttribute('aria-label')==='Progress for '+arguments[0]).click(); return 1",
    title,
  );
  await w.until("return !!document.querySelector('[data-progress]')", "the Progress page", 10000);
  await w.until("return !!document.querySelector('[data-progress] h1')", "its header", 10000);
  await w.settle();
};
const toLibrary = async () => {
  await clickButton("← Library");
  await w.until("return document.querySelectorAll('li h3').length >= 6", "the library", 10000);
  await w.settle();
};

export default async function progress() {
  // Three days of practice: bar 6 hesitated before (4 s, 4 s, then 1 s), one
  // wrong key in bar 3 on the first day, and the last day stops in bar 9.
  await session({ pause: 4000, wrong: true });
  await relaunch();
  await session({ pause: 4000 });
  await relaunch();
  await session({ pause: 1000, into9: true });
  await relaunch();

  await w.until("return document.querySelectorAll('li h3').length >= 6", "the library", 30000);
  await w.settle();
  const link = await cardLink("Minuet in G Major");
  check("the card's Progress link shows without hovering", link && link.opacity === 1 && link.width > 0, JSON.stringify(link));
  await shot("progress-library");

  await openProgress("Minuet in G Major");
  const facts = await w.run("return document.querySelector('[data-progress-facts]')?.textContent ?? ''");
  check("the header counts 3 sessions and says when", /^3 sessions · .+ · last practised today$/.test(facts), facts);
  const rows = await w.run("return document.querySelectorAll('[data-bar-column=\"1\"] [data-cell]').length");
  check("one row per session", rows === 3, String(rows));
  const now = await w.run(
    "return [...document.querySelectorAll('[data-bar-column]')].map(c=>({bar:Number(c.dataset.barColumn), now:c.querySelector('[data-now]').dataset.now, h:c.querySelector('[data-now] > span')?.getBoundingClientRect().height ?? 0})).filter(c=>c.now!=='')",
  );
  log("    now:", JSON.stringify(now));
  const tallest = now.reduce((best, bar) => (bar.h > best.h ? bar : best), { h: -1 });
  check("bar 6 is the tallest now", tallest.bar === 6, `bar ${tallest.bar}`);
  check("bar 9, played once, is a dotted stub", now.find((bar) => bar.bar === 9)?.now === "null");
  const red = await w.run(
    "return [...document.querySelectorAll('[data-progress] *')].filter(e=>{const c=getComputedStyle(e).backgroundColor; return /rgb\\(220, 38, 38/.test(c)}).length",
  );
  check("no red on the page", red === 0, String(red));
  const first = await w.run("return document.querySelector('[data-work-on-next]')?.textContent ?? ''");
  check("Work on next starts with bar 6, practised from bar 5", /^Bar 6 · avg pause [\d.]+ s.*⟲ Practise bars 5–6$/.test(first), first);
  const listed = await w.run("return document.querySelectorAll('[data-work-on-next]').length");
  check("at most 3 passages", listed >= 1 && listed <= 3, String(listed));
  await shot("progress-minuet");

  await w.run("document.querySelector('[data-bar-column=\"6\"]').click(); return 1");
  await w.settle();
  const story = await w.run("return document.querySelector('[data-bar-detail]')?.textContent ?? ''");
  check(
    "bar 6's story: earliest against now, over 3 sessions",
    /^Bar 6 · avg pause [\d.]+ s → [\d.]+ s over 3 sessions · wrong notes [\d.]+ → [\d.]+ per pass⟲ Practise bars 5–6$/.test(story),
    story,
  );
  const marked = await w.run("return document.querySelector('[data-bar-column=\"6\"]').getAttribute('aria-pressed')");
  check("its column is marked", marked === "true");
  await shot("progress-bar-detail");

  // Practise: opens the piece looping 5–6 and starts on the keyboard last
  // started on, without Start.
  await w.run("document.querySelector('[data-work-on-next] button').click(); return 1");
  await w.until("return document.body.innerText.includes('← Library')", "the practice screen", 20000);
  await waitForButton("Stop", 20000);
  check("Practise starts at once, no Start pressed", true);
  check("looping bars 5–6", await w.run("return [...document.querySelectorAll('button')].some(b=>b.textContent.trim()==='⟲ Loop 5–6')"));
  // The cursor moves on a key from the virtual keyboard: the attempt is on it.
  await w.settle();
  const before = await cursorX();
  // Bar 5 opens on a chord: its first few keys move the cursor on.
  for (const { pitch } of expected.filter((e) => e.bar === 5).slice(0, 3)) {
    kb(`on ${pitch} 80`, "wait 40", `off ${pitch}`, "wait 100");
  }
  await w.sleep(400);
  const after = await cursorX();
  check("on the virtual keyboard, remembered across launches", before !== null && after !== before, `${before} → ${after}`);
  await clickButton("Stop");
  await w.settle();

  await toLibrary();
  await openProgress("Lightly Row");
  const empty = await w.run("return document.querySelector('[data-progress-empty]')?.textContent ?? ''");
  check("a piece never played: one line and Practise", empty === "Play this piece once and your progress shows here.Practise", empty);
  check("and no grid", (await w.run("return document.querySelectorAll('[data-bar-column]').length")) === 0);
  await shot("progress-empty");
}
