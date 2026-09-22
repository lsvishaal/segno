//! The performer's MIDI keyboard, behind the port `hogwash-capture` defines.
//!
//! `MVP §6.3` makes a connected MIDI instrument the only source of an
//! Observed Performance, and `Product_Discovery.md §11` confirms it as the
//! canonical and only performance input. This is the crate that talks to it,
//! and the only one that does.
//!
//! Two things live here and nothing else: decoding a MIDI message
//! ([`message`], pure and tested without hardware), and getting messages off
//! `midir`'s thread and onto the one that draws (`ADR-0009`).

mod keyboard;
mod message;
mod output;

pub use keyboard::MidiInstruments;
pub use output::{NoOutput, OutputPort, Thru, output_ports};
