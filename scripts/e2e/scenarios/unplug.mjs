// Unplugging the keyboard mid-attempt: the banner, reconnecting, and giving up after a minute.
import * as w from "../wd.mjs";
import { check, log, sessions, newSessions, minuet, openPiece, chooseKeyboard, clickButton, waitForButton, cursorX, play, kb, shot } from "../lib.mjs";

const BANNER = "const p=[...document.querySelectorAll('p')].find(p=>/stopped reporting/.test(p.textContent)); return p && p.textContent.trim()";

export default async function unplug() {
  const before = sessions();
  await openPiece("Minuet in G Major");
  await w.until("const s=document.querySelector('select[aria-label=\"Instrument\"]'); return s && s.options.length>0", "the picker", 10000);
  const picker = await w.run("const s=document.querySelector('select[aria-label=\"Instrument\"]'); return {options:[...s.options].map(o=>o.value), chosen:s.value}");
  check("the picker does not offer Midi Through", !picker.options.some((o) => /Midi Through/.test(o)), JSON.stringify(picker.options));
  await chooseKeyboard();
  await clickButton("Start");
  await waitForButton("Stop");
  await play(minuet.slice(0, 4));
  const x0 = await cursorX();

  kb("close");
  const lost = await w.until(BANNER, "the lost banner", 8000).catch(() => null);
  await shot("unplug-banner");
  check("unplugging says so in the status line, with a countdown", /waiting \d+ s/.test(lost ?? ""), String(lost));
  await w.sleep(2500);
  kb("open");
  const restored = await w.until("return !([...document.querySelectorAll('p')].some(p=>/stopped reporting/.test(p.textContent)))", "the banner to go", 12000).catch(() => null);
  check("plugging it back in clears the banner", restored);

  await play(minuet.slice(4, 9));
  check("playing on after reconnecting advances the cursor", (await cursorX()) > x0);

  kb("close");
  log("    keyboard gone; waiting out the one-minute reconnect window...");
  const ended = await w.until("return document.body.innerText.includes('did not reconnect')", "the attempt to end by itself", 90000).catch(() => null);
  await shot("unplug-gave-up");
  check("after a minute away the attempt ends by itself, with a reason", ended);
  const saved = newSessions(before);
  check("what was played is kept", saved.length === 1 && saved[0].attempts[0] === 9, JSON.stringify(saved));
  kb("open");
  await w.sleep(1000);
}
