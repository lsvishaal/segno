// Phase 2.6: a real piece that opens with a pickup (Schumann, "Schlusslied
// des Narren", CC0 OpenScore; tests/fixtures). The pickup is bar 0 in the
// drawing and "the pickup" in words; the cursor starts there and waits;
// Shift+click on bar 1 loops from the pickup; Progress shows a Pickup box.
import { execFileSync } from "node:child_process";
import * as w from "../wd.mjs";
import { ROOT, check, importScore, openPiece, chooseKeyboard, clickButton, waitForButton, cursorX, barBox, clickBar, hintOver, play, shot } from "../lib.mjs";

const FIXTURE = "app/src-tauri/tests/fixtures/schlusslied-des-narren.musicxml";
const TITLE = "Schlusslied des Narren";
let pickupNotes = [];

export function seed() {
  importScore(FIXTURE);
  const expected = JSON.parse(execFileSync(`${ROOT}/target/debug/segno-cli`, ["expected", `${ROOT}/${FIXTURE}`], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }));
  pickupNotes = expected.filter((note) => note.bar === 0).map((note) => note.pitch);
}

const cursorIn = async (n) => {
  const box = await barBox(n);
  const x = await cursorX();
  return box !== null && x !== null && x >= box.left - 2 && x <= box.right + 2;
};
const loopButton = () =>
  w.run("const b=[...document.querySelectorAll('button')].find(b=>b.textContent.startsWith('⟲')); return b ? b.textContent.trim() : null");
const saysBarZero = () => w.run("return /\\bbar 0\\b/i.test(document.body.innerText)");

export default async function pickup() {
  check("the pickup's notes were read", pickupNotes.length > 0, `${pickupNotes.length}`);
  await openPiece(TITLE);

  check("the drawing has the pickup as bar 0", (await barBox(0)) !== null);
  check("the hint over the pickup says the pickup", (await hintOver(0))?.startsWith("Start here (the pickup)"), String(await hintOver(0)));

  await chooseKeyboard();
  await clickButton("Start");
  await waitForButton("Stop");
  check("the cursor starts in the pickup", await cursorIn(0));
  await shot("pickup-start");

  await play(pickupNotes);
  check("playing the pickup's notes moves the cursor into bar 1", await cursorIn(1));

  // Click the pickup to start there, Shift+click bar 1: a loop from the pickup.
  await clickBar(0);
  check("clicking the pickup starts there", await cursorIn(0));
  await clickBar(1, true);
  check("Shift+click on bar 1 loops from the pickup", (await loopButton()) === "⟲ Loop pickup–bar 1", String(await loopButton()));
  check("nothing on screen says bar 0", !(await saysBarZero()));
  await shot("pickup-loop");

  await clickButton("Stop");
  await w.settle();
  await clickButton("Progress");
  await w.until("return !!document.querySelector('[data-piece-map]')", "the Progress map", 10000);
  await w.settle();
  const first = await w.run("const b=document.querySelector('[data-piece-map] button'); return b ? [b.getAttribute('data-bar'), b.getAttribute('aria-label')] : null");
  check("Progress's first box is the pickup", first?.[0] === "0" && first?.[1]?.startsWith("Pickup:"), JSON.stringify(first));
  check("and nothing there says bar 0", !(await saysBarZero()));
  await shot("pickup-progress");
}
