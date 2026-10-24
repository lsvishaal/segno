//! Per-bar stats: where in the music an attempt went slowly, or wrong.
//!
//! People don't play like a metronome, so a pause is judged against the
//! performer's own pace in that attempt and the written rhythm: a half note
//! is meant to take twice as long as a quarter, and a slow day is not a
//! hesitant one.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::Arc;

use hogwash_capture::ObservedPerformance;
use hogwash_domain::MeasureNumber;
use hogwash_notation::{ExpectedNotes, ReferencePerformance, TICKS_PER_QUARTER};
use hogwash_practice::Cursor;

use crate::{Analysis, Finding};

/// A gap is a break (the phone, a sip of water), not a pause, when it is
/// longer than this many times what the music asks for at the performer's
/// pace…
const BREAK_FACTOR: u64 = 10;
/// …and longer than this.
const BREAK_FLOOR_US: u64 = 20_000_000;

/// Hunting around the keys for one note is one problem, not dozens: at most
/// this many wrong notes count while the cursor waits at one step.
const SEARCHING: u32 = 3;

/// What happened in one bar, over one attempt or many added together.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Bar {
    /// How many times the bar was played into. A repeat's bars count once
    /// per time through.
    pub passes: u32,
    /// How many of its steps had a gap that could be timed.
    pub timed: u32,
    /// Time spent beyond what the music asks for, at the performer's pace.
    pub pause_us: u64,
    /// The longest single such pause.
    pub longest_pause_us: u64,
    /// The part of `pause_us` spent on the bar's first note: the change
    /// from the bar before.
    pub into_bar_pause_us: u64,
    /// Gaps long enough to be breaks. Counted, but not as pauses.
    pub breaks: u32,
    /// Wrong notes played while the cursor waited in this bar, at most
    /// three per note waited for. Grace notes and ornaments are not wrong.
    pub wrong: u32,
    /// Attempts given up (restart or stop) while in this bar.
    pub gave_up: u32,
}

impl Bar {
    /// How hard this bar was, in thousandths, per pass: seconds of pause,
    /// plus one per wrong note, plus two per time given up. Zero with no
    /// passes. Only ever compared with other bars, never shown.
    #[must_use]
    pub fn difficulty(&self) -> u64 {
        let total =
            self.pause_us / 1000 + 1000 * u64::from(self.wrong) + 2000 * u64::from(self.gave_up);
        total.checked_div(u64::from(self.passes)).unwrap_or(0)
    }

    fn add(&mut self, other: &Self) {
        self.passes += other.passes;
        self.timed += other.timed;
        self.pause_us += other.pause_us;
        self.longest_pause_us = self.longest_pause_us.max(other.longest_pause_us);
        self.into_bar_pause_us += other.into_bar_pause_us;
        self.breaks += other.breaks;
        self.wrong += other.wrong;
        self.gave_up += other.gave_up;
    }
}

/// Per-bar stats, by written bar number. Bars never reached are absent:
/// no data is not the same as easy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Bars(BTreeMap<MeasureNumber, Bar>);

impl Bars {
    /// The stats of one attempt that began at `steps.start` and could go no
    /// further than `steps.end` (a loop pass, or the whole piece), compared
    /// against `reference`. `gave_up` is whether it ended by restarting or
    /// stopping before the end.
    #[must_use]
    pub fn of(
        observed: &ObservedPerformance,
        reference: &ReferencePerformance,
        steps: Range<usize>,
        gave_up: bool,
    ) -> Self {
        let mut bars = BTreeMap::<MeasureNumber, Bar>::new();
        let measure_of = |step: usize| reference.at(step).map(ExpectedNotes::measure);

        // The same replay as Analysis and the screen: when each step was
        // completed, and which steps the performer was ever waiting at.
        let mut cursor = Cursor::starting_at(Arc::new(reference.clone()), steps.start);
        let mut completed: BTreeMap<usize, Option<u64>> = BTreeMap::new();
        let mut visited = BTreeSet::new();
        for (index, &pitch) in observed.played().iter().enumerate() {
            let at = cursor.position();
            if at >= steps.end {
                break;
            }
            visited.insert(at);
            cursor.observe(pitch);
            let time = observed.touches().get(index).copied().flatten();
            for step in at..cursor.position() {
                completed.insert(step, time.map(|touch| touch.at_us));
            }
        }
        // Given up in a bar only if it was begun: stopping just after a
        // bar's last note gives up on nothing.
        let stopped = cursor.position();
        if gave_up
            && visited.contains(&stopped)
            && let Some(measure) = measure_of(stopped)
        {
            bars.entry(measure).or_default().gave_up += 1;
        }

        // A pass begins wherever the bar changes or the steps jump.
        let mut previous: Option<usize> = None;
        for &step in &visited {
            let Some(measure) = measure_of(step) else {
                continue;
            };
            let continues = previous
                .is_some_and(|before| before + 1 == step && measure_of(before) == Some(measure));
            if !continues {
                bars.entry(measure).or_default().passes += 1;
            }
            previous = Some(step);
        }

        for (step, gap, expected) in gaps(reference, &completed, steps.start) {
            let Some(measure) = measure_of(step) else {
                continue;
            };
            let bar = bars.entry(measure).or_default();
            bar.timed += 1;
            if gap > BREAK_FLOOR_US.max(BREAK_FACTOR * expected) {
                bar.breaks += 1;
                continue;
            }
            // Half as long again as the music asks is still just playing.
            let pause = gap.saturating_sub(expected * 3 / 2);
            bar.pause_us += pause;
            bar.longest_pause_us = bar.longest_pause_us.max(pause);
            if measure_of(step - 1) != Some(measure) {
                bar.into_bar_pause_us += pause;
            }
        }

        let mut wrong_at = BTreeMap::<usize, (MeasureNumber, u32)>::new();
        for finding in Analysis::over(observed, reference, steps).findings() {
            if let Finding::IncorrectPitch { at, .. } = finding {
                wrong_at.entry(at.position()).or_insert((at.measure(), 0)).1 += 1;
            }
        }
        for (measure, wrong) in wrong_at.into_values() {
            bars.entry(measure).or_default().wrong += wrong.min(SEARCHING);
        }

        Self(bars)
    }

