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
  /** What the performer's file was called, for an imported piece. */
  readonly fileName: string | null;  /** Imported by the performer (only those can be renamed or deleted). */
  readonly imported: boolean;
}

/** The written letter name of a note — `"C"` through `"B"`. */
export type Step = "C" | "D" | "E" | "F" | "G" | "A" | "B";

/** A letter and octave, without the accidental. */
export interface StaffPosition {
  readonly step: Step;
  /** Scientific pitch notation, where middle C is octave 4. */
  readonly octave: number;
}

/** One notehead Verovio drew, and the note it sounds (under an 8va line, not where it is drawn). */
export interface DrawnNote extends StaffPosition {
  /** The identifier Verovio gave this notehead. */
  readonly elementId: string;
  /** The MIDI number it sounds as. */
  readonly pitch: number;
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
  /** Which drawn `g.measure` element is which bar. */
  readonly bars: readonly Bar[];
}

/** A bar as drawn: its `g.measure` element, and its number. */
export interface Bar {
  readonly elementId: string;
  readonly measure: number;
}

/** A loop over bars `first` to `last`; attempts start at `position`. */
export interface LoopRange {
  readonly position: number;
  readonly first: number;
  readonly last: number;
}

/** Where the next attempt starts: a step in played order, and its bar. */
export interface StartPoint {
  readonly position: number;
  readonly measure: number;
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

/**
 * What importing a `MusicXML` file produced, once accepted.
 *
 * `lines` is `ValidationReport::lines()`, verbatim — "The whole score was
 * read." when nothing was lost, one sentence per loss otherwise. Nothing on
 * this side could reconstruct it after the fact, so it travels with the
 * result.
 */
export interface ImportedPiece {
  readonly piece: PieceSummary;
  readonly lines: readonly string[];
  /** Identical to a piece already in the library; nothing was added. */
  readonly already: boolean;
}

/** A MIDI instrument the performer could play (FR-307). */
export interface InstrumentSummary {
  /** What to show, and what `startAttempt` names it by. */
  readonly name: string;
}

/** A MIDI output device the performer could route sound to. */
export interface OutputPortSummary {
  /** What to show, and what `setThru` names it by. */
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
  /** What to draw once playing stops. */
  readonly review: Review;
}

/** The results after a run (Start to stopping): every number and sentence comes from Rust. */
export interface Review {
  readonly headline: string;
  readonly numbers: Numbers;
  /** One per bar, in bar-number order, from where the run began. */
  readonly bars: readonly ReviewBar[];
  /** One per completed pass, when the run stopped in a loop. */
  readonly passes: readonly ReviewPass[];
  /** What happened at each step played. */
  readonly marks: readonly ReviewMark[];
  readonly suggestion: Suggested | null;
}

/** The numbers at a glance. */
export interface Numbers {
  /** Right notes out of every note played, in percent. */
  readonly accuracy: number | null;
  /** Quarter notes per minute, pauses included. */
  readonly tempo: number | null;
  /** Whether `tempo` follows the written rhythm (else every step counts as one). */
  readonly rhythm: boolean;
  /** Evenness of tempo across bars, in percent. */
  readonly steadiness: number | null;
  readonly right: number;
  readonly wrong: number;
  readonly missed: number;
  readonly longestPauseMs: number;
  readonly longestPauseBar: number | null;
  readonly timeMs: number | null;
  readonly restarts: number;
}

export interface ReviewBar {
  readonly measure: number;
  readonly state: "played" | "notReached" | "empty";
  readonly tempo: number | null;
  readonly pauseMs: number;
  readonly intoMs: number;
  readonly wrong: number;
  readonly breaks: number;
  /** Said when pointing at it. */
  readonly line: string;
}

export interface ReviewPass {
  readonly number: number;
  readonly tempo: number | null;
  readonly wrong: number;
  readonly clean: boolean;
  readonly line: string;
}

export interface ReviewMark {
  readonly position: number;
  readonly correct: boolean;
  readonly wrong: number;
}

/** The suggestion's button: loop `first`–`last`, or play it again when absent. */
export interface Suggested {
  readonly label: string;
  readonly first: number | null;
  readonly last: number | null;
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
  | { readonly kind: "ended"; readonly summary: AttemptSummary }
  /** A wrong or unexpected note spoiled this pass of the loop. */
  | { readonly kind: "loopCount"; readonly cleanInARow: number }
  /** A pass reached the loop's end; the next began at `position`. */
  | { readonly kind: "looped"; readonly position: number; readonly cleanInARow: number }
  /** The loop reached its target and is over; the cursor waits at `position`. */
  | {
      readonly kind: "loopDone";
      readonly position: number;
      readonly passes: number;
      readonly first: number;
      readonly last: number;
      /** What to practise now: the loop a bar wider each side. */
      readonly widen: Suggested | null;
    };
