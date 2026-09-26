import { act, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { Mark } from "../bridge";
import minuet from "./__fixtures__/minuet-in-g.page1.svg?raw";
import { Score, unionBox, wrongNoteBox } from "./Score";

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

const RECTS: Record<string, { left: number; top: number; width: number; height: number }> = {
  "note-a": { left: 32, top: 44, width: 16, height: 12 },
  "note-b": { left: 82, top: 44, width: 16, height: 12 },
  "chord-1": { left: 132, top: 34, width: 16, height: 12 },
  "chord-2": { left: 132, top: 49, width: 16, height: 12 },
};

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

/** A container with real elements at known ids, and stubbed geometry for them. */
function stage(): HTMLDivElement {
  const root = document.createElement("div");

  for (const id of Object.keys(RECTS)) {
    const element = document.createElement("g");
    element.id = id;
    root.append(element);
  }

  vi.spyOn(root, "getBoundingClientRect").mockReturnValue(stubbedRect(0, 0, 400, 100));

  for (const [id, rect] of Object.entries(RECTS)) {
    const element = root.querySelector(`#${id}`)!;
    vi.spyOn(element, "getBoundingClientRect").mockReturnValue(
      stubbedRect(rect.left, rect.top, rect.width, rect.height),
    );
  }

  return root;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("unionBox", () => {
  it("is a single note's own box", () => {
    const root = stage();

    expect(unionBox(root, [{ elementId: "note-a", step: "C", octave: 4 }])).toEqual({
      left: 32,
      top: 44,
      width: 16,
      height: 12,
    });
  });

  // A Position may be a chord: every notehead in it belongs to the same
  // place in the music, so the box has to cover all of them.
  it("spans every note of a chord", () => {
    const root = stage();

    const box = unionBox(root, [
      { elementId: "chord-1", step: "E", octave: 4 },
      { elementId: "chord-2", step: "G", octave: 4 },
    ]);

    expect(box).toEqual({ left: 132, top: 34, width: 16, height: 27 });
  });

  it("is nothing for a position that was never drawn", () => {
    const root = stage();

    expect(unionBox(root, undefined)).toBeNull();
    expect(unionBox(root, [])).toBeNull();
  });
});

describe("wrongNoteBox", () => {
  const marks: readonly Mark[] = [
    { measure: 1, position: 0, page: 0, notes: [{ elementId: "note-a", step: "C", octave: 4 }] },
    { measure: 1, position: 1, page: 0, notes: [{ elementId: "note-b", step: "D", octave: 4 }] },
  ];

  it("sits at the expected note's horizontal position", () => {
    const root = stage();
    const anchor = marks[1]!.notes;

    const box = wrongNoteBox(
      root,
      { kind: "wrong", position: 1, pitch: 62 },
      anchor,
      new Map([[1, 0]]),
      marks,
    );

    // Pitch 62 is D natural — exactly the expected note — so the ghost
    // should land right where D was actually drawn.
    expect(box).toEqual({ left: 82, top: 44, width: 16, height: 12 });
  });

  it("falls back to the expected note's own box with only one calibration point", () => {
    const root = stage();
    const single: readonly Mark[] = [marks[1]!];

    const box = wrongNoteBox(
      root,
      { kind: "wrong", position: 1, pitch: 65 },
      single[0]!.notes,
      new Map([[1, 0]]),
      single,
    );

    expect(box).toEqual({ left: 82, top: 44, width: 16, height: 12 });
  });

  it("is nothing when the expected position was never drawn", () => {
    const root = stage();

    expect(
      wrongNoteBox(root, { kind: "wrong", position: 9, pitch: 60 }, undefined, new Map(), marks),
    ).toBeNull();
  });
});

describe("the score component", () => {
  const PAGE = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 100">
    <g id="note-a" class="note"><ellipse cx="40" cy="50" rx="8" ry="6"/></g>
  </svg>`;

  const MARKS: readonly Mark[] = [
    { measure: 1, position: 0, page: 0, notes: [{ elementId: "note-a", step: "C", octave: 4 }] },
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
      { measure: 1, position: 0, page: 0, notes: [{ elementId: "note-a", step: "C", octave: 4 }] },
      { measure: 1, position: 1, page: 0, notes: [{ elementId: "note-a", step: "C", octave: 4 }] },
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

    expect(container.querySelector(".hw-dot")).toBeNull();
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
      { measure: 1, position: 0, page: 0, notes: [{ elementId: "note-a", step: "C", octave: 4 }] },
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
  // Bar 1, beat 1: D5 over G3.
  const MARKS: readonly Mark[] = [
    {
      measure: 1,
      position: 0,
      page: 0,
      notes: [
        { elementId: "b37hn12", step: "D", octave: 5 },
        { elementId: "j14qhuvu", step: "G", octave: 3 },
      ],
    },
  ];

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
      <Score pages={[minuet]} marks={MARKS} cursor={{ position: 5 }} dot={null} followCursor={false} />,
    );

    expect(container.querySelector(".hw-cursor-line")).toBeNull();
  });
});
