/**
 * The performer's settings, kept by Rust in `settings.json` beside their
 * history. Loaded once before the first render, so reading one is
 * synchronous; changing one saves the file.
 */
import { logLine, saveSettings, settings } from "./bridge";

export interface Settings {
  /** Whether marks stay on the score while playing. */
  readonly marks: boolean;
  /** The keyboard last started on, by name. */
  readonly keyboard: string | null;
  /** The score's zoom, in percent. */
  readonly zoom: number;
}

export const DEFAULT_SETTINGS: Settings = { marks: true, keyboard: null, zoom: 100 };

let current: Settings = DEFAULT_SETTINGS;

/** One setting, as last loaded or changed. */
export function setting<K extends keyof Settings>(key: K): Settings[K] {
  return current[key];
}

/** Changes some settings and saves them; a failed save is logged by Rust. */
export function remember(change: Partial<Settings>): void {
  current = { ...current, ...change };
  try {
    void saveSettings(current).catch(() => undefined);
  } catch {
    // No Rust behind this window (a test): kept for this run only.
  }
}

/** Replaces every setting without saving: for tests. */
export function useSettings(settings: Settings): void {
  current = settings;
}

/** Loads the settings kept. The first time there are none, the defaults are saved. */
export async function loadSettings(): Promise<void> {
  try {
    const kept = await settings();
    if (kept) {
      current = kept;
    } else {
      current = DEFAULT_SETTINGS;
      await saveSettings(current);
    }
  } catch {
    // Startup failed, so there is nothing to keep them in: the defaults.
  }
}

/** Sends errors the window catches to Segno's log. */
export function logErrors(): void {
  const send = (line: string) => {
    try {
      void logLine(line).catch(() => undefined);
    } catch {
      // Nowhere to send it.
    }
  };
  window.addEventListener("error", (event) => send(`${event.message} (${event.filename}:${event.lineno})`));
  window.addEventListener("unhandledrejection", (event) => send(`unhandled: ${String(event.reason)}`));
}
