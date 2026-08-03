//! The port between written music and something a performer can look at.
//!
//! `004 §6.3` requires ports and adapters: the pure side states what it
//! needs, and an adapter supplies it. This crate is the stating. It draws
//! nothing, reads no file, and links to no engine — every implementation
//! lives outside it, and today there is exactly one
//! (`hogwash-engraving-verovio`).
//!
//! This is **not a new component**. `004 §14` forbids inventing component
//! boundaries; an interface is not a boundary. What is declared here is what
//! `004 §8.8` Presentation requires and what `004 §8.2` Acquisition &
//! Validation produces on acceptance.
//!
//! # Why reading and drawing are one operation
//!
//! An engraver is asked for a Score *and* its engraving in a single call,
//! which looks like two responsibilities and is deliberately one.
//! `004 §6.5` requires a single source of truth: if the domain `Score` came
//! from one parse of a file and the drawn glyphs from another, keeping a Note
//! aligned with the glyph that represents it would depend on the two parses
//! agreeing. They would not always agree, and the symptom would be the
//! cursor highlighting the wrong note — the most damaging bug this product
//! could have.
//!
//! # Why identifiers do not leave the adapter
//!
//! The Verovio spike (`S0` Q3) measured that engraving identifiers are
//! regenerated on every render, differ between two renders in one process,
//! and may be renumbered by an engine upgrade. An [`ElementId`] is therefore
//! **per-render ephemera**: valid for the [`EngravedScore`] that produced it
//! and nothing else. Nothing persists one. What persists is
//! [`Location`](hogwash_domain::Location), which the Score fixes.

mod engraved;

pub use engraved::{
    Details, ElementId, EngravedScore, Engraver, EngravingFailure, MalformedEngraving, PageIndex,
    Placement,
};
