//! What to practise next, after an attempt: one suggestion, never a grade.

use segno_domain::MeasureNumber;
use segno_notation::ReferencePerformance;

use crate::bars::Bars;

/// A bar this hard (per pass) is worth practising: about a second of pause,
/// or one wrong note.
const TROUBLE: u64 = 1000;
/// A trouble spot grows to take in hard neighbours, up to this many bars.
const MOST_BARS: u32 = 4;
/// Fewer timed steps than this, with nothing wrong, is too little to judge.
const ENOUGH_STEPS: u32 = 4;

/// The one thing worth doing next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suggestion {
    /// Too little was played to say anything.
    TooShort,
    /// Nothing stood out: play it again.
    Again,
    /// Loop bars `first` to `last`. `trouble` is the bar that stood out, and
    /// `into` whether the trouble was the change into it, which is why
    /// `first` is then the bar before.
    Loop {
        first: MeasureNumber,
        last: MeasureNumber,
        trouble: MeasureNumber,
        into: bool,
    },
    /// A loop reached its goal: loop one bar wider each side, to join the
    /// spot back up with what is around it.
    Widen {
        first: MeasureNumber,
        last: MeasureNumber,
    },
}

impl Suggestion {
    /// What to practise after an attempt whose stats are `bars`, begun at
    /// bar `start`.
    #[must_use]
    pub fn after(bars: &Bars, start: MeasureNumber, reference: &ReferencePerformance) -> Self {
        let timed: u32 = bars.iter().map(|(_, bar)| bar.timed).sum();
        let wrong: u32 = bars.iter().map(|(_, bar)| bar.wrong).sum();
        if timed < ENOUGH_STEPS && wrong == 0 {
            return Self::TooShort;
        }

        // The hardest bar; the earliest of equals, so the choice is stable.
        let Some((trouble, _)) = bars
            .iter()
            .filter(|(_, bar)| bar.difficulty() >= TROUBLE)
            .max_by_key(|(measure, bar)| (bar.difficulty(), std::cmp::Reverse(*measure)))
        else {
            return Self::Again;
        };

        let hard = |measure: MeasureNumber| {
            bars.get(measure)
                .is_some_and(|bar| bar.difficulty() >= TROUBLE)
        };
        let (mut first, mut last) = (trouble, trouble);
        while last.get() - first.get() + 1 < MOST_BARS {
            if let Some(before) = reference.bar_before(first).filter(|&before| hard(before)) {
                first = before;
            } else if let Some(after) = reference.bar_after(last).filter(|&after| hard(after)) {
                last = after;
            } else {
                break;
            }
        }

        // Trouble mostly on the hardest bar's first note is trouble with the
        // change into it: the bar before belongs in the loop, unless the
        // attempt began there and there was no change to make.
        let hardest = bars.get(trouble).copied().unwrap_or_default();
        let into = trouble != start
            && hardest.pause_us > 0
            && 2 * hardest.into_bar_pause_us >= hardest.pause_us;
        let widened = reference
            .bar_before(first)
            .filter(|_| into && first == trouble && last.get() - first.get() + 1 < MOST_BARS)
            .unwrap_or(first);

        // A loop has to be playable in a row (not across a repeat); the
        // trouble bar on its own always is.
        let (first, last) = [(widened, last), (first, last)]
            .into_iter()
            .find(|&(first, last)| reference.range_of(first, last).is_some())
            .unwrap_or((trouble, trouble));
        Self::Loop {
            first,
            last,
            trouble,
            into,
        }
    }

    /// What to practise once a loop over `first` to `last` reached its goal:
    /// the same bars and one more each side, where the piece has them and
    /// they can be played in a row.
    #[must_use]
    pub fn after_the_goal(
        first: MeasureNumber,
        last: MeasureNumber,
        reference: &ReferencePerformance,
    ) -> Self {
        let wider = |first: MeasureNumber, last: MeasureNumber| {
            reference
                .range_of(first, last)
                .is_some()
                .then_some((first, last))
        };
        let before = reference
            .bar_before(first)
            .filter(|&bar| reference.first_position_of(bar).is_some());
        let after = reference
            .bar_after(last)
            .filter(|&bar| reference.first_position_of(bar).is_some());

        let (first, last) = wider(before.unwrap_or(first), after.unwrap_or(last))
            .or_else(|| wider(before.unwrap_or(first), last))
            .or_else(|| wider(first, after.unwrap_or(last)))
            .unwrap_or((first, last));
        Self::Widen { first, last }
    }
}

#[cfg(test)]
mod tests {
    use segno_domain::{MeasureNumber, Pitch};

    use crate::bars::Bars;
    use crate::fixtures::{minuet_in_time, played_in_time};
    use crate::suggestion::Suggestion;

    const QUARTER_US: u64 = 600_000;

