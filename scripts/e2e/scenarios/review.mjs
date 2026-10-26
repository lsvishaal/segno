// M1.4: after Stop, the results replace the score: big numbers, a point per
// bar (or per pass of a loop), the small numbers, one line, and what to do
// next. Esc shows the score with the run's marks left on; Enter takes the
// suggestion. A loop run, a too-short run and nothing played, too.
import fs from "node:fs";
import * as w from "../wd.mjs";
import { DIR, check, log, openPiece, chooseKeyboard, clickButton, waitForButton, kb, shot, flashed, clickBar } from "../lib.mjs";

const expected = JSON.parse(fs.readFileSync(`${DIR}/minuet.expected.json`, "utf8"));
/** Bars 1-8 the first time through, each note with its bar. */
const opening = expected.slice(0, expected.findIndex((e) => e.bar === 9));
const notesOf = (bar) => opening.filter((e) => e.bar === bar).map((e) => e.pitch);

const results = () =>
  w.run("const r = document.querySelector('section[aria-label=Results]'); return r ? r.innerText : null");
const points = () =>
  w.run(
    "return [...document.querySelectorAll('section[aria-label=Results] [role=listitem]')].map(b => ({ line: b.getAttribute('aria-label'), reached: b.dataset.reached }))",
  );
const count = (selector) => w.run(`return document.querySelectorAll('${selector}').length`);
const status = () => w.run("const s = document.querySelector('header [role=status]'); return s ? s.textContent : ''");
const loopButton = () =>
  w.run("const b=[...document.querySelectorAll('button')].find(b=>b.textContent.startsWith('⟲ Loop')); return b ? b.textContent.trim() : null");
const untilResults = () => w.until("return !!document.querySelector('section[aria-label=Results]')", "the results", 10000);

/** Plays `notes` (each `{ pitch, bar }`), waiting `pauseBefore[bar]` ms before a bar's first note, and `wrongBefore` bars' first notes get C#5 first. */
async function perform(notes, { pauseBefore = {}, wrongBefore = [] } = {}) {
  const lines = [];
  let total = 0;
  const seen = new Set();
  for (const note of notes) {
    if (!seen.has(note.bar)) {
      seen.add(note.bar);
      if (wrongBefore.includes(note.bar)) lines.push("on 73 80", "wait 40", "off 73");
      if (pauseBefore[note.bar]) {
        lines.push(`wait ${pauseBefore[note.bar]}`);
        total += pauseBefore[note.bar];
      }
    }
    lines.push(`on ${note.pitch} 80`, "wait 40", `off ${note.pitch}`);
    total += 45;
  }
  kb(...lines);
  await w.sleep(total + 1500);
}

