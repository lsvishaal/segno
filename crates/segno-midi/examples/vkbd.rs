//! A virtual MIDI keyboard, for `just e2e`: drives the real app with no
//! hardware. Linux only (ALSA virtual ports).
//!
//! Reads one command per line on stdin and answers `ok` after each:
//!
//! ```text
//! on <key> [velocity]   off <key>   cc <controller> <value>
//! wait <ms>             close       open          quit
//! ```
//!
//! `close` makes the port vanish the way an unplugged USB keyboard does;
//! `open` plugs it back in (ALSA may give it a new client number).

#[cfg(target_os = "linux")]
fn main() {
    use std::io::{BufRead, Write};
    use std::time::Duration;

    use midir::os::unix::VirtualOutput;
    use midir::{MidiOutput, MidiOutputConnection};

    let open = || -> MidiOutputConnection {
        MidiOutput::new("vkbd")
            .map(|output| output.create_virtual("Segno Test Keyboard"))
            .ok()
            .and_then(Result::ok)
            .unwrap_or_else(|| {
                eprintln!("vkbd: no ALSA sequencer");
                std::process::exit(1)
            })
    };

    let mut port = Some(open());
    let mut stdout = std::io::stdout();
    drop(writeln!(stdout, "ready"));

    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let words: Vec<&str> = line.split_whitespace().collect();
        let number = |index: usize| words.get(index).and_then(|word| word.parse::<u8>().ok());
        let message = match words.first().copied() {
            Some("on") => number(1).map(|key| [0x90, key, number(2).unwrap_or(80)]),
            Some("off") => number(1).map(|key| [0x80, key, 0]),
            Some("cc") => number(1)
                .zip(number(2))
                .map(|(cc, value)| [0xB0, cc, value]),
            Some("wait") => {
                let ms = words.get(1).and_then(|word| word.parse().ok()).unwrap_or(0);
                std::thread::sleep(Duration::from_millis(ms));
                None
            }
            Some("close") => {
                port = None;
                None
            }
            Some("open") => {
                port = Some(open());
                None
            }
            Some("quit") => break,
            _ => None,
        };

        if let (Some(message), Some(connection)) = (message, port.as_mut()) {
            let _ = connection.send(&message);
        }
        drop(writeln!(stdout, "ok"));
        drop(stdout.flush());
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("vkbd needs ALSA virtual ports (Linux)");
}
