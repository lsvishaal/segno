//! The provided material, against a real engraving engine.
//!
//! `#[ignore]`d like the engraving tests, and for the same reason
//! (`ADR-0005`): Verovio is loaded at run time so that `cargo build` and
//! `just ci` need no C++ toolchain, and a test that silently passed without
//! it would stop testing anything. Run with `just provided-test`.
//!
//! # Why this test exists
//!
//! Because the failure it catches is invisible. `Hogwash::start` skips a
//! provided piece that will not engrave, so that one bad file does not stop
//! the application — which is right, and means a piece can quietly vanish
//! from the shelf with nothing but a log line.
//!
//! It caught exactly that. Verovio identifies a format by searching the
//! **first 600 characters** for a root element (`toolkit.cpp`,
//! `IdentifyInputFrom`). *Ode to Joy* carried a long provenance comment, so
//! its `<score-partwise>` sat at offset 751 and the file was rejected as
//! unknown XML. The application started, the shelf looked plausible, and one
//! of three pieces was missing.

#![expect(
    clippy::expect_used,
    reason = "test helpers; clippy's allow-expect-in-tests cannot see a tests/ helper fn"
)]

use std::path::PathBuf;

use hogwash_engraving::Engraver;
use hogwash_engraving_verovio::VerovioEngraver;

/// Every file in `library/`, read from disk rather than compiled in, so the
/// test sees what ships rather than what was compiled last.
fn provided() -> Vec<(String, Vec<u8>)> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library");

    let mut pieces: Vec<(String, Vec<u8>)> = std::fs::read_dir(&directory)
        .expect("the library directory ships with the application")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|kind| kind == "musicxml"))
        .map(|path| {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();

            (
                name,
                std::fs::read(&path).expect("a provided piece is readable"),
            )
        })
        .collect();

    pieces.sort();
    pieces
}

fn engraver() -> VerovioEngraver {
    VerovioEngraver::from_environment()
        .expect("set HOGWASH_VEROVIO_LIB and HOGWASH_VEROVIO_DATA, or run `just provided-test`")
}

// FR-206, FR-207, MVP §6.1 — a first session requires no externally obtained
// file. Every provided piece has to survive the whole journey to a Score, not
// merely exist on disk.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn every_provided_piece_engraves() {
    let mut engraver = engraver();

    for (name, source) in provided() {
        let drawn = engraver
            .engrave(&source)
            .unwrap_or_else(|failure| panic!("{name} did not engrave: {failure:?}"));

        assert!(
            !drawn.score().measures().is_empty(),
            "{name} engraved to no music at all"
        );
    }
}

// The specific trap, pinned so it cannot come back the next time someone
// adds a piece with a generous header. Verovio searches only the first 600
// characters for the root element; a comment longer than that makes the file
// unloadable, and nothing downstream says so.
#[test]
fn the_root_element_is_within_verovio_s_search_window() {
    const WINDOW: usize = 600;

    for (name, source) in provided() {
        let text = String::from_utf8(source).expect("provided pieces are text");

        let root = text
            .find("<score-partwise")
            .unwrap_or_else(|| panic!("{name} has no score-partwise element"));

        assert!(
            root < WINDOW,
            "{name} puts its root element at {root}, past the {WINDOW} characters \
             Verovio searches. It will be rejected as unknown XML, and the piece will \
             vanish from the library without an error. Shorten the header comment; the \
             provenance belongs in PROVENANCE.md."
        );
    }
}

// Every piece has music a performer can actually be asked for. A file that
// parsed but expected nothing would put an unplayable entry on the shelf.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn every_provided_piece_expects_notes() {
    let mut engraver = engraver();

    for (name, source) in provided() {
        let drawn = engraver.engrave(&source).expect("engraves");

        let positions: usize = drawn
            .score()
            .measures()
            .iter()
            .map(|measure| measure.positions().len())
            .sum();

        assert!(positions > 0, "{name} expects no notes");
    }
}

// Stage B's real claim, checked without a window: the application starts, and
// every provided piece reaches the shelf.
//
// `Hogwash::start` skips a piece that will not engrave. That is deliberate —
// one bad file should not stop the application — and it is exactly why this
// counts them. The window could look perfectly reasonable with a piece
// missing.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn starting_puts_every_provided_piece_on_the_shelf() {
    let temporary = std::env::temp_dir().join("hogwash-provided-test");

    let engraving = hogwash_app_lib::engraving::Engraving::start().expect("the engine starts");
    let records = hogwash_records::Records::under(&temporary);
    let practice = hogwash_app_lib::practice::Practice::start(records.clone(), |_| {})
        .expect("the practice thread starts");

    let hogwash = hogwash_app_lib::state::Hogwash::start(engraving, records, practice)
        .expect("the provided material prepares");

    let titles: Vec<&str> = hogwash
        .library()
        .pieces()
        .into_iter()
        .map(hogwash_library::Piece::title)
        .collect();

    assert_eq!(titles.len(), provided().len());
    assert!(titles.contains(&"Ode to Joy"), "got {titles:?}");
    assert!(titles.contains(&"Lightly Row"), "got {titles:?}");
    assert!(titles.contains(&"A Little Chord Study"), "got {titles:?}");
    assert!(
        titles.contains(&"Für Elise (opening, simplified)"),
        "got {titles:?}"
    );
}

// `PROVENANCE.md` says Für Elise is the only provided piece with a second
// staff, and the whole point of adding it was to exercise the claim in
// `translate.rs` that a right-hand and left-hand note sounding together are
// one Position regardless of which staff wrote them. This is that claim,
// checked against the real engine rather than trusted from the doc comment.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_two_hand_piece_merges_notes_that_sound_together() {
    let mut engraver = engraver();

    let source =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/fur-elise.musicxml"))
            .expect("fur-elise.musicxml ships with the application");

    let drawn = engraver.engrave(&source).expect("engraves");

    // Every bar's first and third beats put a right-hand note over a
    // left-hand one starting at the same instant, so each should be a
    // two-note Position rather than two separate ones.
    let first_bar = &drawn.score().measures()[0];
    let chords: usize = first_bar
        .positions()
        .iter()
        .filter(|position| position.notes().len() == 2)
        .count();

    assert_eq!(
        chords,
        2,
        "expected two two-note chords in the first bar (beats one and three), got {:?}",
        first_bar
            .positions()
            .iter()
            .map(|position| position.notes().len())
            .collect::<Vec<_>>()
    );
}

// FR-201 — a title is always available, and FR-207 — the shelf is never
// empty. Both are guaranteed in Rust rather than checked in the interface,
// so this is where they are demonstrated.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn every_piece_on_the_shelf_can_be_named_and_played() {
    let temporary = std::env::temp_dir().join("hogwash-provided-test");

    let engraving = hogwash_app_lib::engraving::Engraving::start().expect("the engine starts");
    let records = hogwash_records::Records::under(&temporary);
    let practice = hogwash_app_lib::practice::Practice::start(records.clone(), |_| {})
        .expect("the practice thread starts");

    let hogwash =
        hogwash_app_lib::state::Hogwash::start(engraving, records, practice).expect("prepares");

    assert!(hogwash.library().count() >= 1);

    for piece in hogwash.library().pieces() {
        assert!(!piece.title().is_empty());
        assert!(
            !piece.reference().is_empty(),
            "{} expects no notes",
            piece.title()
        );
    }
}
