import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { AttemptSummary, OpenedPiece, PracticeEvent, ReviewBar, ReviewCard } from "../bridge";
import type { CursorState, Dot, ScoreHandle, ScoreReview } from "../components/Score";
import { PracticeScreen } from "./Practice";

// The score itself is drawn from Verovio's SVG and has its own tests; here
// only the practice loop around it is under test, so the score only records
// what it was given.
interface ScoreProps {
  readonly onBarClick?: (measure: number, how: { readonly shift: boolean }) => void;
  readonly startPosition?: number;
  readonly loop?: { readonly first: number; readonly last: number } | null;
  readonly review?: ScoreReview | null;
  readonly weak?: ReadonlyMap<number, number> | null;
  readonly showNoteNames?: boolean;
  readonly cursor: CursorState;
  readonly dot: Dot | null;
  readonly ref?: { current: ScoreHandle | null };
}
const drawn = vi.hoisted(() => ({ last: null as ScoreProps | null }));
/** What the screen asks of the drawn score: the live marks. */
const score = vi.hoisted(() => ({
  cutOut: vi.fn(() => null),
  showBar: vi.fn(),
  markPlayed: vi.fn(),
  clearMarks: vi.fn(),
}));
vi.mock("../components/Score", () => ({
  Score: (props: ScoreProps) => {
    drawn.last = props;
    if (props.ref) props.ref.current = score;
    return null;
  },
}));

const bridge = vi.hoisted(() => ({
  clearLoop: vi.fn(),
  endAttempt: vi.fn(),
  engravePiece: vi.fn(),
  exportMusicXml: vi.fn(),
  instruments: vi.fn(),
  listenToPractice: vi.fn(),
  loopBars: vi.fn(),
  loopTarget: vi.fn(),
  startAtBar: vi.fn(),
  startAttempt: vi.fn(),
  weakBars: vi.fn(),
}));

/** What the practice thread says, delivered as it would be. */
let hear: (event: PracticeEvent) => void = () => undefined;
vi.mock("../bridge", () => bridge);

const PIECE: OpenedPiece = {
  id: "minuet",
  title: "Minuet in G Major",
  composer: "Petzold",
  pages: [],
  marks: [],
  bars: [],
};

class QuietResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

beforeEach(() => {
  vi.stubGlobal("ResizeObserver", QuietResizeObserver);
  vi.stubGlobal("localStorage", { getItem: () => null, setItem: () => undefined });

  bridge.instruments.mockResolvedValue([{ name: "Digital Piano" }]);
  bridge.listenToPractice.mockImplementation((listener: (event: PracticeEvent) => void) => {
    hear = listener;
    return Promise.resolve(() => undefined);
  });
  // Bar 9 of the Minuet is step 36 in played order.
  bridge.startAtBar.mockResolvedValue({ position: 36, measure: 9 });
  bridge.startAttempt.mockResolvedValue(undefined);
  bridge.loopBars.mockResolvedValue({ position: 36, first: 9, last: 12 });
  bridge.clearLoop.mockResolvedValue(undefined);
  bridge.loopTarget.mockResolvedValue(undefined);
  bridge.endAttempt.mockResolvedValue({});
  bridge.engravePiece.mockResolvedValue(PIECE);
  bridge.weakBars.mockResolvedValue({ runs: 0, bars: [] });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

async function renderScreen() {
  const onLeave = vi.fn();
  let rendered: ReturnType<typeof render> | undefined;
  await act(async () => {
    rendered = render(<PracticeScreen piece={PIECE} onLeave={onLeave} />);
  });
  return { onLeave, unmount: () => rendered?.unmount() };
}

async function startPracticing() {
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: "Start" }));
  });
}

describe("leaving the practice screen", () => {
  it("ends a running attempt, so what was played is kept", async () => {
    const { onLeave } = await renderScreen();
    await startPracticing();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "← Library" }));
    });

    expect(bridge.endAttempt).toHaveBeenCalledWith("left");
    expect(onLeave).toHaveBeenCalledOnce();
  });

  it("says it is leaving even before an attempt has started, so the start resets", async () => {
    const { onLeave } = await renderScreen();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "← Library" }));
    });

    expect(bridge.endAttempt).toHaveBeenCalledWith("left");
    expect(onLeave).toHaveBeenCalledOnce();
  });

  it("ends a running attempt when the screen is removed some other way", async () => {
    const { unmount } = await renderScreen();
    await startPracticing();

    await act(async () => {
      unmount();
    });

    expect(bridge.endAttempt).toHaveBeenCalledOnce();
    expect(bridge.endAttempt).toHaveBeenCalledWith("left");
  });
});

