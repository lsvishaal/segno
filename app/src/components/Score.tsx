import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";

import type { Bar, DrawnNote, Mark, ReviewMark } from "../bridge";
import { SCORE_MAX_WIDTH } from "../screens/zoom";
import { NoteNames } from "./NoteNames";
import { showGeometry as drawGeometry } from "./scoreDebug";
import { cursorLine, drawingSurface, readPage } from "./scoreGeometry";
import type { PageGeometry } from "./scoreGeometry";
import { wrongNoteMark } from "./staffGeometry";
import type { WrongNoteMark } from "./staffGeometry";

/** Which position the cursor is on. */
export interface CursorState {
  readonly position: number;
}

/** One flash of feedback on a played note. */
export interface Dot {
  readonly kind: "correct" | "wrong";
  readonly position: number;
  /** The chromatic pitch actually played — only used for the wrong case. */
  readonly pitch: number;
}

/** A box relative to the score's own container, in pixels. */
export interface Box {
  readonly left: number;
  readonly top: number;
  readonly width: number;
  readonly height: number;
}

/**
 * The engraved score, with a cursor and correctness dots drawn on it.
 *
 * # Why the SVG is not React's business
 *
 * A page of engraving is a few thousand elements. Rendering it through React
 * would put all of them through reconciliation every time something moved —
 * ten times a second while someone is playing. So the markup is injected
 * once, imperatively, into its own node; React never touches what is inside
 * it. The cursor and the wrong-note mark are portalled into layers inside
 * the page (behind and above the ink), placed in the page's own units from
 * its geometry, so no zoom, resize or scroll can pull them out of place.
 *
 * # Correct notes recolor real ink; wrong notes cannot
 *
 * A correct note was written, so it has a real element — its own `color` is
 * set directly (Verovio draws with `currentColor`, inherited from
 * `.hw-page`) and transitions back, no positioning involved. A wrong note
 * has no such element: a keyboard reports a struck key's sound, never its
 * spelling, so there is nothing to recolor. It is drawn where it would be
 * written instead (`staffGeometry.ts`).
 */
/** What to leave drawn once an attempt has ended. */
export interface ScoreReview {
  readonly marks: readonly ReviewMark[];
  /** Bars after where the attempt stopped, faded. */
  readonly notReached: readonly number[];
}

/** The heatmap's single amber hue, light to deep, by level 1–4. */
const WEAK_TINT = ["", "bg-pause/8", "bg-pause/16", "bg-pause/26", "bg-pause/38"];

/** No bars known: one array, so a default does not change on every render. */
const NO_BARS: readonly Bar[] = [];

