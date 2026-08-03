//! The Reference Performance derived from a Score (`003 §7.4`).

use hogwash_domain::{MeasureNumber, Pitch};

use crate::{Note, Score};

/// One step of a Reference Performance: the Notes required together at one
/// point, and the Measure they are written in.
///
/// `003 §7.9` names this role — *"an expected Note — a Note the Reference
/// Performance requires at some position"*. The Measure travels with it
/// because Feedback locates findings by Measure (`003 §7.7`), and the
/// Cursor's position is an index into the sequence (`003 §7.16`) — an index
/// alone cannot say "bar four".
///
/// It carries no time. `003 §7.4` scopes the Reference Performance to an
/// ordered sequence with "no timing significance" in Version 1.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExpectedNotes {
    measure: MeasureNumber,
    notes: Vec<Note>,
}

impl ExpectedNotes {
    /// The step requiring `notes` together, written in `measure`, or `None`
    /// if `notes` is empty.
    ///
    /// A step expecting nothing would be one the Cursor could never advance
    /// past, since no note could satisfy it (`003 §9.2`).
    #[must_use]
    pub fn new(measure: MeasureNumber, notes: Vec<Note>) -> Option<Self> {
        if notes.is_empty() {
            None
        } else {
            Some(Self { measure, notes })
        }
    }

    /// The Measure these Notes are written in.
    #[must_use]
    pub const fn measure(&self) -> MeasureNumber {
        self.measure
    }

    /// The Notes required together here. Never empty.
    #[must_use]
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    /// The pitches these Notes sound.
    ///
    /// What Analysis compares, per ADR-0001 — a performer presses keys, and a
    /// key has no spelling.
    pub fn sounding(&self) -> impl Iterator<Item = Pitch> + '_ {
        self.notes.iter().map(|note| note.sounding())
    }
}

/// The sequence of Notes a Piece's Score requires, in the order the Score
/// requires them (`003 §7.4`).
///
/// This is what establishes what *correct* means. Without it there is nothing
/// to compare a performance against.
///
/// **Derived only from a Score, never from anything the performer does.**
/// `003 §9.1` states it as a structural invariant — what was expected and
/// what was played are permanently distinct, and neither is ever derived from
/// the other. [`derived_from`](Self::derived_from) taking `&Score` and
/// nothing else is how that is carried rather than merely intended.
///
/// **It carries no timing.** `003 §7.4` is explicit, and gives the reason:
/// with practice waiting for the performer (`MVP §5`), the tempo *is*
/// whatever the performer plays, so "when a note was due" has nothing to be
/// compared against. Should a timed practice mode arrive, this concept gains
/// a temporal dimension rather than being replaced.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReferencePerformance {
    steps: Vec<ExpectedNotes>,
}

impl ReferencePerformance {
    /// Derives the Reference Performance of `score`.
    ///
    /// Measures contributing no positions — bars of rests — contribute no
    /// steps. A step expecting nothing would stop the Cursor at a place no
    /// note could advance it past, and practice would hang.
    #[must_use]
    pub fn derived_from(score: &Score) -> Self {
        let steps = score
            .measures()
            .iter()
            .flat_map(|measure| {
                measure.positions().iter().filter_map(move |position| {
                    ExpectedNotes::new(measure.number(), position.notes().to_vec())
                })
            })
            .collect();

        Self { steps }
    }

    /// How many steps the performer is expected to play.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether the Score asks for nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The step at `index`, or `None` if the sequence has ended.
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&ExpectedNotes> {
        self.steps.get(index)
    }

    /// The steps in the order the Score requires them.
    pub fn iter(&self) -> core::slice::Iter<'_, ExpectedNotes> {
        self.steps.iter()
    }
}

impl<'a> IntoIterator for &'a ReferencePerformance {
    type Item = &'a ExpectedNotes;
    type IntoIter = core::slice::Iter<'a, ExpectedNotes>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use hogwash_domain::{MeasureNumber, PitchSpelling, Step};

    use crate::{ExpectedNotes, Measure, Note, Position, ReferencePerformance, Score};

    fn note(step: Step, octave: i8) -> Note {
        Note::new(PitchSpelling::new(step, 0, octave)).unwrap()
    }

    fn melody(number: u32, steps: &[Step]) -> Measure {
        let positions = steps
            .iter()
            .map(|&step| Position::new(vec![note(step, 4)]).unwrap())
            .collect();

        Measure::new(MeasureNumber::new(number).unwrap(), positions)
    }

    fn score_of(measures: Vec<Measure>) -> Score {
        Score::new(measures).unwrap()
    }

    fn steps_of(reference: &ReferencePerformance) -> Vec<Step> {
        reference
            .iter()
            .flat_map(ExpectedNotes::notes)
            .map(|note| note.spelling().step())
            .collect()
    }