describe("restarting", () => {
  it("starts again on the same instrument without ending the attempt", async () => {
    await renderScreen();
    await startPracticing();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Restart" }));
    });

    expect(bridge.startAttempt).toHaveBeenCalledTimes(2);
    expect(bridge.startAttempt).toHaveBeenLastCalledWith("minuet", "Digital Piano");
    expect(bridge.endAttempt).not.toHaveBeenCalled();
  });

  it("does the same on the R key", async () => {
    await renderScreen();
    await startPracticing();

    await act(async () => {
      fireEvent.keyDown(window, { key: "r" });
    });

    expect(bridge.startAttempt).toHaveBeenCalledTimes(2);
  });

  it("ignores R with a modifier key held, and before an attempt has started", async () => {
    await renderScreen();

    await act(async () => {
      fireEvent.keyDown(window, { key: "r" });
    });
    expect(bridge.startAttempt).not.toHaveBeenCalled();

    await startPracticing();
    await act(async () => {
      fireEvent.keyDown(window, { key: "r", ctrlKey: true });
    });
    expect(bridge.startAttempt).toHaveBeenCalledTimes(1);
  });
});

describe("choosing the instrument", () => {
  it("offers every connected keyboard in a select, and starts on the chosen one", async () => {
    bridge.instruments.mockResolvedValue([{ name: "Digital Piano" }, { name: "Second Keyboard" }]);
    await renderScreen();

    const select = screen.getByRole("combobox", { name: "Instrument" });
    expect(select).toHaveValue("Digital Piano");

    fireEvent.change(select, { target: { value: "Second Keyboard" } });
    await startPracticing();

    expect(bridge.startAttempt).toHaveBeenCalledWith("minuet", "Second Keyboard");
  });
});

describe("starting from a bar", () => {
  it("asks the practice thread to start at the clicked bar, and marks where", async () => {
    await renderScreen();

    await act(async () => {
      drawn.last?.onBarClick?.(9, { shift: false });
    });

    expect(bridge.startAtBar).toHaveBeenCalledWith("minuet", 9);
    expect(drawn.last?.startPosition).toBe(36);
    expect(drawn.last?.cursor.position).toBe(36);
    expect(bridge.startAttempt).not.toHaveBeenCalled();
  });

  it("Start and Restart rest the cursor on the start point", async () => {
    await renderScreen();
    await act(async () => {
      drawn.last?.onBarClick?.(9, { shift: false });
    });
    await startPracticing();
    expect(drawn.last?.cursor.position).toBe(36);

    await act(async () => {
      hear({ kind: "advanced", position: 37 });
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Restart" }));
    });

    expect(drawn.last?.cursor.position).toBe(36);
  });

  it("ignores clicks on the score while the keyboard is lost", async () => {
    await renderScreen();
    await startPracticing();

    await act(async () => {
      hear({ kind: "instrumentLost", instrument: "Digital Piano", secondsRemaining: 60 });
    });
    expect(drawn.last?.onBarClick).toBeUndefined();

    await act(async () => {
      hear({ kind: "instrumentRestored" });
    });
    expect(drawn.last?.onBarClick).toBeTypeOf("function");
  });

  it("says why when a bar has nothing to start at", async () => {
    bridge.startAtBar.mockRejectedValue("There is nothing to start at from bar 40.");
    await renderScreen();

    await act(async () => {
      drawn.last?.onBarClick?.(40, { shift: false });
    });

    expect(screen.getByRole("status")).toHaveTextContent("nothing to start at from bar 40");
    expect(drawn.last?.startPosition).toBe(0);
  });
});

