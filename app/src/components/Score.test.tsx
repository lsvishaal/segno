import { render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { Mark } from "../bridge";
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
});
