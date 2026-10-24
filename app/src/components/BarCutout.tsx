import { useLayoutEffect, useRef, useState } from "react";

import type { DrawnNote, ReviewMark } from "../bridge";

const SVG = "http://www.w3.org/2000/svg";
/** Room around the bars, as a share of their height: enough for a ring on an edge note. */
const MARGIN = 0.16;

/**
 * Bars `first` to `last`, cut out of the engraved score (which stays laid out
 * behind the results): right notes green, notes played through wrong keys
 * ringed in red, and an amber mark where a pause came before `pauseInto`.
 * Only the first line of music the bars start on is shown, so a cut-out never
 * wraps. The bar groups carry their own staff lines; clefs and key signatures
 * live at the start of a line, so they are not in it.
 *
 * Without a laid-out score (or in a test DOM with no SVG geometry), it shows
 * the bar numbers instead.
 */
export function BarCutout({
  root,
  first,
  last,
  marks,
  notesAt,
  pauseInto,
  onPick,
}: {
  readonly root: HTMLElement | null;
  readonly first: number;
  readonly last: number;
  readonly marks: readonly ReviewMark[];
  readonly notesAt: ReadonlyMap<number, readonly DrawnNote[]>;
  readonly pauseInto: number | null;
  /** The cut-out was clicked: show bar `first` on the score. */
  readonly onPick: (measure: number) => void;
}) {
  const holder = useRef<HTMLDivElement>(null);
  const [drawn, setDrawn] = useState(false);

  useLayoutEffect(() => {
    const target = holder.current;
    if (!root || !target) return;

    const measures: SVGGraphicsElement[] = [];
    let line: Element | null = null;
    for (let bar = first; bar <= last; bar++) {
      const measure = root.querySelector<SVGGraphicsElement>(`g.measure[data-bar="${bar}"]`);
      if (!measure || typeof measure.getBBox !== "function") continue;
      const system = measure.closest("g.system");
      if (line && system !== line) break;
      line = system;
      measures.push(measure);
    }
    const boxes = measures.map((measure) => measure.getBBox());
    if (boxes.length === 0 || boxes.every((box) => box.width === 0)) {
      setDrawn(false);
      target.replaceChildren();
      return;
    }

    const left = Math.min(...boxes.map((box) => box.x));
    const top = Math.min(...boxes.map((box) => box.y));
    const right = Math.max(...boxes.map((box) => box.x + box.width));
    const bottom = Math.max(...boxes.map((box) => box.y + box.height));
    const margin = (bottom - top) * MARGIN;

    const cut = document.createElementNS(SVG, "svg");
    cut.setAttribute(
      "viewBox",
      `${left - margin} ${top - margin} ${right - left + 2 * margin} ${bottom - top + 2 * margin}`,
    );
    cut.setAttribute("preserveAspectRatio", "xMidYMid meet");
    cut.setAttribute("class", "hw-cutout h-full w-full");
    cut.setAttribute("aria-hidden", "true");

    for (const measure of measures) {
      cut.append(measure.cloneNode(true));
    }

    // Marks are found on the originals, where the geometry is, and drawn
    // on the cut-out, whose copies are recoloured by id.
    const wrongRings: SVGCircleElement[] = [];
    for (const mark of marks) {
      for (const note of notesAt.get(mark.position) ?? []) {
        const copy = cut.querySelector<SVGElement>(`#${CSS.escape(note.elementId)}`);
        if (mark.correct && copy) copy.style.color = "var(--color-correct)";
        const original = root.querySelector<SVGGraphicsElement>(`#${CSS.escape(note.elementId)}`);
        if (mark.wrong === 0 || !original || !copy || note !== notesAt.get(mark.position)?.[0]) {
          continue;
        }
        // Ringed by its head, not its stem.
        const head = original.querySelector<SVGGraphicsElement>("g.notehead") ?? original;
        const box = head.getBBox();
        const ring = document.createElementNS(SVG, "circle");
        ring.setAttribute("cx", String(box.x + box.width / 2));
        ring.setAttribute("cy", String(box.y + box.height / 2));
        ring.setAttribute("r", String(Math.max(box.width, box.height) * 1.1));
        ring.setAttribute("fill", "none");
        ring.setAttribute("stroke", "var(--color-wrong)");
        ring.setAttribute("stroke-width", String(Math.max(box.width, box.height) * 0.16));
        ring.setAttribute("data-wrong-ring", "");
        wrongRings.push(ring);
      }
    }
    cut.append(...wrongRings);

    const paused = pauseInto === null ? -1 : measures.findIndex((measure) => measure.dataset.bar === String(pauseInto));
    const pausedBox = boxes[paused];
    if (pausedBox) {
      const wedge = document.createElementNS(SVG, "rect");
      const width = (bottom - top) * 0.05;
      wedge.setAttribute("x", String(pausedBox.x - width / 2));
      wedge.setAttribute("y", String(top));
      wedge.setAttribute("width", String(width));
      wedge.setAttribute("height", String(bottom - top));
      wedge.setAttribute("rx", String(width / 2));
      wedge.setAttribute("fill", "var(--color-pause)");
      wedge.setAttribute("opacity", "0.85");
      wedge.setAttribute("data-pause-mark", "");
      cut.append(wedge);
    }

    // The copies' ids would clash with the score's; only the glyphs they
    // point at (in the score's own definitions) need theirs.
    for (const element of cut.querySelectorAll("[id]")) {
      element.removeAttribute("id");
    }
    target.replaceChildren(cut);
    setDrawn(true);
  }, [root, first, last, marks, notesAt, pauseInto]);

  const label = first === last ? `Bar ${first}` : `Bars ${first}–${last}`;

  return (
    <button
      type="button"
      onClick={() => onPick(first)}
      title={`${label} · show on the score`}
      aria-label={`${label} · show on the score`}
      data-bar-cutout
      className="relative block h-full w-full cursor-pointer overflow-hidden rounded-xl border border-rule bg-paper-raised text-ink transition-colors duration-[120ms] hover:border-accent focus-visible:outline-2 focus-visible:outline-accent"
    >
      <div ref={holder} className="h-full w-full px-2 py-1" />
      {!drawn && (
        <span className="absolute inset-0 flex items-center justify-center font-serif text-title text-ink-muted">
          {label}
        </span>
      )}
      <span className="absolute bottom-1 left-2 font-mono text-[10px] text-ink-muted">{label}</span>
    </button>
  );
}
