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
use hogwash_engraving_verovio::{Layout, VerovioEngraver};
use hogwash_notation::{Position, ReferencePerformance};

const TWO_BARS: &[u8] = include_bytes!("fixtures/two-bars.musicxml");

/// The two-bar fixture, repeated until it is longer than a sheet of paper.
///
/// Built here rather than checked in: the point is only "longer than a page",
/// and a fixture file would fix a length that has no meaning of its own.
///
/// 200 copies — 400 bars — because these are four-note bars that pack several
/// to a system. Sixty bars came to 2684px against A4's 2970, which is to say
/// they still fitted, and a test that cannot tell the two layouts apart is
/// not testing the layout.
fn many_bars() -> String {
    let source = std::str::from_utf8(TWO_BARS).expect("the fixture is text");
    let (head, rest) = source.split_once("<measure").expect("the fixture has bars");
    let (bars, tail) = rest
        .rsplit_once("</measure>")
        .expect("the fixture has bars");
    let bars = format!("<measure{bars}</measure>");

    let mut repeated = String::from(head);
    for copy in 0..200 {
        // Bar numbers must ascend across the whole score, so each copy is
        // renumbered rather than repeated verbatim.
        repeated.push_str(
            &bars
                .replace("number=\"1\"", &format!("number=\"{}\"", copy * 2 + 1))
                .replace("number=\"2\"", &format!("number=\"{}\"", copy * 2 + 2)),
        );
    }
    repeated.push_str(tail);
    repeated
}

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

// Zoom (`BFR-4`) re-engraves the same score at a different `Layout.scale`
// while an Attempt may be in progress. That is only safe if an element's
// identifier depends on the music's content, not on the size it was drawn
// at — otherwise the frontend's `Mark.notes[].elementId`s would go stale the
// instant a performer zoomed mid-attempt. This is that assumption, checked
// rather than assumed: if it ever fails, the design still holds (the
// frontend always replaces `marks` wholesale on a rescale, never patches
// them), but nobody should be surprised by it happening.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn rescaling_keeps_the_same_element_ids_but_changes_geometry() {
    let mut engraver = engraver();

    engraver.lay_out(Layout {
        scale: 100,
        ..Layout::default()
    });
    let small = engraver.engrave(TWO_BARS).expect("engraves at 100%");

    engraver.lay_out(Layout {
        scale: 200,
        ..Layout::default()
    });
    let large = engraver.engrave(TWO_BARS).expect("engraves at 200%");

    // Same music either way — scale is not content.
    assert_eq!(small.score(), large.score());

    let ids_of = |engraved: &EngravedScore| -> Vec<_> {
        engraved
            .placements()
            .map(|(_, placement)| placement.elements().to_vec())
            .collect()
    };

    assert_eq!(
        ids_of(&small),
        ids_of(&large),
        "element ids must not depend on scale"
    );

    // And the geometry actually did change — otherwise the assertion above
    // would be vacuously true rather than meaningful.
    assert_ne!(
        height_of(&small.pages()[0]),
        height_of(&large.pages()[0]),
        "200% should not render the same page height as 100%"
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

/// The pixel height a rendered page declares.
///
/// Read from `viewBox` rather than a `height="...px"` attribute: `Layout`
/// asks for `svgViewBox: true` (so the score scales responsively in the DOM
/// — `.hw-page svg { width: 100%; height: auto }`), and Verovio answers with
/// a `viewBox` instead of fixed pixel dimensions. This test's own parsing
/// silently stopped matching anything when that option was added; found
/// while re-checking these tests rather than assumed still correct.
fn height_of(svg: &str) -> u32 {
    let view_box = svg
        .split_once("viewBox=\"")
        .map(|(_, rest)| {
            rest.split_once('"')
                .expect("viewBox is a quoted attribute")
                .0
        })
        .expect("a page states a viewBox");

    view_box
        .split_whitespace()
        .nth(3)
        .and_then(|number| number.parse::<u32>().ok())
        .expect("a viewBox has four numbers: min-x, min-y, width, height")
}

// ADR-0011 / MVP §6.4 — a practice score is scrolled, not turned, so a short
// exercise should come back as a short strip rather than a mostly-empty sheet
// of A4, which at this scale is a little over 3200px.
//
// Removing `adjustPageHeight` makes this fail, which was checked rather than
// assumed.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn a_short_piece_is_a_short_page_rather_than_a_mostly_empty_sheet() {
    let engraved = engraved();
    let height = height_of(&engraved.pages()[0]);

    assert_eq!(engraved.pages().len(), 1);
    assert!(
        height < 1500,
        "two bars should not occupy a sheet of paper: {height}px"
    );
}

