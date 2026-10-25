//! Shared domain vocabulary.
//!
//! Every type here is a concept from the design, expressed in
//! Rust. Nothing else belongs in this crate: it holds the words that more
//! than one component needs to agree on, and it has no dependencies at all.
//!
//! The design gives the test for admission — *if the implementation were
//! rebuilt from scratch tomorrow with entirely different technology, would
//! this concept still exist?* A `Pitch` survives that. A buffer does not.

mod measure;
mod piece;
mod pitch;

pub use measure::{Location, MeasureNumber};
pub use piece::PieceId;
pub use pitch::{Pitch, PitchSpelling, Step};