export function Score({
  pages,
  marks,
  bars = NO_BARS,
  onBarClick,
  startPosition = 0,
  loop = null,
  review = null,
  weak = null,
  cursor,
  dot,
  followCursor = true,
  hidden = false,
  showNoteNames = false,
  showGeometry = false,
}: {
  readonly pages: readonly string[];
  readonly marks: readonly Mark[];
  /** Which drawn `g.measure` element is which bar. */
  readonly bars?: readonly Bar[];
  /** Called with a bar's number when it is clicked. Without it, bars do nothing. */
  /** A bar was clicked; `shift` if Shift was held, which loops to it. */
  readonly onBarClick?: ((measure: number, how: { readonly shift: boolean }) => void) | undefined;
  /** The bars being looped, drawn as a band behind them. */
  readonly loop?: { readonly first: number; readonly last: number } | null;
  /** After an attempt: right notes left green, wrong ones crossed, bars not reached faded. */
  readonly review?: ScoreReview | null;
  /** Bar number → how hard it has been over time, 1–4, tinted behind it. */
  readonly weak?: ReadonlyMap<number, number> | null;
  /** Where attempts start; marked on the score unless it is the very beginning. */
  readonly startPosition?: number;
  readonly cursor: CursorState;
  readonly dot: Dot | null;
  readonly followCursor?: boolean;
  /** Laid out but not shown: while the music is being fitted to the window. */
  readonly hidden?: boolean;
  readonly showNoteNames?: boolean;
  readonly showGeometry?: boolean;
}) {
  const wrapper = useRef<HTMLDivElement>(null);
  const container = useRef<HTMLDivElement>(null);
  // Per page: its geometry, a layer behind the ink (the cursor), one above
  // it (the wrong-note mark), and the id of the page's black notehead glyph.
  const [surfaces, setSurfaces] = useState<readonly Surface[]>([]);
  const [wrong, setWrong] = useState<{ id: number; mark: WrongNoteMark; surface: Surface } | null>(null);
  const wrongCount = useRef(0);
  const [noteLabels, setNoteLabels] = useState<readonly { elementId: string; step: string; box: Box }[]>([]);
  // Bumped by the `ResizeObserver` below — its value is never read, only its
  // identity change matters, to force every measurement effect to re-run.
  const [layoutTick, setLayoutTick] = useState(0);

  const [hovered, setHovered] = useState<{ readonly measure: number; readonly box: Box } | null>(null);
  const [startBox, setStartBox] = useState<Box | null>(null);
  const [loopBoxes, setLoopBoxes] = useState<readonly Box[]>([]);
  const [fadedBoxes, setFadedBoxes] = useState<readonly Box[]>([]);
  const [weakBoxes, setWeakBoxes] = useState<readonly { readonly box: Box; readonly level: number }[]>([]);
  const [crosses, setCrosses] = useState<readonly { readonly box: Box; readonly wrong: number }[]>([]);

  /** Bar element id → its number. */
  const barOf = useMemo(() => new Map(bars.map((bar) => [bar.elementId, bar.measure])), [bars]);

  /**
   * The drawn bar under a point on screen. Measured by area, not by what was
   * hit: most of a bar is blank paper, and the cursor and marks lie on top.
   * Where boxes overlap (a slur reaching into the next line), the smallest
   * one containing the point wins.
   */
  const barAt = useCallback(
    (x: number, y: number) => {
      let found: { measure: number; id: string; area: number } | null = null;
      for (const element of container.current?.querySelectorAll("g.measure") ?? []) {
        const measure = barOf.get(element.id);
        if (measure === undefined) continue;
        const rect = element.getBoundingClientRect();
        if (x < rect.left || x > rect.right || y < rect.top || y > rect.bottom) continue;
        const area = rect.width * rect.height;
        if (!found || area < found.area) found = { measure, id: element.id, area };
      }
      return found;
    },
    [barOf],
  );

  useEffect(() => {
    if (!onBarClick) setHovered(null);
  }, [onBarClick]);

  const onPointerMove = useCallback(
    (event: React.PointerEvent) => {
      const root = wrapper.current;
      const bar = onBarClick && root ? barAt(event.clientX, event.clientY) : null;
      const box = bar && root ? boxOf(root, bar.id) : null;
      setHovered((current) =>
        bar && box
          ? current?.measure === bar.measure && current.box.top === box.top && current.box.left === box.left
            ? current
            : { measure: bar.measure, box }
          : null,
      );
    },
    [barAt, onBarClick],
  );

  const onClick = useCallback(
    (event: React.MouseEvent) => {
      if (!onBarClick) return;
      const bar = barAt(event.clientX, event.clientY);
      if (bar) onBarClick(bar.measure, { shift: event.shiftKey });
    },
    [barAt, onBarClick],
  );

  /** Position in the piece → the notes drawn for it. */
  const notesAt = useMemo(() => {
    const index = new Map<number, readonly DrawnNote[]>();

    for (const mark of marks) {
      index.set(mark.position, mark.notes);
    }

    return index;
  }, [marks]);

  // Every stored box is a snapshot of `getBoundingClientRect()`, which goes
  // stale the instant the container's actual on-screen size changes — a
  // window resize, an orientation change, or simply the app running on a
  // differently-shaped screen than whatever it was last measured against.
  // `.hw-page svg { width: 100%; height: auto }` means the score's pixel
  // size depends entirely on this element's width, so that's what's watched.
  useEffect(() => {
    const root = wrapper.current;
    if (!root) return;

    const observer = new ResizeObserver(() => {
      setLayoutTick((tick) => tick + 1);
    });

    observer.observe(root);
    return () => observer.disconnect();
  }, []);

  // The pages, injected once per piece. `marks` deliberately is not a
  // dependency: new marks for the same score must not re-inject it.
  useEffect(() => {
    const panel = container.current;
    if (!panel) return;

    panel.replaceChildren();

    for (const page of pages) {
      const holder = document.createElement("div");
      holder.className = "hw-page";
      // Sanitised before it reaches the DOM. Verovio generates this markup
      // rather than copying it, so the risk is low — but it derives from a
      // file the performer chose, and stripping scripts costs nothing.
      holder.innerHTML = sanitise(page);
      panel.append(holder);
    }

    setSurfaces(
      [...panel.querySelectorAll(".hw-page")].flatMap((page) => {
        const surface = drawingSurface(page);
        if (!surface) return [];
        const under = document.createElementNS(SVG, "g");
        under.setAttribute("class", "hw-under");
        surface.prepend(under);
        const over = document.createElementNS(SVG, "g");
        over.setAttribute("class", "hw-over");
        surface.append(over);
        const notehead = page.querySelector('defs [id^="E0A4-"]')?.id ?? null;
        return [{ under, over, notehead, geometry: readPage(page) }];
      }),
    );
  }, [pages]);

  // Each drawn bar carries its number, for anyone reading the page.
  useEffect(() => {
    const panel = container.current;
    if (!panel) return;

    for (const element of panel.querySelectorAll("g.measure")) {
      const measure = barOf.get(element.id);
      if (measure !== undefined) element.setAttribute("data-bar", String(measure));
    }
  }, [pages, barOf]);

  // What the geometry layer reads from each page, drawn inside the page.
  // Declared after the injection effect, so it always sees the pages.
  useEffect(() => {
    const panel = container.current;
    if (!panel) return;

    for (const page of panel.querySelectorAll(".hw-page")) {
      drawGeometry(page, showGeometry);
    }
  }, [pages, showGeometry]);

  // The cursor, in the page's own units: it scales with the page, so no
  // zoom, resize or scroll can pull it off its notes.
  const cursorIds = (notesAt.get(cursor.position) ?? []).map((note) => note.elementId);
  const cursorOn = surfaces
    .map((surface) => ({ layer: surface.under, line: cursorLine(surface.geometry, cursorIds) }))
    .find((candidate) => candidate.line);

  // A correct note recolors the real notehead. A wrong note is drawn where
  // it would be written, and retired on its own timer; the next one
  // replaces it, so there is never more than one.
  useEffect(() => {
    if (!dot) return;

    const root = wrapper.current;
    if (!root) return;

    const notes = notesAt.get(dot.position);

    if (dot.kind === "correct") {
      const elements = (notes ?? []).flatMap((note) => {
        const element = root.querySelector(`#${CSS.escape(note.elementId)}`);
        return element instanceof SVGElement ? [element] : [];
      });

      for (const element of elements) {
        element.style.color = "var(--hw-correct)";
      }

      const timeout = window.setTimeout(() => {
        for (const element of elements) {
          element.style.color = "";
        }
      }, 420);

      return () => {
        window.clearTimeout(timeout);
        // The next note arrived before this flash was over: start its fade
        // now, so no note is ever left green.
        for (const element of elements) {
          element.style.color = "";
        }
      };
    }

    for (const surface of surfaces) {
      const mark = wrongNoteMark(surface.geometry, notes ?? [], marks, dot.pitch);
      if (!mark) continue;

      wrongCount.current += 1;
      setWrong({ id: wrongCount.current, mark, surface });
      const timeout = window.setTimeout(() => setWrong(null), WRONG_NOTE_MS);
      return () => window.clearTimeout(timeout);
    }
    // `dot` is a fresh object per event even when its fields repeat, so a
    // second wrong note in a row still restarts the fade.
  }, [dot, notesAt, marks, surfaces]);

  // Note-name labels, one per drawn note — recomputed whenever the toggle
  // turns on or the pages themselves change. Declared after the pages
  // effect above so it always measures a DOM that effect already injected
  // into (effects within one component fire in declaration order).
  useEffect(() => {
    if (!showNoteNames) {
      setNoteLabels([]);
      return;
    }

    const root = wrapper.current;
    if (!root) return;

    // A repeated bar is marked once per pass but drawn once, so each drawn
    // note is labelled once.
    const seen = new Set<string>();
    const labels = marks.flatMap((mark) =>
      mark.notes.flatMap((note) => {
        if (seen.has(note.elementId)) return [];
        seen.add(note.elementId);
        const box = boxOf(root, note.elementId);
        return box ? [{ elementId: note.elementId, step: note.step, box }] : [];
      }),
    );

    setNoteLabels(labels);
  }, [showNoteNames, marks, pages, layoutTick]);

  // The start marker sits on the left edge of the bar the start position is
  // drawn in. Not shown at the very beginning, where the start is obvious.
  useEffect(() => {
    const root = wrapper.current;
    const first = notesAt.get(startPosition)?.[0];
    if (startPosition === 0 || !root || !first) {
      setStartBox(null);
      return;
    }
    const measure = root.querySelector(`#${CSS.escape(first.elementId)}`)?.closest("g.measure");
    setStartBox(measure ? boxOf(root, measure.id) : null);
  }, [startPosition, notesAt, pages, layoutTick]);

  // The loop band: one box per drawn bar in the loop, so a loop across a
  // line break is two pieces of band.
  const loopFirst = loop?.first;
  const loopLast = loop?.last;
  useEffect(() => {
    const root = wrapper.current;
    if (loopFirst === undefined || loopLast === undefined || !root) {
      setLoopBoxes([]);
      return;
    }
    setLoopBoxes(
      [...root.querySelectorAll("g.measure")].flatMap((element) => {
        const measure = barOf.get(element.id);
        if (measure === undefined || measure < loopFirst || measure > loopLast) return [];
        const box = boxOf(root, element.id);
        return box ? [box] : [];
      }),
    );
  }, [loopFirst, loopLast, barOf, pages, layoutTick]);

  // Weak bars over time: each tinted amber, deeper the harder it has been.
  useEffect(() => {
    const root = wrapper.current;
    if (!weak || !root) {
      setWeakBoxes([]);
      return;
    }
    setWeakBoxes(
      [...root.querySelectorAll("g.measure")].flatMap((element) => {
        const level = weak.get(barOf.get(element.id) ?? 0) ?? 0;
        const box = level > 0 ? boxOf(root, element.id) : null;
        return box ? [{ box, level }] : [];
      }),
    );
  }, [weak, barOf, pages, layoutTick]);

  // The review: right notes stay green, a red ✕ above each note waited on
  // through wrong ones (a repeated bar's two passes add up), and the bars
  // not reached faded behind a wash of paper.
  useEffect(() => {
    const root = wrapper.current;
    if (!review || !root) {
      setFadedBoxes([]);
      setCrosses([]);
      return;
    }

    const notReached = new Set(review.notReached);
    setFadedBoxes(
      [...root.querySelectorAll("g.measure")].flatMap((element) => {
        const measure = barOf.get(element.id);
        if (measure === undefined || !notReached.has(measure)) return [];
        const box = boxOf(root, element.id);
        return box ? [box] : [];
      }),
    );

    const green: SVGElement[] = [];
    const wrongAt = new Map<string, number>();
    for (const mark of review.marks) {
      const notes = notesAt.get(mark.position) ?? [];
      if (mark.correct) {
        for (const note of notes) {
          const element = root.querySelector(`#${CSS.escape(note.elementId)}`);
          if (element instanceof SVGElement) green.push(element);
        }
      }
      const first = notes[0];
      if (mark.wrong > 0 && first) {
        wrongAt.set(first.elementId, (wrongAt.get(first.elementId) ?? 0) + mark.wrong);
      }
    }
    for (const element of green) element.style.color = "var(--hw-correct)";
    setCrosses(
      [...wrongAt].flatMap(([elementId, wrong]) => {
        const box = boxOf(root, elementId);
        return box ? [{ box, wrong }] : [];
      }),
    );

    return () => {
      for (const element of green) element.style.color = "";
    };
  }, [review, barOf, notesAt, pages, layoutTick]);

  // Keep the cursor's line of music in view: scrolled to when the cursor
  // reaches a new line, or the music reflows, and not on every note. The
  // scroll container's `scroll-padding` puts it in the upper third, with
  // the next line visible below.
  const cursorSystem = cursorOn?.line?.system;
  useEffect(() => {
    if (!followCursor || cursorSystem === undefined) return;

    wrapper.current
      ?.querySelector(".hw-cursor-line")
      ?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, [cursorSystem, surfaces, followCursor]);

  return (
    <div
      ref={wrapper}
      className="relative mx-auto w-full select-none"
      style={{
        maxWidth: SCORE_MAX_WIDTH,
        visibility: hidden ? "hidden" : undefined,
        cursor: hovered ? "pointer" : undefined,
      }}
      title={hovered ? `Start here (bar ${hovered.measure}) · Shift+click to loop to it` : undefined}
      onPointerMove={onBarClick ? onPointerMove : undefined}
      onPointerLeave={() => setHovered(null)}
      onClick={onClick}
    >
      <div ref={container} className="py-6" />

      <div className="pointer-events-none absolute inset-0">
        {weakBoxes.map(({ box, level }) => (
          <div
            key={`weak-${box.left},${box.top}`}
            data-weak-bar={level}
            className={`absolute rounded-sm mix-blend-multiply ${WEAK_TINT[level] ?? ""}`}
            style={box}
          />
        ))}

        {loopBoxes.map((box) => (
          <div
            key={`${box.left},${box.top}`}
            data-loop-band
            className="absolute rounded-sm bg-loop-band/8"
            style={box}
          />
        ))}

        {fadedBoxes.map((box) => (
          <div
            key={`faded-${box.left},${box.top}`}
            data-not-reached
            className="absolute bg-paper-raised/70"
            style={box}
          />
        ))}

        {crosses.map(({ box, wrong }) => (
          <span
            key={`cross-${box.left},${box.top}`}
            data-wrong-mark
            className="absolute -translate-x-1/2 text-xs leading-none font-semibold text-wrong"
            style={{ left: box.left + box.width / 2, top: box.top - 16 }}
          >
            ✕{wrong > 1 ? <sub className="ml-px text-[9px]">{wrong}</sub> : null}
          </span>
        ))}

        {hovered && (
          <div
            data-bar-hover
            className="absolute rounded-sm bg-ink/5 transition-[left,top,width,height] duration-[120ms]"
            style={hovered.box}
          />
        )}

        {startBox && (
          <div
            data-start-marker
            className="absolute"
            style={{ left: startBox.left - 4, top: startBox.top, height: startBox.height }}
          >
            <div className="h-full w-[3px] rounded-full bg-accent" />
            <span className="absolute -top-5 left-0 text-[11px] font-medium tracking-wide text-accent">
              Start
            </span>
          </div>
        )}

        <NoteNames labels={noteLabels} />

        {cursorOn?.line &&
          createPortal(
            // Keyed by system: it glides along a line, and jumps to the next.
            <line
              key={cursorOn.line.system}
              className="hw-cursor-line"
              x1={0}
              x2={0}
              y1={cursorOn.line.y1}
              y2={cursorOn.line.y2}
              style={{ transform: `translateX(${cursorOn.line.x}px)` }}
            />,
            cursorOn.layer,
          )}

        {wrong &&
          createPortal(
            <WrongNote key={wrong.id} mark={wrong.mark} notehead={wrong.surface.notehead} />,
            wrong.surface.over,
          )}
      </div>
    </div>
  );
}

/** One drawn element's own box, relative to `root` — or nothing if it isn't there. */
export function boxOf(root: HTMLElement, elementId: string): Box | null {
  const element = root.querySelector(`#${CSS.escape(elementId)}`);
  if (!element) return null;

  const origin = root.getBoundingClientRect();
  const rect = element.getBoundingClientRect();

  return {
    left: rect.left - origin.left,
    top: rect.top - origin.top,
    width: rect.width,
    height: rect.height,
  };
}

const SVG = "http://www.w3.org/2000/svg";
/** How long a wrong-note mark stays; keep in step with `hw-wrong-fade`. */
const WRONG_NOTE_MS = 1000;
const ACCIDENTALS = { sharp: "♯", flat: "♭", natural: "♮" } as const;

interface Surface {
  readonly under: SVGGElement;
  readonly over: SVGGElement;
  readonly notehead: string | null;
  readonly geometry: PageGeometry;
}

/** A wrong note, drawn as it would be written: notehead, ledger lines, accidental, name. */
function WrongNote({ mark, notehead }: { readonly mark: WrongNoteMark; readonly notehead: string | null }) {
  const left = mark.x - mark.width / 2;
  // SMuFL glyphs are 1000 units to four staff spaces.
  const glyphScale = (mark.spacing * 4) / 1000;

  return (
    <g className="hw-wrong">
      {mark.ledgers.map((y) => (
        <line
          key={y}
          className="hw-wrong-ledger"
          x1={mark.x - mark.ledgerWidth / 2}
          x2={mark.x + mark.ledgerWidth / 2}
          y1={y}
          y2={y}
          strokeWidth={mark.spacing * 0.12}
        />
      ))}
      {notehead ? (
        <use href={`#${notehead}`} transform={`translate(${left}, ${mark.y}) scale(${glyphScale})`} />
      ) : (
        <ellipse cx={mark.x} cy={mark.y} rx={mark.width / 2} ry={mark.spacing / 2} />
      )}
      {mark.accidental && (
        <text
          x={left - mark.spacing * 0.25}
          y={mark.y}
          fontSize={mark.spacing * 2.2}
          textAnchor="end"
          dominantBaseline="central"
        >
          {ACCIDENTALS[mark.accidental]}
        </text>
      )}
      <text
        className="hw-wrong-label"
        x={mark.x + mark.width / 2 + mark.spacing * 0.4}
        y={mark.y - mark.spacing * 0.6}
        fontSize={mark.spacing * 1.3}
      >
        {mark.label}
      </text>
    </g>
  );
}

/**
 * Removes anything executable from engraver output.
 *
 * SVG can carry `<script>` and `on*` handlers, and this markup — though
 * generated by Verovio rather than copied from the source — ultimately
 * derives from a file the performer supplied. The application's CSP already
 * refuses inline script, so this is the second of two locks rather than the
 * only one.
 */
function sanitise(svg: string): string {
  const parsed = new DOMParser().parseFromString(svg, "image/svg+xml");

  for (const script of parsed.querySelectorAll("script")) {
    script.remove();
  }

  for (const element of parsed.querySelectorAll("*")) {
    for (const attribute of [...element.attributes]) {
      const name = attribute.name.toLowerCase();

      if (name.startsWith("on") || (name === "href" && attribute.value.trim().toLowerCase().startsWith("javascript:"))) {
        element.removeAttribute(attribute.name);
      }
    }
  }

  return new XMLSerializer().serializeToString(parsed);
}