    /// Adds `other`'s stats to these: another attempt, or another session.
    pub fn add(&mut self, other: &Self) {
        for (measure, bar) in &other.0 {
            self.0.entry(*measure).or_default().add(bar);
        }
    }

    /// The stats of bar `measure`, if it was ever reached.
    #[must_use]
    pub fn get(&self, measure: MeasureNumber) -> Option<&Bar> {
        self.0.get(&measure)
    }

    /// Every bar reached, in bar order.
    pub fn iter(&self) -> impl Iterator<Item = (MeasureNumber, &Bar)> {
        self.0.iter().map(|(measure, bar)| (*measure, bar))
    }
}

/// Each timed step after the first: its gap since the step before, and how
/// long the music asks that gap to be at the performer's pace, both in µs.
///
/// The pace is the attempt's median time per quarter note, so one long
/// hesitation doesn't make every other note look rushed. Without a written
/// rhythm every step counts as a quarter.
fn gaps(
    reference: &ReferencePerformance,
    completed: &BTreeMap<usize, Option<u64>>,
    from: usize,
) -> Vec<(usize, u64, u64)> {
    let timed: Vec<(usize, u64, u64)> = completed
        .iter()
        .filter(|&(&step, _)| step > from)
        .filter_map(|(&step, &time)| {
            let before = completed.get(&(step - 1)).copied().flatten()?;
            let gap = time?.checked_sub(before)?;
            let written = u64::from(reference.written_gap(step).unwrap_or(TICKS_PER_QUARTER));
            (written > 0).then_some((step, gap, written))
        })
        .collect();

    let mut per_quarter: Vec<u64> = timed
        .iter()
        .map(|&(_, gap, written)| gap * u64::from(TICKS_PER_QUARTER) / written)
        .collect();
    per_quarter.sort_unstable();
    let Some(&pace) = per_quarter.get(per_quarter.len() / 2) else {
        return Vec::new();
    };

    timed
        .into_iter()
        .map(|(step, gap, written)| (step, gap, written * pace / u64::from(TICKS_PER_QUARTER)))
        .collect()
}

#[cfg(test)]
mod tests {
    use hogwash_domain::MeasureNumber;

    use crate::bars::Bars;
    use crate::fixtures::{minuet_in_time, played_in_time};

    const QUARTER_US: u64 = 600_000;

    fn bar(number: u32) -> MeasureNumber {
        MeasureNumber::new(number).expect("a bar number")
    }

    #[test]
    fn steady_playing_has_no_pauses() {
        let reference = minuet_in_time();
        let observed = played_in_time(&reference, 0..reference.len(), QUARTER_US, &[]);

        let bars = Bars::of(&observed, &reference, 0..reference.len(), false);

        assert!(
            bars.iter()
                .all(|(_, bar)| bar.pause_us == 0 && bar.wrong == 0)
        );
    }

    #[test]
    fn playing_at_half_the_speed_is_not_hesitating() {
        let reference = minuet_in_time();
        let observed = played_in_time(&reference, 0..reference.len(), 2 * QUARTER_US, &[]);

        let bars = Bars::of(&observed, &reference, 0..reference.len(), false);

        assert!(bars.iter().all(|(_, bar)| bar.pause_us == 0));
    }

    // Bar 2's three quarters against bar 1's eighths: the written rhythm,
    // not the number of notes, says how long each gap should be.
    #[test]
    fn a_long_written_note_is_not_a_pause() {
        let reference = minuet_in_time();
        let observed = played_in_time(&reference, 0..reference.len(), QUARTER_US, &[]);

        let bars = Bars::of(&observed, &reference, 0..reference.len(), false);

        assert_eq!(bars.get(bar(2)).map(|bar| bar.timed), Some(6));
        assert_eq!(bars.get(bar(2)).map(|bar| bar.pause_us), Some(0));
    }

