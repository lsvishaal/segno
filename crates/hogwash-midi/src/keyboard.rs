//! `midir` behind the [`MidiSource`] port.

use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use hogwash_capture::{Instrument, Keyboard, MidiSource, NoInstrument};
use hogwash_domain::Pitch;
use midir::{MidiInput, MidiInputConnection};

use crate::message::note_struck;

/// The name `midir` is asked to give this application's ports.
const CLIENT: &str = "Hogwash";

/// The performer's MIDI instruments.
#[derive(Debug)]
pub struct MidiInstruments;

impl MidiSource for MidiInstruments {
    fn instruments(&self) -> Result<Vec<Instrument>, NoInstrument> {
        let midi = MidiInput::new(CLIENT).map_err(|failure| NoInstrument::Unavailable {
            instrument: Instrument::new("MIDI".to_owned()),
            detail: failure.to_string(),
        })?;

        let instruments: Vec<Instrument> = midi
            .ports()
            .iter()
            .filter_map(|port| midi.port_name(port).ok())
            .map(Instrument::new)
            .collect();

        if instruments.is_empty() {
            // Not an empty list. `004 §8.5` wants the absence *said*, and an
            // empty list invites a caller to carry on and discover it later.
            return Err(NoInstrument::NoneConnected);
        }

        Ok(instruments)
    }

    fn open(&mut self, instrument: &Instrument) -> Result<Box<dyn Keyboard>, NoInstrument> {
        let midi = MidiInput::new(CLIENT).map_err(|failure| NoInstrument::Unavailable {
            instrument: instrument.clone(),
            detail: failure.to_string(),
        })?;

        // Matched by name rather than by index. An index is meaningless to a
        // performer and shifts when anything else is plugged in, so a
        // remembered one would eventually open the wrong device.
        let port = midi
            .ports()
            .into_iter()
            .find(|port| midi.port_name(port).as_deref() == Ok(instrument.name()))
            .ok_or_else(|| NoInstrument::Gone {
                instrument: instrument.clone(),
            })?;

        let (notes, played) = channel();

        // `midir` calls this on its own thread, whenever a key moves. It does
        // as little as possible and never blocks: `ADR-0009` keeps every
        // decision on the one thread that drains the other end, so this side
        // only decodes and forwards.
        //
        // A send that fails means the receiver is gone, which means the
        // attempt is over. There is nothing to do about it and nothing to
        // report — dropping the note is correct.
        let connection = midi
            .connect(
                &port,
                "hogwash-practice",
                move |_timestamp, message, ()| forward(message, &notes),
                (),
            )
            .map_err(|failure| NoInstrument::Unavailable {
                instrument: instrument.clone(),
                detail: failure.to_string(),
            })?;

        Ok(Box::new(Connected {
            played,
            connection: Some(connection),
        }))
    }
}

/// Decodes one message and passes on anything that was a note.
///
/// Free-standing so the closure above stays one line, and so the only thing
/// crossing the thread boundary is a `Pitch` — not a byte slice whose
/// lifetime is `midir`'s business.
fn forward(message: &[u8], notes: &Sender<Pitch>) {
    if let Some(pitch) = note_struck(message) {
        // The result is deliberately unexamined: an error means the
        // receiving end has been dropped, which means nobody is practising
        // any more, and there is nothing useful to do about it here.
        let _ = notes.send(pitch);
    }
}

/// A live connection to one instrument.
struct Connected {
    played: Receiver<Pitch>,
    // An `Option` only so that `Drop` can take it. `midir` disconnects when
    // the connection is dropped, so the whole lifecycle is ownership.
    connection: Option<MidiInputConnection<()>>,
}

impl std::fmt::Debug for Connected {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Connected").finish_non_exhaustive()
    }
}

impl Keyboard for Connected {
    fn played(&mut self) -> Vec<Pitch> {
        let mut struck = Vec::new();

        // Drains what is there and returns. Never blocks, never waits: this
        // runs inside a frame, and a frame that waited for a note would stop
        // drawing the score the performer is reading from.
        loop {
            match self.played.try_recv() {
                Ok(pitch) => struck.push(pitch),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // The sending side is gone — the instrument went away.
                    // Whatever was already in the channel has been taken, so
                    // nothing captured is lost (`004 §8.5`).
                    self.connection = None;
                    break;
                }
            }
        }

        struck
    }

    fn is_connected(&self) -> bool {
        self.connection.is_some()
    }
}