// The other half, and the one that stops the first being achieved by simply
// cutting the music off: a long score keeps **every** position placed, across
// however many pages it takes.
//
// Pages are real, and that is the correction this test records.
// `adjustPageHeight` trims a page to its content; it does not abolish
// pagination. A 400-bar score comes back as six pages, so the view stacks
// pages rather than assuming one — asking instead for a single enormous page
// would only have traded a seam for an enormous texture.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn a_long_score_keeps_every_position_placed_across_its_pages() {
    let engraved = engraver()
        .engrave(many_bars().as_bytes())
        .expect("the longer piece engraves");

    let positions: usize = engraved
        .score()
        .measures()
        .iter()
        .map(|measure| measure.positions().len())
        .sum();

    assert!(
        engraved.pages().len() > 1,
        "the fixture is meant to need several pages"
    );
    assert_eq!(engraved.placements().count(), positions);
    assert!(
        engraved
            .placements()
            .all(|(_, placement)| placement.page().get() < engraved.pages().len())
    );
}

/// A small grand-staff score with the notation real pieces use: a tie, a
/// repeat, a grace note and a trill. Bar 1 (inside the repeat): a treble C5
/// half note tied into a C5 quarter, then D5, over a bass C3 and E3.
const REAL_NOTATION: &[u8] = include_bytes!("fixtures/real-notation.musicxml");

/// The pitches each step of the Reference Performance requires, in order.
fn required(engraved: &EngravedScore) -> Vec<Vec<u8>> {
    hogwash_notation::ReferencePerformance::derived_from(engraved.score())
        .iter()
        .map(|step| step.sounding().map(u8::from).collect())
        .collect()
}

// Verovio's timemap lists the tied-to C5 as sounding again at beat 3. It is
// held, not struck, so the cursor must not wait for it: beat 3 asks only for
// the bass E3.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn a_tied_note_is_required_once_against_a_real_engine() {
    let engraved = engraver()
        .engrave(REAL_NOTATION)
        .expect("the fixture engraves");

    assert_eq!(
        required(&engraved).get(..3),
        Some(&[vec![72, 48], vec![52], vec![74]][..])
    );
}

// The fixture's bars 1-2 sit inside repeat signs. Verovio writes the repeat
// out for its timemap under copied identifiers; the cursor must follow the
// performed order: 1 2 1 2 3.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn a_repeat_is_played_twice_against_a_real_engine() {
    let engraved = engraver()
        .engrave(REAL_NOTATION)
        .expect("the fixture engraves");

    let mut bars: Vec<u32> = hogwash_notation::ReferencePerformance::derived_from(engraved.score())
        .iter()
        .map(|step| step.measure().get())
        .collect();
    bars.dedup();

    assert_eq!(bars, [1, 2, 1, 2, 3]);
    assert_eq!(
        engraved.placements().count(),
        hogwash_notation::ReferencePerformance::derived_from(engraved.score()).len(),
        "every performed step is placed on the page"
    );
}

// Bar 2 opens with a grace D5 before E5 (over a bass G2), then a trilled F5.
// The grace note is never required, and is tolerated where the performer
// plays it; the trill's neighbours are tolerated at F5 and the step after.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn grace_notes_and_trills_are_tolerated_against_a_real_engine() {
    let engraved = engraver()
        .engrave(REAL_NOTATION)
        .expect("the fixture engraves");
    let reference = hogwash_notation::ReferencePerformance::derived_from(engraved.score());
    let pitch = |number| hogwash_domain::Pitch::new(number).expect("in range");

    assert_eq!(
        required(&engraved).get(3..7),
        Some(&[vec![43], vec![76], vec![77], vec![79]][..]),
        "bar 2 without its grace note"
    );

    let (g2, e5, f5, g5) = (
        reference.at(3).expect("G2"),
        reference.at(4).expect("E5"),
        reference.at(5).expect("F5"),
        reference.at(6).expect("G5"),
    );
    assert!(g2.tolerates(pitch(74)) && e5.tolerates(pitch(74)));
    assert!(f5.tolerates(pitch(78)) && g5.tolerates(pitch(78)) && g5.tolerates(pitch(77)));
    assert!(!f5.tolerates(pitch(74)));
}

