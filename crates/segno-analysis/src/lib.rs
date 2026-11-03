//! What happened, compared against what was written.
//!
//! Analysis is a comparison and nothing more: it measures. It names exactly
//! four outcomes, and so does [`Finding`]: which notes were correct, which were wrong, which
//! expected notes never arrived, and which unexpected ones did.
//!
//! # Reproducibility is not a quality goal here, it is the definition
//!
//! The design puts it plainly: *"A comparison that could differ between runs
//! would not be a comparison of these two things."* the requirement and
//! The requirement requires identical results across separate executions, and
//! The design makes that release-blocking.
//!
//! Two concrete mechanisms rather than good intentions:
//!
//! - Nothing here is ordered by a hash. `scripts/check-arch.sh` rejects
//!   `HashMap` and `HashSet` in the pure crates, because Rust randomises
//!   their iteration order **per process** — the classic way a comparison
//!   that looks deterministic silently is not.
//! - `segno-cli` runs a fixed performance in two separate processes and CI
//!   diffs the bytes. Equality asserted inside one process would not catch
//!   the failure this guards against.
//!
//! # Why this crate depends on `segno-practice`
//!
//! It reuses [`Cursor`](segno_practice::Cursor) rather than re-deriving
//! which played note answers which written one. The Cursor is not an input to
//! Analysis — the design names the inputs, and they are an Observed
//! Performance and a Reference Performance — it is the shared *definition* of
//! what matches what.
//!
//! Duplicating that rule would risk the worst bug this product could have:
//! the cursor advancing on screen while Analysis later calls the same note
//! wrong. The design requires one source of truth, and this is it.

mod analysis;
mod bars;
pub mod coach;
mod finding;
mod fixtures;
mod suggestion;

pub use analysis::{Analysis, Summary};
pub use bars::{Bar, Bars, StepTrace, Timing, Trace};
pub use finding::Finding;
pub use suggestion::Suggestion;

/// Which rules judged an attempt: bump this whenever a change here would
/// judge an already-kept attempt differently (what counts as a pause, a
/// wrong note, a level). Each session records the number it was played
/// under. Results are still recomputed with today's rules (D5) and the
/// performer is not told (DECISIONS, 3 Nov, Q5); the number is kept so a
/// change can be traced in the data.
pub const RULES: u32 = 1;