export default async function review() {
  await openPiece("Minuet in G Major");
  await chooseKeyboard();
  await clickButton("Start");
  await waitForButton("Stop");

  // Bars 1-8, a wrong C#5 in bar 5, four seconds' pause before bar 6.
  await perform(opening, { pauseBefore: { 6: 4000 }, wrongBefore: [5] });
  await clickButton("Stop");
  await untilResults();
  await w.sleep(500);
  await shot("results");

  const text = (await results()) ?? "";
  log("    results:", JSON.stringify(text));
  check("the results replace the score", text.length > 0 && (await w.run("return getComputedStyle(document.querySelector('.hw-score')).visibility")) === "hidden");
  check("the top line says what was played and how long", /^Bars 1–8 · \d+ s/.test(text), text.split("\n")[0]);
  check("the numbers in one quiet line: accuracy and tempo", /accuracy \d+%/.test(text) && /♩ \d+/.test(text), text);
  const cards = await w.run(
    "return [...document.querySelectorAll('section[aria-label=Results] article')].map(a => ({ kind: a.dataset.card, title: a.getAttribute('aria-label'), text: a.innerText, cut: !!a.querySelector('[data-bar-cutout] svg'), pause: !!a.querySelector('[data-pause-mark]'), rings: a.querySelectorAll('[data-wrong-ring]').length }))",
  );
  log("    cards:", JSON.stringify(cards.map(({ kind, title, cut, pause, rings }) => ({ kind, title, cut, pause, rings }))));
  check("card 1 is the trouble going into bar 6", cards[0]?.title === "Going into bar 6", cards[0]?.title);
  check("it says what happened, why, and how to fix it", /You paused \d+\.\d s before bar 6\./.test(cards[0]?.text ?? "") && (cards[0]?.text ?? "").includes("isn't automatic yet") && (cards[0]?.text ?? "").includes("Play the last beat of bar 5"), cards[0]?.text);
  check("it shows bars 5–6 cut out of the score, the pause marked and the wrong note ringed", cards[0]?.cut && cards[0]?.pause && cards[0]?.rings >= 1, JSON.stringify(cards[0] && { cut: cards[0].cut, pause: cards[0].pause, rings: cards[0].rings }));
  check("the wrong key in bar 5 is its own card, said as where it landed", cards[1]?.title === "Wrong notes in bar 5" && (cards[1]?.text ?? "").includes("one key too high"), cards[1]?.text);
  check("at most three cards", cards.length >= 1 && cards.length <= 3, String(cards.length));
  const chart = await points();
  check("a point per bar, bars 1 to 32", chart.length === 32, String(chart.length));
  check("bars 9 on not reached", chart.slice(8).every((p) => p.reached === "false") && chart.slice(0, 8).every((p) => p.reached === "true"));
  check("bar 6's point says its pause", /Bar 6 · ♩ = \d+ · \d+\.\d s pause/.test(chart[5]?.line ?? ""), chart[5]?.line);
  check("bar 5's point counts the wrong note", (chart[4]?.line ?? "").includes("1 wrong"), chart[4]?.line);
  check("the chart crosses the wrong note", (await count("[data-chart-wrong]")) === 1);
  check("its button: Practise bars 5–6", await w.run("return [...document.querySelectorAll('button')].some(b => b.textContent.trim() === '⟲ Practise bars 5–6')"));

  await w.press("Escape");
  await w.sleep(500);
  await shot("results-on-the-score");
  check("Esc shows the score with the run marked", (await results()) === null && (await status()).includes("Your last run is marked on the score"));
  check("the run's marks stay: the wrong key ringed, right notes green, bars not reached faded", (await count("[data-live-wrong]")) >= 1 && (await flashed()) > 0 && (await count("[data-not-reached]")) >= 1);
  await w.press("Escape");
  await w.sleep(500);
  check("Esc again goes back to the results", (await results()) !== null);

  await w.press("Enter");
  await waitForButton("Stop");
  await w.sleep(600);
  check("Enter loops bars 5–6 and starts playing", (await loopButton()) === "⟲ Loop 5–6", String(await loopButton()));
  check("a new attempt starts with a clean score", (await results()) === null && (await count("[data-live-wrong]")) === 0 && (await flashed()) === 0);

  // Two clean passes of bars 5-6, then Stop part way into a third.
  const loopNotes = [...opening.filter((e) => e.bar === 5), ...opening.filter((e) => e.bar === 6)];
  await perform(loopNotes);
  await perform(loopNotes);
  await perform([{ pitch: notesOf(5)[0], bar: 5 }]);
  await clickButton("Stop");
  await untilResults();
  await w.sleep(500);
  await shot("results-loop");
  const loopText = (await results()) ?? "";
  log("    loop results:", JSON.stringify(loopText));
  check("a loop run's line: 2 passes, 2 clean", loopText.includes("Bars 5–6 · 2 passes · 2 clean"));
  check("a point per pass", (await points()).length === 2 && (await points())[0].line.startsWith("Pass 1"));

  // A note is too little to say much about. (Practice again carries the
  // loop on; Esc while practising stops it.)
  await clickButton("↻ Practice again");
  await waitForButton("Stop");
  await w.press("Escape");
  await w.sleep(300);
  await clickBar(1);
  await perform([{ pitch: notesOf(1)[0], bar: 1 }]);
  await clickButton("Stop");
  await untilResults();
  await w.sleep(400);
  await shot("results-too-short");
  check("too short: says so, and no cards", ((await results()) ?? "").includes("Play a few bars more to get feedback.") && (await count("section[aria-label=Results] article")) === 0);

  await clickButton("↻ Practice again");
  await waitForButton("Stop");
  await clickButton("Stop");
  await waitForButton("Start");
  await w.sleep(400);
  check("nothing played: no results, a quiet note, Start again", (await results()) === null && (await status()).includes("Nothing played."), JSON.stringify({ results: await results(), status: await status() }));
}
