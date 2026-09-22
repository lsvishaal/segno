//! Score and performance fixtures shared by this crate's tests.

#![cfg(test)]

use hogwash_capture::ObservedPerformance;
use hogwash_domain::{MeasureNumber, Pitch, PitchSpelling, Step};
use hogwash_notation::{Measure, Note, Position, ReferencePerformance, Score};

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
