// The settings panel, zoom, note names, and the score at two window sizes.
import * as w from "../wd.mjs";
import { check, log, openPiece, shot } from "../lib.mjs";

const setZoom = (value) =>
  w.run(
    "const r=document.querySelector('#zoom'); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(r,String(arguments[0])); r.dispatchEvent(new Event('input',{bubbles:true})); return r.value",
    value,
  );
const svgHeight = () => w.run("return Math.round(document.querySelector('.hw-page svg').getBoundingClientRect().height)");

export default async function settings() {
  await openPiece("Minuet in G Major");

  await w.run("document.querySelector('button[aria-label=\"Settings\"]').click(); return 1");
  await w.until("return !!document.querySelector('[data-settings-panel]')", "the settings panel", 5000);
  await w.settle();
  const panel = await w.run("return document.querySelector('[data-settings-panel]').innerText");
  await shot("settings-panel");
  check("About shows the version and licence", /Segno \d+\.\d+\.\d+ · Apache-2\.0/.test(panel));
  check("About credits Verovio", panel.includes("Verovio"));
  check("there is no dark score surface option", !/surface|dark/i.test(panel));
  check("MIDI input and output are native selects", (await w.run("return document.querySelectorAll('[data-settings-panel] select').length")) === 2);
  const inputs = await w.run("return [...document.querySelector('[data-settings-panel] select[aria-label=\"MIDI input\"]').options].map(o=>o.textContent)");
  check("the MIDI input list offers the connected keyboard", inputs.some((o) => /Test Keyboard/.test(o)), JSON.stringify(inputs));

  const names = () => w.run("return document.querySelectorAll('.hw-note-name').length");
  const namesBefore = await names();
  await w.run("[...document.querySelectorAll('[data-settings-panel] button')].find(b=>b.textContent.trim()==='Off' && !b.getAttribute('aria-label')).click(); return 1");
  await w.settle();
  check("turning on Note names labels the notes", (await names()) > namesBefore, `${namesBefore} -> ${await names()}`);

  const h0 = await svgHeight();
  await setZoom(150);
  // Zoom re-engraves after a 250 ms pause in the slider's movement.
  await w.until(`return Math.round(document.querySelector('.hw-page svg').getBoundingClientRect().height) > ${h0}`, "the reflow at 150%", 10000).catch(() => null);
  check("the zoom slider reflows the score (taller at 150%)", (await svgHeight()) > h0, `${h0} -> ${await svgHeight()}`);
  await setZoom(100);
  await w.until(`return Math.round(document.querySelector('.hw-page svg').getBoundingClientRect().height) === ${h0}`, "the reflow back at 100%", 10000).catch(() => null);
  await w.press("Escape");
  await w.settle();
  check("Escape closes the settings panel", await w.run("return !document.querySelector('[data-settings-panel]')"));

  // 2560×1440 isn't tried: the score is already at its widest at 1920.
  for (const [width, height] of [[1280, 720], [1920, 1080]]) {
    await w.setSize(width, height);
    // Longer than the 250 ms re-engrave debounce, so the reflow is waited for.
    await w.settle(600, 8000);
    const over = await w.run("return document.documentElement.scrollWidth - window.innerWidth");
    await shot(`settings-${width}x${height}`);
    log(`    ${width}x${height}: horizontal overflow ${over}`);
    check(`${width}x${height}: no horizontal scrolling`, over <= 0, `overflow ${over}`);
  }
}
