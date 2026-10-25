//! The Reference Performance derived from a Score.

use core::ops::Range;

use hogwash_domain::{MeasureNumber, Pitch};

use crate::{Note, Score};

/// One step of a Reference Performance: the Notes required together at one
/// point, and the Measure they are written in.
///
/// The design names this role — *"an expected Note — a Note the Reference
/// Performance requires at some position"*. The Measure travels with it
/// because Feedback locates findings by Measure, and the
/// Cursor's position is an index into the sequence — an index
/// alone cannot say "bar four".
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExpectedNotes {
    measure: MeasureNumber,
    notes: Vec<Note>,
    /// Pitches that are neither required nor wrong here. Never one of the
    /// required pitches.
    tolerated: Vec<Pitch>,
    /// On the first step of a measure: the notes tied into that measure,
    /// required only when an attempt starts here.
    held_into: Vec<Note>,
}

impl ExpectedNotes {
    /// The step requiring `notes` together, written in `measure`, or `None`
    /// if `notes` is empty.
    ///
    /// A step expecting nothing would be one the Cursor could never advance
    /// past, since no note could satisfy it.
    #[must_use]
    pub fn new(measure: MeasureNumber, notes: Vec<Note>) -> Option<Self> {
        if notes.is_empty() {
            None
        } else {
            Some(Self {
                measure,
                notes,
                tolerated: Vec::new(),
                held_into: Vec::new(),
            })
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
    /// What Analysis compares, per the design — a performer presses keys, and a
    /// key has no spelling.
    pub fn sounding(&self) -> impl Iterator<Item = Pitch> + '_ {
        self.notes.iter().map(|note| note.sounding())
    }

    /// The notes tied into this step's measure, if this is the measure's
    /// first step. Played through they are already sounding; starting here
    /// they have to be struck.
    #[must_use]
    pub fn held_into(&self) -> &[Note] {
        &self.held_into
    }

    /// This step as it is when an attempt starts on it: the held notes join
    /// the ones struck here, like one chord. A held note the step also
    /// strikes is asked for once.
    #[must_use]
    pub fn starting_here(&self) -> Self {
        let mut started = self.clone();
        for held in &self.held_into {
            if !started.sounding().any(|pitch| pitch == held.sounding()) {
                started.notes.push(*held);
            }
        }
        started
            .tolerated
            .retain(|pitch| !started.notes.iter().any(|note| note.sounding() == *pitch));
        started
    }

    /// Whether `pitch`, played here, is tolerated: a grace note or part of an
    /// ornament. It does not advance the cursor, and it is not a mistake.
    #[must_use]
    pub fn tolerates(&self, pitch: Pitch) -> bool {
        self.tolerated.binary_search(&pitch).is_ok()
    }
}

/// The sequence of Notes a Piece's Score requires, in the order the Score
/// requires them.
///
/// This is what establishes what *correct* means. Without it there is nothing
/// to compare a performance against.
///
/// **Derived only from a Score, never from anything the performer does.**
/// The design states it as a structural invariant — what was expected and
/// what was played are permanently distinct, and neither is ever derived from
/// the other. [`derived_from`](Self::derived_from) taking `&Score` and
/// nothing else is how that is carried rather than merely intended.
///
/// It knows the written rhythm, when the Score does, but never a tempo:
/// with practice waiting for the performer, the tempo is whatever the
/// performer plays.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReferencePerformance {
    steps: Vec<ExpectedNotes>,
    /// Each step's beat from the start, in ticks, in played order. Empty
    /// when any bar's rhythm is unknown.
    beats: Vec<u32>,
    /// Each measure in played order, with the step it starts at: the index
    /// of its first step, or of the next measure's when it has none.
    openings: Vec<(MeasureNumber, usize)>,
    /// What the last step tolerates at "the next step", which is past the
    /// end: the tail of a trill on the final note.
    after_the_end: Vec<Pitch>,
}

