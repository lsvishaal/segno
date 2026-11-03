// What the scenarios share: where things are, the virtual keyboard, the
// app's private data folder, and the steps every scenario takes.
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import * as w from "./wd.mjs";

export const ROOT = path.resolve(import.meta.dirname, "../..");
export const DIR = `${ROOT}/target/e2e`;
export const APP = `${DIR}/app/squashfs-root/AppRun`;
export const SHOTS = `${DIR}/shots`;
/** The app's data, via XDG_DATA_HOME. The real folder is never written. */
export const DATA = `${DIR}/data/com.segno.practice`;
export const HISTORY = `${DATA}/history`;

export const log = (...a) => console.log(...a);
export const results = [];
export function check(name, ok, detail = "") {
  results.push({ name, ok: Boolean(ok), detail });
  log(`  ${ok ? "PASS" : "FAIL"}  ${name}${detail ? "  — " + detail : ""}`);
}

/** Commands for the virtual keyboard; `run.mjs` points this at its stdin. */
let keyboard = null;
export const useKeyboard = (stdin) => (keyboard = stdin);
export const kb = (...lines) => keyboard.write(lines.join("\n") + "\n");

/** An empty data folder. Never a copy of the owner's: a scenario that needs
 * an import seeds its own (`importScore`). */
export function resetData() {
  fs.rmSync(DATA, { recursive: true, force: true });
  fs.mkdirSync(DATA, { recursive: true });
}

/** Imports `score` into the test data folder before the app starts, through
 * the same store code as the app's import; answers the new piece's id. */
export function importScore(score, title) {
  const args = ["import", DATA, `${ROOT}/${score}`, ...(title ? ["--title", title] : [])];
  return execFileSync(`${ROOT}/target/debug/segno-cli`, args, { encoding: "utf8" }).trim();
}

export const shot = (name) => w.shot(`${SHOTS}/${name}.png`);

/** Waits until no copy of the app is running, so a closing app's last
 * writes can't land while the next one starts or the data folder is cleared. */
export async function appExited(ms = 15000) {
  const until = Date.now() + ms;
  while (spawnSync("pgrep", ["-f", `${DIR}/app/squashfs-root`]).status === 0) {
    if (Date.now() > until) throw new Error(`the app was still running ${ms / 1000} s after it was closed`);
    await w.sleep(100);
  }
}

/** Closes the app and opens it again on the same data: a new day's practice. */
export async function relaunch() {
  await w.stop();
  await appExited();
  await w.start(APP);
}

/** Waits until `test()` holds (read in node: the data folder, say), for at most `ms`; answers whether it did. */
export async function eventually(test, ms = 5000) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    if (await test()) return true;
    await w.sleep(50);
  }
  return false;
}

/** Every saved session file, parsed, oldest name first. */
export function sessionFiles() {
  if (!fs.existsSync(HISTORY)) return [];
  return fs
    .readdirSync(HISTORY)
    .filter((file) => file.endsWith(".json"))
    .sort()
    .map((file) => ({ file, ...JSON.parse(fs.readFileSync(`${HISTORY}/${file}`, "utf8")) }));
}
/** Each session's attempts, as note counts. */
export const sessions = () =>
  sessionFiles().map((s) => ({ file: s.file, piece: s.piece, attempts: s.attempts.map((a) => a.notes.length) }));
export const newSessions = (before) => sessions().filter((s) => !before.some((b) => b.file === s.file));

/** The Minuet's notes in the order the cursor wants them (`just expected`). */
export const minuet = JSON.parse(fs.readFileSync(`${DIR}/minuet.expected.json`, "utf8")).map((n) => n.pitch);

export async function openPiece(title) {
  await w.until("return document.querySelectorAll('li h3').length >= 5", "the library", 30000);
  await w.settle();
  await w.run(
    "[...document.querySelectorAll('li button h3')].find(h=>h.textContent===arguments[0]).closest('button').click(); return 1",
    title,
  );
  await w.until("return document.body.innerText.includes('← Library')", "the practice screen", 20000);
  await w.until("return document.querySelector('.hw-page svg') && document.querySelectorAll('g.note').length > 0", "the score", 20000);
  await w.settle();
}

