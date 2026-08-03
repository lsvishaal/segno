//! An Analysis, in terms the performer can understand.
//!
//! `003 §7.7` — Feedback *"is what stands between an accurate measurement and
//! a person who can use it."* It is expressed in musical terms rather than
//! measurements — *where* a note was wrong, not a figure describing it.
//!
//! # Two boundaries this crate exists to hold
//!
//! **It describes; it does not advise.** `003 §7.7` — *"it says what
//! happened, not what to do next"* — and FR-503 forbids prescriptive practice
//! recommendations outright. `feedback_contains_no_prescriptive_language`
//! scans the rendered output for advisory wording, so this is a build
//! failure rather than a style preference.
//!
//! **It never names the note the performer played.** ADR-0001: *"nothing
//! reconstructs a spelling from a sounding pitch, because that reconstruction
//! is a guess and this project does not guess about notation."* F♯ and G♭ are
//! one key on a keyboard; the instrument reports the key, never which reading
//! produced it. So Feedback says what the Score **wrote**, and reports that
//! something else was played without inventing a name for it. See ADR-0004.
//!
//! # A note on FR-501
//!
//! FR-501 is titled *"identify areas requiring improvement"* and its
//! rationale mentions knowing "what to improve next", which reads as though
//! it asks for advice. Its **acceptance criterion** does not: *"the areas of
//! the piece associated with those deviations are identified"*. That is
//! location, and location is what [`Feedback::measures_with_deviations`]
//! provides. Where the rationale and the acceptance criterion disagree, `003
//! §7.7` and FR-503 both side with the criterion.
//!
//! That is also **not** FR-407, which asks for analysis results *aggregated*
//! per measure and remains deferred (`MVP §8`). A list of bars where
//! something happened is not a per-bar result.
//!
//! # Structured, then rendered
//!
//! `004 §8.7` sends Feedback to Presentation **and to Practice Record
//! Keeping**. Persisting prose would mean that rewording it later makes old
//! records inconsistent with new ones, so Feedback holds located, typed
//! [`Statement`]s and produces English on demand.

mod feedback;
mod fixtures;
mod statement;

pub use feedback::Feedback;
pub use statement::Statement;
