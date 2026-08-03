//! Reading MIDI messages. Pure: bytes in, notes out.
//!
//! Separated from the device so that the one piece of real judgement in this
//! crate can be tested without a keyboard attached. There is not much of it,
//! and all of it is the sort of thing that is wrong in the field rather than
//! in review.

use hogwash_domain::Pitch;

/// The status nibble for a note-on message.
const NOTE_ON: u8 = 0x90;

/// The note a message says was struck, if it says one.
///
/// # What counts as a note being played
///
/// `003 §7.5` records what the performer **played**, so a key going down is a
/// note and a key coming up is not. That is nearly the whole rule, and the
/// exception is the one every MIDI implementation meets:
///
/// > **A note-on with velocity zero is a note-off.** The MIDI specification
/// > permits it, and many instruments use it exclusively, because it lets a
/// > run of notes share one status byte. Treating it as a strike would record
/// > every release as a second press — so a performer playing a scale
/// > correctly would appear to have played every note twice, and the cursor
/// > would advance through the piece at double speed.
///
/// Everything else — control changes, pitch bend, aftertouch, clock,
/// system-exclusive — is not a note and produces `None`. Pedal is a control
/// change, and `002 §10` defers pedal analysis, so ignoring it is the
/// specified behaviour rather than an omission.
///
/// **Channel is ignored.** All sixteen are accepted. A performer's keyboard
/// may transmit on any of them, split keyboards use two, and `C-100` scopes
/// Version 1 to one performer at one instrument — so there is nothing a
/// channel could usefully distinguish.
#[must_use]
pub(crate) fn note_struck(message: &[u8]) -> Option<Pitch> {
    let [status, note, velocity, ..] = message else {
        return None;
    };

    if status & 0xF0 != NOTE_ON {
        return None;
    }

    if *velocity == 0 {
        return None;
    }

    Pitch::new(*note)
}

#[cfg(test)]
mod tests {
    use hogwash_domain::Pitch;

    use crate::message::note_struck;

    #[test]
    fn a_key_going_down_is_a_note() {
        assert_eq!(note_struck(&[0x90, 60, 100]), Pitch::new(60));
    }

    #[test]
    fn a_key_coming_up_is_not_a_note() {
        assert_eq!(note_struck(&[0x80, 60, 0]), None);
    }

    // The one that would otherwise be found by a performer, not by a test.
    // Many keyboards signal a release as note-on with velocity zero; counted
    // as a strike, every note in a scale would be recorded twice and the
    // cursor would run through the piece at double speed.
    #[test]
    fn a_note_on_with_no_velocity_is_a_release_not_a_strike() {
        assert_eq!(note_struck(&[0x90, 60, 0]), None);
    }

    // C-100 scopes Version 1 to one performer at one instrument, so there is
    // nothing a channel could usefully distinguish -- and a keyboard
    // transmitting on channel 5 must not be silently ignored.
    #[test]
    fn a_note_on_any_channel_is_a_note() {
        for channel in 0..16 {
            assert_eq!(
                note_struck(&[0x90 | channel, 64, 80]),
                Pitch::new(64),
                "channel {channel} was not heard"
            );
        }
    }

    // 002 §10 defers pedal analysis, and pedal is a control change. Ignoring
    // it is the specified behaviour rather than something left undone.
    #[test]
    fn a_pedal_press_is_not_a_note() {
        assert_eq!(note_struck(&[0xB0, 64, 127]), None);
    }

    #[test]
    fn other_traffic_is_not_a_note() {
        for message in [
            [0xE0, 0x00, 0x40].as_slice(), // pitch bend
            [0xA0, 60, 90].as_slice(),     // aftertouch
            [0xF8].as_slice(),             // clock
            [0xC0, 4].as_slice(),          // program change
            [].as_slice(),
        ] {
            assert_eq!(
                note_struck(message),
                None,
                "{message:?} was taken for a note"
            );
        }
    }

    // A truncated message is not a note. A device that sends one is
    // malfunctioning, and inventing a pitch from what did arrive would put a
    // note in the record that nobody played.
    #[test]
    fn a_message_cut_short_is_not_a_note() {
        assert_eq!(note_struck(&[0x90]), None);
        assert_eq!(note_struck(&[0x90, 60]), None);
    }

    // Pitch::new rejects anything above 127, and the byte cannot exceed that
    // once the status bit is clear -- but the check is real rather than
    // assumed, because a malformed message could carry one.
    #[test]
    fn a_note_number_outside_the_midi_range_is_not_a_note() {
        assert_eq!(note_struck(&[0x90, 200, 100]), None);
    }
}
