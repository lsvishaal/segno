import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { OpenedPiece, PracticeEvent } from "../bridge";
import type { CursorState } from "../components/Score";
import { PracticeScreen } from "./Practice";

// The score itself is drawn from Verovio's SVG and has its own tests; here
// only the practice loop around it is under test, so the score only records
// what it was given.
interface ScoreProps {
  readonly onBarClick?: (measure: number) => void;
  readonly startPosition?: number;
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
  endAttempt: vi.fn(),
  engravePiece: vi.fn(),
  exportMusicXml: vi.fn(),
  instruments: vi.fn(),
  listenToPractice: vi.fn(),
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
      drawn.last?.onBarClick?.(9);
    });

    expect(bridge.startAtBar).toHaveBeenCalledWith("minuet", 9);
    expect(drawn.last?.startPosition).toBe(36);
    expect(drawn.last?.cursor.position).toBe(36);
    expect(bridge.startAttempt).not.toHaveBeenCalled();
  });

  it("Start and Restart rest the cursor on the start point", async () => {
    await renderScreen();
    await act(async () => {
      drawn.last?.onBarClick?.(9);
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
      drawn.last?.onBarClick?.(40);
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

