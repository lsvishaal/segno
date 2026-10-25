import type { DrawnNote, ReviewMark } from "../bridge";

const SVG = "http://www.w3.org/2000/svg";
/** Room around the bars, as a share of their height: enough for a ring on an edge note. */
const MARGIN = 0.16;

/** Which bars to cut out, and what to draw on them. */
export interface CutOutRequest {
  readonly first: number;
  readonly last: number;
  /** Right notes green, notes played through wrong keys ringed in red. */
  readonly marks: readonly ReviewMark[];
  /** An amber mark where a pause came before this bar, if any. */
  readonly pauseInto: number | null;
}

/**
 * Bars `first` to `last`, cut out of the engraved score under `root` as a
 * new SVG. Only the first line of music the bars start on is used, so a
 * cut-out never wraps. The bar groups carry their own staff lines; clefs and
 * key signatures live at the start of a line, so they are not in it.
 *
 * `null` without a laid-out score (or in a test DOM with no SVG geometry).
 * Used only by `Score`, which owns the score's drawing.
 */
export function cutOut(
  root: Element,
  notesAt: ReadonlyMap<number, readonly DrawnNote[]>,
  { first, last, marks, pauseInto }: CutOutRequest,
): SVGSVGElement | null {
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
  if (boxes.length === 0 || boxes.every((box) => box.width === 0)) return null;

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
  return cut;
}
