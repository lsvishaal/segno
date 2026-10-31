import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ProgressView } from "../bridge";
import { ProgressScreen, playedFor } from "./Progress";

const bridge = vi.hoisted(() => ({ progress: vi.fn() }));
vi.mock("../bridge", () => bridge);

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

// Sat 31 Oct 2026, 15:00 local time.
const NOW = new Date(2026, 9, 31, 15, 0).getTime();
const day = (date: number) => new Date(2026, 9, date, 10).getTime();
const cell = (measure: number, level: number) => ({
  measure,
  passes: 2,
  pauseUs: 0,
  wrong: 0,
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
  sessions: [
    { startedMs: day(30), playedUs: 40_000_000, bars: [cell(5, 0), cell(6, 2)] },
    { startedMs: day(29), playedUs: 35_000_000, bars: [cell(5, 0), cell(6, 3)] },
  ],
  playedUs: 75_000_000,
  lastPractisedMs: day(30),
  now: [
    { measure: 5, level: 0 },
    { measure: 6, level: 2 },
    { measure: 7, level: null },
  ],
  workOnNext: [{ first: 6, last: 6, from: 5, to: 6, facts: "avg pause 3.1 s" }],
  details: [
    detail(5, "avg pause 0.0 s → 0.0 s over 2 sessions · wrong notes 0 → 0 per pass"),
    detail(6, "avg pause 6.2 s → 3.1 s over 2 sessions · wrong notes 0 → 0 per pass"),
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
  it("says how much has been practised, and when", async () => {
    await show(MINUET);

    expect(screen.getByText("Minuet in G Major")).toBeTruthy();
    expect(screen.getByText(/^2 sessions · 1 min · last practised yesterday$/)).toBeTruthy();
  });

  it("lists what to work on next, each practised from the bar before", async () => {
    const onPractise = await show(MINUET);

    const row = screen.getByText("Bar 6").closest("li")!;
    expect(row.textContent).toContain("Bar 6 · avg pause 3.1 s");
    fireEvent.click(screen.getByRole("button", { name: "⟲ Practise bars 5–6" }));
    expect(onPractise).toHaveBeenCalledWith({ from: 5, to: 6 });
  });

  it("says when every bar is clean, and offers to play it through", async () => {
    const onPractise = await show({ ...MINUET, workOnNext: [] });

    expect(screen.getByText("Every bar is clean in your recent sessions.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Play it through" }));
    expect(onPractise).toHaveBeenCalledWith(null);
  });

  it("draws one row per session, newest first, with now as heights", async () => {
    await show(MINUET);

    const column = (measure: number) => document.querySelector(`[data-bar-column="${measure}"]`)!;
    const cells = (measure: number) =>
      [...column(measure).querySelectorAll("[data-cell]")].map((element) => element.getAttribute("data-cell"));
    expect(cells(6)).toEqual(["2", "3"]);
    expect(cells(1)).toEqual(["", ""]);
    expect(column(6).querySelector("[data-now]")?.getAttribute("data-now")).toBe("2");
    expect(column(7).querySelector("[data-now]")?.getAttribute("data-now")).toBe("null");
  });

  it("opens a bar's story on a click, marking its column", async () => {
    const onPractise = await show(MINUET);

    fireEvent.click(screen.getByRole("button", { name: /^Bar 6:/ }));

    expect(screen.getByRole("button", { name: /^Bar 6:/ }).getAttribute("aria-pressed")).toBe("true");
    const panel = document.querySelector("[data-bar-detail]")!;
    expect(panel.textContent).toContain("Bar 6 · avg pause 6.2 s → 3.1 s over 2 sessions");
    fireEvent.click(screen.getAllByRole("button", { name: "⟲ Practise bars 5–6" })[1]!);
    expect(onPractise).toHaveBeenCalledWith({ from: 5, to: 6 });
  });

  it("shows the last 12 sessions until asked for all", async () => {
    const sessions = Array.from({ length: 14 }, (_, index) => ({
      startedMs: day(30) - index * 86_400_000,
      playedUs: 1_000_000,
      bars: [cell(1, 0)],
    }));
    await show({ ...MINUET, sessions });

    const rows = () => document.querySelectorAll('[data-bar-column="1"] [data-cell]').length;
    expect(rows()).toBe(12);
    fireEvent.click(screen.getByRole("button", { name: "Show all 14 sessions" }));
    expect(rows()).toBe(14);
  });

  it("for a piece never played: one line and Practise, no grid", async () => {
    const onPractise = await show({ ...MINUET, sessions: [], now: [], workOnNext: [], details: [], lastPractisedMs: null, playedUs: 0 });

    expect(screen.getByText("Play this piece once and your progress shows here.")).toBeTruthy();
    expect(document.querySelector("[data-bar-column]")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Practise" }));
    expect(onPractise).toHaveBeenCalledWith(null);
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
