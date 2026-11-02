//! What a keyboard sends arrives with when it happened.
//!
//! Needs the ALSA sequencer, so it is `#[ignore]`d; run with
//! `cargo test -p segno-midi -- --ignored`. A virtual keyboard plays the
//! Minuet's first two notes with the pedal, at known gaps.

#![cfg(target_os = "linux")]

use std::thread::sleep;
use std::time::{Duration, Instant};

use midir::MidiOutput;
use midir::os::unix::VirtualOutput;
use segno_capture::{Action, Heard, MidiSource};
use segno_domain::Pitch;
use segno_midi::MidiInstruments;

const DEVICE: &str = "Timing Test Keyboard";

#[test]
#[ignore = "needs the ALSA sequencer; run with --ignored"]
fn presses_releases_and_the_pedal_arrive_stamped_in_order() {
    let mut device = MidiOutput::new("segno-test")
        .expect("the sequencer is there")
        .create_virtual(DEVICE)
        .expect("a virtual port can be made");
    let instrument = MidiInstruments
        .instruments()
        .expect("the virtual keyboard is listed")
        .into_iter()
        .find(|instrument| instrument.name().contains(DEVICE))
        .expect("the virtual keyboard is among the instruments");
    let mut keyboard = MidiInstruments
        .open(&instrument)
        .expect("the keyboard opens");

    // D5 struck, pedal down, D5 up, G4 struck, pedal up, G4 up via
    // note-on velocity 0. 200 ms apart.
    let sent: [&[u8]; 6] = [
        &[0x90, 74, 80],
        &[0xB0, 64, 127],
        &[0x80, 74, 0],
        &[0x90, 67, 45],
        &[0xB0, 64, 0],
        &[0x90, 67, 0],
    ];
    for message in sent {
        device.send(message).expect("sends");
        sleep(Duration::from_millis(200));
    }

    let mut heard: Vec<Heard> = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(2);
    while heard.len() < sent.len() && Instant::now() < deadline {
        heard.extend(keyboard.heard());
        sleep(Duration::from_millis(10));
    }

    let d5 = Pitch::new(74).expect("a key");
    let g4 = Pitch::new(67).expect("a key");
    let actions: Vec<Action> = heard.iter().map(|heard| heard.action).collect();
    assert_eq!(
        actions,
        [
            Action::Pressed {
                pitch: d5,
                velocity: 80
            },
            Action::Pedal { down: true },
            Action::Released { pitch: d5 },
            Action::Pressed {
                pitch: g4,
                velocity: 45
            },
            Action::Pedal { down: false },
            Action::Released { pitch: g4 },
        ]
    );

    // Each gap is the 200 ms they were sent apart, within 30 ms.
    for pair in heard.windows(2) {
        let gap = pair[1].at.duration_since(pair[0].at).as_millis();
        assert!((170..=230).contains(&gap), "a gap of {gap} ms");
    }
}
