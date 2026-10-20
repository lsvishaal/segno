// The whole Minuet, from its real expected notes: end of piece, summary, practise again.
import * as w from "../wd.mjs";
import { check, log, sessions, newSessions, minuet, startPractising, buttonExists, clickButton, waitForButton, play, shot } from "../lib.mjs";

export default async function wholePiece() {
  const before = sessions();
  await startPractising();

  log(`    playing all ${minuet.length} notes of the Minuet (repeats included)...`);
  await play(minuet, 22);
  await w.sleep(500);
  await shot("whole-piece-end");
  check("reaching the end of the piece changes Stop to Finish", await buttonExists("Finish"));

  await clickButton("Finish");
  await w.until("return document.body.innerText.includes('Practice again')", "the summary", 10000);
  await shot("whole-piece-summary");
  const tiles = await w.run("return [...document.querySelectorAll('p.font-mono')].map(p => p.textContent + ' ' + p.nextElementSibling.textContent)");
  check("the summary shows the four counts", tiles.length === 4, JSON.stringify(tiles));
  check("every note correct; none wrong, missed or unexpected", tiles[0]?.startsWith(String(minuet.length)) && tiles.slice(1).every((t) => t.startsWith("0")), JSON.stringify(tiles));
  check("the attempt was saved with every note", newSessions(before)[0]?.attempts[0] === minuet.length, JSON.stringify(newSessions(before)));

  check("Practice again and Choose a different instrument are offered", (await buttonExists("Practice again")) && (await buttonExists("Choose a different instrument")));
  await clickButton("Practice again");
  await waitForButton("Stop");
  check("Practice again starts a new attempt", !(await w.bodyText()).includes("Never played"));
}
