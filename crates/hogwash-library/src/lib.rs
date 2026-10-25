//! The Pieces available to practise.
//!
//! Small, and deliberately so. The design makes the Library the set of
//! Pieces available to practise from, and gives it one invariant worth
//! defending: it is **never empty**. Everything here exists to make that
//! invariant impossible to break rather than merely unlikely.

mod library;
mod piece;

pub use library::Library;
pub use piece::{Piece, title_from};
