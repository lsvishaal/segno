// Unplugging the keyboard mid-attempt: the banner, and reconnecting. Giving up
// after the one-minute window is `practice.rs`'s
// a_keyboard_gone_for_the_whole_window_ends_the_attempt_and_keeps_it.
import * as w from "../wd.mjs";
import { check, minuet, openPiece, chooseKeyboard, clickButton, waitForButton, cursorX, play, kb, shot } from "../lib.mjs";

const BANNER = "const p=[...document.querySelectorAll('p')].find(p=>/stopped reporting/.test(p.textContent)); return p && p.textContent.trim()";

export default async function unplug() {
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
}
