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
  await w.until("return document.querySelectorAll('li h3').length >= 5", "the library", 10000);
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

  await w.until("return document.querySelectorAll('li h3').length >= 5", "the library", 30000);
  await w.settle();
  const link = await cardLink("Minuet in G Major");
  check("the card's Progress link shows without hovering", link && link.opacity === 1 && link.width > 0, JSON.stringify(link));
  await shot("progress-library");

  await openProgress("Minuet in G Major");
  const facts = await w.run("return document.querySelector('[data-progress-facts]')?.textContent ?? ''");
  check("the header counts 3 sessions and says when", /^Practised 3 times · .+ in total · last practised today$/.test(facts), facts);
  const map = await w.run(
    "return [...document.querySelectorAll('[data-bar]')].map(b=>({bar:Number(b.dataset.bar), level:b.dataset.level, label:b.getAttribute('aria-label'), text:b.textContent}))",
  );
  log("    map:", JSON.stringify(map.filter((bar) => bar.level !== "")));
  check("one box per bar", map.length === 32, String(map.length));
  check("every played box shows its word", map.every((bar) => bar.level === "" || bar.text.endsWith(bar.label.split(": ")[1])));
  const worst = map.reduce((best, bar) => (bar.level !== "" && bar.level !== "null" && Number(bar.level) > Number(best.level) ? bar : best), { level: "-1" });
  check("bar 6 is the hardest now, in amber words", worst.bar === 6 && /Small slips|Uneven|Shaky|Needs work/.test(worst.label), worst.label);
  check("bar 9, played once, says not enough yet", map.find((bar) => bar.bar === 9)?.label === "Bar 9: Not enough yet");
  check("the legend explains the words", /Secure.*Needs work.*Not enough yet.*Blank/.test(await w.run("return document.querySelector('[data-legend]')?.textContent ?? ''")));
  const red = await w.run(
    "return [...document.querySelectorAll('[data-progress] *')].filter(e=>{const c=getComputedStyle(e).backgroundColor; return /rgb\\(220, 38, 38/.test(c)}).length",
  );
  check("no red on the page", red === 0, String(red));
  const first = await w.run("return document.querySelector('[data-work-on-next]')?.textContent ?? ''");
  check("Practise next starts with bar 6, practised from bar 5", /^Bar 6 — about [\d.]+ s of hesitation each time through.*⟲ Practise bars 5–6$/.test(first), first);
  const listed = await w.run("return document.querySelectorAll('[data-work-on-next]').length");
  check("at most 3 passages", listed >= 1 && listed <= 3, String(listed));
  const opened = await w.run("return document.querySelector('[data-bar-detail] h3')?.textContent ?? ''");
  check("opens on bar 6 over time", opened === "Bar 6 over time", opened);
  await shot("progress-minuet");

  await w.run("document.querySelector('[data-bar=\"3\"]').click(); return 1");
  await w.settle();
  const story = await w.run("return document.querySelector('[data-bar-detail]')?.textContent ?? ''");
  check("bar 3's sentence: now, then its first sessions", /Bar 3 over timeNow: .+\. In your first sessions: .+\./.test(story), story);
  const points = await w.run("return document.querySelectorAll('[data-bar-detail] [data-line-graph]')[1]?.querySelectorAll('[data-point]').length ?? 0");
  check("its wrong-notes graph has a dot per session", points === 3, String(points));
  const marked = await w.run("return document.querySelector('[data-bar=\"3\"]').getAttribute('aria-pressed')");
  check("its box is marked", marked === "true");
  await w.run("document.querySelector('[data-bar-detail]').scrollIntoView({block:'end'}); return 1");
  await w.settle();
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
  check("and no map", (await w.run("return document.querySelectorAll('[data-piece-map]').length")) === 0);
  await shot("progress-empty");
}
