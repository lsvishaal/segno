import { createRef } from "react";

import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

// `SettingsPanel` mounts `SoundRouting`, which talks to Tauri directly
// (`instruments`/`outputPorts`/`setThru`) — no `window.__TAURI_INTERNALS__`
// exists in jsdom, so these are stubbed the way every other bridge-calling
// screen in this suite would need to be, were one under test.
vi.mock("../bridge", () => ({
  appVersion: vi.fn().mockResolvedValue("0.1.0"),
  instruments: vi.fn().mockResolvedValue([]),
  outputPorts: vi.fn().mockResolvedValue([]),
  setThru: vi.fn().mockResolvedValue(undefined),
}));

import { instruments, setThru } from "../bridge";
import { SettingsPanel } from "./PracticeSettings";

// `SoundRouting`'s device-fetch effects resolve after mount (mocked, but
// still a real microtask), so every test flushes them once before asserting
// — otherwise React logs an "update not wrapped in act" warning for a state
// change no test cares about the timing of.
async function renderPanel(overrides: Partial<Parameters<typeof SettingsPanel>[0]> = {}) {
  const triggerRef = createRef<HTMLButtonElement>();
  const onClose = vi.fn();
  const onShowNoteNamesChange = vi.fn();
  const onShowGeometryChange = vi.fn();
  const onMarksWhilePlayingChange = vi.fn();
  const onScaleChange = vi.fn();
  const onExport = vi.fn();

  await act(async () => {
    render(
      <div>
        <button ref={triggerRef} type="button">
          trigger
        </button>
        <SettingsPanel
          open={true}
          onClose={onClose}
          triggerRef={triggerRef}
          showNoteNames={false}
          onShowNoteNamesChange={onShowNoteNamesChange}
          showGeometry={false}
          onShowGeometryChange={onShowGeometryChange}
          marksWhilePlaying={true}
          onMarksWhilePlayingChange={onMarksWhilePlayingChange}
          scale={150}
          onScaleChange={onScaleChange}
          zoomError={null}
          onExport={onExport}
          exportError={null}
          {...overrides}
        />
      </div>,
    );
  });

  return { onClose, onShowNoteNamesChange, onShowGeometryChange, onMarksWhilePlayingChange, onScaleChange, onExport };
}

describe("SettingsPanel", () => {
  it("Marks while playing is on, and turns off", async () => {
    const { onMarksWhilePlayingChange } = await renderPanel();
    const toggle = screen.getByRole("button", { name: "Marks while playing" });
    expect(toggle).toHaveAttribute("aria-pressed", "true");

    fireEvent.click(toggle);

    expect(onMarksWhilePlayingChange).toHaveBeenCalledWith(false);
  });

  it("renders nothing when closed", async () => {
    await renderPanel({ open: false });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  // The dialog, the MIDI selects and Export are each used by a test below.
  it("the zoom slider shows the zoom and changes it", async () => {
    const { onScaleChange } = await renderPanel();
    const slider = screen.getByRole("slider");
    expect(slider).toHaveValue("150");

    fireEvent.change(slider, { target: { value: "175" } });

    expect(onScaleChange).toHaveBeenCalledWith(175);
  });

  it("calls onExport when the export button is clicked", async () => {
    const { onExport } = await renderPanel();

    fireEvent.click(screen.getByRole("button", { name: "Export MusicXML" }));

    expect(onExport).toHaveBeenCalledOnce();
  });

  it("calls onShowNoteNamesChange when the note names toggle is clicked", async () => {
    const { onShowNoteNamesChange } = await renderPanel();

    fireEvent.click(screen.getByRole("button", { name: "Off" }));

    expect(onShowNoteNamesChange).toHaveBeenCalledWith(true);
  });

  it("calls onShowGeometryChange when the geometry toggle is clicked", async () => {
    const { onShowGeometryChange } = await renderPanel();

    fireEvent.click(screen.getByRole("button", { name: "Show geometry" }));

    expect(onShowGeometryChange).toHaveBeenCalledWith(true);
  });

  it("shows zoomError and exportError when set", async () => {
    await renderPanel({ zoomError: "Could not rescale.", exportError: "Could not export." });

    expect(screen.getByText("Could not rescale.")).toBeInTheDocument();
    expect(screen.getByText("Could not export.")).toBeInTheDocument();
  });

  it("calls onClose on Escape", async () => {
    const { onClose } = await renderPanel();

    fireEvent.keyDown(document, { key: "Escape" });

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("calls onClose on an outside click", async () => {
    const { onClose } = await renderPanel();

    fireEvent.mouseDown(document.body);

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("does not close on a click inside the panel", async () => {
    const { onClose } = await renderPanel();

    fireEvent.mouseDown(screen.getByRole("dialog", { name: "Settings" }));

    expect(onClose).not.toHaveBeenCalled();
  });
});

describe("About", () => {
  it("names the version, the licence and Verovio", async () => {
    await renderPanel();

    expect(screen.getByText("Hogwash 0.1.0 · Apache-2.0")).toBeInTheDocument();
    expect(screen.getByText(/engraved by Verovio/)).toBeInTheDocument();
  });
});

describe("MIDI device selects", () => {
  it("are the browser's own selects, with None chosen to begin with", async () => {
    await renderPanel();

    expect(screen.getByRole("combobox", { name: "MIDI input" })).toHaveValue("");
    expect(screen.getByRole("combobox", { name: "MIDI output" })).toHaveValue("");
    expect(screen.getAllByRole("option", { name: "None", selected: true })).toHaveLength(2);
  });

  it("route Thru when a device is chosen", async () => {
    vi.mocked(instruments).mockResolvedValueOnce([{ name: "Digital Piano" }]);
    await renderPanel();

    fireEvent.change(screen.getByRole("combobox", { name: "MIDI input" }), {
      target: { value: "Digital Piano" },
    });

    expect(setThru).toHaveBeenLastCalledWith("Digital Piano", null);
  });

  it("choosing one does not close the settings panel", async () => {
    const { onClose } = await renderPanel();

    fireEvent.mouseDown(screen.getByRole("combobox", { name: "MIDI output" }));

    expect(screen.getByRole("dialog", { name: "Settings" })).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });
});
