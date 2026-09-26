import { act, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { Mark } from "../bridge";
import minuet from "./__fixtures__/minuet-in-g.page1.svg?raw";
import { marksOf } from "./__fixtures__/pages";
import { Score } from "./Score";

/**
 * The mechanism `MVP §6.6` rests on.
 *
 * `ADR-0012` chose a webview on the strength of one claim: that a note's
 * on-screen position can be trusted, because the browser's own layout is
 * doing the measuring rather than a static-file geometry library parsing
 * output after the fact. jsdom does no real layout at all — every element's
 * `getBoundingClientRect()` reports zero — so the geometry helpers below are
 * tested directly, against a stubbed set of rects standing in for what a
 * real browser would report, rather than through a full component render
 * (which would only prove Motion mounted, not that the geometry is right).
 */

function stubbedRect(left: number, top: number, width: number, height: number): DOMRect {
  return {
    left,
    top,
    width,
    height,
    right: left + width,
    bottom: top + height,
    x: left,
    y: top,
    toJSON: () => ({}),
  };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("the score component", () => {
  const PAGE = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 100">
    <g id="note-a" class="note"><ellipse cx="40" cy="50" rx="8" ry="6"/></g>
  </svg>`;

  const MARKS: readonly Mark[] = [
    { measure: 1, position: 0, page: 0, notes: [{ elementId: "note-a", step: "C", octave: 4, pitch: 60 }] },
  ];

  it("puts the engraving in the document", () => {
    const { container } = render(
      <Score pages={[PAGE]} marks={MARKS} cursor={{ position: 0 }} dot={null} followCursor={false} />,
    );

    expect(container.querySelector("#note-a")).not.toBeNull();
  });

  // The SVG derives from a file the performer supplied. The CSP already
  // refuses inline script; this is the second lock.
  it("labels notes with their letter name when enabled", async () => {
    const { container, findByText } = render(
      <Score
        pages={[PAGE]}
        marks={MARKS}
        cursor={{ position: 0 }}
        dot={null}
        followCursor={false}
        showNoteNames
      />,
    );

    const label = await findByText("C");
    expect(label).toHaveClass("hw-note-name");
    expect(container.querySelector(".hw-note-name")).not.toBeNull();
  });

  // A repeat marks the same drawn note once per pass.
  it("labels a note in a repeated bar once", async () => {
    const repeated: readonly Mark[] = [
      { measure: 1, position: 0, page: 0, notes: [{ elementId: "note-a", step: "C", octave: 4, pitch: 60 }] },
      { measure: 1, position: 1, page: 0, notes: [{ elementId: "note-a", step: "C", octave: 4, pitch: 60 }] },
    ];

    const { container, findByText } = render(
      <Score pages={[PAGE]} marks={repeated} cursor={{ position: 0 }} dot={null} followCursor={false} showNoteNames />,
    );

    await findByText("C");
    expect(container.querySelectorAll(".hw-note-name")).toHaveLength(1);
  });

  it("shows no labels when the toggle is off", () => {
    const { container } = render(
      <Score pages={[PAGE]} marks={MARKS} cursor={{ position: 0 }} dot={null} followCursor={false} />,
    );

    expect(container.querySelector(".hw-note-name")).toBeNull();
  });

  it("strips anything executable from the engraving", () => {
    const hostile = `<svg xmlns="http://www.w3.org/2000/svg">
      <script>globalThis.__hogwash_escaped = true;</script>
      <g id="note-a" class="note" onload="globalThis.__hogwash_escaped = true">
        <ellipse cx="10" cy="10" rx="5" ry="5"/>
      </g>
    </svg>`;

    const { container } = render(
      <Score
        pages={[hostile]}
        marks={MARKS}
        cursor={{ position: 0 }}
        dot={null}
        followCursor={false}
      />,
    );

    expect(container.querySelector("script")).toBeNull();
    expect(container.querySelector("#note-a")).not.toHaveAttribute("onload");
  });

  // A correct note was written, so it has a real element to recolour
  // directly — no separate dot to position, and therefore nothing that can
  // drift out of alignment with it (`Score.tsx`'s module doc).
  it("flashes the real notehead green on a correct note, then fades it back", () => {
    vi.useFakeTimers();

    const { container, rerender } = render(
      <Score pages={[PAGE]} marks={MARKS} cursor={{ position: 0 }} dot={null} followCursor={false} />,
    );

    rerender(
      <Score
        pages={[PAGE]}
        marks={MARKS}
        cursor={{ position: 0 }}
        dot={{ kind: "correct", position: 0, pitch: 60 }}
        followCursor={false}
      />,
    );

    const note = container.querySelector("#note-a") as SVGElement;
    expect(note.style.color).toBe("var(--hw-correct)");

    vi.advanceTimersByTime(420);
    expect(note.style.color).toBe("");

    vi.useRealTimers();
  });

  it("never produces a green dot overlay — the notehead itself is what flashes", () => {
    const { container } = render(
      <Score
        pages={[PAGE]}
        marks={MARKS}
        cursor={{ position: 0 }}
        dot={{ kind: "correct", position: 0, pitch: 60 }}
        followCursor={false}
      />,
    );

    expect(container.querySelector(".hw-wrong")).toBeNull();
  });
});

describe("resize-driven re-measurement", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  // The root cause of the cursor drifting on anything other than the exact
  // screen shape it was last measured against: nothing used to re-measure
  // on a resize at all. This proves the fix without needing a real browser
  // layout — a mock `ResizeObserver` stands in for "the container's size
  // just changed," the same way `getBoundingClientRect` already stands in
  // for real layout throughout this file.
  it("re-measures note positions when the container is resized", () => {
    const PAGE = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 100">
      <g id="note-a" class="note"><ellipse cx="40" cy="50" rx="8" ry="6"/></g>
    </svg>`;
    const MARKS: readonly Mark[] = [
      { measure: 1, position: 0, page: 0, notes: [{ elementId: "note-a", step: "C", octave: 4, pitch: 60 }] },
    ];

    const callbacks: (() => void)[] = [];

    class MockResizeObserver {
      constructor(callback: () => void) {
        callbacks.push(callback);
      }
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    }

    vi.stubGlobal("ResizeObserver", MockResizeObserver);

    const { container } = render(
      <Score pages={[PAGE]} marks={MARKS} cursor={{ position: 0 }} dot={null} followCursor={false} showNoteNames />,
    );

    const note = container.querySelector("#note-a")!;
    const label = () => container.querySelector(".hw-note-name") as HTMLElement;
    const rectSpy = vi.spyOn(note, "getBoundingClientRect");

    rectSpy.mockReturnValue(stubbedRect(40, 50, 16, 12));
    act(() => {
      for (const callback of callbacks) callback();
    });
    expect(label().style.left).toBe("48px");

    // The container's size just changed — same element, different box.
    rectSpy.mockReturnValue(stubbedRect(140, 50, 16, 12));
    act(() => {
      for (const callback of callbacks) callback();
    });

    expect(label().style.left).toBe("148px");
  });
});

