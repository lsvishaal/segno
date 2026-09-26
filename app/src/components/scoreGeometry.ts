import type { Spelling } from "./staffGeometry";

/**
 * Where things are on an engraved page, read straight out of Verovio's SVG.
 *
 * Everything is in the page's own drawing units (the inner
 * `svg.definition-scale` coordinate space), never screen pixels, so it does
 * not change when the page is zoomed, scrolled or resized. Nothing here
 * measures layout: positions come from the coordinates Verovio wrote, which
 * makes this pure and testable against saved real pages.
 *
 * What Verovio writes, and this relies on:
 * - `g.system` > `g.measure` > `g.staff` > `g.layer` > (`g.chord` >) `g.note`
 * - a staff's five lines are its first `<path d="M x0 y L x1 y">` children
 * - `g.clef > use` names a SMuFL clef glyph and is translated onto the clef's
 *   reference line; `g.keySig > g.keyAccid > use` names sharps or flats
 * - `g.note > g.notehead > use` is translated to the notehead's left edge and
 *   its exact staff position
 */

/** A clef: which line it sits on, and which note that line is. */
export interface Clef {
  readonly sign: "G" | "F" | "C";
  /** The note on the clef's reference line. */
  readonly reference: Spelling;
  /** That line, counted 1 (bottom) to 5 (top). */
  readonly line: number;
}

/** A key signature: how many sharps or flats. */
export interface Key {
  readonly accidental: "sharp" | "flat" | null;
  readonly count: number;
}

/** One staff of one system. */
export interface Staff {
  /** 0 for the top staff of the system. */
  readonly index: number;
  /** The five line heights, top to bottom. */
  readonly lines: readonly number[];
  /** Distance between adjacent lines. */
  readonly spacing: number;
  readonly x0: number;
  readonly x1: number;
}

/** One line of music. */
export interface System {
  readonly index: number;
  /** The top line of its first staff. */
  readonly top: number;
  /** The bottom line of its last staff. */
  readonly bottom: number;
  readonly staves: readonly Staff[];
}

/** One bar, as drawn on one system. */
export interface Bar {
  readonly id: string;
  readonly system: number;
  readonly x0: number;
  readonly x1: number;
  readonly top: number;
  readonly bottom: number;
  /** The clef in effect on each staff, by staff index. */
  readonly clefs: readonly (Clef | null)[];
  /** The key signature in effect on each staff, by staff index. */
  readonly keys: readonly Key[];
}

/** One drawn notehead. */
export interface Notehead {
  readonly id: string;
  /** Left edge. */
  readonly x: number;
  /** Centre: the staff position. */
  readonly y: number;
  readonly width: number;
  readonly height: number;
  readonly system: number;
  readonly staff: number;
  readonly bar: string;
}

export interface PageGeometry {
  readonly systems: readonly System[];
  readonly bars: readonly Bar[];
  readonly noteheads: ReadonlyMap<string, Notehead>;
}

// SMuFL glyph codes Verovio uses.
const CLEFS: Record<string, { sign: Clef["sign"]; reference: Spelling }> = {
  E050: { sign: "G", reference: { step: "G", octave: 4 } },
  E052: { sign: "G", reference: { step: "G", octave: 3 } }, // treble 8vb
  E053: { sign: "G", reference: { step: "G", octave: 5 } }, // treble 8va
  E05C: { sign: "C", reference: { step: "C", octave: 4 } },
  E062: { sign: "F", reference: { step: "F", octave: 3 } },
  E064: { sign: "F", reference: { step: "F", octave: 2 } }, // bass 8vb
};
const SHARP = "E262";
const FLAT = "E260";
/** Notehead widths in staff spaces (SMuFL metrics). */
const WHOLE_NOTEHEAD = "E0A2";
const NOTEHEAD_WIDTH = 1.18;
const WHOLE_NOTEHEAD_WIDTH = 1.69;

