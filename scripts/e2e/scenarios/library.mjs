// The library: shelves, then rename (Enter, Esc, blank) and delete
// (confirmation, history) on a piece imported for the test (the bundled
// Minuet's file, imported under its own title) and played once.
import fs from "node:fs";
import * as w from "../wd.mjs";
import { check, DATA, HISTORY, shot, importScore, openPiece, chooseKeyboard, clickButton, waitForButton, play, minuet, eventually } from "../lib.mjs";

const IMPORTED = "Minuet in G (imported)";
let id = null;
export function seed() {
  id = importScore("app/src-tauri/library/minuet-in-g.musicxml", IMPORTED);
}

const titles = () => w.run("return [...document.querySelectorAll('li h3')].map(h=>h.textContent)");
const cardButtons = (title) =>
  w.run(
    "const li=[...document.querySelectorAll('li')].find(l=>l.querySelector('h3')?.textContent===arguments[0]); return li ? [...li.querySelectorAll('button')].map(b=>b.getAttribute('aria-label')||b.textContent.trim()) : null",
    title,
  );
const clickLabel = (label) => w.run("const b=document.querySelector(`button[aria-label=\"${arguments[0]}\"]`); if(!b) return false; b.click(); return true", label);
const setInput = (value) =>
  w.run(
    "const i=document.querySelector('li input'); i.focus(); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(i,arguments[0]); i.dispatchEvent(new Event('input',{bubbles:true})); return i.value",
    value,
  );
const stored = (id) => JSON.parse(fs.readFileSync(`${DATA}/scores/${id}.json`, "utf8"));
const scoreIds = () => (fs.existsSync(`${DATA}/scores`) ? fs.readdirSync(`${DATA}/scores`).filter((f) => f.endsWith(".json")).map((f) => f.slice(0, -5)) : []);
const sessionsOf = (id) => (fs.existsSync(HISTORY) ? fs.readdirSync(HISTORY).filter((f) => f.startsWith(`${id}-`)).length : 0);

export default async function library() {
  await w.until("return document.querySelectorAll('li h3').length >= 5", "the library", 30000);
  await w.settle();
  await shot("library");
  const shelves = await w.run("return [...document.querySelectorAll('section[aria-label]')].map(s => s.getAttribute('aria-label'))");
  check("the library is in shelves: your pieces, then the starters by level", ["Your pieces", "Starter · Beginner", "Starter · Intermediate"].every((label) => shelves.includes(label)), JSON.stringify(shelves));

  check("the imported piece is on the shelf", (await titles()).includes(IMPORTED));

  // Played once, so deleting it has history to take.
  await openPiece(IMPORTED);
  await chooseKeyboard();
  await clickButton("Start");
  await waitForButton("Stop");
  await play(minuet.slice(0, 4));
  await clickButton("← Library");
  await w.until("return document.querySelectorAll('li h3').length >= 6", "the library", 30000);
  await w.settle();
  check("playing it kept one session under its id", await eventually(() => sessionsOf(id) === 1), `${sessionsOf(id)}`);

  check("bundled pieces offer no Rename or Delete", !(await cardButtons("Minuet in G Major")).some((label) => /^(Rename|Delete)/.test(label)));
  check("imported pieces do", (await cardButtons(IMPORTED)).join("|").includes("Rename"));

  await clickLabel(`Rename ${IMPORTED}`);
  await w.settle();
  await setInput("Should not stick");
  await w.press("Escape");
  await w.settle();
  check("Esc cancels a rename", (await titles()).includes(IMPORTED) && !(await titles()).includes("Should not stick"));

  await clickLabel(`Rename ${IMPORTED}`);
  await w.settle();
  await setInput("   ");
  await w.press("Enter");
  await w.settle();
  check("a blank name is refused with a message", (await w.bodyText()).includes("A title cannot be blank."));
  await w.press("Escape");
  await w.settle();
  check("and cancelling leaves the old name", (await titles()).includes(IMPORTED));

  const RENAMED = "Renamed by e2e";
  await clickLabel(`Rename ${IMPORTED}`);
  await w.settle();
  await setInput(RENAMED);
  await w.press("Enter");
  await w.settle();
  check("Enter saves the new name on the card", (await titles()).includes(RENAMED) && !(await titles()).includes(IMPORTED));
  check("a Renamed notice appears", (await w.bodyText()).includes("Renamed"));
  check("the name is stored on disk, marked renamed", stored(id).title === RENAMED && stored(id).renamed === true);

  await clickLabel(`Delete ${RENAMED}`);
  await w.settle();
  const confirm = await w.run("return document.querySelector('[role=alertdialog]')?.innerText ?? null");
  await shot("library-delete-confirm");
  check("the delete confirmation names the piece and counts its session", confirm?.includes(RENAMED) && /1 session/.test(confirm ?? ""), String(confirm));
  await w.run("[...document.querySelectorAll('[role=alertdialog] button')].find(b=>b.textContent.trim()==='Cancel').click(); return 1");
  await w.settle();
  check("Cancel keeps the piece", (await titles()).includes(RENAMED) && scoreIds().includes(id));

  await clickLabel(`Delete ${RENAMED}`);
  await w.settle();
  await w.run("[...document.querySelectorAll('[role=alertdialog] button')].find(b=>b.textContent.trim()==='Delete').click(); return 1");
  await w.settle();
  check("Delete removes the card", !(await titles()).includes(RENAMED));
  check("and its score files", !scoreIds().includes(id));
  check("and its practice history, nobody else's", sessionsOf(id) === 0);
  check("a Deleted notice appears", (await w.bodyText()).includes("Deleted"));
  check("the bundled Minuet is still there", (await titles()).includes("Minuet in G Major"));
}
