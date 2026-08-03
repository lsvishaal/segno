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

/**
 * One place in the music, and the elements drawn for it.
 *
 * This is the whole mechanism behind showing correctness on the score
 * (`MVP §6.6`): Verovio writes the same identifier into its own document and
 * into the SVG, so a position in the piece names elements in the DOM.
 */
export interface Mark {
  /** Which bar, as a performer counts them (from one). */
  readonly measure: number;
  /** Which position in the piece, counting from zero across the whole score. */
  readonly position: number;
  /** Which page it was drawn on, counting from zero. */
  readonly page: number;
  /** The identifiers Verovio gave the notes here. Several for a chord. */
  readonly elementIds: readonly string[];
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
  | { readonly kind: "played"; readonly position: number; readonly correct: boolean }
  | { readonly kind: "advanced"; readonly position: number }
  | { readonly kind: "reachedEnd" }
  | {
      readonly kind: "instrumentLost";
      readonly instrument: string;
      readonly secondsRemaining: number;
    }
  | { readonly kind: "instrumentRestored" }
  | { readonly kind: "ended"; readonly summary: AttemptSummary };
