// A tiny W3C WebDriver client (plain fetch) for driving the real Hogwash
// window through tauri-driver.
import fs from "node:fs";

const BASE = "http://127.0.0.1:4444";
let sid = null;

async function call(method, path, body) {
  const response = await fetch(`${BASE}${path}`, {
    method,
    headers: { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const json = await response.json();
  if (!response.ok || json.value?.error) {
    throw new Error(`${method} ${path}: ${json.value?.error} ${json.value?.message ?? ""}`.slice(0, 400));
  }
  return json.value;
}

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

export async function ready(ms = 10000) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    if (await fetch(`${BASE}/status`).then((r) => r.ok, () => false)) return;
    await sleep(200);
  }
  throw new Error("tauri-driver did not start");
}

export async function start(application) {
  const value = await call("POST", "/session", {
    capabilities: { alwaysMatch: { browserName: "wry", "tauri:options": { application } } },
  });
  sid = value.sessionId;
}

export async function stop() {
  if (sid) await call("DELETE", `/session/${sid}`).catch(() => undefined);
  sid = null;
}

export const run = (script, ...args) => call("POST", `/session/${sid}/execute/sync`, { script, args });
/** Runs `script` in the page until it calls its last argument with the result. */
export const runAsync = (script, ...args) => call("POST", `/session/${sid}/execute/async`, { script, args });

/** Waits until `script` returns something truthy, and returns it. */
export async function until(script, what, ms = 15000) {
  const end = Date.now() + ms;
  let last;
  while (Date.now() < end) {
    last = await run(script).catch((e) => `err:${e.message}`);
    if (last && !String(last).startsWith("err:")) return last;
    await sleep(150);
  }
  throw new Error(`timed out waiting for ${what} (last: ${last})`);
}

export const bodyText = () => run("return document.body.innerText");

export async function shot(path) {
  const png = await call("GET", `/session/${sid}/screenshot`);
  fs.writeFileSync(path, Buffer.from(png, "base64"));
}

export const setSize = (width, height) => call("POST", `/session/${sid}/window/rect`, { x: 0, y: 0, width, height });

const KEYS = { Escape: "", Enter: "", Tab: "" };

/** Presses one key, as if on the computer keyboard. */
export const press = (name) => {
  const value = KEYS[name] ?? name;
  return call("POST", `/session/${sid}/actions`, {
    actions: [{ type: "key", id: "k", actions: [{ type: "keyDown", value }, { type: "keyUp", value }] }],
  });
};

/** A real mouse click at a point in the window (CSS pixels from its top left), Shift held if `shift`. */
export const clickAt = (x, y, { shift = false } = {}) => {
  const SHIFT = "\uE008";
  const pointer = [
    { type: "pointerMove", origin: "viewport", x: Math.round(x), y: Math.round(y) },
    { type: "pointerDown", button: 0 },
    { type: "pointerUp", button: 0 },
    { type: "pause" },
  ];
  const keys = shift
    ? [{ type: "keyDown", value: SHIFT }, { type: "pause" }, { type: "pause" }, { type: "keyUp", value: SHIFT }]
    : [];
  return call("POST", `/session/${sid}/actions`, {
    actions: [
      ...(shift ? [{ type: "key", id: "shift", actions: keys }] : []),
      { type: "pointer", id: "mouse", parameters: { pointerType: "mouse" }, actions: pointer },
    ],
  });
};
