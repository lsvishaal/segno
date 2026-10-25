//! Verovio as Hogwash's engraver.
//!
//! This is the sole implementor of [`Engraver`], and the only
//! crate in the workspace that knows C exists. Everything above it deals in
//! [`EngravedScore`] and never learns which engine drew the notes.
//!
//! [`Engraver`]: hogwash_engraving::Engraver
//! [`EngravedScore`]: hogwash_engraving::EngravedScore

mod engraver;
mod failure;
mod intact;
mod mei;
mod timemap;
mod translate;

pub use engraver::{Layout, VerovioEngraver};
mod toolkit;
