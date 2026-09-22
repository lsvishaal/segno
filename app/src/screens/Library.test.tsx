import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PieceSummary } from "../bridge";
import { LibraryScreen } from "./Library";

const PIECES: PieceSummary[] = [
  { id: "a1", title: "Ode to Joy", composer: "Beethoven", bars: 8, notes: 24 },
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
