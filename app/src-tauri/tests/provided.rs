//! The provided material, against a real engraving engine.
//!
//! `#[ignore]`d like the engraving tests, and for the same reason
//!: Verovio is loaded at run time so that `cargo build` and
//! `just ci` need no C++ toolchain, and a test that silently passed without
//! it would stop testing anything. Run with `just provided-test`.
//!
//! # Why this test exists
//!
//! Because the failure it catches is invisible. `Segno::start` skips a
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

use segno_engraving::Engraver;
use segno_engraving_verovio::VerovioEngraver;

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
        .expect("set SEGNO_VEROVIO_LIB and SEGNO_VEROVIO_DATA, or run `just provided-test`")
}

// a first session requires no externally obtained
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
// `Segno::start` skips a piece that will not engrave. That is deliberate —
// one bad file should not stop the application — and it is exactly why this
// counts them. The window could look perfectly reasonable with a piece
// missing.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn starting_puts_every_provided_piece_on_the_shelf() {
    let temporary = std::env::temp_dir().join("segno-provided-test");

    let engraving = segno_app_lib::engraving::Engraving::start().expect("the engine starts");
    let records = segno_records::Records::under(&temporary);
    let practice = segno_app_lib::practice::Practice::start(records.clone(), |_| {})
        .expect("the practice thread starts");

    let segno = segno_app_lib::state::Segno::start(engraving, records, practice)
        .expect("the provided material prepares");

    let titles: Vec<&str> = segno
        .library()
        .pieces()
        .into_iter()
        .map(segno_library::Piece::title)
        .collect();

    assert_eq!(titles.len(), provided().len());
    assert!(titles.contains(&"Ode to Joy"), "got {titles:?}");
    assert!(titles.contains(&"Lightly Row"), "got {titles:?}");
    assert!(
        titles.contains(&"Für Elise (opening, simplified)"),
        "got {titles:?}"
    );
    assert!(titles.contains(&"Minuet in G Major"), "got {titles:?}");
    assert!(
        titles.contains(&"Prelude in C Major, BWV 846"),
        "got {titles:?}"
    );
}

// Für Elise was the first provided piece with a second staff, added to
// exercise the claim in
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

// a title is always available, and the requirement — the shelf is never
// empty. Both are guaranteed in Rust rather than checked in the interface,
// so this is where they are demonstrated.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn every_piece_on_the_shelf_can_be_named_and_played() {
    let temporary = std::env::temp_dir().join("segno-provided-test");

    let engraving = segno_app_lib::engraving::Engraving::start().expect("the engine starts");
    let records = segno_records::Records::under(&temporary);
    let practice = segno_app_lib::practice::Practice::start(records.clone(), |_| {})
        .expect("the practice thread starts");

    let segno = segno_app_lib::state::Segno::start(engraving, records, practice).expect("prepares");

    assert!(segno.library().count() >= 1);

    for piece in segno.library().pieces() {
        assert!(!piece.title().is_empty());
        assert!(
            !piece.reference().is_empty(),
            "{} expects no notes",
            piece.title()
        );
    }
}

// The pairing `commands::open_piece` relies on to build `Mark.notes`: each
// drawn element, zipped against the same position's expected notes in
// order. Checked on the Minuet's first beat, D5 over a G3-B3-D4 chord,
// because a chord is where a misalignment between the two sequences would
// actually show up — a melody's positions are all length one, so a shift
// there would be invisible.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn drawn_elements_pair_with_the_notes_written_at_the_same_position() {
    let mut engraver = engraver();
    let source = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/minuet-in-g.musicxml"),
    )
    .expect("minuet-in-g.musicxml ships with the application");

    let drawn = engraver.engrave(&source).expect("engraves");
    let reference = segno_notation::ReferencePerformance::derived_from(drawn.score());

    let first = drawn
        .placements()
        .find(|(at, _)| at.position() == 0)
        .map(|(_, placement)| placement)
        .expect("the first beat was drawn");

    let expected = reference.at(0).expect("position 0 is within the piece");
    assert_eq!(first.elements().len(), expected.notes().len());

    let mut spelled: Vec<(String, i8)> = first
        .elements()
        .iter()
        .zip(expected.notes())
        .map(|(_, note)| {
            (
                format!("{:?}", note.spelling().step()),
                note.spelling().octave(),
            )
        })
        .collect();
    spelled.sort();

    assert_eq!(
        spelled,
        vec![
            ("B".to_owned(), 3),
            ("D".to_owned(), 4),
            ("D".to_owned(), 5),
            ("G".to_owned(), 3),
        ]
    );
}

/// The engraved Reference Performance of one provided piece.
fn reference_of(file: &str) -> segno_notation::ReferencePerformance {
    let source = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("library")
            .join(file),
    )
    .expect("the piece ships with the application");
    let drawn = engraver().engrave(&source).expect("engraves");

    segno_notation::ReferencePerformance::derived_from(drawn.score())
}

// Playing exactly what each real piece asks for, in order, is correct from
// start to finish: nothing wrong, nothing unexpected, the end reached.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_perfect_performance_of_each_real_piece_reaches_the_end_cleanly() {
    for file in ["minuet-in-g.musicxml", "prelude-in-c.musicxml"] {
        let reference = reference_of(file);

        let mut perfect = segno_capture::ObservedPerformance::new();
        for step in &reference {
            for pitch in step.sounding() {
                perfect.record(pitch);
            }
        }

        let summary = segno_analysis::Analysis::of(&perfect, &reference).summary();

        assert_eq!(summary.incorrect(), 0, "{file}");
        assert_eq!(summary.unexpected(), 0, "{file}");
        assert_eq!(summary.omitted(), 0, "{file}");
        assert!(summary.reached_the_end(), "{file}");
    }
}

// The Minuet is |: 1-16 :| |: 17-32 :|. The cursor follows the performed
// order, so each half is expected twice.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn the_minuets_repeats_are_played_twice() {
    let mut bars: Vec<u32> = reference_of("minuet-in-g.musicxml")
        .iter()
        .map(|step| step.measure().get())
        .collect();
    bars.dedup();

    let half = |range: std::ops::RangeInclusive<u32>| range.collect::<Vec<_>>();
    let expected = [half(1..=16), half(1..=16), half(17..=32), half(17..=32)].concat();

    assert_eq!(bars, expected);
}

// The Prelude's left hand holds its notes with ties. Counted straight from
// the MusicXML, independently of Verovio: every pitched note is required
// once, except those a tie arrives at.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn the_prelude_never_asks_for_a_held_note_again() {
    let text = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/prelude-in-c.musicxml"),
    )
    .expect("ships with the application");

    let notes: Vec<&str> = text
        .split("<note")
        .skip(1)
        .filter_map(|rest| rest.split("</note>").next())
        .filter(|note| note.contains("<pitch>") && !note.contains("<grace"))
        .collect();
    let held = notes
        .iter()
        .filter(|note| note.contains(r#"<tie type="stop""#))
        .count();

    let required: usize = reference_of("prelude-in-c.musicxml")
        .iter()
        .map(|step| step.notes().len())
        .sum();

    assert!(held > 0, "the Prelude has tied notes");
    assert_eq!(required, notes.len() - held);
}