describe("ending", () => {
  it("Stop says stop", async () => {
    await renderScreen();
    await startPracticing();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    });

    expect(bridge.endAttempt).toHaveBeenCalledWith("stop");
  });

  it("the end of the piece finishes by itself, a moment after the last note", async () => {
    // Only the timeouts: the screen's animations keep their own clock.
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    try {
      await renderScreen();
      await startPracticing();
      await act(async () => {
        hear({ kind: "reachedEnd" });
      });
      await act(async () => {
        vi.advanceTimersByTime(1000);
      });
      // A trill's tail: the wait starts again.
      await act(async () => {
        hear({ kind: "played", position: 407, pitch: 67, correct: true });
      });
      await act(async () => {
        vi.advanceTimersByTime(1000);
      });
      expect(bridge.endAttempt).not.toHaveBeenCalled();

      await act(async () => {
        vi.advanceTimersByTime(600);
      });
      expect(bridge.endAttempt).toHaveBeenCalledWith("finished");
    } finally {
      vi.useRealTimers();
    }
  });

  it("Finish at the end of the piece says finished", async () => {
    await renderScreen();
    await startPracticing();
    await act(async () => {
      hear({ kind: "reachedEnd" });
    });

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Finish" }));
    });

    expect(bridge.endAttempt).toHaveBeenCalledWith("finished");
  });
});