const NO_KEY: Key = { accidental: null, count: 0 };

/** Reads one engraved page: pass the page's `<svg>` (or anything containing it). */
export function readPage(page: Element): PageGeometry {
  const systems: System[] = [];
  const bars: Bar[] = [];
  const noteheads = new Map<string, Notehead>();

  // Clefs and keys carry across bars and systems until changed.
  const clefs: (Clef | null)[] = [];
  const keys: Key[] = [];

  page.querySelectorAll("g.system").forEach((systemElement, systemIndex) => {
    const staves: Staff[] = [];

    for (const measure of systemElement.querySelectorAll("g.measure")) {
      const staffElements = [...measure.children].filter((child) => child.classList.contains("staff"));
      let top = Infinity;
      let bottom = -Infinity;
      let x0 = Infinity;
      let x1 = -Infinity;

      staffElements.forEach((staffElement, staffIndex) => {
        const lines = staffLines(staffElement);
        if (lines.length !== 5) return;

        const ys = lines.map((line) => line.y);
        const spacing = (ys[4]! - ys[0]!) / 4;
        const left = Math.min(...lines.map((line) => line.x0));
        const right = Math.max(...lines.map((line) => line.x1));

        top = Math.min(top, ys[0]!);
        bottom = Math.max(bottom, ys[4]!);
        x0 = Math.min(x0, left);
        x1 = Math.max(x1, right);

        const existing = staves[staffIndex];
        staves[staffIndex] = existing
          ? { ...existing, x0: Math.min(existing.x0, left), x1: Math.max(existing.x1, right) }
          : { index: staffIndex, lines: ys, spacing, x0: left, x1: right };

        // A clef or key at the start of the bar (system start, or a change
        // inside a layer) takes effect from this bar on.
        const clef = firstClef(staffElement, ys);
        if (clef) clefs[staffIndex] = clef;
        const key = keyOf(staffElement);
        if (key) keys[staffIndex] = key;

        for (const note of staffElement.querySelectorAll("g.note")) {
          const head = notehead(note, spacing);
          if (!head || !note.id) continue;

          noteheads.set(note.id, {
            id: note.id,
            ...head,
            system: systemIndex,
            staff: staffIndex,
            bar: measure.id,
          });
        }
      });

      if (Number.isFinite(top)) {
        bars.push({
          id: measure.id,
          system: systemIndex,
          x0,
          x1,
          top,
          bottom,
          clefs: staffElements.map((_, staffIndex) => clefs[staffIndex] ?? null),
          keys: staffElements.map((_, staffIndex) => keys[staffIndex] ?? NO_KEY),
        });
      }
    }

    if (staves.length > 0) {
      systems.push({
        index: systemIndex,
        top: staves[0]!.lines[0]!,
        bottom: staves[staves.length - 1]!.lines[4]!,
        staves,
      });
    }
  });

  return { systems, bars, noteheads };
}

interface Line {
  readonly x0: number;
  readonly x1: number;
  readonly y: number;
}

/** A staff's own lines: its direct `<path>` children drawn as `M x0 y L x1 y`. */
function staffLines(staff: Element): Line[] {
  const lines: Line[] = [];

  for (const child of staff.children) {
    if (child.tagName.toLowerCase() !== "path") continue;

    const match = /^M\s*(-?[\d.]+)[\s,]+(-?[\d.]+)\s*L\s*(-?[\d.]+)[\s,]+(-?[\d.]+)\s*$/.exec(
      child.getAttribute("d") ?? "",
    );
    if (!match || match[2] !== match[4]) continue;

    lines.push({ x0: Number(match[1]), x1: Number(match[3]), y: Number(match[2]) });
  }

  return lines.sort((a, b) => a.y - b.y).slice(0, 5);
}

/** The glyph code a `<use>` draws: `#E050-xyz` -> `E050`. */
function glyphOf(use: Element | null): string | null {
  const href = use?.getAttribute("xlink:href") ?? use?.getAttribute("href") ?? "";
  return /^#([0-9A-F]{4})/.exec(href)?.[1] ?? null;
}

