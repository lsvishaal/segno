// The whole Minuet, from its real expected notes: the end finishes the run by itself, the results, practise again.
import * as w from "../wd.mjs";
import { check, log, sessions, newSessions, minuet, startPractising, buttonExists, clickButton, waitForButton, play, shot } from "../lib.mjs";

export default async function wholePiece() {
  const before = sessions();
  await startPractising();

  log(`    playing all ${minuet.length} notes of the Minuet (repeats included)...`);
  await play(minuet, 22);
  await shot("whole-piece-end");
  check("reaching the end of the piece changes Stop to Finish", await buttonExists("Finish"));

  // No click: the run finishes by itself a moment after the last note.
  await w.until("return !!document.querySelector('section[aria-label=Results]')", "the run to finish by itself", 5000);
  check("the end of the piece finishes the run by itself", true);
  await shot("whole-piece-summary");
  const results = await w.run(
    "const r = document.querySelector('section[aria-label=Results]'); return r ? r.innerText : null",
  );
  log("    results:", JSON.stringify(results));
  check("the results replace the score", results !== null);
  const details = await w.run("const r = document.querySelector('section[aria-label=Results] details'); return r ? r.textContent : ''");
  check("every note right: accuracy 100% and " + minuet.length + "/0/0 under Details", results?.includes("accuracy 100%") && details.includes(`${minuet.length}/0/0`), results);
  check("the attempt was saved with every note", newSessions(before)[0]?.attempts[0] === minuet.length, JSON.stringify(newSessions(before)));

  check("Practice again and Show on the score are offered", (await buttonExists("↻ Practice again")) && (await buttonExists("♪ Show on the score")));
  await clickButton("↻ Practice again");
  await waitForButton("Stop");
  check("Practice again starts a new attempt", !(await w.bodyText()).includes("Never played"));
}