describe("looping", () => {
  const click = async (measure: number, shift = false) => {
    await act(async () => {
      drawn.last?.onBarClick?.(measure, { shift });
    });
  };
  const press = async (key: string) => {
    await act(async () => {
      fireEvent.keyDown(window, { key });
    });
  };

  it("Shift+click loops from the start bar to the bar clicked, and shows the band", async () => {
    await renderScreen();
    await click(9);
    await click(12, true);

    expect(bridge.loopBars).toHaveBeenCalledWith("minuet", 9, 12, 3);
    expect(drawn.last?.loop).toEqual({ first: 9, last: 12 });
    expect(drawn.last?.startPosition).toBe(36);
    expect(drawn.last?.cursor.position).toBe(36);
    expect(screen.getByRole("button", { name: "⟲ Loop 9–12" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("a plain click starts there and ends the loop", async () => {
    await renderScreen();
    await click(12, true);
    await click(3);

    expect(drawn.last?.loop).toBeNull();
  });

  it("L loops the bar the cursor is in, and again stops looping", async () => {
    await renderScreen();
    await press("l");
    expect(bridge.loopBars).toHaveBeenCalledWith("minuet", 1, 1, 3);

    await press("l");
    expect(bridge.clearLoop).toHaveBeenCalledOnce();
    expect(drawn.last?.loop).toBeNull();
  });

  it("Esc stops looping", async () => {
    await renderScreen();
    await click(12, true);
    await press("Escape");

    expect(bridge.clearLoop).toHaveBeenCalledOnce();
    expect(drawn.last?.loop).toBeNull();
  });

  it("counts clean passes in a row, and a spoiled pass shows zero at once", async () => {
    await renderScreen();
    await startPracticing();
    await click(12, true);

    await act(async () => {
      hear({ kind: "looped", position: 36, cleanInARow: 2 });
    });
    expect(screen.getByText("2 clean in a row")).toBeInTheDocument();
    expect(drawn.last?.cursor.position).toBe(36);

    await act(async () => {
      hear({ kind: "loopCount", cleanInARow: 0 });
    });
    expect(screen.getByText("0 clean in a row")).toBeInTheDocument();
  });

  it("changing the target while looping tells the practice thread", async () => {
    await renderScreen();
    await click(12, true);

    await act(async () => {
      fireEvent.change(screen.getByRole("combobox", { name: "stop at" }), {
        target: { value: "" },
      });
    });

    expect(bridge.loopTarget).toHaveBeenCalledWith(null);
  });

  it("says so calmly when the target is reached, and the band goes", async () => {
    await renderScreen();
    await startPracticing();
    await click(12, true);

    await act(async () => {
      hear({ kind: "loopDone", position: 36, passes: 3, first: 9, last: 12, widen: null });
    });

    expect(screen.getByRole("status")).toHaveTextContent("3 clean passes · bars 9–12");
    expect(drawn.last?.loop).toBeNull();
    expect(drawn.last?.cursor.position).toBe(36);
  });

  it("cannot loop while the keyboard is lost", async () => {
    await renderScreen();
    await startPracticing();
    await act(async () => {
      hear({ kind: "instrumentLost", instrument: "Digital Piano", secondsRemaining: 60 });
    });

    expect(screen.getByRole("button", { name: "⟲ Loop" })).toBeDisabled();
    await press("l");
    expect(bridge.loopBars).not.toHaveBeenCalled();
  });
});

describe("the results after a run", () => {
  const point = (measure: number, state: ReviewBar["state"], tempo: number | null, wrong = 0): ReviewBar => ({
    measure,
    state,
    tempo,
    pauseMs: 0,
    intoMs: 0,
    wrong,
    breaks: 0,
    line: `Bar ${measure}`,
  });
  const card = (kind: ReviewCard["kind"], first: number, last: number, title: string): ReviewCard => ({
    kind,
    first,
    last,
    drillFirst: kind === "clean" ? null : first,
    drillLast: kind === "clean" ? null : last,
    pauseInto: kind === "intoBar" ? last : null,
    title,
    what: `What happened in ${title}.`,
    why: kind === "clean" ? "" : "Why it happens.",
    steps: ["First this.", "Then that."],
    button: kind === "clean" ? null : first === last ? `Practise bar ${first}` : `Practise bars ${first}–${last}`,
    also: null,
  });
  const INTO_9 = card("intoBar", 8, 9, "Going into bar 9");
  // Started at bar 8, trouble going into bar 9, stopped in bar 10.
  const summary = (
    cards: readonly ReviewCard[],
    bars: readonly ReviewBar[] = [
      point(8, "played", 80),
      point(9, "played", 31, 2),
      point(10, "played", 76),
      point(11, "notReached", null),
      point(12, "notReached", null),
    ],
  ): AttemptSummary => ({
    lines: ["Bar 9: G4 was written; something else was played."],
    correct: 12,
    incorrect: 2,
    omitted: 80,
    unexpected: 0,
    reachedTheEnd: false,
    autoEnded: false,
    reason: null,
    review: {
      headline: "Bars 8–10 · 24 s · most trouble going into bar 9 (3.2 s pause, 2 wrong).",
      numbers: {
        accuracy: 86,
        tempo: 72,
        rhythm: true,
        steadiness: 81,
        right: 12,
        wrong: 2,
        missed: 80,
        longestPauseMs: 3200,
        longestPauseBar: 9,
        timeMs: 24000,
        restarts: 0,
      },
      bars,
      passes: [],
      marks: [{ position: 36, correct: true, wrong: 2 }],
      suggestion: null,
      cards,
      wentWell: bars.length > 0 ? ["Bars 1–4 with no mistakes and no stops"] : [],
      note: null,
    },
  });
  const stopWith = async (ended: AttemptSummary) => {
    await renderScreen();
    await startPracticing();
    await act(async () => {
      hear({ kind: "ended", summary: ended });
    });
  };
  const press = async (key: string) => {
    await act(async () => {
      fireEvent.keyDown(window, { key });
    });
  };

  it("replaces the score with what to work on: each card says what, why, how, with its bars", async () => {
    await stopWith(summary([INTO_9, card("wrongNotes", 10, 10, "Wrong notes in bar 10")]));

    const results = within(screen.getByRole("region", { name: "Results" }));
    expect(results.getByText(/most trouble going into bar 9/)).toBeInTheDocument();
    expect(results.getByText(/accuracy 86%/)).toBeInTheDocument();
    expect(results.getByText("Work on these")).toBeInTheDocument();
    const first = within(results.getByRole("article", { name: "Going into bar 9" }));
    expect(first.getByText("What happened in Going into bar 9.")).toBeInTheDocument();
    expect(first.getByText("Why it happens.")).toBeInTheDocument();
    expect(first.getAllByRole("listitem")).toHaveLength(2);
    expect(first.getByRole("button", { name: "Bars 8–9 · show on the score" })).toBeInTheDocument();
    expect(results.getByRole("article", { name: "Wrong notes in bar 10" })).toBeInTheDocument();
    expect(results.getByText("Bars 1–4 with no mistakes and no stops")).toBeInTheDocument();
  });

  it("a card's button loops its bars and starts playing them, by click or Enter", async () => {
    await stopWith(summary([INTO_9]));
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "⟲ Practise bars 8–9" }));
    });
    expect(bridge.loopBars).toHaveBeenCalledWith("minuet", 8, 9, 3);
    expect(bridge.startAttempt).toHaveBeenCalledTimes(2);
    // Practising again: the results fade out as the score comes back.
    expect(screen.getByRole("button", { name: "Stop" })).toBeInTheDocument();

    cleanup();
    vi.clearAllMocks();
    bridge.loopBars.mockResolvedValue({ position: 32, first: 8, last: 9 });
    bridge.startAttempt.mockResolvedValue(undefined);
    bridge.instruments.mockResolvedValue([{ name: "Digital Piano" }]);
    await stopWith(summary([INTO_9]));
    await press("Enter");
    expect(bridge.loopBars).toHaveBeenCalledWith("minuet", 8, 9, 3);
  });

  it("stopping in the same bar loops it with the bar before; a reading card can turn note names on", async () => {
    const stopping = { ...card("keepsStopping", 9, 9, "Stopping at bar 9"), drillFirst: 8, button: "Practise bars 8–9" };
    bridge.loopBars.mockResolvedValue({ position: 32, first: 8, last: 9 });
    await stopWith(summary([stopping, card("wrongNotes", 10, 10, "Wrong notes in bar 10")]));
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Turn on note names" }));
    });
    expect(drawn.last?.showNoteNames).toBe(true);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "⟲ Practise bars 8–9" }));
    });
    expect(bridge.loopBars).toHaveBeenCalledWith("minuet", 8, 9, 3);
    expect(bridge.startAtBar).not.toHaveBeenCalled();
    expect(bridge.startAttempt).toHaveBeenCalledTimes(2);
  });

  it("a clean run has no list to work on; R plays again", async () => {
    await stopWith(summary([card("clean", 8, 10, "A clean run")]));
    expect(screen.queryByText("Work on these")).not.toBeInTheDocument();
    expect(screen.getByRole("article", { name: "A clean run" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "↻ Practice again" })).toBeInTheDocument();

    await press("r");
    expect(bridge.startAttempt).toHaveBeenCalledTimes(2);
    expect(bridge.loopBars).not.toHaveBeenCalled();
  });

  it("the numbers and the chart wait under Details", async () => {
    await stopWith(summary([INTO_9]));

    const results = within(screen.getByRole("region", { name: "Results" }));
    expect(results.getByText("Details")).toBeInTheDocument();
    expect(results.getByText("12/2/80")).toBeInTheDocument();
    const chart = within(results.getByRole("figure"));
    expect(chart.getAllByRole("listitem")).toHaveLength(5);
  });

  it("Show on the score leaves the marks there; Esc goes back and forth", async () => {
    await stopWith(summary([INTO_9]));
    expect(drawn.last?.review?.notReached).toEqual([11, 12]);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "♪ Show on the score" }));
    });
    // The results fade out before they go.
    await waitFor(() => expect(screen.queryByRole("region", { name: "Results" })).not.toBeInTheDocument());
    expect(screen.getByRole("status")).toHaveTextContent("Your last run is marked on the score");
    // The run's own marks are still drawn: cleared only on opening the piece and at Start.
    expect(score.clearMarks).toHaveBeenCalledTimes(2);

    await press("Escape");
    expect(screen.getByRole("region", { name: "Results" })).toBeInTheDocument();
  });

  it("nothing played: no results, a quiet note, and Start again", async () => {
    await stopWith(summary([], []));

    expect(screen.queryByRole("region", { name: "Results" })).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Nothing played.");
    expect(screen.getByRole("button", { name: "Start" })).toBeInTheDocument();
  });

  it("nothing sits above the score while practising: a loop done goes to the status line", async () => {
    await renderScreen();
    await startPracticing();
    await act(async () => {
      hear({
        kind: "loopDone",
        position: 36,
        passes: 3,
        first: 9,
        last: 9,
        widen: { label: "Loop bars 8–10", first: 8, last: 10 },
      });
    });

    expect(screen.getByRole("status")).toHaveTextContent("3 clean passes · bar 9");
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Join it back up · Loop bars 8–10" }));
    });
    expect(bridge.loopBars).toHaveBeenCalledWith("minuet", 8, 10, 3);
  });
});

