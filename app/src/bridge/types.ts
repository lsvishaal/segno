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
