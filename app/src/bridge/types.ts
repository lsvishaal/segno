/**
 * The shapes that cross the boundary from Rust.
 *
 * Hand-written to mirror the `#[derive(Serialize)]` types in
 * `app/src-tauri/src/commands.rs`. They are deliberately *not* the domain's
 * types: the same translation the design made for storage, for the same
 * reason — an interface bound directly to `segno_notation::Score` would
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
  /** Which starter shelf a bundled piece goes on; `null` for imports. */
  readonly level: "beginner" | "intermediate" | null;
  /** When it was last practised, in Unix milliseconds. */
  readonly lastPractisedMs: number | null;
  /** Up to two of its hardest bars over time, hardest first. */
  readonly weakest: readonly number[];
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
 *: Verovio writes the same identifier into its own document and
 * into the SVG, so a position in the piece names elements in the DOM. The
 * spelling on each `DrawnNote` is what lets a wrong note — which has no
 * position of its own, the design — be placed relative to notes that really
 * were written.
 */
export interface Mark {
  /** Which bar, as printed: from one, or 0 for a pickup. */
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
 * error, it is Segno being unable to start, and the design asks for an
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
  /**
   * Same title and composer as one of your own pieces (`piece`): nothing was
   * added yet; `resolveImport` updates that piece or adds a new one.
   */
  readonly asks: boolean;
}

/** A MIDI instrument the performer could play. */
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
 * The design leaves an attempt summary's exact content an open design
 * question beyond "one overall result"; this is Stage D's minimum
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

/** One bar in one session, from 1 pass: 0 went fine, 1–4 lighter to deeper. */
export interface ProgressCell {
  readonly measure: number;
  readonly passes: number;
  readonly pauseUs: number;
  readonly wrong: number;
  readonly stops: number;
  readonly level: number;
}

/** Bars next to each other to practise together, looped from `from` to `to`. */
export interface Passage {
  readonly first: number;
  readonly last: number;
  readonly from: number;
  readonly to: number;
  /** "about 3.1 s of hesitation each time through". */
  readonly facts: string;
}

/** Everything the Progress page shows for one piece; the words come from Rust. */
export interface ProgressView {
  readonly id: string;
  readonly title: string;
  readonly composer: string | null;
  readonly bars: number;
  /** Every bar's printed number in written order; a pickup is 0, first. */
  readonly measures: readonly number[];
  /** Newest first; a bar not played that session has no cell. */
  readonly sessions: readonly {
    readonly startedMs: number;
    readonly playedUs: number;
    readonly bars: readonly ProgressCell[];
  }[];
  /** Every session, whatever version of the score it was played on. */
  readonly practised: number;
  readonly playedUs: number;
  /** When the score's bars changed, if older sessions are left out of the bars. */
  readonly updatedMs: number | null;
  readonly lastPractisedMs: number | null;
  /** Each bar played: its level now, or null when it needs another pass. */
  readonly now: readonly { readonly measure: number; readonly level: number | null }[];
  readonly workOnNext: readonly Passage[];
  /** Every bar: its sentence ("Now: clean."), and the loop its Practise button plays. */
  readonly details: readonly {
    readonly measure: number;
    readonly line: string;
    readonly from: number;
    readonly to: number;
  }[];
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
  /** What to work on, most important first: at most three, or one saying the run was clean. */
  readonly cards: readonly ReviewCard[];
  /** What went well, a line each. */
  readonly wentWell: readonly string[];
  /** Said instead of cards when too little was played. */
  readonly note: string | null;
}

/** One thing to work on, in words, with the bars to show and to practise. */
export interface ReviewCard {
  readonly kind:
    | "intoBar"
    | "jump"
    | "fastNotes"
    | "wrongNotes"
    | "rushedLongNotes"
    | "slowBars"
    | "keepsStopping"
    | "bigStep"
    | "clean";
  /** The bars to show from the score. */
  readonly first: number;
  readonly last: number;
  /** The bars its button practises; null for a clean run. */
  readonly drillFirst: number | null;
  readonly drillLast: number | null;
  /** The bar a pause going into it came before, to mark. */
  readonly pauseInto: number | null;
  readonly title: string;
  readonly what: string;
  readonly why: string;
  readonly steps: readonly string[];
  readonly button: string | null;
  readonly also: string | null;
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
 * Mirrors `segno_app_lib::practice::PracticeEvent` — a single tagged enum,
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
