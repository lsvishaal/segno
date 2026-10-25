//! `midir` behind the [`MidiSource`] port.

use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::time::{Duration, Instant};

use hogwash_capture::{Heard, Instrument, Keyboard, MidiSource, NoInstrument};
use midir::{MidiInput, MidiInputConnection};

use crate::message::decode;

/// The name `midir` is asked to give this application's ports.
const CLIENT: &str = "Hogwash";

/// How often a connected instrument is checked for still being there.
///
/// `midir` never reports a device that vanishes: its callback just goes
/// quiet. So the port list is asked instead, and not on every tick.
const WATCH_EVERY: Duration = Duration::from_millis(500);

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
            .filter(|name| !is_loopback(name))
            .map(Instrument::new)
            .collect();

        if instruments.is_empty() {
            // Not an empty list. The design wants the absence *said*, and an
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

        // A second, idle input: connecting consumes `midi`, and the port list
        // is how a vanished device is noticed afterwards.
        let watcher = MidiInput::new(CLIENT).map_err(|failure| NoInstrument::Unavailable {
            instrument: instrument.clone(),
            detail: failure.to_string(),
        })?;

        // Matched by name rather than by index. An index is meaningless to a
        // performer and shifts when anything else is plugged in, so a
        // remembered one would eventually open the wrong device. A device
        // that comes back may have a new ALSA client number in its name, so
        // failing an exact match the name without those numbers is tried.
        let ports = midi.ports();
        let wanted = instrument.name();
        let exact = ports
            .iter()
            .find(|port| midi.port_name(port).as_deref() == Ok(wanted));
        let renumbered = || {
            ports.iter().find(|port| {
                midi.port_name(port)
                    .is_ok_and(|name| without_ids(&name) == without_ids(wanted))
            })
        };
        let port = exact
            .or_else(renumbered)
            .cloned()
            .ok_or_else(|| NoInstrument::Gone {
                instrument: instrument.clone(),
            })?;
        let name = midi
            .port_name(&port)
            .unwrap_or_else(|_| instrument.name().to_owned());

        let (notes, heard) = channel();

        // `midir` calls this on its own thread, whenever a key moves. It does
        // as little as possible and never blocks: the design keeps every
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
            heard,
            connection: Some(connection),
            watcher,
            name,
            checked: Instant::now(),
        }))
    }
}

/// Decodes one message and passes on anything worth recording, stamped with
/// when it arrived.
///
/// Free-standing so the closure above stays one line, and so the only thing
/// crossing the thread boundary is a `Heard` — not a byte slice whose
/// lifetime is `midir`'s business.
///
/// The stamp is taken here rather than read from `midir`'s timestamp, which
/// on ALSA counts from each connection and so starts again at zero when a
/// keyboard is plugged back in mid-attempt. An `Instant` taken on arrival is
/// as precise and runs on across a reconnect.
fn forward(message: &[u8], notes: &Sender<Heard>) {
    if let Some(action) = decode(message) {
        let heard = Heard {
            at: Instant::now(),
            action,
        };
        // The result is deliberately unexamined: an error means the
        // receiving end has been dropped, which means nobody is practising
        // any more, and there is nothing useful to do about it here.
        let _ = notes.send(heard);
    }
}

/// A live connection to one instrument.
struct Connected {
    heard: Receiver<Heard>,
    // An `Option` only so that `Drop` can take it. `midir` disconnects when
    // the connection is dropped, so the whole lifecycle is ownership.
    connection: Option<MidiInputConnection<()>>,
    /// Lists ports, to notice the instrument disappearing.
    watcher: MidiInput,
    /// The port's name as it was opened.
    name: String,
    checked: Instant,
}

impl Connected {
    fn still_listed(&self) -> bool {
        self.watcher
            .ports()
            .iter()
            .any(|port| self.watcher.port_name(port).as_deref() == Ok(self.name.as_str()))
    }
}

/// ALSA's built-in loopback port, present on every Linux machine and never a
/// keyboard. Listed first, it would be the default choice.
fn is_loopback(name: &str) -> bool {
    name.starts_with("Midi Through")
}

/// A port name without the `client:port` numbers ALSA appends, e.g.
/// `Digital Piano:Digital Piano MIDI 1 20:0` becomes
/// `Digital Piano:Digital Piano MIDI 1`. Other names are returned unchanged.
fn without_ids(name: &str) -> &str {
    let is_ids = |text: &str| {
        text.split_once(':').is_some_and(|(client, port)| {
            !client.is_empty()
                && !port.is_empty()
                && client.bytes().all(|byte| byte.is_ascii_digit())
                && port.bytes().all(|byte| byte.is_ascii_digit())
        })
    };

    match name.trim_end().rsplit_once(' ') {
        Some((rest, ids)) if is_ids(ids) => rest,
        _ => name.trim_end(),
    }
}

impl std::fmt::Debug for Connected {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Connected").finish_non_exhaustive()
    }
}

impl Keyboard for Connected {
    fn heard(&mut self) -> Vec<Heard> {
        let mut heard = Vec::new();

        // Drains what is there and returns. Never blocks, never waits: this
        // runs inside a frame, and a frame that waited for a note would stop
        // drawing the score the performer is reading from.
        loop {
            match self.heard.try_recv() {
                Ok(event) => heard.push(event),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // The sending side is gone — the instrument went away.
                    // Whatever was already in the channel has been taken, so
                    // nothing captured is lost.
                    self.connection = None;
                    break;
                }
            }
        }

        // Quiet is not the same as gone: ask whether the port is still there.
        if self.connection.is_some() && self.checked.elapsed() >= WATCH_EVERY {
            self.checked = Instant::now();
            if !self.still_listed() {
                self.connection = None;
            }
        }

        heard
    }

    fn is_connected(&self) -> bool {
        self.connection.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::{is_loopback, without_ids};

    #[test]
    fn only_alsas_loopback_is_left_out_of_the_keyboards() {
        assert!(is_loopback("Midi Through:Midi Through Port-0 14:0"));
        assert!(!is_loopback("Digital Piano:Digital Piano MIDI 1 20:0"));
        assert!(!is_loopback("Through Keyboard"));
    }

    #[test]
    fn alsa_ids_are_dropped_from_the_end_of_a_name() {
        assert_eq!(
            without_ids("Digital Piano:Digital Piano MIDI 1 20:0"),
            "Digital Piano:Digital Piano MIDI 1"
        );
        assert_eq!(
            without_ids("vkbd:Test Keyboard 128:0"),
            "vkbd:Test Keyboard"
        );
        assert_eq!(
            without_ids("Midi Through:Midi Through Port-0 14:0"),
            "Midi Through:Midi Through Port-0"
        );
    }

    #[test]
    fn a_name_without_ids_is_unchanged() {
        assert_eq!(without_ids("Digital Piano"), "Digital Piano");
        assert_eq!(without_ids("Piano 2"), "Piano 2");
        assert_eq!(without_ids("Keyboard 1:"), "Keyboard 1:");
        assert_eq!(without_ids(""), "");
    }

    #[test]
    fn a_renumbered_device_matches_itself_but_not_another() {
        let before = "Digital Piano:Digital Piano MIDI 1 20:0";
        let after = "Digital Piano:Digital Piano MIDI 1 24:0";
        let other = "Practice Pad:Practice Pad MIDI 1 24:0";

        assert_eq!(without_ids(before), without_ids(after));
        assert_ne!(without_ids(before), without_ids(other));
    }
}
