import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PieceSummary } from "../bridge";
import { LibraryScreen, distinguishing } from "./Library";

const PIECES: PieceSummary[] = [
  { id: "a1", title: "Ode to Joy", composer: "Beethoven", bars: 8, notes: 24, fileName: null },
];

function renderScreen(overrides: Partial<Parameters<typeof LibraryScreen>[0]> = {}) {
  const onOpen = vi.fn();
  const onImport = vi.fn();
  const onDismissError = vi.fn();

  render(
    <LibraryScreen
      pieces={PIECES}
      onOpen={onOpen}
      onImport={onImport}
      importing={false}
      importError={null}
      onDismissError={onDismissError}
      {...overrides}
    />,
  );

  return { onOpen, onImport, onDismissError };
}

describe("LibraryScreen import", () => {
  it("calls onImport when the import button is clicked", () => {
    const { onImport } = renderScreen();

    fireEvent.click(screen.getByRole("button", { name: "Import MusicXML" }));

    expect(onImport).toHaveBeenCalledOnce();
  });

  it("disables the button and shows an in-flight label while importing", () => {
    renderScreen({ importing: true });

    const button = screen.getByRole("button", { name: "Reading…" });
    expect(button).toBeDisabled();
  });

  it("shows the refusal's reason and route, and wires dismissal", () => {
    const { onDismissError } = renderScreen({
      importError: "This is a picture.\n\nTry MusicXML instead.",
    });

    expect(screen.getByText(/This is a picture\./)).toBeInTheDocument();
    expect(screen.getByText(/Try MusicXML instead\./)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(onDismissError).toHaveBeenCalledOnce();
  });

  it("shows no error banner when there is nothing to say", () => {
    renderScreen();

    expect(screen.queryByRole("button", { name: "Dismiss" })).not.toBeInTheDocument();
  });

  it("still renders existing pieces alongside the new import affordance", () => {
    renderScreen();

    expect(screen.getByText("Ode to Joy")).toBeInTheDocument();
  });
});

function summary(id: string, title: string, composer: string | null, fileName: string | null): PieceSummary {
  return { id, title, composer, bars: 1, notes: 1, fileName };
}

describe("telling alike pieces apart", () => {
  it("leaves pieces with their own titles alone", () => {
    expect(distinguishing([summary("a", "Canon in D", null, "Canon.mxl"), summary("b", "Minuet", null, null)]).size).toBe(0);
  });

  // The performer's two real files, as Hogwash reads them.
  it("tells the two real Für Elise files apart, though their titles differ only by an accent", () => {
    const notes = distinguishing([
      summary("full", "Für Elise", "arr. Verona", "Fur_Elise.mxl"),
      summary("easy", "Fur Elise", null, "Fur_Elise_Easy.mxl"),
    ]);

    expect(notes.get("full")).toBeUndefined(); // its composer already tells it apart
    expect(notes.get("easy")).toBe("Fur_Elise_Easy.mxl");
  });

  it("names the files when the composers are the same", () => {
    const notes = distinguishing([
      summary("a", "Ode to Joy", "Beethoven", "ode_easy.mxl"),
      summary("b", "ode  to JOY", "beethoven", null),
    ]);

    expect(notes.get("a")).toBe("ode_easy.mxl");
    expect(notes.get("b")).toBe("Bundled with Hogwash");
  });

  it("shows the note on the card", () => {
    renderScreen({
      pieces: [summary("full", "Für Elise", "arr. Verona", "Fur_Elise.mxl"), summary("easy", "Fur Elise", null, "Fur_Elise_Easy.mxl")],
    });

    expect(screen.getByText("Fur_Elise_Easy.mxl")).toBeInTheDocument();
    expect(screen.queryByText("Fur_Elise.mxl")).toBeNull();
  });
});
