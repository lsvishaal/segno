import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PieceSummary } from "../bridge";
import { LibraryScreen, distinguishing, practisedWhen, weakestLine } from "./Library";

const PIECES: PieceSummary[] = [
  { id: "a1", title: "Ode to Joy", composer: "Beethoven", bars: 8, notes: 24, fileName: null, imported: false, level: "beginner", lastPractisedMs: null, weakest: [] },
];

function renderScreen(overrides: Partial<Parameters<typeof LibraryScreen>[0]> = {}) {
  const onOpen = vi.fn();
  const onProgress = vi.fn();
  const onImport = vi.fn();
  const onDismissError = vi.fn();
  const onRename = vi.fn(async () => null as string | null);
  const onCountHistory = vi.fn(async () => 3);
  const onDelete = vi.fn(async () => null as string | null);

  render(
    <LibraryScreen
      pieces={PIECES}
      onOpen={onOpen}
      onProgress={onProgress}
      onRename={onRename}
      onCountHistory={onCountHistory}
      onDelete={onDelete}
      onImport={onImport}
      importing={false}
      importError={null}
      onDismissError={onDismissError}
      {...overrides}
    />,
  );

  return { onOpen, onProgress, onImport, onDismissError, onRename, onCountHistory, onDelete };
}

describe("LibraryScreen import", () => {
  it("asks Update or Add as a new piece when an import matches one of yours, Update first", () => {
    const onAnswer = vi.fn();
    const canon = { ...PIECES[0]!, id: "imp-1", title: "Canon in D", imported: true };
    renderScreen({ pieces: [...PIECES, canon], asking: canon, onAnswer });

    const update = screen.getByRole("button", { name: "Update Canon in D (keeps your history)" });
    expect(update).toHaveFocus();
    fireEvent.click(update);
    expect(onAnswer).toHaveBeenLastCalledWith(true);
    fireEvent.click(screen.getByRole("button", { name: "Add as a new piece" }));
    expect(onAnswer).toHaveBeenLastCalledWith(false);
    fireEvent.keyDown(update, { key: "Escape" });
    expect(onAnswer).toHaveBeenLastCalledWith(null);
  });

  it("asks nothing otherwise", () => {
    renderScreen();
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("calls onImport when the import button is clicked", () => {
    const { onImport } = renderScreen();

    fireEvent.click(screen.getByRole("button", { name: "+ Add a piece" }));

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
  return { id, title, composer, bars: 1, notes: 1, fileName, imported: fileName !== null, level: fileName === null ? "beginner" : null, lastPractisedMs: null, weakest: [] };
}

describe("telling alike pieces apart", () => {
  it("leaves pieces with their own titles alone", () => {
    expect(distinguishing([summary("a", "Canon in D", null, "Canon.mxl"), summary("b", "Minuet", null, null)]).size).toBe(0);
  });

  // The performer's two real files, as Segno reads them.
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
    expect(notes.get("b")).toBe("Bundled with Segno");
  });

  it("shows the note on the card", () => {
    renderScreen({
      pieces: [summary("full", "Für Elise", "arr. Verona", "Fur_Elise.mxl"), summary("easy", "Fur Elise", null, "Fur_Elise_Easy.mxl")],
    });

    expect(screen.getByText("Fur_Elise_Easy.mxl")).toBeInTheDocument();
    expect(screen.queryByText("Fur_Elise.mxl")).toBeNull();
  });
});

describe("renaming a piece", () => {
  const MOONLIGHT = summary("moon", "Sonate No. 14", "Ludwig van Beethoven", "Moonlight.mxl");
  const MINUET = summary("minuet", "Minuet in G Major", "Christian Petzold", null);

  it("is offered for imported pieces only", () => {
    renderScreen({ pieces: [MOONLIGHT, MINUET] });

    expect(screen.getByRole("button", { name: "Rename Sonate No. 14" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Rename Minuet in G Major" })).toBeNull();
  });

  it("saves the new title on Enter", async () => {
    const { onRename } = renderScreen({ pieces: [MOONLIGHT] });

    fireEvent.click(screen.getByRole("button", { name: "Rename Sonate No. 14" }));
    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "Moonlight Sonata" } });
    fireEvent.submit(screen.getByLabelText("Title").closest("form")!);

    await vi.waitFor(() => expect(onRename).toHaveBeenCalledWith("moon", "Moonlight Sonata"));
  });

  it("refuses a blank title without asking", () => {
    const { onRename } = renderScreen({ pieces: [MOONLIGHT] });

    fireEvent.click(screen.getByRole("button", { name: "Rename Sonate No. 14" }));
    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "   " } });
    fireEvent.submit(screen.getByLabelText("Title").closest("form")!);

    expect(screen.getByText("A title cannot be blank.")).toBeInTheDocument();
    expect(onRename).not.toHaveBeenCalled();
  });

  it("puts everything back on Escape", () => {
    renderScreen({ pieces: [MOONLIGHT] });

    fireEvent.click(screen.getByRole("button", { name: "Rename Sonate No. 14" }));
    fireEvent.keyDown(screen.getByLabelText("Title"), { key: "Escape" });

    expect(screen.queryByLabelText("Title")).toBeNull();
    expect(screen.getByText("Sonate No. 14")).toBeInTheDocument();
  });
});

