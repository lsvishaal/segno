import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { AttemptSummary, OpenedPiece, PracticeEvent, ReviewBar } from "../bridge";
import type { CursorState, ScoreReview } from "../components/Score";
import { PracticeScreen } from "./Practice";

// The score itself is drawn from Verovio's SVG and has its own tests; here
// only the practice loop around it is under test, so the score only records
// what it was given.
interface ScoreProps {
  readonly onBarClick?: (measure: number, how: { readonly shift: boolean }) => void;
  readonly startPosition?: number;
  readonly loop?: { readonly first: number; readonly last: number } | null;
  readonly review?: ScoreReview | null;
  readonly cursor: CursorState;
}
const drawn = vi.hoisted(() => ({ last: null as ScoreProps | null }));
vi.mock("../components/Score", () => ({
  Score: (props: ScoreProps) => {
    drawn.last = props;
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
  // Started at bar 8, trouble going into bar 9, stopped in bar 10.
  const summary = (
    suggestion: AttemptSummary["review"]["suggestion"],
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
      suggestion,
    },
  });
  const LOOP = { label: "Loop bars 8–9", first: 8, last: 9 };
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

  it("replaces the score with the results: big numbers, the chart, the line", async () => {
    await stopWith(summary(LOOP));

    const results = within(screen.getByRole("region", { name: "Results" }));
    expect(results.getByText("86%")).toBeInTheDocument();
    expect(results.getByText("72")).toBeInTheDocument();
    expect(results.getByText("tempo · ♩ / min")).toBeInTheDocument();
    expect(results.getByText("12/2/80")).toBeInTheDocument();
    expect(results.getByText("81%")).toBeInTheDocument();
    expect(results.getByText(/most trouble going into bar 9/)).toBeInTheDocument();
    const chart = within(results.getByRole("list"));
    expect(chart.getAllByRole("listitem")).toHaveLength(5);
    expect(chart.getByRole("listitem", { name: "Bar 11 · not reached" })).toHaveAttribute("data-reached", "false");
  });

  it("the suggestion loops its bars and starts playing them, by click or Enter", async () => {
    await stopWith(summary(LOOP));
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "⟲ Loop bars 8–9" }));
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
    await stopWith(summary(LOOP));
    await press("Enter");
    expect(bridge.loopBars).toHaveBeenCalledWith("minuet", 8, 9, 3);
  });

  it("R plays again; with nothing to loop, Practice again is the main button", async () => {
    await stopWith(summary({ label: "Again", first: null, last: null }));
    expect(screen.queryByRole("button", { name: /Again$/ })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "↻ Practice again" })).toBeInTheDocument();

    await press("r");
    expect(bridge.startAttempt).toHaveBeenCalledTimes(2);
    expect(bridge.loopBars).not.toHaveBeenCalled();
  });

  it("Show on the score leaves the marks there; Esc goes back and forth", async () => {
    await stopWith(summary(LOOP));
    expect(drawn.last?.review?.notReached).toEqual([11, 12]);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "♪ Show on the score" }));
    });
    expect(screen.queryByRole("region", { name: "Results" })).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Your last run is marked on the score");
    expect(drawn.last?.review?.marks).toEqual([{ position: 36, correct: true, wrong: 2 }]);

    await press("Escape");
    expect(screen.getByRole("region", { name: "Results" })).toBeInTheDocument();
  });

  it("nothing played: no results, a quiet note, and Start again", async () => {
    await stopWith(summary(null, []));

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
