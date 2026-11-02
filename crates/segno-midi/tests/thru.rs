//! MIDI Thru forwards what a keyboard sends, byte for byte, to a synth.
//!
//! Needs the ALSA sequencer, so it is `#[ignore]`d; run with
//! `just midi-test`. A virtual keyboard plays into a virtual synth port
//! through a real `Thru` connection.

#![cfg(target_os = "linux")]

use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::Duration;

use midir::os::unix::{VirtualInput, VirtualOutput};
use midir::{MidiInput, MidiOutput};
use segno_capture::MidiSource;
use segno_midi::{MidiInstruments, Thru, output_ports};

const KEYBOARD: &str = "Thru Test Keyboard";
const SYNTH: &str = "Thru Test Synth";

#[test]
#[ignore = "needs the ALSA sequencer; run with --ignored"]
fn what_the_keyboard_sends_reaches_the_synth_unchanged() {
    let mut keyboard = MidiOutput::new("segno-test")
        .expect("the sequencer is there")
        .create_virtual(KEYBOARD)
        .expect("a virtual keyboard can be made");
    let (heard, arrived) = channel::<Vec<u8>>();
    let _synth = MidiInput::new("segno-test")
        .expect("the sequencer is there")
        .create_virtual(
            SYNTH,
            move |_timestamp, message, ()| drop(heard.send(message.to_vec())),
            (),
        )
        .expect("a virtual synth can be made");

    let instrument = MidiInstruments
        .instruments()
        .expect("instruments are listed")
        .into_iter()
        .find(|instrument| instrument.name().contains(KEYBOARD))
        .expect("the virtual keyboard is an instrument");
    let synth = output_ports()
        .expect("outputs are listed")
        .into_iter()
        .find(|port| port.name().contains(SYNTH))
        .expect("the virtual synth is an output");
    let _thru = Thru::connect(&instrument, &synth).expect("Thru connects");
    // ALSA makes the subscription a moment after `connect` returns.
    sleep(Duration::from_millis(100));

    // D5 struck at velocity 80, the pedal down, D5 let go.
    let sent: [&[u8]; 3] = [&[0x90, 74, 80], &[0xB0, 64, 127], &[0x80, 74, 0]];
    for message in sent {
        keyboard.send(message).expect("sends");
    }

    let received: Vec<Vec<u8>> = (0..sent.len())
        .map(|_| {
            arrived
                .recv_timeout(Duration::from_secs(1))
                .expect("forwarded within a second")
        })
        .collect();
    assert_eq!(received, sent.map(<[u8]>::to_vec));
}
