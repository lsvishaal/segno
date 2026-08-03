//! Shared domain vocabulary.
//!
//! Every type here is a concept from `003_Domain_Model.md §7`, expressed in
//! Rust. Nothing else belongs in this crate: it holds the words that more
//! than one component needs to agree on, and it has no dependencies at all.
//!
//! `003 §6.2` gives the test for admission — *if the implementation were
//! rebuilt from scratch tomorrow with entirely different technology, would
//! this concept still exist?* A `Pitch` survives that. A buffer does not.

mod measure;
mod piece;
mod pitch;

pub use measure::{Location, MeasureNumber};
pub use piece::PieceId;
pub use pitch::{Pitch, PitchSpelling, Step};