    // 003 §7.4 — "The sequence of notes a Piece's Score requires, in the order
    // the Score requires them." Order across measures, not just within one.
    #[test]
    fn the_reference_performance_follows_the_order_the_score_requires() {
        let score = score_of(vec![
            melody(1, &[Step::C, Step::D]),
            melody(2, &[Step::E, Step::F]),
        ]);

        let reference = ReferencePerformance::derived_from(&score);

        assert_eq!(
            steps_of(&reference),
            vec![Step::C, Step::D, Step::E, Step::F]
        );
    }

    // 003 §7.4 Business Invariant — "The same Score always produces the same
    // Reference Performance."
    #[test]
    fn the_same_score_always_yields_the_same_reference_performance() {
        let score = score_of(vec![melody(1, &[Step::C, Step::D, Step::E])]);

        assert_eq!(
            ReferencePerformance::derived_from(&score),
            ReferencePerformance::derived_from(&score)
        );
    }

    // 003 §7.4 Domain Classification — "two Reference Performances with the
    // same expected notes in the same order are the same thing." Two Scores
    // built independently, never compared as objects, must agree.
    #[test]
    fn reference_performances_with_the_same_expected_notes_are_the_same() {
        let one = score_of(vec![melody(1, &[Step::G, Step::A])]);
        let other = score_of(vec![melody(1, &[Step::G, Step::A])]);

        assert_eq!(
            ReferencePerformance::derived_from(&one),
            ReferencePerformance::derived_from(&other)
        );
    }

    // 003 §7.4 — it "says what comes next, and which notes are required
    // together". A chord is one step of the sequence, not three.
    #[test]
    fn a_chord_is_one_step_requiring_several_notes() {
        let chord =
            Position::new(vec![note(Step::C, 4), note(Step::E, 4), note(Step::G, 4)]).unwrap();
        let score = score_of(vec![Measure::new(MeasureNumber::FIRST, vec![chord])]);

        let reference = ReferencePerformance::derived_from(&score);

        assert_eq!(reference.len(), 1);
        assert_eq!(reference.at(0).unwrap().notes().len(), 3);
    }

    // 003 §7.7 — Feedback locates findings by Measure. That is only possible
    // if the derivation carries the measure through; the Cursor's position is
    // an index into this sequence (003 §7.16), and an index alone cannot say
    // "bar four".
    #[test]
    fn each_step_knows_which_measure_it_is_in() {
        let score = score_of(vec![melody(1, &[Step::C]), melody(2, &[Step::D])]);

        let reference = ReferencePerformance::derived_from(&score);

        assert_eq!(reference.at(0).unwrap().measure(), MeasureNumber::FIRST);
        assert_eq!(
            reference.at(1).unwrap().measure(),
            MeasureNumber::new(2).unwrap()
        );
    }

    // A bar of rests asks the performer for nothing, so it contributes no
    // step. If it contributed an empty one, the Cursor would stop at a place
    // where no note could ever advance it (003 §9.2) -- practice would hang.
    #[test]
    fn a_measure_of_rests_contributes_no_step() {
        let score = score_of(vec![
            melody(1, &[Step::C]),
            Measure::new(MeasureNumber::new(2).unwrap(), vec![]),
            melody(3, &[Step::D]),
        ]);

        let reference = ReferencePerformance::derived_from(&score);

        assert_eq!(reference.len(), 2);
        assert_eq!(steps_of(&reference), vec![Step::C, Step::D]);
    }

    // 003 §7.4 Scope in Version 1 — the Reference Performance "carries no
    // timing significance". Asserted by round-trip: a step is fully
    // reconstructible from its measure and its notes. Give it a time, an
    // offset or a duration and this stops holding, because the round trip
    // cannot carry it.
    #[test]
    fn a_step_carries_nothing_but_its_measure_and_its_notes() {
        let score = score_of(vec![melody(1, &[Step::C])]);
        let reference = ReferencePerformance::derived_from(&score);
        let step = reference.at(0).unwrap();

        let rebuilt = ExpectedNotes::new(step.measure(), step.notes().to_vec()).unwrap();

        assert_eq!(&rebuilt, step);
    }

    // A Score of nothing but rests yields a Reference Performance with no
    // steps. Degenerate, but representable -- 001 §11 forbids altering
    // notation to make it more convenient, so it is reported as what it is.
    #[test]
    fn a_score_of_only_rests_expects_nothing() {
        let score = score_of(vec![Measure::new(MeasureNumber::FIRST, vec![])]);

        let reference = ReferencePerformance::derived_from(&score);

        assert!(reference.is_empty());
    }
}
