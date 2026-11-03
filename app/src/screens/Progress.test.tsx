import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ProgressView } from "../bridge";
import { ProgressScreen, barHistory, playedFor } from "./Progress";

const bridge = vi.hoisted(() => ({ progress: vi.fn() }));
vi.mock("../bridge", () => bridge);

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

// Sat 31 Oct 2026, 15:00 local time.
const NOW = new Date(2026, 9, 31, 15, 0).getTime();
const day = (date: number) => new Date(2026, 9, date, 10).getTime();
const cell = (measure: number, level: number, pauseUs = 0, wrong = 0) => ({
  measure,
  passes: 2,
  pauseUs,
  wrong,
  stops: 0,
  level,
});
const detail = (measure: number, line: string) => ({
  measure,
  line,
  from: Math.max(1, measure - 1),
  to: measure,
});

/** The Minuet after two sessions: bar 6 paused in both. */
const MINUET: ProgressView = {
  id: "minuet",
  title: "Minuet in G Major",
  composer: "Christian Petzold",
  bars: 8,
  measures: [1, 2, 3, 4, 5, 6, 7, 8],
  sessions: [
    { startedMs: day(30), playedUs: 40_000_000, bars: [cell(5, 0), cell(6, 2, 6_200_000, 1)] },
    { startedMs: day(29), playedUs: 35_000_000, bars: [cell(5, 0), cell(6, 3, 12_400_000, 2), cell(7, 1)] },
  ],
  practised: 2,
  playedUs: 75_000_000,
  updatedMs: null,
  lastPractisedMs: day(30),
  now: [
    { measure: 5, level: 0 },
    { measure: 6, level: 2 },
    { measure: 7, level: null },
  ],
  workOnNext: [{ first: 6, last: 6, from: 5, to: 6, facts: "about 3.1 s of hesitation each time through" }],
  details: [
    detail(5, "Now: clean. In your first sessions: clean."),
    detail(6, "Now: about 3.1 s of hesitation each time through. In your first sessions: about 6.2 s of hesitation each time through."),
    detail(7, "Played once so far. Play it again to see how it's going."),
  ],
};

async function show(view: ProgressView) {
  bridge.progress.mockResolvedValue(view);
  const onPractise = vi.fn();
  await act(async () => {
    render(<ProgressScreen id={view.id} backLabel="Library" onBack={vi.fn()} onPractise={onPractise} now={NOW} />);
  });
  return onPractise;
}

