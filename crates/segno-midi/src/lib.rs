//! The performer's MIDI keyboard, behind the port `segno-capture` defines.
//!
//! The design makes a connected MIDI instrument the only source of an
//! Observed Performance, and the design confirms it as the
//! canonical and only performance input. This is the crate that talks to it,
//! and the only one that does.
//!
//! Two things live here and nothing else: decoding a MIDI message
//! ([`message`], pure and tested without hardware), and getting messages off
//! `midir`'s thread and onto the one that draws.

mod keyboard;
mod message;
mod output;

pub use keyboard::MidiInstruments;
pub use output::{NoOutput, OutputPort, Thru, output_ports};
