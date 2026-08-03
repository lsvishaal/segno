import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { Mark } from "../bridge";
import { Score } from "./Score";
import type { Marked } from "./Score";

/**
 * The mechanism `MVP §6.6` rests on.
 *
 * `ADR-0012` chose a webview on the strength of one claim: that marking a
 * note is a class on an element, because Verovio writes the same identifier
 * into its document and into the SVG. If that stops being true, feedback on
 * the score silently stops working — the application still runs, notes are
 * still judged correctly, and nothing is ever marked.
 *
 * Silently is the problem, so this is the guard. The previous approach had a
 * test at exactly this point too, and it is what caught the geometry library
 * reporting every notehead in the wrong place.
 */

/** A page shaped like Verovio's: notes as `<g>` elements carrying ids. */
const PAGE = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 100">
  <g id="note-a" class="note"><ellipse cx="40" cy="50" rx="8" ry="6"/></g>
  <g id="note-b" class="note"><ellipse cx="90" cy="50" rx="8" ry="6"/></g>
  <g id="chord-1" class="note"><ellipse cx="140" cy="40" rx="8" ry="6"/></g>
  <g id="chord-2" class="note"><ellipse cx="140" cy="55" rx="8" ry="6"/></g>
</svg>`;

const MARKS: readonly Mark[] = [
  { measure: 1, position: 0, page: 0, elementIds: ["note-a"] },
  { measure: 1, position: 1, page: 0, elementIds: ["note-b"] },
  { measure: 2, position: 2, page: 0, elementIds: ["chord-1", "chord-2"] },
];

function draw(marked: Marked) {
  return render(<Score pages={[PAGE]} marks={MARKS} marked={marked} followCursor={false} />);
}

describe("the score", () => {
  it("puts the engraving in the document", () => {
    const { container } = draw(new Map());

    expect(container.querySelector("#note-a")).not.toBeNull();
    expect(container.querySelectorAll("g.note")).toHaveLength(4);
  });

  // The claim ADR-0012 was decided on.
  it("marks the note at the cursor and no other", () => {
    const { container } = draw(new Map([[1, "cursor"]]));

    expect(container.querySelector("#note-b")).toHaveClass("hw-cursor");
    expect(container.querySelector("#note-a")).not.toHaveClass("hw-cursor");
  });

  it("tells a correct note from a wrong one", () => {
    const { container } = draw(
      new Map([
        [0, "correct"],
        [1, "wrong"],
      ]),
    );

    expect(container.querySelector("#note-a")).toHaveClass("hw-correct");
    expect(container.querySelector("#note-b")).toHaveClass("hw-wrong");
  });

  // A Position may be a chord, and every notehead in it belongs to the same
  // place in the music. Marking only the first would show a chord half-lit.
  it("marks every note of a chord together", () => {
    const { container } = draw(new Map([[2, "correct"]]));

    expect(container.querySelector("#chord-1")).toHaveClass("hw-correct");
    expect(container.querySelector("#chord-2")).toHaveClass("hw-correct");
  });

  // The cursor moves on. A mark left behind would leave a trail of cursors
  // down the page.
  it("clears a mark when it moves", () => {
    const { container, rerender } = draw(new Map([[0, "cursor"]]));

    expect(container.querySelector("#note-a")).toHaveClass("hw-cursor");

    rerender(
      <Score
        pages={[PAGE]}
        marks={MARKS}
        marked={new Map([[1, "cursor"]])}
        followCursor={false}
      />,
    );

    expect(container.querySelector("#note-a")).not.toHaveClass("hw-cursor");
    expect(container.querySelector("#note-b")).toHaveClass("hw-cursor");
  });

  // One state at a time. A note that went from wrong to correct must not
  // carry both, or the CSS decides which wins by declaration order.
  it("replaces a mark rather than accumulating one", () => {
    const { container, rerender } = draw(new Map([[0, "wrong"]]));

    rerender(
      <Score
        pages={[PAGE]}
        marks={MARKS}
        marked={new Map([[0, "correct"]])}
        followCursor={false}
      />,
    );

    const note = container.querySelector("#note-a");

    expect(note).toHaveClass("hw-correct");
    expect(note).not.toHaveClass("hw-wrong");
  });

  // A note the engraver never drew is simply not marked. MVP §6.6 degrades
  // to no highlight rather than to a highlight in the wrong place.
  it("ignores a position that was never drawn", () => {
    expect(() => draw(new Map([[99, "cursor"]]))).not.toThrow();
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
      <Score pages={[hostile]} marks={MARKS} marked={new Map()} followCursor={false} />,
    );

    expect(container.querySelector("script")).toBeNull();
    expect(container.querySelector("#note-a")).not.toHaveAttribute("onload");
  });
});
