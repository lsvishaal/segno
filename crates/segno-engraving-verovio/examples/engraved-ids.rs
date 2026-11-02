//! Prints where each note in a fixture score was drawn.
//!
//! An example rather than a binary, so it is not part of what the crate
//! ships. `scripts/check-engraving-determinism.sh` runs it **twice in
//! separate processes** and diffs the bytes.
//!
//! That separation is the whole point. The Verovio spike measured that its
//! element identifiers are random per render, and that seeding the generator
//! before loading makes them reproducible — including across restarts. The
//! requirement is exactly that, and the design makes it release-blocking. A comparison made inside one process cannot show it.

use segno_engraving::Engraver;
use segno_engraving_verovio::VerovioEngraver;

const TWO_BARS: &[u8] = include_bytes!("../tests/fixtures/two-bars.musicxml");

fn main() -> std::process::ExitCode {
    let mut engraver = match VerovioEngraver::from_environment() {
        Ok(engraver) => engraver,
        Err(failure) => {
            eprintln!("engraved-ids: {failure:?}");
            return std::process::ExitCode::from(2);
        }
    };

    let drawn = match engraver.engrave(TWO_BARS) {
        Ok(score) => score,
        Err(failure) => {
            eprintln!("engraved-ids: {failure:?}");
            return std::process::ExitCode::from(3);
        }
    };

    println!("pages: {}", drawn.pages().len());

    for (at, placement) in drawn.placements() {
        let elements: Vec<&str> = placement
            .elements()
            .iter()
            .map(segno_engraving::ElementId::as_str)
            .collect();

        println!(
            "bar {:<3} position {:<3} page {:<3} {}",
            at.measure().get(),
            at.position(),
            placement.page().get(),
            elements.join(" ")
        );
    }

    std::process::ExitCode::SUCCESS
}
