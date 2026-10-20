// Practice: real Minuet notes, a wrong note, Restart (button and R), leaving.
import * as w from "../wd.mjs";
import { check, log, sessions, newSessions, minuet, openPiece, chooseKeyboard, buttonExists, clickButton, waitForButton, cursorX, wrongMarks, play, kb, shot } from "../lib.mjs";

export default async function practice() {
  const before = sessions();
  await openPiece("Minuet in G Major");
  check("the Minuet's notes are drawn", (await w.run("return document.querySelectorAll('g.note').length")) > 200);
  check("the picker offers a Start button", await buttonExists("Start"));

  await chooseKeyboard();
  await clickButton("Start");
  await waitForButton("Stop");
  check("after Start the header shows Stop and Restart", (await buttonExists("Stop")) && (await buttonExists("Restart")));

  const x0 = await cursorX();
  // Bar 1, first beat: D5 over G3 B3 D4.
  await play(minuet.slice(0, 4));
  const x1 = await cursorX();
  check("the first chord advances the cursor", x0 !== null && x1 > x0, `x ${x0} -> ${x1}`);

  // Next expected is G4 (67); E5 (76) is wrong.
  kb("on 76 80", "wait 30", "off 76");
  await w.sleep(250);
  const marks = await wrongMarks();
  await shot("practice-wrong-note");
  check("a wrong note (E5) draws the red mark", marks === 1, `marks=${marks}`);
  check("and the cursor stays put", (await cursorX()) === x1);
  await w.sleep(1300);
  check("the red mark fades away", (await wrongMarks()) === 0);

  await play(minuet.slice(4, 9));
  check("correct notes after a wrong one carry on", (await cursorX()) > x1);

  await clickButton("Restart");
  await w.sleep(800);
  check("Restart shows no summary", !(await w.bodyText()).includes("Never played"));
  check("Restart keeps practising (Stop is shown)", await buttonExists("Stop"));
  const saved = newSessions(before);
  log("    sessions after Restart:", JSON.stringify(saved));
  check("Restart kept the interrupted attempt on disk", saved.length === 1 && saved[0].attempts.length === 1 && saved[0].attempts[0] > 0);

  await clickButton("Restart");
  await w.sleep(800);
  const again = newSessions(before);
  check("Restart with nothing played keeps nothing new", again.length === 1 && again[0].attempts.length === 1, JSON.stringify(again));

  await play(minuet.slice(0, 4));
  await w.press("r");
  await w.sleep(800);
  check("the R key restarts and keeps that attempt", newSessions(before)[0].attempts.length === 2, JSON.stringify(newSessions(before)));

  await play(minuet.slice(0, 6));
  await clickButton("← Library");
  await w.sleep(1200);
  check("leaving mid-attempt keeps what was played", newSessions(before)[0].attempts.length === 3, JSON.stringify(newSessions(before)));
  check("we are back on the library", /your music/i.test(await w.bodyText()));
}