describe("weak bars over time", () => {
  const history = { runs: 3, bars: [{ measure: 6, level: 4 }, { measure: 2, level: 0 }] };
  const press = async (key: string) => {
    await act(async () => {
      fireEvent.keyDown(window, { key });
    });
  };

  it("tints the bars that have been hard, with a legend saying what it means", async () => {
    bridge.weakBars.mockResolvedValue(history);
    await renderScreen();

    expect(bridge.weakBars).toHaveBeenCalledWith("minuet");
    expect([...(drawn.last?.weak ?? [])]).toEqual([[6, 4]]);
    expect(screen.getByText(/Weak bars from 3 runs/)).toBeTruthy();
  });

  it("W hides them and shows them again", async () => {
    bridge.weakBars.mockResolvedValue(history);
    await renderScreen();

    await press("w");
    expect(drawn.last?.weak).toBeNull();
    // The legend fades out before it goes.
    await waitFor(() => expect(screen.queryByText(/Weak bars/)).toBeNull());

    await press("W");
    expect(drawn.last?.weak?.get(6)).toBe(4);
  });

  it("are off while playing, and come back up to date once the run ends", async () => {
    bridge.weakBars.mockResolvedValue(history);
    await renderScreen();
    await startPracticing();
    await act(async () => {
      hear({ kind: "played", position: 0, pitch: 67, correct: true });
    });
    expect(drawn.last?.weak).toBeNull();

    // Nothing worth reviewing: back to the start, with the history re-read.
    await act(async () => {
      hear({ kind: "ended", summary: { review: { bars: [] }, reason: null } as unknown as AttemptSummary });
    });
    expect(bridge.weakBars).toHaveBeenCalledTimes(2);
    expect(drawn.last?.weak?.get(6)).toBe(4);
  });

  it("says so when every bar has gone fine, and shows nothing for a piece never played", async () => {
    bridge.weakBars.mockResolvedValue({ runs: 2, bars: [{ measure: 1, level: 0 }] });
    const { unmount } = await renderScreen();
    expect(screen.getByText("No weak bars yet (from 2 runs)")).toBeTruthy();
    unmount();

    bridge.weakBars.mockResolvedValue({ runs: 0, bars: [] });
    await renderScreen();
    expect(screen.queryByText(/weak bars/i)).toBeNull();
  });
});