describe("the cursor, on the Minuet", () => {
  // Bar 1, beat 1: D5 over the G3 chord.
  const MARKS = marksOf("minuet-in-g");

  it("is a line inside the shifted music group, behind the music", () => {
    const { container } = render(
      <Score pages={[minuet]} marks={MARKS} cursor={{ position: 0 }} dot={null} followCursor={false} />,
    );

    // Inside Verovio's page-margin group: its translate is what puts the
    // notehead coordinates where the noteheads are drawn.
    const surface = container.querySelector("svg.definition-scale > g.page-margin")!;
    const line = container.querySelector("line.hw-cursor-line");

    expect(line).not.toBeNull();
    expect(surface.firstElementChild).toHaveClass("hw-under");
    expect(surface.firstElementChild!.contains(line)).toBe(true);
  });

  it("draws nothing for a position with no drawn notes", () => {
    const { container } = render(
      <Score pages={[minuet]} marks={MARKS} cursor={{ position: 9999 }} dot={null} followCursor={false} />,
    );

    expect(container.querySelector(".hw-cursor-line")).toBeNull();
  });
});

describe("a wrong note, on the Minuet", () => {
  const MARKS = marksOf("minuet-in-g");
  const score = (pitch: number) => (
    <Score
      pages={[minuet]}
      marks={MARKS}
      cursor={{ position: 0 }}
      dot={{ kind: "wrong", position: 0, pitch }}
      followCursor={false}
    />
  );

  it("is drawn inside the shifted music group, above the music", () => {
    const { container } = render(score(76));

    const surface = container.querySelector("svg.definition-scale > g.page-margin")!;
    expect(surface.lastElementChild).toHaveClass("hw-over");
    expect(surface.lastElementChild!.querySelector(".hw-wrong use")).not.toBeNull();
    expect(container.querySelector(".hw-wrong-label")?.textContent).toBe("E5");
  });

  it("is replaced by the next one, never shown twice", () => {
    const { container, rerender } = render(score(76));
    rerender(score(72));

    expect(container.querySelectorAll(".hw-wrong")).toHaveLength(1);
    expect(container.querySelector(".hw-wrong-label")?.textContent).toBe("C5");
  });

  it("goes after a second", () => {
    vi.useFakeTimers();
    const { container } = render(score(76));

    act(() => vi.advanceTimersByTime(1000));
    expect(container.querySelector(".hw-wrong")).toBeNull();
    vi.useRealTimers();
  });
});