impl ReferencePerformance {
    /// Derives the Reference Performance of `score`.
    ///
    /// Measures contributing no positions — bars of rests — contribute no
    /// steps. A step expecting nothing would stop the Cursor at a place no
    /// note could advance it past, and practice would hang.
    #[must_use]
    pub fn derived_from(score: &Score) -> Self {
        let mut steps: Vec<ExpectedNotes> = Vec::new();
        let mut openings = Vec::new();
        // What the previous step tolerates at the next one.
        let mut carried: Vec<Pitch> = Vec::new();
        // Where the bar being read starts, while the rhythm is known.
        let mut beats = Vec::new();
        let mut bar_starts = Some(0_u32);

        for measure in score.performance() {
            openings.push((measure.number(), steps.len()));
            let first = steps.len();

            for position in measure.positions() {
                let Some(mut step) =
                    ExpectedNotes::new(measure.number(), position.notes().to_vec())
                else {
                    continue;
                };

                let mut tolerated: Vec<Pitch> = carried
                    .iter()
                    .chain(position.tolerated())
                    .copied()
                    .filter(|pitch| !step.sounding().any(|required| required == *pitch))
                    .collect();
                tolerated.sort_unstable();
                tolerated.dedup();
                step.tolerated = tolerated;

                if steps.len() == first {
                    step.held_into = measure.held_into().to_vec();
                }

                carried = position.tolerated().to_vec();
                steps.push(step);
                if let (Some(start), Some(beat)) = (bar_starts, position.beat()) {
                    beats.push(start.saturating_add(beat));
                } else {
                    bar_starts = None;
                }
            }

            bar_starts = bar_starts
                .zip(measure.length())
                .map(|(start, length)| start.saturating_add(length));
        }

        if bar_starts.is_none() {
            beats.clear();
        }

        Self {
            steps,
            beats,
            openings,
            after_the_end: carried,
        }
    }

    /// How many ticks the Score writes between step `index - 1` and step
    /// `index`, in played order. `None` for the first step, or when the
    /// rhythm is unknown.
    #[must_use]
    pub fn written_gap(&self, index: usize) -> Option<u32> {
        let earlier = self.beats.get(index.checked_sub(1)?)?;
        let later = self.beats.get(index)?;
        later.checked_sub(*earlier)
    }