/** `translate(x, y)` out of a transform attribute. */
function translation(element: Element | null): { x: number; y: number } | null {
  const match = /translate\(\s*(-?[\d.]+)[\s,]+(-?[\d.]+)\s*\)/.exec(element?.getAttribute("transform") ?? "");
  return match ? { x: Number(match[1]), y: Number(match[2]) } : null;
}

/** The first clef drawn in a staff's bar, with the line it sits on. */
function firstClef(staff: Element, lines: readonly number[]): Clef | null {
  for (const element of staff.querySelectorAll("g.clef")) {
    const use = element.querySelector("use");
    const known = CLEFS[glyphOf(use) ?? ""];
    const at = translation(use);
    if (!known || !at) continue;

    // Nearest line; lines are top to bottom, so line 1 is lines[4].
    let nearest = 0;
    for (let i = 1; i < lines.length; i += 1) {
      if (Math.abs(lines[i]! - at.y) < Math.abs(lines[nearest]! - at.y)) nearest = i;
    }

    return { sign: known.sign, reference: known.reference, line: 5 - nearest };
  }

  return null;
}

/** The key signature drawn in a staff's bar, if one is. */
function keyOf(staff: Element): Key | null {
  const signature = staff.querySelector("g.keySig");
  if (!signature) return null;

  const glyphs = [...signature.querySelectorAll("g.keyAccid use")].map(glyphOf);
  const sharps = glyphs.filter((glyph) => glyph === SHARP).length;
  const flats = glyphs.filter((glyph) => glyph === FLAT).length;

  if (sharps > 0) return { accidental: "sharp", count: sharps };
  if (flats > 0) return { accidental: "flat", count: flats };
  return NO_KEY;
}

/** A note's notehead box: left edge and centre from its glyph's placement. */
function notehead(
  note: Element,
  spacing: number,
): { x: number; y: number; width: number; height: number } | null {
  const use = note.querySelector("g.notehead use");
  const at = translation(use);
  if (!at) return null;

  const width = (glyphOf(use) === WHOLE_NOTEHEAD ? WHOLE_NOTEHEAD_WIDTH : NOTEHEAD_WIDTH) * spacing;
  return { x: at.x, y: at.y, width, height: spacing };
}

/** The cursor's band, in the same drawing units as the page. */
export interface Band {
  readonly system: number;
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/**
 * The cursor for the notes at one position: as wide as their noteheads, and
 * always the full height of their system, so a note still held in the other
 * hand is covered too. Null if none of the notes is drawn on this page.
 */
export function cursorBand(geometry: PageGeometry, elementIds: readonly string[]): Band | null {
  const heads = elementIds.flatMap((id) => {
    const head = geometry.noteheads.get(id);
    return head ? [head] : [];
  });
  const system = geometry.systems.find((candidate) => candidate.index === heads[0]?.system);
  if (!system) return null;

  const spacing = system.staves[0]!.spacing;
  const onSystem = heads.filter((head) => head.system === system.index);
  const x0 = Math.min(...onSystem.map((head) => head.x)) - spacing / 2;
  const x1 = Math.max(...onSystem.map((head) => head.x + head.width)) + spacing / 2;
  // Reach past ledger-line notes as well as the staves themselves.
  const y0 = Math.min(system.top, ...onSystem.map((head) => head.y - head.height / 2)) - spacing;
  const y1 = Math.max(system.bottom, ...onSystem.map((head) => head.y + head.height / 2)) + spacing;

  return { system: system.index, x: x0, y: y0, width: x1 - x0, height: y1 - y0 };
}

/** The inner drawing surface of a page, where every coordinate above lives. */
export function drawingSurface(page: Element): SVGSVGElement | null {
  return page.querySelector("svg.definition-scale");
}
