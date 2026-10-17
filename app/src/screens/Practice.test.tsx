import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { OpenedPiece } from "../bridge";
import { PracticeScreen } from "./Practice";

// The score itself is drawn from Verovio's SVG and has its own tests; here
// only the practice loop around it is under test.
vi.mock("../components/Score", () => ({ Score: () => null }));

const bridge = vi.hoisted(() => ({
  endAttempt: vi.fn(),
  engravePiece: vi.fn(),
  exportMusicXml: vi.fn(),
  instruments: vi.fn(),
  listenToPractice: vi.fn(),
  startAttempt: vi.fn(),
}));
vi.mock("../bridge", () => bridge);

const PIECE: OpenedPiece = {
  id: "minuet",
  title: "Minuet in G Major",
  composer: "Petzold",
  pages: [],
  marks: [],
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
  bridge.listenToPractice.mockResolvedValue(() => undefined);
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

    expect(bridge.endAttempt).toHaveBeenCalled();
    expect(onLeave).toHaveBeenCalledOnce();
  });

  it("has nothing to end before an attempt has started", async () => {
    const { onLeave } = await renderScreen();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "← Library" }));
    });

    expect(bridge.endAttempt).not.toHaveBeenCalled();
    expect(onLeave).toHaveBeenCalledOnce();
  });

  it("ends a running attempt when the screen is removed some other way", async () => {
    const { unmount } = await renderScreen();
    await startPracticing();

    await act(async () => {
      unmount();
    });

    expect(bridge.endAttempt).toHaveBeenCalledOnce();
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
