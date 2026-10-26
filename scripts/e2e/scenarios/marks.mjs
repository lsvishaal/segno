// M2.11: on the Prelude, this attempt's marks stay: right keys green, a wrong
// key in red with a ring. R and each pass of a loop clear them. Also times
// each key, from its MIDI timestamp, to the frame its mark is painted in
// (docs/DESIGN.md budget: < 30 ms, median).
import fs from "node:fs";
import * as w from "../wd.mjs";
import { DIR, check, log, startPractising, flashed, kb, shot, sessionFiles, eventually } from "../lib.mjs";

const prelude = JSON.parse(fs.readFileSync(`${DIR}/prelude.expected.json`, "utf8"));
const bar1 = prelude.filter((e) => e.bar === 1).map((e) => e.pitch);
const live = () => w.run("return document.querySelectorAll('[data-live-wrong]').length");
const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];

export default async function marks() {
  await startPractising("Prelude in C Major, BWV 846");

  // When a mark lands on screen: the frame after a notehead turns green or a
  // wrong key is drawn, on the same wall clock as the presses below.
  await w.run(`
    window.__marked = [];
    let pending = false;
    new MutationObserver((records) => {
      const marked = records.some((r) => (r.type === "attributes" && r.target.matches?.("g.note")) || [...r.addedNodes].some((n) => n.matches?.("[data-live-wrong]")));
      if (!marked || pending) return;
      pending = true;
      requestAnimationFrame(() => { window.__marked.push(Date.now()); pending = false; });
    }).observe(document.querySelector(".hw-score"), { subtree: true, childList: true, attributes: true, attributeFilter: ["style"] });
    return 1`);

  const timed = prelude.slice(0, 40).map((e) => e.pitch);
  for (const pitch of timed) {
    kb(`on ${pitch} 80`, "wait 40", `off ${pitch}`);
    await w.sleep(300);
  }
  const marked = await w.run("return window.__marked");
  // R keeps the attempt: each key's MIDI timestamp, on the wall clock.
  await w.press("r");
  await eventually(() => sessionFiles().at(-1)?.attempts.some((a) => a.notes?.length === timed.length));
  const attempt = sessionFiles().at(-1)?.attempts.find((a) => a.notes?.length === timed.length);
  const keys = (attempt?.notes ?? []).map((n) => attempt.began_ms + n.at_us / 1000);
  const delays = keys.map((at) => Math.round((marked.find((t) => t >= at) ?? Infinity) - at));
  log("    key → mark painted (ms):", delays.join(" "), " median:", median(delays), " max:", Math.max(...delays));
  check("key → mark on the Prelude: median under 30 ms", keys.length === timed.length && median(delays) < 30, `median ${median(delays)} ms, max ${Math.max(...delays)} ms`);

  for (const pitch of timed.slice(0, 8)) kb(`on ${pitch} 80`, "wait 40", `off ${pitch}`, "wait 60");
  kb("on 61 80", "wait 40", "off 61");
  await w.sleep(2000);
  await shot("marks-mid-run");
  check("right notes stay green", (await flashed()) > 0, String(await flashed()));
  check("a wrong key stays, red and ringed", (await live()) === 1 && (await w.run("return !!document.querySelector('[data-live-wrong] [data-wrong-ring]')")));

  await w.press("r");
  await w.settle();
  check("R clears them", (await flashed()) === 0 && (await live()) === 0);

  await w.press("l");
  await w.settle();
  for (const pitch of bar1.slice(0, -1)) kb(`on ${pitch} 80`, "wait 40", `off ${pitch}`, "wait 60");
  await w.sleep(bar1.length * 100);
  await w.settle();
  check("a loop of bar 1: marks while in the pass", (await flashed()) > 0);
  const last = bar1.at(-1);
  kb(`on ${last} 80`, "wait 40", `off ${last}`);
  await w.settle();
  check("and none once the pass wraps", (await flashed()) === 0, String(await flashed()));
  await w.press("Escape");
}
