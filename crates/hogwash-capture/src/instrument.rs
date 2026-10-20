//! The port between the performer's keyboard and the notes it produces.
//!
//! `004 §8.5` says Performance Capture *"requires live input from a connected
//! MIDI instrument"* and, in the same breath, that it *"knows nothing about
//! what was expected"*. Both are honoured here by declaring what an
//! instrument must provide and implementing none of it: the hardware lives in
//! an adapter (`hogwash-midi`), and every test of practice, analysis and
//! feedback runs with no keyboard attached.
//!
//! # Why the notes are pulled rather than pushed
//!
//! A MIDI library delivers on its own thread, whenever a key moves. A port
//! shaped that way — one that took a callback — would put threading into
//! every pure crate that touched it, and would make "what did the performer
//! play?" a question whose answer depends on when it is asked.
//!
//! So the port is a **pull**: whoever is driving asks what has arrived since
//! last time. The adapter does the thread-crossing behind it, and the domain
//! stays single-threaded with no locks (`ADR-0009`).

use std::time::Instant;

use hogwash_domain::Pitch;

/// A MIDI instrument the performer could play.
///
/// Named rather than numbered. A port index is meaningless to a performer and
/// changes when devices are plugged in; "Digital Piano" is what they are
/// looking for in a list.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instrument {
    name: String,
}

impl Instrument {
    /// The instrument the system calls `name`.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self { name }
    }

    /// What to show the performer.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Why no notes can be captured.
///
/// `004 §8.5` requires a disconnected or malfunctioning instrument not to
/// corrupt what was already captured, and requires capture to stop
/// **visibly**. Visibly is the hard part: a performer whose keyboard has
/// quietly stopped reporting sees a cursor that will not move and concludes
/// they are playing wrong notes. So there is no silent failure here — every
/// case has something to say.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NoInstrument {
    /// Nothing is plugged in.
    NoneConnected,

    /// The named instrument is not among those available.
    ///
    /// Ordinary rather than exceptional: a device is unplugged between being
    /// listed and being chosen more often than one might expect.
    Gone {
        /// What was asked for.
        instrument: Instrument,
    },

    /// The system refused to open it.
    Unavailable {
        /// What was asked for.
        instrument: Instrument,
        /// What went wrong, for the log.
        detail: String,
    },
}

impl NoInstrument {
    /// The obstacle in words, for the performer (NFR-800).
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::NoneConnected => {
                "No MIDI keyboard is connected. Plug one in and it will appear here.".to_owned()
            }
            Self::Gone { instrument } => format!("{} is no longer connected.", instrument.name()),
            Self::Unavailable { instrument, detail } => {
                format!("{} could not be opened: {detail}", instrument.name())
            }
        }
    }
}

/// Something the performer did at the instrument, and when.
///
/// `at` is an [`Instant`] the adapter took as the message arrived. Carrying
/// one reads no clock; whoever receives it measures it against when the
/// Attempt began.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heard {
    /// When it arrived.
    pub at: Instant,
    /// What it was.
    pub action: Action,
}

/// What a performer can do at an instrument that is worth recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// A key went down: a note played.
    Pressed {
        /// Which key.
        pitch: Pitch,
        /// How hard, 1–127.
        velocity: u8,
    },
    /// A key came up.
    Released {
        /// Which key.
        pitch: Pitch,
    },
    /// The sustain pedal went down, or came up.
    Pedal {
        /// Whether it is now down.
        down: bool,
    },
}

/// A live connection to an instrument.
///
/// Dropping it disconnects. There is no `close`: an attempt ends by the
/// connection going out of scope, which is one fewer state to get wrong.
pub trait Keyboard {
    /// Everything heard since this was last called, oldest first.
    ///
    /// Only a [`Action::Pressed`] is a note played; releases and the pedal
    /// are recorded beside the notes, never compared with the Score.
    ///
    /// Empty is the ordinary answer — most of the time, nothing has happened
    /// since the last frame.
    fn heard(&mut self) -> Vec<Heard>;

    /// Whether the instrument is still there.
    ///
    /// `004 §8.5`: capture stops **visibly**. This is what makes that
    /// possible, and the reason it is on the connection rather than reported
    /// once at the start.
    fn is_connected(&self) -> bool;
}

/// Somewhere instruments can be found and opened.
pub trait MidiSource {
    /// The instruments currently available, in a stable order.
    ///
    /// # Errors
    ///
    /// [`NoInstrument::NoneConnected`] if there are none, rather than an
    /// empty list — an empty list invites a caller to carry on and find out
    /// later, and `004 §8.5` wants the absence said.
    fn instruments(&self) -> Result<Vec<Instrument>, NoInstrument>;

    /// Opens `instrument` and begins capturing.
    ///
    /// # Errors
    ///
    /// [`NoInstrument`] if it has gone, or cannot be opened.
    fn open(&mut self, instrument: &Instrument) -> Result<Box<dyn Keyboard>, NoInstrument>;
}

#[cfg(test)]
mod tests {
    use crate::instrument::{Instrument, NoInstrument};

    #[test]
    fn an_instrument_is_known_by_the_name_a_performer_would_recognise() {
        let instrument = Instrument::new("Digital Piano".to_owned());

        assert_eq!(instrument.name(), "Digital Piano");
    }

    // 004 §8.5 — capture stops *visibly*. A performer whose keyboard has
    // quietly stopped reporting sees a cursor that will not move and
    // concludes they are playing wrong notes, which is the worst available
    // outcome. Every way of having no instrument therefore has something to
    // say.
    #[test]
    fn every_way_of_having_no_instrument_can_be_explained() {
        let instrument = Instrument::new("Digital Piano".to_owned());

        for obstacle in [
            NoInstrument::NoneConnected,
            NoInstrument::Gone {
                instrument: instrument.clone(),
            },
            NoInstrument::Unavailable {
                instrument,
                detail: "in use by another program".to_owned(),
            },
        ] {
            assert!(!obstacle.describe().is_empty(), "{obstacle:?} says nothing");
        }
    }

    #[test]
    fn a_disconnection_names_the_instrument_that_went() {
        let gone = NoInstrument::Gone {
            instrument: Instrument::new("Digital Piano".to_owned()),
        };

        assert!(gone.describe().contains("Digital Piano"));
    }
}
