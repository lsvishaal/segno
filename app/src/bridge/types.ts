/**
 * The shapes that cross the boundary from Rust.
 *
 * Hand-written to mirror the `#[derive(Serialize)]` types in
 * `app/src-tauri/src/commands.rs`. They are deliberately *not* the domain's
 * types: the same translation `ADR-0010` made for storage, for the same
 * reason — an interface bound directly to `hogwash_notation::Score` would
 * break silently the moment that type was refactored.
 *
 * If one of these drifts from its Rust counterpart, the symptom is a missing
 * field at run time. That is the cost of the seam, and it is why the shapes
 * are kept small and flat.
 */

/** A Piece, as the library screen needs it. */
export interface PieceSummary {
  /** Its durable name, used to ask for it again. */
  readonly id: string;
  /** What to call it. Never empty. */
  readonly title: string;
  /** Who wrote it, where the source said so. */
  readonly composer: string | null;
  /** How many bars. */
  readonly bars: number;
  /** How many notes are expected in total. */
  readonly notes: number;
}

/** The written letter name of a note — `"C"` through `"B"`. */
export type Step = "C" | "D" | "E" | "F" | "G" | "A" | "B";

/**
 * A written pitch, for placing things on the staff.
 *
 * Not the domain's spelling — no `alter` — because this exists for exactly
 * one purpose: computing a vertical position. An accidental draws a glyph
 * beside a note, it never moves the note up or down.
 */
export interface StaffPosition {
  readonly step: Step;
  /** Scientific pitch notation, where middle C is octave 4. */
  readonly octave: number;
}

/** One notehead Verovio drew, and the spelling it was drawn from. */
export interface DrawnNote extends StaffPosition {
  /** The identifier Verovio gave this notehead. */
  readonly elementId: string;
}

/**
 * One place in the music, and the elements drawn for it.
 *
 * This is the whole mechanism behind showing correctness on the score
 * (`MVP §6.6`): Verovio writes the same identifier into its own document and
 * into the SVG, so a position in the piece names elements in the DOM. The
 * spelling on each `DrawnNote` is what lets a wrong note — which has no
 * position of its own, `ADR-0001` — be placed relative to notes that really
 * were written.
 */
export interface Mark {
  /** Which bar, as a performer counts them (from one). */
  readonly measure: number;
  /** Which position in the piece, counting from zero across the whole score. */
  readonly position: number;
  /** Which page it was drawn on, counting from zero. */
  readonly page: number;
  /** Every note drawn here. Several for a chord. */
  readonly notes: readonly DrawnNote[];
}

/** A Piece, engraved and ready to practise from. */
export interface OpenedPiece {
  readonly id: string;
  readonly title: string;
  readonly composer: string | null;
  /** The engraved pages, as SVG markup. */
  readonly pages: readonly string[];
  /** Where every expected note was drawn, in reading order. */
  readonly marks: readonly Mark[];
}

/**
 * Why the application cannot do its job.
 *
 * Rendered as a whole screen rather than a toast: this is not a transient
 * error, it is Hogwash being unable to start, and `004 §8.2` asks for an
 * explanation rather than a shrug.
 */
export interface Unavailable {
  /** One line, for a heading. */
  readonly summary: string;
  /** What went wrong, in full. */
  readonly detail: string;
  /** Every place that was looked in, so it can be put in one. */
  readonly lookedIn: readonly string[];
}

/** A MIDI instrument the performer could play (FR-307). */
export interface InstrumentSummary {
  /** What to show, and what `startAttempt` names it by. */
  readonly name: string;
}

/**
 * What an ended attempt is reported as.
 *
 * `MVP §10` leaves an attempt summary's exact content an open design
 * question beyond "one overall result" (FR-408); this is Stage D's minimum
 * to make the loop demonstrable end-to-end. `lines` is `Feedback::lines()`,
 * verbatim — every sentence about the performance arrives from Rust already
 * written, and nothing here composes one.
 */
export interface AttemptSummary {
  readonly lines: readonly string[];
  readonly correct: number;
  readonly incorrect: number;
  readonly omitted: number;
  readonly unexpected: number;
  readonly reachedTheEnd: boolean;
  /** Whether the performer chose to end it, or the instrument never came back. */
  readonly autoEnded: boolean;
  readonly reason: string | null;
}

/**
 * One update the practice loop reports, while an attempt is running.
 *
 * Mirrors `hogwash_app_lib::practice::PracticeEvent` — a single tagged enum,
 * so the whole practice loop is one `switch` on `kind` here too.
 */
export type PracticeEvent =
  | {
      readonly kind: "played";
      readonly position: number;
      readonly correct: boolean;
      /** The bare chromatic (MIDI) pitch a keyboard reported — no spelling. */
      readonly pitch: number;
    }
  | { readonly kind: "advanced"; readonly position: number }
  | { readonly kind: "reachedEnd" }
  | {
      readonly kind: "instrumentLost";
      readonly instrument: string;
      readonly secondsRemaining: number;
    }
  | { readonly kind: "instrumentRestored" }
  | { readonly kind: "ended"; readonly summary: AttemptSummary };