/// The Minuet, as bundled, with an 8va line added over bars 1-2 of the right
/// hand: drawn an octave lower than it sounds.
fn minuet_under_an_octave_line() -> String {
    let source = include_str!("../../../app/src-tauri/library/minuet-in-g.musicxml");
    let shift = |kind: &str| {
        format!(
            r#"<direction><direction-type><octave-shift type="{kind}" size="8" number="1"/></direction-type><staff>1</staff></direction>"#
        )
    };
    let before_first_note_of = |text: &str, bar: &str| {
        let start = text
            .find(&format!(r#"<measure number="{bar}""#))
            .expect("the bar");
        start + text[start..].find("<note").expect("a note in it")
    };

    let mut edited = source.to_owned();
    let stop = before_first_note_of(&edited, "3");
    edited.insert_str(stop, &shift("stop"));
    let start = before_first_note_of(&edited, "1");
    edited.insert_str(start, &shift("down"));
    edited
}

// Under the 8va the first note is drawn as D4 but sounds as D5, and D5 is
// the key the performer has to press.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn a_note_under_an_octave_line_is_expected_at_the_pitch_it_sounds() {
    let engraved = engraver()
        .engrave(minuet_under_an_octave_line().as_bytes())
        .expect("the edited Minuet engraves");

    assert_eq!(required(&engraved).first(), Some(&vec![74, 55, 59, 62]));
}

// Zoom reflows: a page fitted to a narrower window (or a bigger zoom) breaks
// the same music into more lines, and every note keeps its id.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn a_narrower_window_reflows_into_more_lines() {
    let minuet = include_bytes!("../../../app/src-tauri/library/minuet-in-g.musicxml");
    let mut engraver = engraver();
    let systems = |engraved: &EngravedScore| -> usize {
        engraved
            .pages()
            .iter()
            .map(|page| page.matches(r#"class="system""#).count())
            .sum()
    };
    let ids = |engraved: &EngravedScore| -> Vec<_> {
        engraved
            .placements()
            .map(|(_, placement)| placement.elements().to_vec())
            .collect()
    };

    engraver.lay_out(Layout::fitting(1600, 100));
    let wide = engraver.engrave(minuet).expect("engraves wide");
    engraver.lay_out(Layout::fitting(1600, 200));
    let zoomed = engraver.engrave(minuet).expect("engraves zoomed in");

    assert!(systems(&zoomed) > systems(&wide));
    assert_eq!(ids(&zoomed), ids(&wide));
}

// The Minuet's first half (bars 1-16) is repeated. Starting at bar 3 means
// its first occurrence: the step after bars 1-2 on the first pass.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn a_bar_inside_a_repeat_starts_at_its_first_occurrence_in_the_minuet() {
    let minuet = include_bytes!("../../../app/src-tauri/library/minuet-in-g.musicxml");
    let engraved = engraver().engrave(minuet).expect("the Minuet engraves");
    let reference = hogwash_notation::ReferencePerformance::derived_from(engraved.score());

    let occurrences: Vec<usize> = reference
        .iter()
        .enumerate()
        .filter(|(index, step)| {
            step.measure() == bar(3)
                && reference
                    .at(index.wrapping_sub(1))
                    .is_none_or(|before| before.measure() != bar(3))
        })
        .map(|(index, _)| index)
        .collect();

    assert_eq!(occurrences.len(), 2, "bar 3 is played twice");
    assert_eq!(reference.first_position_of(bar(3)), Some(occurrences[0]));
}

// Satie's Gymnopédie No. 1, bar 10: its opening note is tied over from bar 9.
// Played through, it is not struck again; starting at bar 10, it must be.
// (One of the repository's test scores; see ROADMAP M0.2.)
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn a_note_tied_into_a_bar_is_required_when_starting_there_in_the_gymnopedie() {
    let satie = include_bytes!("../../../app/src-tauri/library/Erik_Satie_-_Gymnopedie_No.1.mxl");
    let engraved = engraver().engrave(satie).expect("the Gymnopédie engraves");
    let reference = hogwash_notation::ReferencePerformance::derived_from(engraved.score());

    let start = reference
        .first_position_of(bar(10))
        .expect("bar 10 has notes");
    let step = reference.at(start).expect("a step");
    let struck: Vec<u8> = step.sounding().map(u8::from).collect();
    let started: Vec<u8> = step.starting_here().sounding().map(u8::from).collect();

    assert!(
        !step.held_into().is_empty(),
        "bar 10 opens with a held note"
    );
    assert!(started.len() > struck.len(), "{struck:?} -> {started:?}");
}

// A click on the page lands in a `g.measure` element; each one drawn must be
// known by its number, every bar of the Minuet included.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn every_bar_drawn_is_known_by_its_number() {
    let minuet = include_bytes!("../../../app/src-tauri/library/minuet-in-g.musicxml");
    let engraved = engraver().engrave(minuet).expect("the Minuet engraves");

    let mut numbers: Vec<u32> = engraved
        .bars()
        .iter()
        .map(|(_, number)| number.get())
        .collect();
    numbers.sort_unstable();
    assert_eq!(numbers, (1..=32).collect::<Vec<_>>());

    let pages = engraved.pages().join("");
    for (element, number) in engraved.bars() {
        assert!(
            pages.contains(&format!("id=\"{}\" class=\"measure\"", element.as_str())),
            "bar {number:?} ({}) is not a drawn measure",
            element.as_str()
        );
    }
}

// The Minuet is in 3/4: every bar lasts three quarters, and bar 1 opens on a
// quarter then four eighths. Every step after the first has a written gap.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn the_minuet_keeps_its_written_rhythm() {
    let minuet = include_bytes!("../../../app/src-tauri/library/minuet-in-g.musicxml");
    let engraved = engraver().engrave(minuet).expect("the Minuet engraves");
    let measures = engraved.score().measures();

    assert!(
        measures
            .iter()
            .all(|measure| measure.length() == Some(1440))
    );
    assert_eq!(
        measures[0]
            .positions()
            .iter()
            .map(Position::beat)
            .collect::<Vec<_>>(),
        [Some(0), Some(480), Some(720), Some(960), Some(1200)]
    );

    let reference = ReferencePerformance::derived_from(engraved.score());
    assert!((1..reference.len()).all(|step| reference.written_gap(step).is_some()));
}
