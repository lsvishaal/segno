// What the scenarios share: where things are, the virtual keyboard, the
// app's private data folder, and the steps every scenario takes.
import fs from "node:fs";
import path from "node:path";
import * as w from "./wd.mjs";

export const ROOT = path.resolve(import.meta.dirname, "../..");
export const DIR = `${ROOT}/target/e2e`;
export const APP = `${DIR}/app/squashfs-root/AppRun`;
export const SHOTS = `${DIR}/shots`;
/** The app's data, via XDG_DATA_HOME. The real folder is never written. */
export const DATA = `${DIR}/data/com.hogwash.practice`;
export const HISTORY = `${DATA}/history`;
const REAL = `${process.env.HOME}/.local/share/com.hogwash.practice`;

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

/** An empty data folder, or (`withMine`) a copy of the real one. */
export function resetData({ withMine = false } = {}) {
  fs.rmSync(DATA, { recursive: true, force: true });
  if (withMine && fs.existsSync(REAL)) fs.cpSync(REAL, DATA, { recursive: true });
  else fs.mkdirSync(DATA, { recursive: true });
}

export const shot = (name) => w.shot(`${SHOTS}/${name}.png`);

/** Every saved session file, parsed, oldest name first. */
export function sessionFiles() {
  if (!fs.existsSync(HISTORY)) return [];
  return fs
    .readdirSync(HISTORY)
    .filter((file) => file.endsWith(".json"))
    .sort()
    .map((file) => ({ file, ...JSON.parse(fs.readFileSync(`${HISTORY}/${file}`, "utf8")) }));
}
/** Each session's attempts, as note counts (schema 1 or 2). */
export const sessions = () =>
  sessionFiles().map((s) => ({ file: s.file, piece: s.piece, attempts: s.attempts.map((a) => (a.notes ?? a.played).length) }));
export const newSessions = (before) => sessions().filter((s) => !before.some((b) => b.file === s.file));

/** The Minuet's notes in the order the cursor wants them (`just expected`). */
export const minuet = JSON.parse(fs.readFileSync(`${DIR}/minuet.expected.json`, "utf8")).map((n) => n.pitch);

export async function openPiece(title) {
  await w.until("return document.querySelectorAll('li h3').length >= 6", "the library", 30000);
  await w.sleep(1000);
  await w.run(
    "[...document.querySelectorAll('li button h3')].find(h=>h.textContent===arguments[0]).closest('button').click(); return 1",
    title,
  );
  await w.until("return document.body.innerText.includes('← Library')", "the practice screen", 20000);
  await w.until("return document.querySelector('.hw-page svg') && document.querySelectorAll('g.note').length > 0", "the score", 20000);
  await w.sleep(500);
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
  await w.sleep(gap * notes.length + 400);
}
