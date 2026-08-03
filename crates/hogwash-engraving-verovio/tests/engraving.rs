//! The adapter against a real Verovio.
//!
//! **Every test here is `#[ignore]`d**, and that is deliberate rather than
//! neglectful. `ADR-0005` loads Verovio at run time precisely so that
//! `cargo build` and `just ci` need no C++ toolchain and no shared library;
//! a test that silently passed when the library was absent would quietly
//! stop testing anything the day CI lost it.
//!
//! Ignored means `cargo test` *reports* them as skipped rather than passing
//! them. Run them with:
//!
//! ```text
//! just engraving-test
//! ```
//!
//! which supplies `HOGWASH_VEROVIO_LIB` and `HOGWASH_VEROVIO_DATA`.
//!
//! What is *not* here: anything about how MEI or a timemap is read. That is
//! the adapter's real judgement, it is pure, and it is tested against
//! captured engine output in the crate's own unit tests — with no library
//! needed. These tests cover only what cannot be faked: that the engine is
//! driven correctly.

// clippy.toml allows `expect` in tests, but that setting only reaches code
// clippy can see is a test: functions marked `#[test]`, or anything under
// `#[cfg(test)]`. An integration test file is compiled *without* `cfg(test)`,
// so the helpers below — which are test code by any reading — fall outside
// it. This is that gap, named rather than papered over with a bare `allow`.
#![expect(
    clippy::expect_used,
    reason = "test helpers; clippy's allow-expect-in-tests cannot see a tests/ helper fn"
)]

use std::path::Path;

use hogwash_domain::{Location, MeasureNumber, Step};
use hogwash_engraving::{EngravedScore, Engraver, EngravingFailure};
use hogwash_engraving_verovio::VerovioEngraver;

const TWO_BARS: &[u8] = include_bytes!("fixtures/two-bars.musicxml");

fn engraver() -> VerovioEngraver {
    VerovioEngraver::from_environment()
        .expect("set HOGWASH_VEROVIO_LIB and HOGWASH_VEROVIO_DATA, or run `just engraving-test`")
}

fn engraved() -> EngravedScore {
    engraver().engrave(TWO_BARS).expect("the fixture engraves")
}

fn bar(number: u32) -> MeasureNumber {
    MeasureNumber::new(number).expect("fixture measures are numbered from one")
}

#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn a_musicxml_file_becomes_the_music_it_contains() {
    let engraved = engraved();

    assert_eq!(
        engraved
            .score()
            .measures()
            .iter()
            .map(|measure| (measure.number().get(), measure.positions().len()))
            .collect::<Vec<_>>(),
        [(1, 4), (2, 2)]
    );
}

// ADR-0001 against the engine itself: two spellings, one sound.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn an_enharmonic_pair_survives_the_round_trip() {
    let engraved = engraved();

    let f_sharp = engraved.score().measures()[0].positions()[3].notes()[0];
    let g_flat = engraved.score().measures()[1].positions()[0].notes()[0];

    assert_eq!(f_sharp.spelling().step(), Step::F);
    assert_eq!(f_sharp.spelling().alter(), 1);
    assert_eq!(g_flat.spelling().step(), Step::G);
    assert_eq!(g_flat.spelling().alter(), -1);
    assert_eq!(f_sharp.sounding(), g_flat.sounding());
}

// The mechanism MVP §6.6 rests on: every place in the music points at
// something drawn.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn every_position_points_at_something_that_was_drawn() {
    let engraved = engraved();
    let page = engraved.pages().first().expect("one page");

    for (at, placement) in engraved.placements() {
        for element in placement.elements() {
            assert!(
                page.contains(&format!("id=\"{}\"", element.as_str())),
                "nothing with the identifier for {at:?} appears on the page"
            );
        }
    }
}

// S0 Q3 is the reason `resetXmlIdSeed` is called before every load: unseeded
// identifiers differ between two loads *in one process*. So loading twice and
// comparing is enough to catch a missing seed here; whether they also survive
// a restart is checked across processes by `just engraving-determinism`,
// because that is a thing one process cannot demonstrate.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn the_same_file_engraves_identically_twice_over() {
    let mut engraver = engraver();

    let first = engraver.engrave(TWO_BARS).expect("engraves");
    let second = engraver.engrave(TWO_BARS).expect("engraves again");

    assert_eq!(first.score(), second.score());
    assert_eq!(
        first.placements().collect::<Vec<_>>(),
        second.placements().collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn a_place_in_the_music_resolves_to_the_notes_drawn_there() {
    let engraved = engraved();

    let chord = engraved
        .placement_of(Location::new(bar(2), 5))
        .expect("the closing chord was drawn");

    assert_eq!(chord.elements().len(), 3);
}

// 004 §8.2 — a failure resolves into an explanation, never a crash. A
// performer who has moved or deleted the library must be told which file is
// missing, not shown a stack trace.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn a_missing_engine_is_reported_rather_than_fatal() {
    let refused = VerovioEngraver::open(
        Path::new("/nonexistent/libverovio.so"),
        Path::new("/nonexistent/data"),
    );

    match refused {
        Err(EngravingFailure::EngineUnavailable { detail }) => {
            assert!(
                detail.contains("libverovio"),
                "the message should name the missing file, and says: {detail}"
            );
        }
        other => panic!("a missing library should be reported, and gave {other:?}"),
    }
}

// FR-104 — content that is not music is rejected without producing a partial
// piece. Here the rejection comes from the engine; naming *which* format was
// offered and what to do about it is Acquisition & Validation's job
// (`MVP §6.9`), and arrives at M6.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn a_file_that_is_not_music_is_refused() {
    let refused = engraver().engrave(b"this is not a score");

    assert!(
        matches!(refused, Err(EngravingFailure::Unreadable { .. })),
        "expected a readable refusal, got {refused:?}"
    );
}
