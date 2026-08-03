//! Written music.
//!
//! This crate holds the notation side of the domain — the Score a performer
//! practises from, and the Reference Performance derived from it that says
//! what *correct* means (`003 §7.4`, `§7.8`, `§7.9`, `§7.10`).
//!
//! It is pure. It contains no file formats, no rendering, and no I/O.
//! `003 §7.10` is explicit that a Score "does not know how it is drawn" and
//! "does not know the file it arrived in"; keeping those out of this crate is
//! how that stays true rather than merely intended. `scripts/check-arch.sh`
//! enforces it.
//!
//! # What is deliberately absent
//!
//! **Timing and duration.** `003 §15` lists them among the things the domain
//! does not model, because Version 1 does not assess them: with practice
//! waiting for the performer (`MVP §5`), the tempo *is* whatever the performer
//! plays, so there is no reference to measure against (`003 §7.4`). Modelling
//! them would model deferred capability. A written Note therefore carries a
//! pitch and nothing else.

mod measure;
mod note;
mod reference;
mod score;

pub use measure::{Measure, Position};
pub use note::Note;
pub use reference::{ExpectedNotes, ReferencePerformance};
pub use score::Score;