describe("live marks", () => {
  const played = (correct: boolean): PracticeEvent => ({ kind: "played", position: 36, pitch: 67, correct });

  it("each note played is marked on the score and stays; no flash", async () => {
    await renderScreen();
    await startPracticing();
    await act(async () => {
      hear(played(true));
      hear(played(false));
    });

    expect(score.markPlayed).toHaveBeenNthCalledWith(1, { kind: "correct", position: 36, pitch: 67 });
    expect(score.markPlayed).toHaveBeenNthCalledWith(2, { kind: "wrong", position: 36, pitch: 67 });
    expect(drawn.last?.dot).toBeNull();
  });

  it("clear on a new attempt, a loop pass and a click on a bar", async () => {
    await renderScreen();
    await startPracticing();
    expect(score.clearMarks).toHaveBeenCalled();
    score.clearMarks.mockClear();

    await act(async () => {
      fireEvent.keyDown(window, { key: "r" });
    });
    expect(score.clearMarks).toHaveBeenCalledTimes(1);

    await act(async () => {
      hear({ kind: "looped", position: 36, cleanInARow: 0 });
    });
    expect(score.clearMarks).toHaveBeenCalledTimes(2);

    await act(async () => {
      drawn.last?.onBarClick?.(9, { shift: false });
    });
    expect(score.clearMarks).toHaveBeenCalledTimes(3);
  });

  it("turned off in Settings: the brief flash instead, remembered", async () => {
    const stored = new Map<string, string>([["hogwash.marks", "off"]]);
    vi.stubGlobal("localStorage", { getItem: (key: string) => stored.get(key) ?? null, setItem: (key: string, value: string) => stored.set(key, value) });
    await renderScreen();
    await startPracticing();
    await act(async () => {
      hear(played(true));
    });

    expect(score.markPlayed).not.toHaveBeenCalled();
    expect(drawn.last?.dot).toEqual({ kind: "correct", position: 36, pitch: 67 });
    expect(stored.get("hogwash.marks")).toBe("off");
  });
});
