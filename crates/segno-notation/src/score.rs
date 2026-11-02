//! The Score — the notated representation of a Piece.

use segno_domain::MeasureNumber;

use crate::Measure;

/// The notated representation of a Piece — the written music a performer
/// practises from.
///
/// A Score is fixed once it exists. The design puts it first among the
/// structural invariants: *"Written music is never altered. [...] Nothing a
/// performer does while practising modifies what they practise from."* That
/// is carried by the type rather than by discipline — there is no `&mut`
/// accessor anywhere in this crate, and the fields are private, so a Score
/// cannot be edited after construction.
///
/// A Score does not know how it is drawn, and does not know the file it
/// arrived in (the design Non-Responsibilities).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Score {
    measures: Vec<Measure>,
    /// The bars in the order they are played, repeats written out. Empty
    /// means written order.
    played: Vec<MeasureNumber>,
}

impl Score {
    /// The Score written as `measures`, or `None` if they do not form a
    /// well-formed division of a Piece.
    ///
    /// Rejects three things, each from a stated invariant:
    ///
    /// - **No measures at all.** The design says a Score is "divided
    ///   completely into Measures"; a division into none is the absence of
    ///   notation.
    /// - **Not starting at one.** The design — "Measures are numbered from
    ///   one, following the convention performers use."
    /// - **Numbers that repeat or descend.** The design — "Measures are
    ///   ordered, and their order does not change." A repeated number makes
    ///   "bar four" ambiguous, which defeats the reason Measures exist
    ///   (the design, Feedback locates findings by Measure).
    ///
    /// Gaps are permitted: real editions skip numbers, and the design forbids
    /// altering what the notation says. Ascending is what location requires;
    /// consecutive is not.
    #[must_use]
    pub fn new(measures: Vec<Measure>) -> Option<Self> {
        let first = measures.first()?;
        if first.number() != MeasureNumber::FIRST {
            return None;
        }

        let strictly_ascending = measures.windows(2).all(|pair| match pair {
            [earlier, later] => earlier.number() < later.number(),
            // `windows(2)` yields only pairs; no other shape reaches here.
            _ => false,
        });

        if strictly_ascending {
            Some(Self {
                measures,
                played: Vec::new(),
            })
        } else {
            None
        }
    }

    /// The measures of this Score, in written order.
    #[must_use]
    pub fn measures(&self) -> &[Measure] {
        &self.measures
    }

    /// The same Score, played as `order` says: the bars a performer plays,
    /// in the order they play them, with repeats written out (`1 2 1 2 3`).
    ///
    /// `None` if `order` names a bar the Score does not have. An empty order
    /// means written order.
    #[must_use]
    pub fn played_as(mut self, order: Vec<MeasureNumber>) -> Option<Self> {
        if order.iter().all(|number| self.measure(*number).is_some()) {
            self.played = order;
            Some(self)
        } else {
            None
        }
    }

    /// The measures in the order they are played: written order, unless
    /// [`played_as`](Self::played_as) said otherwise.
    #[must_use]
    pub fn performance(&self) -> Vec<&Measure> {
        if self.played.is_empty() {
            self.measures.iter().collect()
        } else {
            self.played
                .iter()
                .filter_map(|number| self.measure(*number))
                .collect()
        }
    }

    fn measure(&self, number: MeasureNumber) -> Option<&Measure> {
        self.measures
            .binary_search_by_key(&number, Measure::number)
            .ok()
            .and_then(|index| self.measures.get(index))
    }
}

#[cfg(test)]
mod tests {
    use segno_domain::{MeasureNumber, PitchSpelling, Step};

    use crate::{Measure, Note, Position, Score};

    fn note(step: Step, octave: i8) -> Note {
        Note::new(PitchSpelling::new(step, 0, octave)).unwrap()
    }

    fn measure(number: u32, steps: &[Step]) -> Measure {
        let positions = steps
            .iter()
            .map(|&step| Position::new(vec![note(step, 4)]).unwrap())
            .collect();

        Measure::new(MeasureNumber::new(number).unwrap(), positions)
    }

