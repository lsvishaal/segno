//! MIDI Thru — routing what a keyboard sends straight to a real synth,
//! hardware or software, instead of Hogwash making its own sound.
//!
//! # Why this is not `hogwash-capture`'s business
//!
//! `MidiSource`'s whole contract (`Keyboard::played()`, `is_connected()`) is
//! about *pulling notes* for practice correctness — meaningless for an
//! output, which only ever has bytes pushed to it. `OutputPort` and
//! [`NoOutput`] are defined here instead, so `hogwash-capture` stays entirely
//! unaware that an output side exists at all.
//!
//! # Why this forwards raw bytes, not decoded pitches
//!
//! `keyboard.rs`'s `Connected` decodes every message down to a [`Pitch`] for
//! the Attempt, which throws away velocity, note-off timing, and anything
//! that is not a note-on — exactly right for correctness, wrong for a synth
//! that should sound like what was actually played. [`Thru`] opens its own,
//! independent connection to the same input port and forwards every message
//! untouched. Two separate subscribers to one input port is ordinary MIDI
//! I/O — it is what makes a hardware MIDI patchbay possible at all — so this
//! never touches or even knows about whatever `keyboard.rs` is doing with the
//! same device.

use std::sync::Mutex;

use hogwash_capture::Instrument;
use midir::{MidiInput, MidiInputConnection, MidiOutput};

/// The name `midir` is asked to give this application's ports. Kept as its
/// own constant rather than shared with `keyboard.rs`'s: the two modules
/// have no other reason to depend on each other, and the value is trivial to
/// keep in sync by eye.
const CLIENT: &str = "Hogwash";

/// One MIDI output device, named the way a performer would recognise it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputPort(String);

impl OutputPort {
    /// The device the system calls `name`.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }

    /// What to show the performer.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.0
    }
}

/// No usable MIDI output, in words a performer can be told.
#[derive(Debug)]
pub enum NoOutput {
    /// Nothing is available to send to.
    NoneConnected,
    /// The named device is not among those available any more.
    Gone {
        /// What was asked for.
        output: OutputPort,
    },
    /// The system refused to open it, or the input side of a Thru
    /// connection.
    Unavailable {
        /// What went wrong.
        detail: String,
    },
}

impl NoOutput {
    /// What to show the performer.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::NoneConnected => {
                "No MIDI output device is available. Plug one in and it will appear here."
                    .to_owned()
            }
            Self::Gone { output } => format!("{} is no longer available.", output.name()),
            Self::Unavailable { detail } => {
                format!("The MIDI output could not be opened: {detail}")
            }
        }
    }
}

/// The MIDI output devices `midir` can see right now.
///
/// # Errors
///
/// [`NoOutput::NoneConnected`] if none are available.
pub fn output_ports() -> Result<Vec<OutputPort>, NoOutput> {
    let midi = MidiOutput::new(CLIENT).map_err(|failure| NoOutput::Unavailable {
        detail: failure.to_string(),
    })?;

    let ports: Vec<OutputPort> = midi
        .ports()
        .iter()
        .filter_map(|port| midi.port_name(port).ok())
        .map(OutputPort::new)
        .collect();

    if ports.is_empty() {
        return Err(NoOutput::NoneConnected);
    }

    Ok(ports)
}

/// A live MIDI Thru connection: everything `input` sends is forwarded, byte
/// for byte, to `output`.
///
/// Dropping it disconnects both ends — the output connection lives inside
/// the input callback's own captured state, so there is exactly one thing to
/// hold onto and exactly one thing to drop.
pub struct Thru {
    _input: MidiInputConnection<()>,
}

impl std::fmt::Debug for Thru {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Thru").finish_non_exhaustive()
    }
}

impl Thru {
    /// Opens `input` and `output` by name and starts forwarding between
    /// them, independent of any other connection either device may already
    /// have open.
    ///
    /// # Errors
    ///
    /// [`NoOutput`] if either device could not be found or opened.
    pub fn connect(input: &Instrument, output: &OutputPort) -> Result<Self, NoOutput> {
        let unavailable = |failure: &dyn std::fmt::Display| NoOutput::Unavailable {
            detail: failure.to_string(),
        };

        let output_client = MidiOutput::new(CLIENT).map_err(|failure| unavailable(&failure))?;

        let output_port = output_client
            .ports()
            .into_iter()
            .find(|port| output_client.port_name(port).as_deref() == Ok(output.name()))
            .ok_or_else(|| NoOutput::Gone {
                output: output.clone(),
            })?;

        let connection = output_client
            .connect(&output_port, "hogwash-thru-out")
            .map_err(|failure| unavailable(&failure))?;

        // Shared with the input callback below, which is the only other
        // thing that ever touches it — a `Mutex` rather than anything
        // fancier because contention is impossible (`midir` calls the
        // callback from one thread, serially).
        let sink = Mutex::new(connection);

        let input_client = MidiInput::new(CLIENT).map_err(|failure| unavailable(&failure))?;

        let input_port = input_client
            .ports()
            .into_iter()
            .find(|port| input_client.port_name(port).as_deref() == Ok(input.name()))
            .ok_or_else(|| NoOutput::Unavailable {
                detail: format!("{} is no longer connected", input.name()),
            })?;

        let input_connection = input_client
            .connect(
                &input_port,
                "hogwash-thru-in",
                move |_timestamp, message, ()| {
                    // A poisoned lock or a refused send both mean the
                    // performer's sound has stopped working; neither is
                    // something this callback — with no way to report
                    // anything — can do more than silently drop.
                    if let Ok(mut connection) = sink.lock() {
                        let _ = connection.send(message);
                    }
                },
                (),
            )
            .map_err(|failure| unavailable(&failure))?;

        Ok(Self {
            _input: input_connection,
        })
    }
}
