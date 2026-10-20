//! Reading MIDI messages. Pure: bytes in, actions out.
//!
//! Separated from the device so that the one piece of real judgement in this
//! crate can be tested without a keyboard attached. There is not much of it,
//! and all of it is the sort of thing that is wrong in the field rather than
//! in review.

use hogwash_capture::Action;
use hogwash_domain::Pitch;

/// The status nibble for a note-off message.
const NOTE_OFF: u8 = 0x80;
/// The status nibble for a note-on message.
const NOTE_ON: u8 = 0x90;
/// The status nibble for a control change.
const CONTROL: u8 = 0xB0;
/// The controller number of the sustain (damper) pedal.
const SUSTAIN: u8 = 64;

/// What a message says the performer did, if it is something recorded.
///
/// # What counts as a note being played
///
/// A key going down is a note played; a key coming up is a release, kept so
/// that how long a note was held is known. The exception every MIDI
/// implementation meets:
///
/// > **A note-on with velocity zero is a note-off.** The MIDI specification
/// > permits it, and many instruments use it exclusively, because it lets a
/// > run of notes share one status byte. Treating it as a strike would record
/// > every release as a second press — so a performer playing a scale
/// > correctly would appear to have played every note twice, and the cursor
/// > would advance through the piece at double speed.
///
/// The sustain pedal is controller 64: 64 or more is down, less is up, as
/// the MIDI specification has it. A keyboard with half-pedalling sends every
/// value in between; each is reported, and the record keeps only changes.
///
/// Everything else — other controllers, pitch bend, aftertouch, clock,
/// system-exclusive — produces `None`.
///
/// **Channel is ignored.** All sixteen are accepted. A performer's keyboard
/// may transmit on any of them, split keyboards use two, and there is one
/// performer at one instrument, so there is nothing a channel could usefully
/// distinguish.
#[must_use]
pub(crate) fn decode(message: &[u8]) -> Option<Action> {
    let [status, data, value, ..] = message else {
        return None;
    };

    match status & 0xF0 {
        NOTE_ON if *value > 0 => Pitch::new(*data).map(|pitch| Action::Pressed {
            pitch,
            velocity: *value,
        }),
        NOTE_ON | NOTE_OFF => Pitch::new(*data).map(|pitch| Action::Released { pitch }),
        CONTROL if *data == SUSTAIN => Some(Action::Pedal { down: *value >= 64 }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use hogwash_capture::Action;
    use hogwash_domain::Pitch;

    use crate::message::decode;

    fn pressed(note: u8, velocity: u8) -> Action {
        Action::Pressed {
            pitch: Pitch::new(note).unwrap(),
            velocity,
        }
    }

    fn released(note: u8) -> Action {
        Action::Released {
            pitch: Pitch::new(note).unwrap(),
        }
    }

    #[test]
    fn a_key_going_down_is_a_note_with_its_velocity() {
        assert_eq!(decode(&[0x90, 60, 100]), Some(pressed(60, 100)));
        assert_eq!(decode(&[0x90, 60, 1]), Some(pressed(60, 1)));
    }

    #[test]
    fn a_key_coming_up_is_a_release() {
        assert_eq!(decode(&[0x80, 60, 0]), Some(released(60)));
        assert_eq!(decode(&[0x80, 60, 64]), Some(released(60)));
    }

    // The one that would otherwise be found by a performer, not by a test.
    // Many keyboards signal a release as note-on with velocity zero; counted
    // as a strike, every note in a scale would be recorded twice and the
    // cursor would run through the piece at double speed.
    #[test]
    fn a_note_on_with_no_velocity_is_a_release_not_a_strike() {
        assert_eq!(decode(&[0x90, 60, 0]), Some(released(60)));
    }

    // One performer at one instrument: a keyboard transmitting on channel 5
    // must not be silently ignored.
    #[test]
    fn a_note_on_any_channel_is_a_note() {
        for channel in 0..16 {
            assert_eq!(
                decode(&[0x90 | channel, 64, 80]),
                Some(pressed(64, 80)),
                "channel {channel} was not heard"
            );
        }
    }

    #[test]
    fn the_sustain_pedal_is_down_from_64() {
        assert_eq!(decode(&[0xB0, 64, 127]), Some(Action::Pedal { down: true }));
        assert_eq!(decode(&[0xB0, 64, 64]), Some(Action::Pedal { down: true }));
        assert_eq!(decode(&[0xB0, 64, 63]), Some(Action::Pedal { down: false }));
        assert_eq!(decode(&[0xB3, 64, 0]), Some(Action::Pedal { down: false }));
    }

    #[test]
    fn other_traffic_is_not_recorded() {
        for message in [
            [0xB0, 66, 127].as_slice(),    // sostenuto pedal
            [0xB0, 67, 127].as_slice(),    // soft pedal
            [0xE0, 0x00, 0x40].as_slice(), // pitch bend
            [0xA0, 60, 90].as_slice(),     // aftertouch
            [0xF8].as_slice(),             // clock
            [0xC0, 4].as_slice(),          // program change
            [].as_slice(),
        ] {
            assert_eq!(decode(message), None, "{message:?} was recorded");
        }
    }

    // A truncated message is nothing. A device that sends one is
    // malfunctioning, and inventing a pitch from what did arrive would put a
    // note in the record that nobody played.
    #[test]
    fn a_message_cut_short_is_nothing() {
        assert_eq!(decode(&[0x90]), None);
        assert_eq!(decode(&[0x90, 60]), None);
        assert_eq!(decode(&[0xB0, 64]), None);
    }

    // Pitch::new rejects anything above 127, and the byte cannot exceed that
    // once the status bit is clear -- but the check is real rather than
    // assumed, because a malformed message could carry one.
    #[test]
    fn a_note_number_outside_the_midi_range_is_nothing() {
        assert_eq!(decode(&[0x90, 200, 100]), None);
        assert_eq!(decode(&[0x80, 200, 0]), None);
    }
}