    /// Whether `pitch`, played after the last step, is the tail of something
    /// the last step tolerated rather than an unexpected note.
    #[must_use]
    pub fn tolerates_after_the_end(&self, pitch: Pitch) -> bool {
        self.after_the_end.contains(&pitch)
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

    /// Where an attempt starting at measure `number` begins: the first step
    /// of its first occurrence in played order. A measure with nothing to
    /// strike (rests, or only held notes) starts at the next one that has
    /// something. `None` if the measure does not exist or nothing follows it.
    #[must_use]
    pub fn first_position_of(&self, number: MeasureNumber) -> Option<usize> {
        self.openings
            .iter()
            .find(|(measure, _)| *measure == number)
            .map(|&(_, step)| step)
            .filter(|&step| step < self.steps.len())
    }

    /// The steps a loop over measures `first` to `last` (either order)
    /// covers: the first place in played order where they are played in a
    /// row, from the first one's opening to the next measure's. `None` if
    /// they never are, or there is nothing in them to strike.
    #[must_use]
    pub fn range_of(&self, first: MeasureNumber, last: MeasureNumber) -> Option<Range<usize>> {
        let (first, last) = (first.min(last), first.max(last));

        self.openings
            .iter()
            .enumerate()
            .find_map(|(from, &(measure, start))| {
                if measure != first {
                    return None;
                }
                let (mut previous, mut end) = (measure, self.steps.len());
                let mut reached = measure == last;
                for &(next, step) in self.openings.get(from + 1..)? {
                    if reached {
                        end = step;
                        break;
                    }
                    if next <= previous || next > last {
                        return None;
                    }
                    previous = next;
                    reached = next == last;
                }
                (reached && start < end).then_some(start..end)
            })
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

    // "The sequence of notes a Piece's Score requires, in the order
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

    // The design Business Invariant — "The same Score always produces the same
    // Reference Performance."
    #[test]
    fn the_same_score_always_yields_the_same_reference_performance() {
        let score = score_of(vec![melody(1, &[Step::C, Step::D, Step::E])]);

        assert_eq!(
            ReferencePerformance::derived_from(&score),
            ReferencePerformance::derived_from(&score)
        );
    }

    // The design Domain Classification — "two Reference Performances with the
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

    // it "says what comes next, and which notes are required
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

    // Feedback locates findings by Measure. That is only possible
    // if the derivation carries the measure through; the Cursor's position is
    // an index into this sequence, and an index alone cannot say
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

    // A repeated bar is played, and so expected, twice, in playing order.
    #[test]
    fn a_repeated_bar_is_expected_again_where_it_is_played_again() {
        let bar = |number| MeasureNumber::new(number).unwrap();
        let score = score_of(vec![melody(1, &[Step::C, Step::D]), melody(2, &[Step::E])])
            .played_as(vec![bar(1), bar(1), bar(2)])
            .unwrap();

        let reference = ReferencePerformance::derived_from(&score);

        assert_eq!(
            steps_of(&reference),
            vec![Step::C, Step::D, Step::C, Step::D, Step::E]
        );
    }

    fn pitch(number: u8) -> hogwash_domain::Pitch {
        hogwash_domain::Pitch::new(number).unwrap()
    }

    // A grace note or an ornament's neighbours are tolerated at their own
    // step and the next one -- no further, and never in place of a
    // required note.
    #[test]
    fn tolerated_pitches_reach_their_step_and_the_next_one_only() {
        let c = Position::new(vec![note(Step::C, 4)]).unwrap().tolerating([
            pitch(61),
            pitch(62),
            pitch(60),
        ]);
        let score = score_of(vec![Measure::new(
            MeasureNumber::FIRST,
            vec![
                c,
                Position::new(vec![note(Step::D, 4)]).unwrap(),
                Position::new(vec![note(Step::E, 4)]).unwrap(),
            ],
        )]);

        let reference = ReferencePerformance::derived_from(&score);
        let (first, second, third) = (
            reference.at(0).unwrap(),
            reference.at(1).unwrap(),
            reference.at(2).unwrap(),
        );

        assert!(first.tolerates(pitch(61)));
        assert!(
            !first.tolerates(pitch(60)),
            "required, so not merely tolerated"
        );
        assert!(second.tolerates(pitch(60)) && second.tolerates(pitch(61)));
        assert!(!second.tolerates(pitch(62)), "required at the second step");
        assert!(
            !third.tolerates(pitch(61)),
            "two steps on, it is a mistake again"
        );
    }

    // A trill on the last note keeps going after the cursor has reached the
    // end. Its tail is tolerated there too.
    #[test]
    fn the_last_steps_tolerance_reaches_past_the_end() {
        let score = score_of(vec![Measure::new(
            MeasureNumber::FIRST,
            vec![
                Position::new(vec![note(Step::C, 4)])
                    .unwrap()
                    .tolerating([pitch(62)]),
            ],
        )]);

        let reference = ReferencePerformance::derived_from(&score);

        assert!(reference.tolerates_after_the_end(pitch(62)));
        assert!(!reference.tolerates_after_the_end(pitch(64)));
    }

    // A bar of rests asks the performer for nothing, so it contributes no
    // step. If it contributed an empty one, the Cursor would stop at a place
    // where no note could ever advance it -- practice would hang.
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

    // The design Scope in Version 1 — the Reference Performance "carries no
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
    // steps. Degenerate, but representable -- the design forbids altering
    // notation to make it more convenient, so it is reported as what it is.
    #[test]
    fn a_score_of_only_rests_expects_nothing() {
        let score = score_of(vec![Measure::new(MeasureNumber::FIRST, vec![])]);

        let reference = ReferencePerformance::derived_from(&score);

        assert!(reference.is_empty());
    }

    /// The Minuet in G's right hand, bars 1-3 as written (D5 G4 A4 B4 C5 |
    /// D5 G4 G4 | E5 C5 D5 E5 F#5).
    fn minuet_bar(number: u32) -> Measure {
        let notes: &[(Step, i8, i8)] = match number {
            1 => &[
                (Step::D, 0, 5),
                (Step::G, 0, 4),
                (Step::A, 0, 4),
                (Step::B, 0, 4),
                (Step::C, 0, 5),
            ],
            2 => &[(Step::D, 0, 5), (Step::G, 0, 4), (Step::G, 0, 4)],
            _ => &[
                (Step::E, 0, 5),
                (Step::C, 0, 5),
                (Step::D, 0, 5),
                (Step::E, 0, 5),
                (Step::F, 1, 5),
            ],
        };
        let positions = notes
            .iter()
            .map(|&(step, alter, octave)| {
                Position::new(vec![
                    Note::new(PitchSpelling::new(step, alter, octave)).unwrap(),
                ])
                .unwrap()
            })
            .collect();
        Measure::new(MeasureNumber::new(number).unwrap(), positions)
    }

    fn bar(number: u32) -> MeasureNumber {
        MeasureNumber::new(number).unwrap()
    }

    /// A Minuet bar with its written rhythm: bars 1 and 3 are a quarter then
    /// four eighths, bar 2 three quarters; every bar is three quarters long.
    fn minuet_bar_in_time(number: u32) -> Measure {
        let beats: &[u32] = if number == 2 {
            &[0, 480, 960]
        } else {
            &[0, 480, 720, 960, 1200]
        };
        let bar = minuet_bar(number);
        let positions = bar
            .positions()
            .iter()
            .zip(beats)
            .map(|(position, &beat)| position.clone().at_beat(Some(beat)))
            .collect();
        Measure::new(bar.number(), positions).lasting(Some(1440))
    }

    #[test]
    fn the_written_gap_follows_the_rhythm_across_barlines_and_repeats() {
        let score = score_of(vec![
            minuet_bar_in_time(1),
            minuet_bar_in_time(2),
            minuet_bar_in_time(3),
        ])
        .played_as(vec![bar(1), bar(2), bar(1), bar(2), bar(3)])
        .unwrap();
        let reference = ReferencePerformance::derived_from(&score);

        assert_eq!(reference.written_gap(0), None);
        // D5 is a quarter, G4 an eighth.
        assert_eq!(reference.written_gap(1), Some(480));
        assert_eq!(reference.written_gap(2), Some(240));
        // C5, the last eighth of bar 1, to bar 2's D5.
        assert_eq!(reference.written_gap(5), Some(240));
        // Bar 2's last G4, a quarter, back to bar 1's D5 at the repeat.
        assert_eq!(reference.written_gap(8), Some(480));
        assert_eq!(reference.written_gap(16), Some(480));
    }

    #[test]
    fn without_the_rhythm_there_is_no_written_gap() {
        let score = score_of(vec![minuet_bar_in_time(1), minuet_bar(2)]);
        let reference = ReferencePerformance::derived_from(&score);

        assert_eq!(reference.written_gap(1), None);
        assert_eq!(reference.written_gap(6), None);
    }

    // Bars 1-2 are repeated, as the Minuet's first half is: a bar inside the
    // repeat starts at its first occurrence in played order.
    #[test]
    fn a_repeated_bar_starts_at_its_first_occurrence() {
        let score = score_of(vec![minuet_bar(1), minuet_bar(2), minuet_bar(3)])
            .played_as(vec![bar(1), bar(2), bar(1), bar(2), bar(3)])
            .unwrap();
        let reference = ReferencePerformance::derived_from(&score);

        assert_eq!(reference.first_position_of(bar(1)), Some(0));
        assert_eq!(reference.first_position_of(bar(2)), Some(5));
        assert_eq!(reference.first_position_of(bar(3)), Some(16));
    }

    #[test]
    fn a_bar_with_nothing_to_strike_starts_at_the_next_one() {
        let empty = Measure::new(bar(2), Vec::new());
        let score = score_of(vec![minuet_bar(1), empty, minuet_bar(3)]);
        let reference = ReferencePerformance::derived_from(&score);

        assert_eq!(reference.first_position_of(bar(2)), Some(5));
        assert_eq!(reference.at(5).unwrap().measure(), bar(3));
    }

    #[test]
    fn a_bar_that_does_not_exist_or_ends_the_piece_empty_has_no_start() {
        let empty = Measure::new(bar(2), Vec::new());
        let reference = ReferencePerformance::derived_from(&score_of(vec![minuet_bar(1), empty]));

        assert_eq!(reference.first_position_of(bar(2)), None);
        assert_eq!(reference.first_position_of(bar(9)), None);
    }

    // A note held over the barline into bar 2 is part of bar 2's first step
    // only when an attempt starts there; played through, it is not struck
    // again.
    #[test]
    fn a_note_tied_into_a_bar_is_required_only_when_starting_there() {
        let held = Note::new(PitchSpelling::new(Step::C, 0, 5)).unwrap();
        let score = score_of(vec![minuet_bar(1), minuet_bar(2).holding(vec![held])]);
        let reference = ReferencePerformance::derived_from(&score);
        let first_of_bar_2 = reference.at(5).unwrap();

        assert_eq!(first_of_bar_2.held_into(), &[held]);
        assert_eq!(first_of_bar_2.notes().len(), 1);
        let started: Vec<u8> = first_of_bar_2
            .starting_here()
            .sounding()
            .map(u8::from)
            .collect();
        assert_eq!(started, [74, 72]);
        assert!(reference.at(6).unwrap().held_into().is_empty());
    }

    #[test]
    fn a_held_note_also_struck_at_the_start_is_asked_for_once() {
        let held = Note::new(PitchSpelling::new(Step::D, 0, 5)).unwrap();
        let score = score_of(vec![minuet_bar(1), minuet_bar(2).holding(vec![held])]);
        let reference = ReferencePerformance::derived_from(&score);

        assert_eq!(reference.at(5).unwrap().starting_here().notes().len(), 1);
    }

    // The Minuet's first half is repeated: a loop that runs into bar 3 can
    // only be the second time through bar 2.
    #[test]
    fn a_loop_is_the_first_place_its_bars_are_played_in_a_row() {
        let score = score_of(vec![minuet_bar(1), minuet_bar(2), minuet_bar(3)])
            .played_as(vec![bar(1), bar(2), bar(1), bar(2), bar(3)])
            .unwrap();
        let reference = ReferencePerformance::derived_from(&score);

        assert_eq!(reference.range_of(bar(1), bar(2)), Some(0..8));
        assert_eq!(reference.range_of(bar(2), bar(2)), Some(5..8));
        assert_eq!(reference.range_of(bar(2), bar(3)), Some(13..21));
        assert_eq!(reference.range_of(bar(3), bar(1)), Some(8..21));
        assert_eq!(reference.range_of(bar(1), bar(9)), None);
    }

    #[test]
    fn a_loop_of_nothing_to_strike_is_no_loop() {
        let empty = Measure::new(bar(2), Vec::new());
        let reference = ReferencePerformance::derived_from(&score_of(vec![
            minuet_bar(1),
            empty,
            minuet_bar(3),
        ]));

        assert_eq!(reference.range_of(bar(2), bar(2)), None);
        assert_eq!(reference.range_of(bar(2), bar(3)), Some(5..10));
    }
}
