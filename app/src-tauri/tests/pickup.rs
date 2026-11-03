//! Pieces that open with a pickup (Phase 2.6, DECISIONS 3 Nov), on a real
//! one: Schumann's "Schlusslied des Narren", Op. 127 No. 5, from the CC0
//! `OpenScore` Lieder corpus (`tests/fixtures`, PROVENANCE.md). Its pickup is
//! printed as bar 0 and marked incomplete; it ends on a short bar.

#![expect(
    clippy::expect_used,
    reason = "test helpers; clippy's allow-expect-in-tests cannot see a tests/ helper fn"
)]

use std::path::PathBuf;

use segno_app_lib::commands::accept_offered;
use segno_domain::MeasureNumber;
use segno_engraving::Engraver as _;
use segno_notation::ReferencePerformance;

fn fixture() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/schlusslied-des-narren.musicxml"),
    )
    .expect("the fixture is tracked")
}

fn temporary(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("segno-pickup-test-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn segno(root: &std::path::Path) -> segno_app_lib::state::Segno {
    let engraving = segno_app_lib::engraving::Engraving::start().expect("the engine starts");
    let records = segno_records::Records::under(root);
    let practice = segno_app_lib::practice::Practice::start(records.clone(), |_| {})
        .expect("the practice thread starts");
    segno_app_lib::state::Segno::start(engraving, records, practice)
        .expect("the provided material prepares")
}

/// Every `<measure number="…">` of the first part, as printed.
fn printed_numbers(source: &str) -> Vec<u32> {
    let part = &source[source.find("<part ").expect("a part")..];
    let part = &part[..part.find("</part>").expect("its end")];
    part.match_indices("<measure number=\"")
        .map(|(at, open)| {
            let rest = &part[at + open.len()..];
            rest[..rest.find('"').expect("closed")]
                .parse()
                .expect("a number")
        })
        .collect()
}

// The bars Segno reads are the bars the score prints, the pickup first, as 0.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn the_pickup_is_bar_zero_and_every_bar_keeps_its_printed_number() {
    let source = fixture();
    let drawn = segno_engraving_verovio::VerovioEngraver::from_environment()
        .expect("libverovio")
        .engrave(source.as_bytes())
        .expect("a score with a pickup engraves");

    let numbers: Vec<u32> = drawn
        .score()
        .measures()
        .iter()
        .map(|m| m.number().get())
        .collect();
    assert_eq!(numbers, printed_numbers(&source));
    assert_eq!(numbers.first(), Some(&0));
    assert!(
        drawn
            .score()
            .measures()
            .first()
            .expect("a bar")
            .number()
            .is_pickup()
    );

    // Its first notes are the pickup's.
    let reference = ReferencePerformance::derived_from(drawn.score());
    assert_eq!(
        reference.at(0).expect("a first step").measure(),
        MeasureNumber::PICKUP
    );
}

// The piece imports, and its card counts the pickup as a bar.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_piece_with_a_pickup_imports() {
    let mut segno = segno(&temporary("imports"));
    let imported = accept_offered(
        fixture().as_bytes(),
        Some("schlusslied.musicxml"),
        &mut segno,
    )
    .expect("supported");

    assert_eq!(imported.piece.title, "Schlusslied des Narren");
    assert_eq!(imported.piece.bars, printed_numbers(&fixture()).len());
}

// A lettered bar number is refused in the owner's words (E3). The fixture
// with one number letter-suffixed: no note changed.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_lettered_bar_number_is_refused_plainly() {
    let lettered = fixture().replacen("<measure number=\"12\"", "<measure number=\"12a\"", 1);
    assert_ne!(lettered, fixture());
    let mut segno = segno(&temporary("lettered"));

    let refused = accept_offered(lettered.as_bytes(), None, &mut segno).expect_err("refused");

    assert_eq!(
        refused,
        "This score numbers a bar 12a; Segno can't open scores with lettered bar numbers yet."
    );
}
