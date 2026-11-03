import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DEFAULT_SETTINGS, loadSettings, setting, useSettings } from "./settings";

const bridge = vi.hoisted(() => ({ settings: vi.fn(), saveSettings: vi.fn(), logLine: vi.fn() }));
vi.mock("./bridge", () => bridge);

beforeEach(() => {
  useSettings(DEFAULT_SETTINGS);
  bridge.saveSettings.mockResolvedValue(undefined);
});
afterEach(() => {
  vi.clearAllMocks();
});

describe("loading settings", () => {
  it("saves the defaults when there's no file yet", async () => {
    useSettings({ marks: false, keyboard: "Stage Piano", zoom: 130 });
    bridge.settings.mockResolvedValue(null);

    await loadSettings();

    expect(bridge.saveSettings).toHaveBeenCalledWith(DEFAULT_SETTINGS);
    expect([setting("marks"), setting("keyboard"), setting("zoom")]).toEqual([true, null, 100]);
  });

  it("uses the file once there is one", async () => {
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