export async function chooseKeyboard() {
  await w.until(
    "const s=document.querySelector('select[aria-label=\"Instrument\"]'); return s && [...s.options].some(o=>/Test Keyboard/.test(o.value))",
    "the virtual keyboard in the picker",
    10000,
  );
  await w.run(
    `const s=document.querySelector('select[aria-label="Instrument"]');
     const o=[...s.options].find(o=>/Test Keyboard/.test(o.value));
     Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,'value').set.call(s,o.value);
     s.dispatchEvent(new Event('change',{bubbles:true})); return o.value`,
  );
}

export const buttonExists = (label) =>
  w.run("return [...document.querySelectorAll('button')].some(b=>b.textContent.trim()===arguments[0])", label);
export const clickButton = (label) =>
  w.run("const b=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()===arguments[0]); if(!b) return false; b.click(); return true", label);
export const waitForButton = (label, ms = 10000) =>
  w.until(`return [...document.querySelectorAll('button')].some(b=>b.textContent.trim()===${JSON.stringify(label)})`, label, ms);

export async function startPractising(title = "Minuet in G Major") {
  await openPiece(title);
  await chooseKeyboard();
  await clickButton("Start");
  await waitForButton("Stop");
}

export const cursorX = () =>
  w.run("const l=document.querySelector('.hw-cursor-line'); return l ? Math.round(l.getBoundingClientRect().left * 10) / 10 : null");
export const wrongMarks = () => w.run("return document.querySelectorAll('.hw-wrong').length");
export const flashed = () => w.run("return [...document.querySelectorAll('g.note')].filter(n=>n.style.color).length");

/** Plays notes (MIDI numbers) one by one, quickly, then lets the app settle. */
export async function play(notes, gap = 35) {
  for (const n of notes) kb(`on ${n} 80`, `wait ${gap}`, `off ${n}`);
  await w.sleep(gap * notes.length);
  await w.settle();
}

/**
 * Bar `n`'s box on screen, scrolled into view and measured only once it has
 * stopped moving: the same in two frames running. A smooth scroll (the
 * cursor following its line) can still be under way, and a box measured
 * mid-scroll points at another bar.
 */
export const barBox = (n) =>
  w.runAsync(
    `const [n, done] = arguments;
     const m = document.querySelector('.hw-page g.measure[data-bar="' + n + '"]');
     if (!m) return done(null);
     m.scrollIntoView({ block: 'center' });
     const box = () => { const r = m.getBoundingClientRect(); return { left: r.left, right: r.right, top: r.top, bottom: r.bottom }; };
     let last = box(), still = 0;
     const until = performance.now() + 3000;
     const look = () => {
       const now = box();
       still = now.left === last.left && now.top === last.top ? still + 1 : 0;
       last = now;
       if (still >= 2 || performance.now() > until) done(now); else requestAnimationFrame(look);
     };
     requestAnimationFrame(look);`,
    n,
  );

/** The pointer over bar `n`'s blank lower right, as a real mouse would move there. */
export async function hintOver(n) {
  const box = await barBox(n);
  await w.run(
    "document.querySelector('.hw-page').parentElement.parentElement.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientX: arguments[0], clientY: arguments[1] }))",
    box.right - 6,
    box.bottom - 6,
  );
  await w.settle();
  return w.run("return document.querySelector('.hw-page').parentElement.parentElement.getAttribute('title')");
}

/** A real click on bar `n`'s blank lower right (a click anywhere in a bar counts), Shift held if `shift`. */
export async function clickBar(n, shift = false) {
  const box = await barBox(n);
  await w.clickAt(box.right - 6, box.bottom - 6, { shift });
  await w.settle();
}