describe("the Progress page", () => {
  it("says how much has been practised, when, and what the page is for", async () => {
    await show(MINUET);

    expect(screen.getByText("Minuet in G Major")).toBeTruthy();
    expect(screen.getByText(/^Practised 2 times · 1 min in total · last practised yesterday$/)).toBeTruthy();
    expect(screen.getByText("Which bars need work, and whether they're getting better.")).toBeTruthy();
    expect(document.querySelector("[data-progress-updated]")).toBeNull();
  });

  it("when the score's bars changed: one line, and the totals still count the older sessions", async () => {
    await show({ ...MINUET, practised: 5, playedUs: 200_000_000, updatedMs: day(29) });

    expect(screen.getByText("This piece was updated on 29 Oct; bar-by-bar progress starts from then.")).toBeTruthy();
    expect(screen.getByText(/^Practised 5 times · 3 min in total/)).toBeTruthy();
    expect(document.querySelector("[data-piece-map]")).toBeTruthy();
  });

  it("lists what to practise next, each practised from the bar before", async () => {
    const onPractise = await show(MINUET);

    const row = document.querySelector("[data-work-on-next]")!;
    expect(row.textContent).toContain("Bar 6 — about 3.1 s of hesitation each time through");
    fireEvent.click(screen.getAllByRole("button", { name: "⟲ Practise bars 5–6" })[0]!);
    expect(onPractise).toHaveBeenCalledWith({ from: 5, to: 6 });
  });

  it("says when every bar is secure, and offers to play it through", async () => {
    const onPractise = await show({ ...MINUET, workOnNext: [] });

    expect(screen.getByText("Every bar is secure in your recent sessions.")).toBeTruthy();
    expect(document.querySelector("[data-bar-detail]")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Play it through" }));
    expect(onPractise).toHaveBeenCalledWith(null);
  });

  it("maps every bar with a word for how it's going, and explains the words", async () => {
    await show(MINUET);

    const label = (measure: number) => document.querySelector(`[data-bar="${measure}"]`)!.getAttribute("aria-label");
    expect(document.querySelectorAll("[data-bar]")).toHaveLength(8);
    expect([label(1), label(5), label(6), label(7)]).toEqual([
      "Bar 1: Not played",
      "Bar 5: Secure",
      "Bar 6: Uneven",
      "Bar 7: Not enough yet",
    ]);
    const legend = document.querySelector("[data-legend]")!.textContent;
    for (const word of ["Secure", "Small slips", "Uneven", "Shaky", "Needs work", "Not enough yet", "Blank"]) {
      expect(legend).toContain(word);
    }
  });

  it("opens on the hardest bar, graphed over time", async () => {
    await show(MINUET);

    expect(screen.getByRole("button", { name: /^Bar 6:/ }).getAttribute("aria-pressed")).toBe("true");
    const panel = document.querySelector("[data-bar-detail]")!;
    expect(panel.textContent).toContain("Bar 6 over time");
    expect(panel.textContent).toContain("In your first sessions: about 6.2 s");
    expect(panel.querySelectorAll("[data-line-graph]")).toHaveLength(2);
    expect(panel.querySelectorAll("[data-point]")).toHaveLength(4);
  });

  it("switches to the bar clicked, with its own Practise", async () => {
    const onPractise = await show(MINUET);

    fireEvent.click(screen.getByRole("button", { name: /^Bar 5:/ }));

    expect(screen.getByRole("button", { name: /^Bar 5:/ }).getAttribute("aria-pressed")).toBe("true");
    expect(document.querySelector("[data-bar-detail]")!.textContent).toContain("Bar 5 over time");
    fireEvent.click(screen.getByRole("button", { name: "⟲ Practise bars 4–5" }));
    expect(onPractise).toHaveBeenCalledWith({ from: 4, to: 5 });
  });

  it("asks for another day before drawing a line", async () => {
    await show(MINUET);

    fireEvent.click(screen.getByRole("button", { name: /^Bar 7:/ }));

    const panel = document.querySelector("[data-bar-detail]")!;
    expect(panel.textContent).toContain("Played once so far.");
    expect(panel.textContent).toContain("Play this bar on another day to see a line.");
    expect(panel.querySelector("[data-line-graph]")).toBeNull();
  });

  it("shows a pickup as Pickup, never bar 0, and practises it from there", async () => {
    const onPractise = await show({
      ...MINUET,
      bars: 9,
      measures: [0, 1, 2, 3, 4, 5, 6, 7, 8],
      now: [{ measure: 0, level: 3 }, ...MINUET.now],
      workOnNext: [{ first: 0, last: 1, from: 0, to: 1, facts: "about 2.0 s of hesitation each time through" }],
      details: [detail(0, "Now: about 2.0 s of hesitation each time through."), ...MINUET.details],
    });

    const first = document.querySelector("[data-piece-map] button")!;
    expect(first.getAttribute("data-bar")).toBe("0");
    expect(first.textContent).toContain("Pickup");
    expect(first.getAttribute("aria-label")).toBe("Pickup: Shaky");
    expect(document.querySelector("[data-work-on-next]")!.textContent).toContain("Pickup–bar 1");
    expect(document.querySelector("[data-bar-detail]")!.textContent).toContain("Pickup over time");
    expect(document.body.textContent).not.toMatch(/bar 0\b/i);
    fireEvent.click(screen.getAllByRole("button", { name: "⟲ Practise pickup–bar 1" })[0]!);
    expect(onPractise).toHaveBeenCalledWith({ from: 0, to: 1 });
  });

  it("for a piece never played: one line and Practise, no map", async () => {
    const onPractise = await show({ ...MINUET, sessions: [], now: [], workOnNext: [], details: [], lastPractisedMs: null, practised: 0, playedUs: 0 });

    expect(screen.getByText("Play this piece once and your progress shows here.")).toBeTruthy();
    expect(document.querySelector("[data-piece-map]")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Practise" }));
    expect(onPractise).toHaveBeenCalledWith(null);
  });
});

describe("a bar over time", () => {
  it("is per time through, oldest first, skipping sessions without it", () => {
    expect(barHistory(MINUET, 6)).toEqual([
      { startedMs: day(29), hesitationS: 6.2, wrong: 1 },
      { startedMs: day(30), hesitationS: 3.1, wrong: 0.5 },
    ]);
    expect(barHistory(MINUET, 7)).toHaveLength(1);
  });
});

describe("time practised", () => {
  it("reads as seconds, minutes, then hours", () => {
    expect([playedFor(12_000_000), playedFor(240_000_000), playedFor(4_320_000_000)]).toEqual([
      "12 s",
      "4 min",
      "1 h 12 min",
    ]);
  });
});
