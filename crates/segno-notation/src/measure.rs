//! Positions and Measures — how a Score is divided.

use segno_domain::{MeasureNumber, Pitch};

use crate::Note;

/// How finely beats are counted: ticks per quarter note. Fine enough for
/// triplets and sixty-fourths alike.
pub const TICKS_PER_QUARTER: u32 = 480;

/// One point in the music, and everything the Score requires there.
///
/// *"Notes may be required together — a chord is several Notes
/// at one position."* A Position is that point. A single melody note is a
/// Position holding one Note; a chord is a Position holding several.
///
/// It may know where in its bar it falls, in written beats, never in
/// seconds: the performer's speed is the tempo.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Position {
    notes: Vec<Note>,
    /// Pitches that are not required but are not wrong either, here and at
    /// the next step: a grace note's, or the neighbours of an ornamented note.
    tolerated: Vec<Pitch>,
    /// Where in its bar this falls, in [`TICKS_PER_QUARTER`]ths of a quarter
    /// note, when the Score's rhythm is known.
    beat: Option<u32>,
}

impl Position {
    /// The point at which the Score requires `notes` together, or `None` if
    /// `notes` is empty.
    ///
    /// A position requiring nothing is silence, and silence is the absence of
    /// a position rather than an empty one. Admitting it would put a step in
    /// the Reference Performance that the Cursor could never advance past,
    /// because there would be no note to play.
    #[must_use]
    pub fn new(notes: Vec<Note>) -> Option<Self> {
        if notes.is_empty() {
            None
        } else {
            Some(Self {
                notes,
                tolerated: Vec::new(),
                beat: None,
            })
        }
    }

    /// The same Position, also tolerating `pitches` here and at the next
    /// step of the performance: played there, they neither advance the
    /// cursor nor count as wrong. How a grace note stays optional, and how
    /// the extra notes of a trill or mordent are not mistakes.
    #[must_use]
    pub fn tolerating(mut self, pitches: impl IntoIterator<Item = Pitch>) -> Self {
        self.tolerated.extend(pitches);
        self.tolerated.sort_unstable();
        self.tolerated.dedup();
        self
    }

    /// The same Position, falling `ticks` into its bar, if that is known.
    #[must_use]
    pub const fn at_beat(mut self, ticks: Option<u32>) -> Self {
        self.beat = ticks;
        self
    }

    /// Where in its bar this falls, in ticks, if the rhythm is known.
    #[must_use]
    pub const fn beat(&self) -> Option<u32> {
        self.beat
    }

    /// The Notes required together here. Never empty.
    #[must_use]
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    /// The pitches tolerated here and at the next step. See
    /// [`tolerating`](Self::tolerating).
    #[must_use]
    pub fn tolerated(&self) -> &[Pitch] {
        &self.tolerated
    }
}

/// A structural division of a Piece, as written in its Score.
///
/// Measures exist so that performer and system share a way of saying *where*.
/// The design is explicit that Version 1 does not aggregate Analysis by
/// Measure (the design defers the requirement) — they are modelled anyway because
/// Feedback must say where something happened.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Measure {
    number: MeasureNumber,
    positions: Vec<Position>,
    /// Notes tied over the barline into this measure's opening: struck in
    /// the measure before and still sounding when this one begins.
    held_into: Vec<Note>,
    /// How long the bar is, in ticks, when the Score's rhythm is known.
    length: Option<u32>,
}

impl Measure {
    /// The measure the Score numbers `number`, containing `positions` in the
    /// order written.
    ///
    /// Unlike [`Position::new`] this is infallible: a Measure with no
    /// positions is a bar of rests, which is legitimate written music.
    /// Rests are not modelled — the design names no such concept — so such a
    /// bar is simply empty, and it must be constructible or the Score cannot
    /// hold the notation faithfully.
    #[must_use]
    pub fn new(number: MeasureNumber, positions: Vec<Position>) -> Self {
        Self {
            number,
            positions,
            held_into: Vec::new(),
            length: None,
        }
    }

    /// This measure, lasting `ticks`, if that is known.
    #[must_use]
    pub const fn lasting(mut self, ticks: Option<u32>) -> Self {
        self.length = ticks;
        self
    }

    /// How long this measure is, in ticks, if the rhythm is known.
    #[must_use]
    pub const fn length(&self) -> Option<u32> {
        self.length
    }

    /// This measure, opening with `notes` held over from the one before.
    #[must_use]
    pub fn holding(mut self, notes: Vec<Note>) -> Self {
        self.held_into = notes;
        self
    }

    /// The notes tied into this measure's opening. Played through, they are
    /// never struck again; starting here, they have to be.
    #[must_use]
    pub fn held_into(&self) -> &[Note] {
        &self.held_into
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
    use segno_domain::{MeasureNumber, PitchSpelling, Step};

    use crate::{Measure, Note, Position};

    fn note(step: Step, octave: i8) -> Note {
        Note::new(PitchSpelling::new(step, 0, octave)).unwrap()
    }

    // "Notes may be required together — a chord is several Notes
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
    // note to play.
    #[test]
    fn a_position_requiring_no_notes_is_not_a_position() {
        assert!(Position::new(vec![]).is_none());
    }

    // "A Measure contains Notes", and they are ordered.
    #[test]
    fn a_measure_holds_its_positions_in_order() {
        let first = Position::new(vec![note(Step::C, 4)]).unwrap();
        let second = Position::new(vec![note(Step::D, 4)]).unwrap();

        let measure = Measure::new(MeasureNumber::FIRST, vec![first.clone(), second.clone()]);

        assert_eq!(measure.positions(), &[first, second]);
    }

    // A bar of rests is legitimate written music. Rests are not modelled --
    // The design names no such concept -- so such a bar is a Measure with no
    // positions, and it must be constructible or the Score cannot represent
    // the notation faithfully.
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