describe("deleting a piece", () => {
  const MOONLIGHT = summary("moon", "Sonate No. 14", "Ludwig van Beethoven", "Moonlight.mxl");
  const MINUET = summary("minuet", "Minuet in G Major", "Christian Petzold", null);

  it("is offered for imported pieces only", () => {
    renderScreen({ pieces: [MOONLIGHT, MINUET] });

    expect(screen.getByRole("button", { name: "Delete Sonate No. 14" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Delete Minuet in G Major" })).toBeNull();
  });

  it("says how much practice history goes with it before deleting", async () => {
    const { onDelete } = renderScreen({ pieces: [MOONLIGHT] });

    fireEvent.click(screen.getByRole("button", { name: "Delete Sonate No. 14" }));

    const confirmation = await screen.findByRole("alertdialog");
    expect(confirmation).toHaveTextContent("Its practice history (3 sessions) will be deleted too.");
    expect(onDelete).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await vi.waitFor(() => expect(onDelete).toHaveBeenCalledWith("moon"));
  });

  it("does nothing if cancelled", async () => {
    const { onDelete } = renderScreen({ pieces: [MOONLIGHT] });

    fireEvent.click(screen.getByRole("button", { name: "Delete Sonate No. 14" }));
    fireEvent.click(await screen.findByRole("button", { name: "Cancel" }));

    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(onDelete).not.toHaveBeenCalled();
  });
});

describe("the shelves", () => {
  // Sun 25 Oct 2026, 15:00 local time.
  const NOW = new Date(2026, 9, 25, 15, 0).getTime();
  const at = (day: number, hour = 10) => new Date(2026, 9, day, hour).getTime();
  const shelfPieces: PieceSummary[] = [
    { ...summary("lr", "Lightly Row", "Traditional", null), level: "beginner" },
    { ...summary("mn", "Minuet in G Major", "Christian Petzold", null), level: "intermediate", lastPractisedMs: at(23), weakest: [6, 14] },
    { ...summary("cn", "Canon in D", "Pachelbel", "Canon_in_D.mxl"), lastPractisedMs: at(20) },
    { ...summary("sn", "Sonate No. 14", "Beethoven", "Moonlight.mxl") },
  ];
  const sections = () => screen.getAllByRole("region").map((section) => section.getAttribute("aria-label"));
  const titlesIn = (label: string) =>
    [...screen.getByRole("region", { name: label }).querySelectorAll("h3")].map((title) => title.textContent);

  it("puts where you left off first, then your pieces, then the starters by level", () => {
    renderScreen({ pieces: shelfPieces, now: NOW });

    expect(sections()).toEqual(["Continue", "Your pieces", "Starter · Beginner", "Starter · Intermediate"]);
    expect(titlesIn("Your pieces")).toEqual(["Canon in D", "Sonate No. 14"]);
    expect(titlesIn("Starter · Intermediate")).toEqual(["Minuet in G Major"]);
  });

  it("continues the piece practised last, saying when and its weakest bars", () => {
    const { onOpen } = renderScreen({ pieces: shelfPieces, now: NOW });

    const button = screen.getByRole("button", { name: "Continue Minuet in G Major" });
    expect(button.textContent).toContain("2 days ago · weakest bars 6, 14");
    fireEvent.click(button);
    expect(onOpen).toHaveBeenCalledWith("mn");
  });

  // Not hidden until hover: the way in to what to practise next.
  it("offers Progress on every card and beside Continue, always visible", () => {
    const { onProgress } = renderScreen({ pieces: shelfPieces, now: NOW });

    const links = screen.getAllByRole("button", { name: /^Progress for / });
    expect(links).toHaveLength(shelfPieces.length + 1);
    for (const link of links) expect(link.closest(".opacity-0")).toBeNull();
    fireEvent.click(screen.getAllByRole("button", { name: "Progress for Minuet in G Major" })[0]!);
    expect(onProgress).toHaveBeenCalledWith("mn");
  });

  it("says on each card when it was last practised, or that it hasn't been", () => {
    renderScreen({ pieces: shelfPieces, now: NOW });

    expect(screen.getByText("Last practised 5 days ago")).toBeTruthy();
    expect(screen.getAllByText("Not practised yet")).toHaveLength(2);
  });

  it("has no Continue before anything is practised, and says where added pieces go", () => {
    renderScreen({ pieces: [shelfPieces[0] as PieceSummary], now: NOW });

    expect(sections()).toEqual(["Your pieces", "Starter · Beginner"]);
    expect(screen.getByText(/Pieces you add appear here/)).toBeTruthy();
  });
});

describe("practisedWhen", () => {
  const now = new Date(2026, 9, 25, 0, 30).getTime();

  it("counts calendar days, not 24-hour spans", () => {
    expect(practisedWhen(new Date(2026, 9, 25, 0, 5).getTime(), now)).toBe("today");
    expect(practisedWhen(new Date(2026, 9, 24, 23, 50).getTime(), now)).toBe("yesterday");
    expect(practisedWhen(new Date(2026, 9, 19, 12).getTime(), now)).toBe("6 days ago");
  });

  it("gives a date after a week, with the year once it isn't this one", () => {
    expect(practisedWhen(new Date(2026, 9, 12).getTime(), now)).toBe("12 Oct");
    expect(practisedWhen(new Date(2025, 11, 30).getTime(), now)).toBe("30 Dec 2025");
  });
});

describe("weakestLine", () => {
  it("names one bar or two, or says nothing", () => {
    expect(weakestLine([9])).toBe("weakest bar 9");
    expect(weakestLine([6, 14])).toBe("weakest bars 6, 14");
    expect(weakestLine([])).toBeNull();
  });

  it("names the pickup in words, never bar 0", () => {
    expect(weakestLine([0])).toBe("weakest: the pickup");
    expect(weakestLine([0, 9])).toBe("weakest: the pickup, bar 9");
  });
});