    // Step 16 is bar 3's first note, after the repeat of bars 1-2.
    #[test]
    fn a_hesitation_going_into_a_bar_is_kept_as_such() {
        let reference = minuet_in_time();
        let observed = played_in_time(
            &reference,
            0..reference.len(),
            QUARTER_US,
            &[(16, 4_000_000)],
        );

        let bars = Bars::of(&observed, &reference, 0..reference.len(), false);
        let third = bars.get(bar(3)).expect("bar 3 was played");

        assert!(third.pause_us >= 3_500_000);
        assert_eq!(third.into_bar_pause_us, third.pause_us);
        assert_eq!(bars.get(bar(1)).map(|bar| bar.pause_us), Some(0));
    }

    #[test]
    fn a_long_gap_is_a_break_not_a_pause() {
        let reference = minuet_in_time();
        let observed = played_in_time(
            &reference,
            0..reference.len(),
            QUARTER_US,
            &[(16, 60_000_000)],
        );

        let third = Bars::of(&observed, &reference, 0..reference.len(), false)
            .get(bar(3))
            .copied()
            .expect("bar 3 was played");

        assert_eq!((third.breaks, third.pause_us), (1, 0));
    }

    // The Minuet's first half is repeated: bars 1 and 2 are played twice.
    #[test]
    fn a_repeated_bar_counts_each_time_through() {
        let reference = minuet_in_time();
        let observed = played_in_time(&reference, 0..reference.len(), QUARTER_US, &[]);

        let bars = Bars::of(&observed, &reference, 0..reference.len(), false);

        assert_eq!(bars.get(bar(1)).map(|bar| bar.passes), Some(2));
        assert_eq!(bars.get(bar(3)).map(|bar| bar.passes), Some(1));
    }

    // Restarting in bar 3, three notes in, after one wrong note there.
    #[test]
    fn a_restart_counts_against_the_bar_it_happened_in() {
        let reference = minuet_in_time();
        let mut observed = played_in_time(&reference, 0..19, QUARTER_US, &[]);
        observed.record(hogwash_domain::Pitch::LOWEST);

        let bars = Bars::of(&observed, &reference, 0..reference.len(), true);
        let third = bars.get(bar(3)).expect("bar 3 was played");

        assert_eq!((third.gave_up, third.wrong, third.passes), (1, 1, 1));
        assert!(third.difficulty() > bars.get(bar(1)).expect("played").difficulty());
    }

    // A loop pass over bar 2's second time through only touches bar 2.
    #[test]
    fn a_loop_pass_touches_only_its_bars() {
        let reference = minuet_in_time();
        let pass = 13..16;
        let observed = played_in_time(&reference, pass.clone(), QUARTER_US, &[]);

        let bars = Bars::of(&observed, &reference, pass, false);

        assert_eq!(
            bars.iter().map(|(measure, _)| measure).collect::<Vec<_>>(),
            [bar(2)]
        );
    }

    // Stopped with the cursor waiting at bar 3's first note, nothing played
    // there yet.
    #[test]
    fn stopping_between_bars_gives_up_on_neither() {
        let reference = minuet_in_time();
        let observed = played_in_time(&reference, 0..16, QUARTER_US, &[]);

        let bars = Bars::of(&observed, &reference, 0..reference.len(), true);

        assert_eq!(bars.get(bar(3)), None);
        assert!(bars.iter().all(|(_, bar)| bar.gave_up == 0));
    }

    #[test]
    fn hunting_for_one_note_counts_three_wrong_at_most() {
        let reference = minuet_in_time();
        let mut observed = hogwash_capture::ObservedPerformance::new();
        for _ in 0..20 {
            observed.record(hogwash_domain::Pitch::LOWEST);
        }

        let bars = Bars::of(&observed, &reference, 0..reference.len(), false);

        assert_eq!(bars.get(bar(1)).map(|bar| bar.wrong), Some(3));
    }

    #[test]
    fn without_timestamps_wrong_notes_still_count() {
        let reference = minuet_in_time();
        let mut observed = hogwash_capture::ObservedPerformance::new();
        observed.record(hogwash_domain::Pitch::LOWEST);
        for expected in &reference {
            for pitch in expected.sounding() {
                observed.record(pitch);
            }
        }

        let bars = Bars::of(&observed, &reference, 0..reference.len(), false);
        let first = bars.get(bar(1)).expect("bar 1 was played");

        assert_eq!((first.wrong, first.timed, first.pause_us), (1, 0, 0));
    }

    #[test]
    fn attempts_add_up() {
        let reference = minuet_in_time();
        let observed = played_in_time(
            &reference,
            0..reference.len(),
            QUARTER_US,
            &[(16, 4_000_000)],
        );
        let once = Bars::of(&observed, &reference, 0..reference.len(), false);

        let mut twice = once.clone();
        twice.add(&once);

        let third = twice.get(bar(3)).expect("played");
        assert_eq!(third.passes, 2);
        assert_eq!(
            third.difficulty(),
            once.get(bar(3)).expect("played").difficulty()
        );
    }
}
