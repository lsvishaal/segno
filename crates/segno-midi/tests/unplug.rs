//! A keyboard that disappears is noticed, and found again.
//!
//! Needs the ALSA sequencer, so it is `#[ignore]`d like the other tests that
//! need something this machine may not have; run with
//! `cargo test -p segno-midi -- --ignored`. A virtual keyboard stands in for
//! a USB one: when it is dropped its ALSA port disappears the same way.

#![expect(
    clippy::expect_used,
    reason = "a test; clippy's allow-expect-in-tests cannot see a tests/ helper fn"
)]
#![cfg(target_os = "linux")]

use std::thread::sleep;
use std::time::{Duration, Instant};

use midir::MidiOutput;
use midir::MidiOutputConnection;
use midir::os::unix::VirtualOutput;
use segno_capture::{Instrument, MidiSource};
use segno_midi::MidiInstruments;

const DEVICE: &str = "Unplug Test Keyboard";

fn plug_in() -> MidiOutputConnection {
    MidiOutput::new("segno-test")
        .expect("the sequencer is there")
        .create_virtual(DEVICE)
        .expect("a virtual port can be made")
}

fn find() -> Instrument {
    MidiInstruments
        .instruments()
        .expect("the virtual keyboard is listed")
        .into_iter()
        .find(|instrument| instrument.name().contains(DEVICE))
        .expect("the virtual keyboard is among the instruments")
}

#[test]
#[ignore = "needs the ALSA sequencer; run with --ignored"]
fn a_keyboard_that_disappears_is_noticed_and_found_again() {
    let device = plug_in();
    let instrument = find();
    let mut keyboard = MidiInstruments
        .open(&instrument)
        .expect("the keyboard opens");
    assert!(keyboard.is_connected());

    drop(device);

    let deadline = Instant::now() + Duration::from_secs(3);
    while keyboard.is_connected() && Instant::now() < deadline {
        drop(keyboard.heard());
        sleep(Duration::from_millis(50));
    }
    assert!(
        !keyboard.is_connected(),
        "the vanished keyboard was not noticed within 3 s"
    );

    // Back again, possibly under a different client number.
    let _again = plug_in();
    let reopened = MidiInstruments
        .open(&instrument)
        .expect("the keyboard is found again by its name");
    assert!(reopened.is_connected());
}
