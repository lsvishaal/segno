//! What happened, kept.
//!
//! The design includes session and attempt recording in Version 1 for a
//! reason it states plainly, and which is worth repeating because it is the
//! only argument for building this now: *analytics can always be built later
//! over history that exists, but history not recorded in v1 can never be
//! reconstructed.*
//!
//! So this crate's job is not to be clever. It is to not lose anything.

mod atomic;
mod store;
mod stored;

pub use store::{AttemptRecord, Ended, PieceRecord, Records, RecordsFailure, SessionRecord};
