//! Fixtures shared by this crate's tests.

#![cfg(test)]

use hogwash_analysis::Analysis;
use hogwash_capture::ObservedPerformance;
use hogwash_domain::{MeasureNumber, Pitch, PitchSpelling, Step};
use hogwash_notation::{Measure, Note, Position, ReferencePerformance, Score};

pub(crate) fn note(step: Step, alter: i8) -> Note {
    Note::new(PitchSpelling::new(step, alter, 4)).expect("octave 4 notes sound")
}

pub(crate) fn pitch(step: Step, alter: i8) -> Pitch {
    note(step, alter).sounding()
}

/// A pitch none of these fixtures ever writes.
pub(crate) fn unexpected() -> Pitch {
    Pitch::LOWEST
}

fn measure(number: u32, notes: &[(Step, i8)]) -> Measure {
    let positions = notes
        .iter()
        .map(|&(step, alter)| {
            Position::new(vec![note(step, alter)]).expect("one note is a position")
        })
        .collect();

    Measure::new(
        MeasureNumber::new(number).expect("fixture measures are numbered from one"),
        positions,
    )
}

/// A Reference Performance of one bar per slice given.
pub(crate) fn bars(bars: &[&[(Step, i8)]]) -> ReferencePerformance {
    let measures = bars
        .iter()
        .enumerate()
        .map(|(index, notes)| {
            let number = u32::try_from(index + 1).expect("fixtures are small");
            measure(number, notes)
        })
        .collect();

    ReferencePerformance::derived_from(&Score::new(measures).expect("fixture is a valid score"))
}

pub(crate) fn played(pitches: &[Pitch]) -> ObservedPerformance {
    let mut observed = ObservedPerformance::new();
    for &pitch in pitches {
        observed.record(pitch);
    }
    observed
}

/// The Analysis of `pitches` played against `reference`.
pub(crate) fn analyse(pitches: &[Pitch], reference: &ReferencePerformance) -> Analysis {
    Analysis::of(&played(pitches), reference)
}

/// The note letters `text` names, in order.
///
/// Whole words only: "Bar" begins with a capital B without naming the note B,
/// and a naive character scan would count it. A note letter always stands
/// alone — "F", or "B" in "B flat".
pub(crate) fn note_letters_in(text: &str) -> Vec<char> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter_map(|word| {
            let mut characters = word.chars();
            match (characters.next(), characters.next()) {
                (Some(letter), None) if letter.is_ascii_uppercase() => Some(letter),
                _ => None,
            }
        })
        .filter(|letter| ('A'..='G').contains(letter))
        .collect()
}
