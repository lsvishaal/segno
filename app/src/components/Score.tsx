import { useEffect, useMemo, useRef, useState } from "react";

import { AnimatePresence, motion } from "motion/react";

import type { DrawnNote, Mark } from "../bridge";
import { nearestSpelling, verticalPositionFor } from "./staffGeometry";
import type { Spelling } from "./staffGeometry";

/** Where the cursor bar sits. */
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
 * The engraved score, with a cursor bar and correctness dots drawn over it.
 *
 * # Why the SVG is not React's business
 *
 * A page of engraving is a few thousand elements. Rendering it through React
 * would put all of them through reconciliation every time something moved —
 * ten times a second while someone is playing. So the markup is injected
 * once, imperatively, into its own node; React never touches what is inside
 * it. The cursor and the dots are a *separate* layer, one React fully owns,
 * sitting on top.
 *
 * # Why marking a note is no longer a class on it
 *
 * The previous approach recoloured the notehead itself. That worked for a
 * note that was written — every wrong note isn't one: a keyboard reports a
 * struck key's sound, never its spelling (`ADR-0001`), so a wrong note has
 * no element to put a class on. The cursor and both dots are therefore an
 * overlay, positioned from real measurements (`getBoundingClientRect()` on
 * whatever was actually drawn — trustworthy, unlike the static-file geometry
 * bug `ADR-0012` records) rather than from anything Verovio drew for them.
 */
