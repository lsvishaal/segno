//! The coach on real scores, as Verovio engraves them: the bundled Minuet in
//! G and Prelude in C. **Every test here is `#[ignore]`d**: each needs a real
//! libverovio. Run them with `just engraving-test`.

// clippy.toml's allow-expect-in-tests only reaches `#[test]` functions and
// `#[cfg(test)]` code; this file's helpers are test code it cannot see.
#![expect(
    clippy::expect_used,
    reason = "test helpers; clippy's allow-expect-in-tests cannot see a tests/ helper fn"
)]

use segno_analysis::Trace;
use segno_analysis::coach::{Coach, Kind, broken_chord, leap_into};
use segno_capture::{ObservedPerformance, Touch};
use segno_domain::MeasureNumber;
use segno_engraving::Engraver as _;
use segno_engraving_verovio::VerovioEngraver;
use segno_notation::{ExpectedNotes, ReferencePerformance, TICKS_PER_QUARTER};

fn reference(source: &[u8]) -> ReferencePerformance {
    let engraved = VerovioEngraver::from_environment()
        .and_then(|mut engraver| engraver.engrave(source))
        .expect("set SEGNO_VEROVIO_LIB and SEGNO_VEROVIO_DATA, or run `just engraving-test`");
    ReferencePerformance::derived_from(engraved.score())
}

fn minuet() -> ReferencePerformance {
    reference(include_bytes!(
        "../../../app/src-tauri/library/minuet-in-g.musicxml"
    ))
}

/// Every note of the piece in its written rhythm at a quarter every 600 ms,
/// each step's arrival moved by `adjust` (in µs, may be negative).
fn played(reference: &ReferencePerformance, adjust: impl Fn(usize) -> i64) -> ObservedPerformance {
    let mut observed = ObservedPerformance::new();
    let mut at_us: i64 = 1_000_000;
    for step in 0..reference.len() {
        if step > 0 {
            let written = i64::from(reference.written_gap(step).unwrap_or(TICKS_PER_QUARTER));
            at_us += written * 600_000 / i64::from(TICKS_PER_QUARTER) + adjust(step);
        }
        for pitch in reference
            .at(step)
            .into_iter()
            .flat_map(ExpectedNotes::sounding)
        {
            let touch = Touch {
                at_us: u64::try_from(at_us).expect("time runs forward"),
                velocity: 64,
                released_us: None,
            };
            observed.record_touched(pitch, touch);
        }
    }
    observed
}

fn coach(reference: &ReferencePerformance, observed: &ObservedPerformance) -> Coach {
    let trace = Trace::of(observed, reference, 0..reference.len(), false);
    Coach::of(reference, &[trace], Some(100))
}

// Somewhere in the Minuet a hand lands more than an octave from every note
// before it. Hesitating there each time is a jump to practise.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn hesitating_before_a_leap_in_the_minuet_is_a_jump() {
    let minuet = minuet();
    let leaps: Vec<usize> = (1..minuet.len())
        .filter(|&step| leap_into(&minuet, step))
        .collect();
    let first = *leaps.first().expect("the Minuet has a leap");
    let bar = minuet.at(first).expect("a step").measure();
    let in_bar: Vec<usize> = leaps
        .iter()
        .copied()
        .filter(|&step| minuet.at(step).map(ExpectedNotes::measure) == Some(bar))
        .collect();

    let observed = played(
        &minuet,
        |step| {
            if in_bar.contains(&step) { 2_000_000 } else { 0 }
        },
    );

    let cards = coach(&minuet, &observed).cards;
    assert!(
        cards.iter().any(|card| card.main.first == bar
            && (card.main.kind == Kind::Jump
                || card.also.is_some_and(|also| also.kind == Kind::Jump))),
        "a jump card for bar {}: {cards:?}",
        bar.get()
    );
}

// Leaving every long note (a half or longer) at a third of its length.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn cutting_the_minuets_long_notes_short_is_rushing_them() {
    let minuet = minuet();
    let observed = played(&minuet, |step| {
        let written = i64::from(minuet.written_gap(step).unwrap_or(0));
        if written >= 2 * i64::from(TICKS_PER_QUARTER) {
            -(written * 600_000 / i64::from(TICKS_PER_QUARTER)) * 2 / 3
        } else {
            0
        }
    });

    let cards = coach(&minuet, &observed).cards;
    assert!(
        cards
            .iter()
            .any(|card| card.main.kind == Kind::RushedLongNotes),
        "{cards:?}"
    );
}

// Bach's Prelude in C is broken chords from its first bar; the Minuet's
// first bar is a run of steps.
#[test]
#[ignore = "needs a real libverovio; run `just engraving-test`"]
fn the_preludes_bars_are_broken_chords_and_the_minuets_are_not() {
    let prelude = reference(include_bytes!(
        "../../../app/src-tauri/library/prelude-in-c.musicxml"
    ));
    let bar = |number| MeasureNumber::new(number).expect("a bar");

    assert!(broken_chord(&prelude, bar(1)));
    assert!(!broken_chord(&minuet(), bar(1)));
}
