import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DEFAULT_SETTINGS, loadSettings, setting, useSettings } from "./settings";

const bridge = vi.hoisted(() => ({ settings: vi.fn(), saveSettings: vi.fn(), logLine: vi.fn() }));
vi.mock("./bridge", () => bridge);

const OLD = new Map([
  ["hogwash.zoom", "130"],
  ["hogwash.marks", "off"],
  ["hogwash.keyboard", "Stage Piano"],
]);

beforeEach(() => {
  useSettings(DEFAULT_SETTINGS);
  bridge.saveSettings.mockResolvedValue(undefined);
  vi.stubGlobal("localStorage", { getItem: (key: string) => OLD.get(key) ?? null });
});
afterEach(() => {
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe("loading settings", () => {
  it("brings the old localStorage values over once, when there's no file yet", async () => {
    bridge.settings.mockResolvedValue(null);

    await loadSettings();

    const brought = { marks: false, keyboard: "Stage Piano", zoom: 130 };
    expect(bridge.saveSettings).toHaveBeenCalledWith(brought);
    expect([setting("marks"), setting("keyboard"), setting("zoom")]).toEqual([false, "Stage Piano", 130]);
  });

  it("uses the file once there is one, and ignores localStorage", async () => {
    bridge.settings.mockResolvedValue({ marks: true, keyboard: "Digital Piano", zoom: 90 });

    await loadSettings();

    expect(bridge.saveSettings).not.toHaveBeenCalled();
    expect([setting("marks"), setting("keyboard"), setting("zoom")]).toEqual([true, "Digital Piano", 90]);
  });

  it("keeps the defaults when Segno didn't start", async () => {
    bridge.settings.mockRejectedValue("not started");

    await loadSettings();

    expect(setting("zoom")).toBe(100);
    expect(setting("keyboard")).toBeNull();
  });
});
