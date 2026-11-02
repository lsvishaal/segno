//! Score and performance fixtures shared by this crate's tests.

#![cfg(test)]

use std::ops::Range;

use segno_capture::{ObservedPerformance, Touch};
use segno_domain::{MeasureNumber, Pitch, PitchSpelling, Step};
use segno_notation::{
    ExpectedNotes, Measure, Note, Position, ReferencePerformance, Score, TICKS_PER_QUARTER,
};

pub(crate) fn note(step: Step) -> Note {
    Note::new(PitchSpelling::new(step, 0, 4)).expect("octave 4 naturals sound")
}

pub(crate) fn pitch(step: Step) -> Pitch {
    note(step).sounding()
}

/// A pitch no fixture ever asks for.
pub(crate) fn unexpected() -> Pitch {
    Pitch::LOWEST
}

fn melody(steps: &[Step]) -> Vec<Position> {
    steps
        .iter()
        .map(|&step| Position::new(vec![note(step)]).expect("one note is a position"))
        .collect()
}

fn measure(number: u32, positions: Vec<Position>) -> Measure {
    Measure::new(
        MeasureNumber::new(number).expect("fixture measures are numbered from one"),
        positions,
    )
}

fn reference(measures: Vec<Measure>) -> ReferencePerformance {
    ReferencePerformance::derived_from(&Score::new(measures).expect("fixture is a valid score"))
}

/// One bar of single notes.
pub(crate) fn one_bar(steps: &[Step]) -> ReferencePerformance {
    reference(vec![measure(1, melody(steps))])
}

/// Two bars of single notes, so findings can be located in either.
pub(crate) fn two_bars(first: &[Step], second: &[Step]) -> ReferencePerformance {
    reference(vec![measure(1, melody(first)), measure(2, melody(second))])
}

/// Two bars of single notes, `held` tied over the barline into the second.
pub(crate) fn tied_into_second(
    first: &[Step],
    held: Step,
    second: &[Step],
) -> ReferencePerformance {
    reference(vec![
        measure(1, melody(first)),
        measure(2, melody(second)).holding(vec![note(held)]),
    ])
}

/// One bar holding one chord.
pub(crate) fn one_chord(steps: &[Step]) -> ReferencePerformance {
    let notes = steps.iter().copied().map(note).collect();
    let chord = Position::new(notes).expect("a chord has notes");

    reference(vec![measure(1, vec![chord])])
}

/// A performance of the given pitches, in order.
pub(crate) fn played(pitches: &[Pitch]) -> ObservedPerformance {
    let mut observed = ObservedPerformance::new();
    for &pitch in pitches {
        observed.record(pitch);
    }
    observed
}

/// A performance of the natural notes named, in order.
pub(crate) fn played_steps(steps: &[Step]) -> ObservedPerformance {
    played(&steps.iter().copied().map(pitch).collect::<Vec<_>>())
}

/// One bar of single notes, where the note at `at` also tolerates
/// `pitches` (at that step and the next): a grace note or an ornament.
pub(crate) fn one_bar_tolerating(
    steps: &[Step],
    at: usize,
    pitches: &[Pitch],
) -> ReferencePerformance {
    let positions = melody(steps)
        .into_iter()
        .enumerate()
        .map(|(index, position)| {
            if index == at {
                position.tolerating(pitches.iter().copied())
            } else {
                position
            }
        })
        .collect();

    reference(vec![measure(1, positions)])
}

/// A trill, mordent or turn on `step`: every pitch within two semitones of
/// it, which is what the Verovio adapter tolerates for an ornamented note.
pub(crate) fn ornament_on(step: Step) -> Vec<Pitch> {
    let main = u8::from(pitch(step));
    (main - 2..=main + 2)
        .map(|number| Pitch::new(number).expect("octave 4 neighbours are in range"))
        .collect()
}

/// The Minuet in G's right hand, bars 1-3 with their written rhythm, the
/// first two repeated as in the piece: D5 G4 A4 B4 C5 | D5 G4 G4 :| E5 C5
/// D5 E5 F#5. Bars 1 and 3 are a quarter then four eighths, bar 2 three
/// quarters.
pub(crate) fn minuet_in_time() -> ReferencePerformance {
    let bar = |number: u32, notes: &[(Step, i8, i8)], beats: &[u32]| {
        let positions = notes
            .iter()
            .zip(beats)
            .map(|(&(step, alter, octave), &beat)| {
                Position::new(vec![
                    Note::new(PitchSpelling::new(step, alter, octave))
                        .expect("the Minuet's notes sound"),
                ])
                .expect("one note is a position")
                .at_beat(Some(beat))
            })
            .collect();
        measure(number, positions).lasting(Some(1440))
    };
    let eighths = [0, 480, 720, 960, 1200];
    let number = |n: u32| MeasureNumber::new(n).expect("bar number");
    let score = Score::new(vec![
        bar(
            1,
            &[
                (Step::D, 0, 5),
                (Step::G, 0, 4),
                (Step::A, 0, 4),
                (Step::B, 0, 4),
                (Step::C, 0, 5),
            ],
            &eighths,
        ),
        bar(
            2,
            &[(Step::D, 0, 5), (Step::G, 0, 4), (Step::G, 0, 4)],
            &[0, 480, 960],
        ),
        bar(
            3,
            &[
                (Step::E, 0, 5),
                (Step::C, 0, 5),
                (Step::D, 0, 5),
                (Step::E, 0, 5),
                (Step::F, 1, 5),
            ],
            &eighths,
        ),
    ])
    .and_then(|score| score.played_as(vec![number(1), number(2), number(1), number(2), number(3)]))
    .expect("fixture is a valid score");
    ReferencePerformance::derived_from(&score)
}

/// Every note of `steps`, played exactly in the written rhythm at
/// `quarter_us` per quarter note, except that each `(step, extra_us)` comes
/// that much late.
pub(crate) fn played_in_time(
    reference: &ReferencePerformance,
    steps: Range<usize>,
    quarter_us: u64,
    late: &[(usize, u64)],
) -> ObservedPerformance {
    let mut observed = ObservedPerformance::new();
    let mut at_us = 1_000_000;
    for step in steps.clone() {
        if step > steps.start {
            let written = u64::from(reference.written_gap(step).unwrap_or(TICKS_PER_QUARTER));
            at_us += written * quarter_us / u64::from(TICKS_PER_QUARTER);
            at_us += late
                .iter()
                .filter(|(at, _)| *at == step)
                .map(|(_, extra)| extra)
                .sum::<u64>();
        }
        for pitch in reference
            .at(step)
            .into_iter()
            .flat_map(ExpectedNotes::sounding)
        {
            observed.record_touched(
                pitch,
                Touch {
                    at_us,
                    velocity: 64,
                    released_us: None,
                },
            );
        }
    }
    observed
}
