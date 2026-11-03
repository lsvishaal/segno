// Runs the real packaged app through every scenario: a virtual MIDI keyboard
// plays, WebDriver (tauri-driver) reads the window. Started by `just e2e`,
// which builds and extracts the AppImage first.
//
//   node scripts/e2e/run.mjs [scenario ...]     (default: all of them)
import { spawn } from "node:child_process";
import fs from "node:fs";
import * as w from "./wd.mjs";
import { ROOT, DIR, APP, SHOTS, log, results, resetData, useKeyboard, appExited } from "./lib.mjs";

const ALL = ["practice", "whole-piece", "flash", "marks", "timing", "start-from-bar", "loop", "review", "progress", "settings", "library", "unplug"];
const chosen = process.argv.slice(2).length ? process.argv.slice(2) : ALL;
fs.mkdirSync(SHOTS, { recursive: true });

const vkbd = spawn(`${ROOT}/target/debug/examples/vkbd`, [], { stdio: ["pipe", "pipe", "inherit"] });
await new Promise((resolve, reject) => {
  vkbd.stdout.once("data", resolve);
  vkbd.once("exit", () => reject(new Error("the virtual keyboard did not start")));
});
vkbd.stdout.resume();
useKeyboard(vkbd.stdin);

// The app inherits this environment: its data goes to target/e2e/data, and
// it finds Verovio inside the AppImage, as an installed copy would.
const env = { ...process.env, XDG_DATA_HOME: `${DIR}/data` };
delete env.SEGNO_VEROVIO_LIB;
delete env.SEGNO_VEROVIO_DATA;
const driver = spawn("tauri-driver", ["--port", "4444", "--native-port", "4445"], { env, stdio: "ignore" });

const summary = [];
try {
  await w.ready();
  for (const name of chosen) {
    const scenario = await import(`./scenarios/${name}.mjs`);
    const first = results.length;
    log(`\n${name}`);
    resetData();
    try {
      scenario.seed?.();
      await w.start(APP);
      await scenario.default();
    } catch (error) {
      results.push({ name: `${name}: script error`, ok: false });
      log(`  FAIL  script error: ${error.message}`);
    } finally {
      await w.stop();
      await appExited();
    }
    const mine = results.slice(first);
    summary.push(`${name.padEnd(12)} ${mine.filter((r) => r.ok).length}/${mine.length}`);
  }
} finally {
  driver.kill();
  vkbd.stdin.end("quit\n");
}

log(`\n${summary.join("\n")}`);
const failed = results.filter((r) => !r.ok).length;
log(failed ? `\n${failed} check(s) failed. Screenshots: ${SHOTS}` : `\nAll ${results.length} checks passed.`);
process.exit(failed ? 1 : 0);
