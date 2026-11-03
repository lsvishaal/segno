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
use segno_practice::{Cursor, CursorMovement};

fn reference() -> std::sync::Arc<ReferencePerformance> {
    let drawn = segno_engraving_verovio::VerovioEngraver::from_environment()
        .expect("libverovio")
        .engrave(fixture().as_bytes())
        .expect("engraves");
    std::sync::Arc::new(ReferencePerformance::derived_from(drawn.score()))
}

fn bar(number: u32) -> MeasureNumber {
    MeasureNumber::printed(number)
}

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

    // Export writes the kept file as it is, so the pickup survives the round
    // trip byte for byte.
    let id = segno_domain::PieceId::new(&imported.piece.id).expect("an id");
    assert_eq!(segno.source_of(&id), Some(fixture().into_bytes()));
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

// From the beginning the cursor starts in the pickup and waits there: bar 1's
// notes don't move it until the pickup's are played.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn the_cursor_waits_through_the_pickup() {
    let reference = reference();
    let mut cursor = Cursor::at_the_beginning(reference.clone());
    assert_eq!(cursor.measure(), Some(MeasureNumber::PICKUP));

    let one = reference.first_position_of(bar(1)).expect("bar 1");
    assert!(one > 0, "the pickup has notes");
    let bar_one: Vec<_> = reference.at(one).expect("bar 1").sounding().collect();
    let pickup_first: Vec<_> = reference.at(0).expect("the pickup").sounding().collect();
    for pitch in bar_one.iter().filter(|pitch| !pickup_first.contains(pitch)) {
        assert_eq!(cursor.observe(*pitch), CursorMovement::Held);
    }
    assert_eq!(cursor.measure(), Some(MeasureNumber::PICKUP));

    for step in 0..one {
        for pitch in reference.at(step).expect("a pickup step").sounding() {
            cursor.observe(pitch);
        }
    }
    assert_eq!(
        cursor.measure(),
        Some(bar(1)),
        "through the pickup into bar 1"
    );

    // Played as written from there, it goes on through bar 1 and bar 2.
    let three = reference.first_position_of(bar(3)).expect("bar 3");
    for step in one..three {
        for pitch in reference.at(step).expect("a step").sounding() {
            cursor.observe(pitch);
        }
    }
    assert_eq!(cursor.measure(), Some(bar(3)));
}

// "One bar before" is written order: bar 1's is the pickup, so trouble in bar 1
// is looped from the pickup, and starting from bar 1 skips the pickup.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn bar_one_loops_from_the_pickup_and_starting_from_bar_one_skips_it() {
    let reference = reference();

    assert_eq!(reference.bar_before(bar(1)), Some(MeasureNumber::PICKUP));
    assert_eq!(reference.bar_before(MeasureNumber::PICKUP), None);
    assert_eq!(reference.bar_after(MeasureNumber::PICKUP), Some(bar(1)));
    assert_eq!(
        segno_analysis::coach::practise_range(&reference, bar(1), bar(1)),
        (MeasureNumber::PICKUP, bar(1))
    );
    assert_eq!(
        reference
            .range_of(MeasureNumber::PICKUP, bar(1))
            .map(|r| r.start),
        Some(0)
    );

    let from_one = reference.first_position_of(bar(1)).expect("bar 1");
    let cursor = Cursor::starting_at(reference.clone(), from_one);
    assert_eq!(cursor.measure(), Some(bar(1)));
}

// The owner's Für Elise (test-scores/, not tracked; skipped when absent): its
// pickup is bar 0 and every bar keeps its printed number, written and played.
// Its repeat goes back to the pickup (0, 1–8, 0, 1–7, 9 …): the "pickup
// joined across a repeat" case, parked until it's taken on (DECISIONS, 3 Nov,
// E4), so the played order isn't pinned here.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn fur_elise_keeps_its_printed_numbers() {
    let folder = std::env::var_os("SEGNO_TEST_SCORES").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-scores"),
        PathBuf::from,
    );
    let Ok(source) = std::fs::read(folder.join("Fur_Elise.mxl")) else {
        eprintln!("skipped: Fur_Elise.mxl not found (personal test scores)");
        return;
    };
    let drawn = segno_engraving_verovio::VerovioEngraver::from_environment()
        .expect("libverovio")
        .engrave(&source)
        .expect("engraves");
    let written: Vec<u32> = drawn
        .score()
        .measures()
        .iter()
        .map(|m| m.number().get())
        .collect();
    assert_eq!(written, (0..=105).collect::<Vec<_>>());

    let reference = ReferencePerformance::derived_from(drawn.score());
    assert!(
        reference
            .iter()
            .all(|step| written.contains(&step.measure().get()))
    );
    assert_eq!(
        reference.at(0).map(segno_notation::ExpectedNotes::measure),
        Some(MeasureNumber::PICKUP)
    );
    assert_eq!(reference.bar_before(bar(1)), Some(MeasureNumber::PICKUP));
}