    fn bar(number: u32) -> MeasureNumber {
        MeasureNumber::new(number).expect("a bar number")
    }

    fn whole(late: &[(usize, u64)]) -> Bars {
        let reference = minuet_in_time();
        let observed = played_in_time(&reference, 0..reference.len(), QUARTER_US, late);
        Bars::of(&observed, &reference, 0..reference.len(), false)
    }

    #[test]
    fn a_couple_of_notes_are_too_short_to_judge() {
        let reference = minuet_in_time();
        let observed = played_in_time(&reference, 0..2, QUARTER_US, &[]);
        let bars = Bars::of(&observed, &reference, 0..reference.len(), false);

        assert_eq!(
            Suggestion::after(&bars, bar(1), &reference),
            Suggestion::TooShort
        );
    }

    #[test]
    fn a_clean_run_suggests_playing_it_again() {
        assert_eq!(
            Suggestion::after(&whole(&[]), bar(1), &minuet_in_time()),
            Suggestion::Again
        );
    }

    // Bar 3's second note (step 17), three seconds late: trouble inside the
    // bar, so the loop is the bar itself.
    #[test]
    fn trouble_inside_a_bar_loops_that_bar() {
        assert_eq!(
            Suggestion::after(&whole(&[(17, 3_000_000)]), bar(1), &minuet_in_time()),
            Suggestion::Loop {
                first: bar(3),
                last: bar(3),
                trouble: bar(3),
                into: false,
            }
        );
    }

    // Bar 3's first note (step 16), three seconds late: the change from bar
    // 2 is the trouble, so the loop takes in bar 2 as well.
    #[test]
    fn trouble_going_into_a_bar_loops_it_with_the_bar_before() {
        assert_eq!(
            Suggestion::after(&whole(&[(16, 3_000_000)]), bar(1), &minuet_in_time()),
            Suggestion::Loop {
                first: bar(2),
                last: bar(3),
                trouble: bar(3),
                into: true,
            }
        );
    }

    // A wrong note in bar 2 joins it to the loop, and the long pause going
    // into bar 3 is still named as such.
    #[test]
    fn trouble_going_into_a_bar_is_named_even_when_the_bar_before_joins() {
        let reference = minuet_in_time();
        // Up to bar 2's second time through (step 14), two wrong notes
        // there, then on from there with the pause going into bar 3.
        let mut start = played_in_time(&reference, 0..14, QUARTER_US, &[]);
        start.record(Pitch::LOWEST);
        start.record(Pitch::LOWEST);
        let mut bars = Bars::of(&start, &reference, 0..reference.len(), false);
        let rest = played_in_time(
            &reference,
            14..reference.len(),
            QUARTER_US,
            &[(16, 3_000_000)],
        );
        bars.add(&Bars::of(&rest, &reference, 14..reference.len(), false));

        assert_eq!(
            Suggestion::after(&bars, bar(1), &reference),
            Suggestion::Loop {
                first: bar(2),
                last: bar(3),
                trouble: bar(3),
                into: true,
            }
        );
    }

    // Started at bar 3, there is no change into it to practise.
    #[test]
    fn the_start_bar_is_not_widened_backwards() {
        let reference = minuet_in_time();
        let steps = 16..reference.len();
        let observed = played_in_time(&reference, steps.clone(), QUARTER_US, &[(17, 3_000_000)]);
        let mut bars = Bars::of(&observed, &reference, steps, false);
        let mut hunting = segno_capture::ObservedPerformance::new();
        hunting.record(Pitch::LOWEST);
        bars.add(&Bars::of(&hunting, &reference, 16..reference.len(), false));

        assert!(matches!(
            Suggestion::after(&bars, bar(3), &reference),
            Suggestion::Loop { first, .. } if first == bar(3)
        ));
    }

    // Bar 3's second note three seconds late, and bar 2's second note
    // (step 14, its second time through) late enough to count too.
    #[test]
    fn neighbouring_trouble_joins_the_loop() {
        assert_eq!(
            Suggestion::after(
                &whole(&[(14, 3_000_000), (17, 3_000_000)]),
                bar(1),
                &minuet_in_time()
            ),
            Suggestion::Loop {
                first: bar(2),
                last: bar(3),
                trouble: bar(3),
                into: false,
            }
        );
    }

    #[test]
    fn after_the_goal_the_loop_widens_by_a_bar_each_side() {
        let reference = minuet_in_time();

        assert_eq!(
            Suggestion::after_the_goal(bar(2), bar(2), &reference),
            Suggestion::Widen {
                first: bar(1),
                last: bar(3),
            }
        );
        // Bar 3 ends the piece: only bar 2 can join.
        assert_eq!(
            Suggestion::after_the_goal(bar(3), bar(3), &reference),
            Suggestion::Widen {
                first: bar(2),
                last: bar(3),
            }
        );
    }
}
