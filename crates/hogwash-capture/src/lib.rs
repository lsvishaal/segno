//! What the performer actually played.
//!
//! `004 §8.5` assigns the Observed Performance to Performance Capture, and
//! constrains that component with a sentence worth repeating: it *"knows
//! nothing about what was expected"*. Nothing in this crate references a
//! Score, a Reference Performance, or correctness — and it depends on no
//! crate that could supply one.
//!
//! # Why this crate is pure
//!
//! Live MIDI input is the only source of an Observed Performance in Version 1
//! (`MVP §6.3`), but the *concept* is not the hardware. Keeping the concept
//! here and the instrument behind a port means `hogwash-practice` and
//! `hogwash-analysis` can be exercised entirely from a test without a
//! keyboard attached — which is what makes `004 §12`'s correctness-critical
//! component testable at all. The port is [`MidiSource`]; its `midir`
//! implementation is `hogwash-midi`, and `scripts/check-arch.sh` fails the
//! build if that dependency ever lands in this crate.

mod instrument;
mod observed;

pub use instrument::{Action, Heard, Instrument, Keyboard, MidiSource, NoInstrument};
pub use observed::{ObservedPerformance, PedalChange, Touch};
