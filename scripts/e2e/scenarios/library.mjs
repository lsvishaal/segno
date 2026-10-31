// The library, on a copy of your own data: rename (Enter, Esc, blank) and
// delete (confirmation, history). Your history files are schema 1, so the
// session count in the confirmation also shows old files still load.
import fs from "node:fs";
import * as w from "../wd.mjs";
import { check, log, DATA, HISTORY, shot } from "../lib.mjs";

export const mine = true;

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
  await w.until("return document.querySelectorAll('li h3').length >= 6", "the library", 30000);
  await w.settle();
  await shot("library");
  const shelves = await w.run("return [...document.querySelectorAll('section[aria-label]')].map(s => s.getAttribute('aria-label'))");
  check("the library is in shelves: your pieces, then the starters by level", ["Your pieces", "Starter · Beginner", "Starter · Intermediate"].every((label) => shelves.includes(label)), JSON.stringify(shelves));

  // Your imported pieces whose stored title is the one on their card.
  const shown = await titles();
  const imported = scoreIds()
    .map((id) => ({ id, title: stored(id).title, sessions: sessionsOf(id) }))
    // Once on screen only: an import may share a bundled piece's title.
    .filter((piece) => shown.filter((title) => title === piece.title).length === 1)
    .sort((a, b) => b.sessions - a.sessions);
  if (imported.length < 2) {
    log("    SKIPPED: you need two imported pieces to test rename and delete on");
    return;
  }
  const [deleted, renamed] = imported;
  log(`    renaming "${renamed.title}"; deleting "${deleted.title}" (${deleted.sessions} session(s))`);

  check("bundled pieces offer no Rename or Delete", !(await cardButtons("Minuet in G Major")).some((label) => /^(Rename|Delete)/.test(label)));
  check("imported pieces do", (await cardButtons(renamed.title)).join("|").includes("Rename"));

  await clickLabel(`Rename ${renamed.title}`);
  await w.settle();
  await setInput("Should not stick");
  await w.press("Escape");
  await w.settle();
  check("Esc cancels a rename", (await titles()).includes(renamed.title) && !(await titles()).includes("Should not stick"));

  await clickLabel(`Rename ${renamed.title}`);
  await w.settle();
  await setInput("   ");
  await w.press("Enter");
  await w.settle();
  check("a blank name is refused with a message", (await w.bodyText()).includes("A title cannot be blank."));
  await w.press("Escape");
  await w.settle();
  check("and cancelling leaves the old name", (await titles()).includes(renamed.title));

  await clickLabel(`Rename ${renamed.title}`);
  await w.settle();
  await setInput("Renamed by e2e");
  await w.press("Enter");
  await w.settle();
  check("Enter saves the new name on the card", (await titles()).includes("Renamed by e2e"));
  check("a Renamed notice appears", (await w.bodyText()).includes("Renamed"));
  check("the name is stored on disk, marked renamed", stored(renamed.id).title === "Renamed by e2e" && stored(renamed.id).renamed === true);

  await clickLabel(`Delete ${deleted.title}`);
  await w.settle();
  const confirm = await w.run("return document.querySelector('[role=alertdialog]')?.innerText ?? null");
  await shot("library-delete-confirm");
  const counted = deleted.sessions === 0 || new RegExp(`${deleted.sessions} session`).test(confirm ?? "");
  check("the delete confirmation names the piece and counts its (schema 1) sessions", confirm?.includes(deleted.title) && counted, String(confirm));
  await w.run("[...document.querySelectorAll('[role=alertdialog] button')].find(b=>b.textContent.trim()==='Cancel').click(); return 1");
  await w.settle();
  check("Cancel keeps the piece", (await titles()).includes(deleted.title) && scoreIds().includes(deleted.id));

  await clickLabel(`Delete ${deleted.title}`);
  await w.settle();
  await w.run("[...document.querySelectorAll('[role=alertdialog] button')].find(b=>b.textContent.trim()==='Delete').click(); return 1");
  await w.settle();
  check("Delete removes the card", !(await titles()).includes(deleted.title));
  check("and its score files", !scoreIds().includes(deleted.id));
  check("and its practice history, nobody else's", sessionsOf(deleted.id) === 0);
  check("a Deleted notice appears", (await w.bodyText()).includes("Deleted"));
}
