//! An Analysis, in terms the performer can understand: *where* a note was
//! wrong, in musical terms, rather than a figure describing it.
//!
//! # It never names the note the performer played
//!
//! F♯ and G♭ are one key on a keyboard; the instrument reports the key, never
//! which reading produced it, and reconstructing a spelling from a key would
//! be a guess. So Feedback says what the Score **wrote**, and reports that
//! something else was played without inventing a name for it.
//!
//! # Structured, then rendered
//!
//! Feedback goes to the screen and to the practice records. Persisting prose
//! would mean that rewording it later makes old records inconsistent with new
//! ones, so Feedback holds located, typed [`Statement`]s and produces English
//! on demand.

mod feedback;
mod fixtures;
pub mod review;
mod statement;

pub use feedback::Feedback;
pub use statement::Statement;
