//! Score fixtures shared by this crate's tests.
//!
//! Hand-constructed, never parsed. The design derives a Reference Performance
//! from a Score, so a test that needs one builds the Score it came from —
//! which keeps these tests honest about the shape the real thing has.

#![cfg(test)]

use std::sync::Arc;

use hogwash_domain::{MeasureNumber, Pitch, PitchSpelling, Step};
use hogwash_notation::{Measure, Note, Position, ReferencePerformance, Score};

/// A natural note in octave 4 — the octave around middle C.
pub(crate) fn note(step: Step) -> Note {
    Note::new(PitchSpelling::new(step, 0, 4)).expect("octave 4 naturals sound")
}

/// The pitch a natural `step` in octave 4 sounds.
pub(crate) fn pitch(step: Step) -> Pitch {
    note(step).sounding()
}

/// A pitch no fixture ever asks for, for testing unexpected input.
pub(crate) fn unexpected() -> Pitch {
    Pitch::LOWEST
}

/// One position per step: a melody, no chords.
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

/// Shared, because a Cursor holds a share of what it indexes into rather
/// than borrowing it — see the note on `Cursor`. Returning the `Arc` here
/// keeps the tests reading the way they did before.
fn reference(measures: Vec<Measure>) -> Arc<ReferencePerformance> {
    Arc::new(ReferencePerformance::derived_from(
        &Score::new(measures).expect("fixture is a valid score"),
    ))
}

/// A single bar of single notes.
pub(crate) fn one_bar(steps: &[Step]) -> Arc<ReferencePerformance> {
    reference(vec![measure(1, melody(steps))])
}

/// Two bars of single notes — needed to show a bar cannot be skipped.
pub(crate) fn two_bars(first: &[Step], second: &[Step]) -> Arc<ReferencePerformance> {
    reference(vec![measure(1, melody(first)), measure(2, melody(second))])
}

/// Two bars of single notes, `held` tied over the barline into the second.
pub(crate) fn tied_into_second(
    first: &[Step],
    held: Step,
    second: &[Step],
) -> Arc<ReferencePerformance> {
    reference(vec![
        measure(1, melody(first)),
        measure(2, melody(second)).holding(vec![note(held)]),
    ])
}

/// A single bar holding one chord: every step required together.
pub(crate) fn one_chord(steps: &[Step]) -> Arc<ReferencePerformance> {
    let notes = steps.iter().copied().map(note).collect();
    let chord = Position::new(notes).expect("a chord has notes");

    reference(vec![measure(1, vec![chord])])
}

/// A single bar holding one chord written with two spellings of one key.
///
/// F sharp and G flat are one key on a keyboard, so this is a
/// chord a performer satisfies with a single press.
pub(crate) fn one_enharmonic_unison() -> Arc<ReferencePerformance> {
    let f_sharp = Note::new(PitchSpelling::new(Step::F, 1, 4)).expect("F sharp 4 sounds");
    let g_flat = Note::new(PitchSpelling::new(Step::G, -1, 4)).expect("G flat 4 sounds");
    let unison = Position::new(vec![f_sharp, g_flat]).expect("a unison has notes");

    reference(vec![measure(1, vec![unison])])
}

/// The pitch both spellings of [`one_enharmonic_unison`] sound.
pub(crate) fn unison_pitch() -> Pitch {
    Note::new(PitchSpelling::new(Step::F, 1, 4))
        .expect("F sharp 4 sounds")
        .sounding()
}
