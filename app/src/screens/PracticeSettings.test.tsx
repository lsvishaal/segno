import { createRef } from "react";

import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

// `SettingsPanel` mounts `SoundRouting`, which talks to Tauri directly
// (`instruments`/`outputPorts`/`setThru`) — no `window.__TAURI_INTERNALS__`
// exists in jsdom, so these are stubbed the way every other bridge-calling
// screen in this suite would need to be, were one under test.
vi.mock("../bridge", () => ({
  instruments: vi.fn().mockResolvedValue([]),
  outputPorts: vi.fn().mockResolvedValue([]),
  setThru: vi.fn().mockResolvedValue(undefined),
}));

import { SettingsPanel } from "./PracticeSettings";

// `SoundRouting`'s device-fetch effects resolve after mount (mocked, but
// still a real microtask), so every test flushes them once before asserting
// — otherwise React logs an "update not wrapped in act" warning for a state
// change no test cares about the timing of.
async function renderPanel(overrides: Partial<Parameters<typeof SettingsPanel>[0]> = {}) {
  const triggerRef = createRef<HTMLButtonElement>();
  const onClose = vi.fn();
  const onSurfaceChange = vi.fn();
  const onShowNoteNamesChange = vi.fn();
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
          surface="paper"
          onSurfaceChange={onSurfaceChange}
          showNoteNames={false}
          onShowNoteNamesChange={onShowNoteNamesChange}
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

  return { onClose, onSurfaceChange, onShowNoteNamesChange, onScaleChange, onExport };
}

describe("SettingsPanel", () => {
  it("renders nothing when closed", async () => {
    await renderPanel({ open: false });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("shows the appearance, sound, and view/export controls when open", async () => {
    await renderPanel();

    expect(screen.getByRole("dialog", { name: "Settings" })).toBeInTheDocument();
    expect(screen.getByRole("slider")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "MIDI input" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "MIDI output" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Export MusicXML" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "paper" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "dark" })).toBeInTheDocument();
  });

  it("calls onExport when the export button is clicked", async () => {
    const { onExport } = await renderPanel();

    fireEvent.click(screen.getByRole("button", { name: "Export MusicXML" }));

    expect(onExport).toHaveBeenCalledOnce();
  });

  it("calls onSurfaceChange when a surface option is chosen", async () => {
    const { onSurfaceChange } = await renderPanel();

    fireEvent.click(screen.getByRole("button", { name: "dark" }));

    expect(onSurfaceChange).toHaveBeenCalledWith("dark");
  });

  it("calls onShowNoteNamesChange when the note names toggle is clicked", async () => {
    const { onShowNoteNamesChange } = await renderPanel();

    fireEvent.click(screen.getByRole("button", { name: "Off" }));

    expect(onShowNoteNamesChange).toHaveBeenCalledWith(true);
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

describe("MIDI device dropdowns", () => {
  it("open on click and show a listbox of options", async () => {
    await renderPanel();

    fireEvent.click(screen.getByRole("button", { name: "MIDI input" }));

    expect(screen.getByRole("listbox", { name: "MIDI input" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "None" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  it("closes after an option is selected", async () => {
    await renderPanel();

    fireEvent.click(screen.getByRole("button", { name: "MIDI input" }));
    fireEvent.click(screen.getByRole("option", { name: "None" }));

    expect(screen.queryByRole("listbox", { name: "MIDI input" })).not.toBeInTheDocument();
  });

  it("closes on an outside click without closing the whole settings panel", async () => {
    const { onClose } = await renderPanel();

    fireEvent.click(screen.getByRole("button", { name: "MIDI input" }));
    expect(screen.getByRole("listbox", { name: "MIDI input" })).toBeInTheDocument();

    fireEvent.mouseDown(screen.getByText("Sound"));

    expect(screen.queryByRole("listbox", { name: "MIDI input" })).not.toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "Settings" })).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });
});