    // "A Score is divided completely into Measures."
    #[test]
    fn a_score_is_divided_into_measures() {
        let score = Score::new(vec![
            measure(1, &[Step::C, Step::D]),
            measure(2, &[Step::E]),
        ])
        .unwrap();

        assert_eq!(score.measures().len(), 2);
    }

    // A Score with no measures is not written music. The design says a Score
    // is "divided completely into Measures", and a division into none is not
    // a division -- it is the absence of notation, which the Library must
    // never hold (the design, "Practice material is always present").
    #[test]
    fn a_score_with_no_measures_is_not_a_score() {
        assert!(Score::new(vec![]).is_none());
    }

    // "Measures are numbered from one, following the convention
    // performers use." Enforced at the Score rather than left to the importer,
    // because Feedback locates findings by Measure and a Score
    // whose first bar is bar seven sends the performer to the wrong place.
    #[test]
    fn a_scores_measures_are_numbered_from_one() {
        let starting_at_two = Score::new(vec![measure(2, &[Step::C])]);

        assert!(starting_at_two.is_none());
    }

    // "Measures are ordered, and their order does not change."
    // Numbers that repeat or go backwards make "bar four" ambiguous, which
    // defeats the only reason Measures are modelled at all.
    #[test]
    fn a_scores_measures_ascend_without_repeating() {
        let repeated = Score::new(vec![measure(1, &[Step::C]), measure(1, &[Step::D])]);
        let backwards = Score::new(vec![
            measure(1, &[Step::C]),
            measure(3, &[Step::D]),
            measure(2, &[Step::E]),
        ]);

        assert!(repeated.is_none());
        assert!(backwards.is_none());
    }

    // Gaps are permitted. Real editions skip numbers -- around repeats, or
    // where an editor numbers a first and second ending -- and the design
    // forbids altering what the notation says. Ascending is what location
    // requires; consecutive is not.
    #[test]
    fn a_score_may_skip_measure_numbers() {
        let with_a_gap = Score::new(vec![measure(1, &[Step::C]), measure(5, &[Step::D])]);

        assert!(with_a_gap.is_some());
    }

    // Repeats: the bars are written once and played as the repeat signs say.
    #[test]
    fn a_score_can_be_played_in_an_order_other_than_written() {
        let bar = |number| MeasureNumber::new(number).unwrap();
        let score = Score::new(vec![
            measure(1, &[Step::C]),
            measure(2, &[Step::D]),
            measure(3, &[Step::E]),
        ])
        .unwrap()
        .played_as(vec![bar(1), bar(2), bar(1), bar(2), bar(3)])
        .unwrap();

        let played: Vec<u32> = score
            .performance()
            .iter()
            .map(|measure| measure.number().get())
            .collect();

        assert_eq!(played, [1, 2, 1, 2, 3]);
        assert_eq!(score.measures().len(), 3, "still written once");
    }

    #[test]
    fn a_score_is_played_in_written_order_unless_told_otherwise() {
        let score = Score::new(vec![measure(1, &[Step::C]), measure(2, &[Step::D])]).unwrap();

        let played: Vec<u32> = score
            .performance()
            .iter()
            .map(|measure| measure.number().get())
            .collect();

        assert_eq!(played, [1, 2]);
    }

    #[test]
    fn a_play_order_naming_a_missing_bar_is_refused() {
        let score = Score::new(vec![measure(1, &[Step::C])]).unwrap();

        assert!(
            score
                .played_as(vec![MeasureNumber::new(2).unwrap()])
                .is_none()
        );
    }

    // "Every Note in a Score belongs to exactly one Measure of
    // it." Guaranteed by construction: a Note reaches a Score only inside a
    // Position inside a Measure, so there is no way to express one that
    // floats free. This counts them to prove nothing is lost or duplicated on
    // the way in.
    #[test]
    fn every_note_belongs_to_exactly_one_measure() {
        let score = Score::new(vec![
            measure(1, &[Step::C, Step::D]),
            measure(2, &[Step::E, Step::F, Step::G]),
        ])
        .unwrap();

        let counted: usize = score
            .measures()
            .iter()
            .flat_map(Measure::positions)
            .map(|position| position.notes().len())
            .sum();

        assert_eq!(counted, 5);
    }
}
