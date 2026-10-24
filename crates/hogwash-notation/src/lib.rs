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
//! **Tempo.** With practice waiting for the performer, the tempo is whatever
//! the performer plays, so nothing here says when a note is due in seconds.
//! The written rhythm is kept (each position's beat in its bar, each bar's
//! length, when known) so that a long note is not mistaken for a pause.

mod measure;
mod note;
mod reference;
mod score;

pub use measure::{Measure, Position, TICKS_PER_QUARTER};
pub use note::Note;
pub use reference::{ExpectedNotes, ReferencePerformance};
pub use score::Score;
