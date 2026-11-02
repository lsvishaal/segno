//! Resolving material a performer offers into a Piece, or into an explained
//! refusal.
//!
//! Pure: it reads no file and links to no engine. What it is given is bytes
//! and an [`Engraver`](segno_engraving::Engraver); whoever opened the file
//! is its caller.
//!
//! # The commitment this component exists to keep
//!
//! never let the performer fail alone. Rejecting
//! a file is easy; rejecting it in a way the performer can act on requires
//! knowing **what they offered**, which is why format recognition happens
//! before any attempt to read the material as music. An engine's parse
//! failure says that reading failed, not what the thing was, and "your
//! `MusicXML` is invalid" sends someone holding a PDF to check the wrong
//! thing.

mod assessment;
mod format;
mod report;

pub use assessment::{Outcome, assess};
pub use format::{OfferedFormat, recognise};
pub use report::{UnsupportedInput, ValidationIssue, ValidationReport};
