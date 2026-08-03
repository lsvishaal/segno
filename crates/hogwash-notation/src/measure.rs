//! Positions and Measures — how a Score is divided (`003 §7.8`, `§7.9`).

use hogwash_domain::MeasureNumber;

use crate::Note;

/// One point in the music, and everything the Score requires there.
///
/// `003 §7.9` — *"Notes may be required together — a chord is several Notes
/// at one position."* A Position is that point. A single melody note is a
/// Position holding one Note; a chord is a Position holding several.
///
/// This carries no time. `003 §7.4` scopes the Reference Performance to an
/// *ordered* sequence with no timing significance, so a Position says
/// **what comes together**, never **when**.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Position {
    notes: Vec<Note>,
}

impl Position {
    /// The point at which the Score requires `notes` together, or `None` if
    /// `notes` is empty.
    ///
    /// A position requiring nothing is silence, and silence is the absence of
    /// a position rather than an empty one. Admitting it would put a step in
    /// the Reference Performance that the Cursor could never advance past,
    /// because there would be no note to play (`003 §9.2`).
    #[must_use]
    pub fn new(notes: Vec<Note>) -> Option<Self> {
        if notes.is_empty() {
            None
        } else {
            Some(Self { notes })
        }
    }

    /// The Notes required together here. Never empty.
    #[must_use]
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }
}

/// A structural division of a Piece, as written in its Score (`003 §7.8`).
///
/// Measures exist so that performer and system share a way of saying *where*.
/// `003 §7.8` is explicit that Version 1 does not aggregate Analysis by
/// Measure (`MVP §8` defers FR-407) — they are modelled anyway because
/// Feedback must say where something happened (`003 §7.7`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Measure {
    number: MeasureNumber,
    positions: Vec<Position>,
}

impl Measure {
    /// The measure the Score numbers `number`, containing `positions` in the
    /// order written.
    ///
    /// Unlike [`Position::new`] this is infallible: a Measure with no
    /// positions is a bar of rests, which is legitimate written music.
    /// Rests are not modelled — `003 §7` names no such concept — so such a
    /// bar is simply empty, and it must be constructible or the Score cannot
    /// hold the notation faithfully (`001 §11`).
    #[must_use]
    pub fn new(number: MeasureNumber, positions: Vec<Position>) -> Self {
        Self { number, positions }
    }

    /// The number the Score gave this measure.
    #[must_use]
    pub const fn number(&self) -> MeasureNumber {
        self.number
    }

    /// The positions this measure contains, in written order.
    #[must_use]
    pub fn positions(&self) -> &[Position] {
        &self.positions
    }
}

#[cfg(test)]
mod tests {
    use hogwash_domain::{MeasureNumber, PitchSpelling, Step};

    use crate::{Measure, Note, Position};

    fn note(step: Step, octave: i8) -> Note {
        Note::new(PitchSpelling::new(step, 0, octave)).unwrap()
    }

    // 003 §7.9 — "Notes may be required together — a chord is several Notes
    // at one position." A Position is that: one point in the music, and
    // everything the Score asks for there.
    #[test]
    fn a_chord_is_several_notes_at_one_position() {
        let chord =
            Position::new(vec![note(Step::C, 4), note(Step::E, 4), note(Step::G, 4)]).unwrap();

        assert_eq!(chord.notes().len(), 3);
    }

    #[test]
    fn a_single_note_is_a_position_with_one_note() {
        let single = Position::new(vec![note(Step::C, 4)]).unwrap();

        assert_eq!(single.notes().len(), 1);
    }

    // A position at which the Score asks for nothing is not a position in the
    // music -- it is silence, and silence is the absence of a position rather
    // than an empty one. Allowing it would put a step in the Reference
    // Performance that the Cursor could never advance past, since there is no
    // note to play (003 §9.2).
    #[test]
    fn a_position_requiring_no_notes_is_not_a_position() {
        assert!(Position::new(vec![]).is_none());
    }

    // 003 §7.8 — "A Measure contains Notes", and they are ordered.
    #[test]
    fn a_measure_holds_its_positions_in_order() {
        let first = Position::new(vec![note(Step::C, 4)]).unwrap();
        let second = Position::new(vec![note(Step::D, 4)]).unwrap();

        let measure = Measure::new(MeasureNumber::FIRST, vec![first.clone(), second.clone()]);

        assert_eq!(measure.positions(), &[first, second]);
    }

    // A bar of rests is legitimate written music. Rests are not modelled --
    // 003 §7 names no such concept -- so such a bar is a Measure with no
    // positions, and it must be constructible or the Score cannot represent
    // the notation faithfully (001 §11).
    #[test]
    fn a_measure_of_rests_contains_no_positions() {
        let measure = Measure::new(MeasureNumber::FIRST, vec![]);

        assert!(measure.positions().is_empty());
    }

    #[test]
    fn a_measure_knows_the_number_the_score_gave_it() {
        let fourth = MeasureNumber::new(4).unwrap();

        assert_eq!(Measure::new(fourth, vec![]).number(), fourth);
    }
}