export function Score({
  pages,
  marks,
  cursor,
  dot,
  followCursor = true,
}: {
  readonly pages: readonly string[];
  readonly marks: readonly Mark[];
  readonly cursor: CursorState;
  readonly dot: Dot | null;
  readonly followCursor?: boolean;
}) {
  const wrapper = useRef<HTMLDivElement>(null);
  const container = useRef<HTMLDivElement>(null);
  const [cursorBox, setCursorBox] = useState<Box | null>(null);
  const [dotBox, setDotBox] = useState<{ box: Box; kind: "correct" | "wrong" } | null>(null);

  /** Position in the piece → the notes drawn for it. */
  const notesAt = useMemo(() => {
    const index = new Map<number, readonly DrawnNote[]>();

    for (const mark of marks) {
      index.set(mark.position, mark.notes);
    }

    return index;
  }, [marks]);

  /** Which page a position was drawn on — for restricting staff geometry
   * calibration to notes actually near the one being placed. */
  const pageOf = useMemo(() => {
    const index = new Map<number, number>();

    for (const mark of marks) {
      index.set(mark.position, mark.page);
    }

    return index;
  }, [marks]);

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
  }, [pages]);

  // The cursor bar, measured against whatever is actually on screen.
  useEffect(() => {
    const root = wrapper.current;
    if (!root) return;

    setCursorBox(unionBox(root, notesAt.get(cursor.position)));
  }, [cursor, notesAt, pages]);

  // A dot, positioned and then retired on its own timer — its lifetime is
  // not tied to whatever event happens to arrive next.
  useEffect(() => {
    if (!dot) return;

    const root = wrapper.current;
    if (!root) return;

    const notes = notesAt.get(dot.position);

    const box =
      dot.kind === "correct"
        ? unionBox(root, notes)
        : wrongNoteBox(root, dot, notes, pageOf, marks);

    if (box) setDotBox({ box, kind: dot.kind });

    const timeout = window.setTimeout(() => setDotBox(null), 420);
    return () => window.clearTimeout(timeout);
    // `dot` is a fresh object per event even when its fields repeat, so a
    // second wrong note in a row still restarts the animation.
  }, [dot, notesAt, pageOf, marks]);

  // Keep the cursor in view. `block: "center"` rather than "nearest" so the
  // performer reads with music above and below, not pinned to an edge.
  useEffect(() => {
    if (!followCursor) return;

    const first = notesAt.get(cursor.position)?.[0];
    const element = first
      ? container.current?.querySelector(`#${CSS.escape(first.elementId)}`)
      : null;

    element?.scrollIntoView({ behavior: "smooth", block: "center" });
  }, [cursor, notesAt, followCursor]);

  return (
    <div ref={wrapper} className="relative mx-auto max-w-6xl">
      <div
        ref={container}
        className="hw-sheet rounded-xl p-8 shadow-[0_20px_60px_-32px_rgb(28_26_23/0.35)]"
      />

      <div className="pointer-events-none absolute inset-0">
        {cursorBox && (
          <motion.div
            className="hw-cursor-bar"
            animate={{
              left: cursorBox.left + cursorBox.width / 2,
              top: cursorBox.top,
              height: cursorBox.height,
            }}
            transition={{ type: "spring", stiffness: 300, damping: 30 }}
          />
        )}

        <AnimatePresence>
          {dotBox && (
            <motion.div
              key={`${dot?.kind}-${dot?.position}-${dot?.pitch}-${dotBox.box.left}-${dotBox.box.top}`}
              className={dotBox.kind === "correct" ? "hw-dot hw-dot-correct" : "hw-dot hw-dot-wrong"}
              style={{
                left: dotBox.box.left + dotBox.box.width / 2,
                top: dotBox.box.top + dotBox.box.height / 2,
              }}
              initial={{ opacity: 0, scale: 0.4 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.6 }}
              transition={{ duration: 0.3, ease: "easeOut" }}
            />
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}

/** The union of every drawn note's box for one position, relative to `root`. */
export function unionBox(root: HTMLElement, notes: readonly DrawnNote[] | undefined): Box | null {
  if (!notes || notes.length === 0) return null;

  const origin = root.getBoundingClientRect();
  let box: Box | null = null;

  for (const note of notes) {
    const element = root.querySelector(`#${CSS.escape(note.elementId)}`);
    if (!element) continue;

    const rect = element.getBoundingClientRect();
    const relative: Box = {
      left: rect.left - origin.left,
      top: rect.top - origin.top,
      width: rect.width,
      height: rect.height,
    };

    box = box ? union(box, relative) : relative;
  }

  return box;
}

function union(a: Box, b: Box): Box {
  const left = Math.min(a.left, b.left);
  const top = Math.min(a.top, b.top);
  const right = Math.max(a.left + a.width, b.left + b.width);
  const bottom = Math.max(a.top + a.height, b.top + b.height);

  return { left, top, width: right - left, height: bottom - top };
}

/**
 * Where a wrong note's dot belongs: the cursor's own horizontal position
 * (the beat it was played at), and a vertical position guessed from real
 * measured notes nearby (`staffGeometry.ts`) — never from anything Verovio
 * drew for this pitch, because nothing was.
 */
export function wrongNoteBox(
  root: HTMLElement,
  dot: Dot,
  anchorNotes: readonly DrawnNote[] | undefined,
  pageOf: ReadonlyMap<number, number>,
  marks: readonly Mark[],
): Box | null {
  const anchorBox = unionBox(root, anchorNotes);
  const anchor = anchorNotes?.[0];
  if (!anchorBox || !anchor) return null;

  const page = pageOf.get(dot.position);
  const origin = root.getBoundingClientRect();

  const calibration = marks
    .filter((mark) => mark.page === page)
    .flatMap((mark) => mark.notes)
    .flatMap((note) => {
      const element = root.querySelector(`#${CSS.escape(note.elementId)}`);
      if (!element) return [];

      const rect = element.getBoundingClientRect();
      return [{ spelling: spellingOf(note), y: rect.top - origin.top }];
    });

  const ghost = nearestSpelling(dot.pitch, spellingOf(anchor));
  const y = verticalPositionFor(ghost, calibration);

  // Nothing to calibrate against (a one-note piece, alone on its line) —
  // an honest fallback next to the cursor rather than nowhere at all.
  const top = y ?? anchorBox.top;

  return { left: anchorBox.left, top, width: anchorBox.width, height: anchorBox.height };
}

function spellingOf(note: DrawnNote): Spelling {
  return { step: note.step, octave: note.octave };
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
