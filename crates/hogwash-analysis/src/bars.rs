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
    /// Time spent on the timed steps that were not breaks, and the written
    /// length of those same steps in ticks: together, the tempo the bar was
    /// played at.
    pub played_us: u64,
    pub written_ticks: u64,
}

impl Bar {
    /// How hard this bar was, in thousandths, per pass: seconds of pause,
    /// plus one per wrong note, plus two per time given up. Zero with no
    /// passes. A total per pass, not per note: a dense bar with several slips
    /// does need the work. Placed on [`Bar::level`]'s scale, never shown.
    #[must_use]
    pub fn difficulty(&self) -> u64 {
        let total =
            self.pause_us / 1000 + 1000 * u64::from(self.wrong) + 2000 * u64::from(self.gave_up);
        total.checked_div(u64::from(self.passes)).unwrap_or(0)
    }

    /// How hard this bar was on one fixed scale, the same for every piece
    /// and session (D10):
    ///
    /// - 0: no wrong notes, never given up in, and under [`CLEAN_PAUSE_US`]
    ///   of pause per pass
    /// - otherwise 1 to 4 by [`difficulty`](Bar::difficulty) against
    ///   [`LEVELS`]: under 2 is 1, under 5 is 2, under 10 is 3, else 4.
    ///
    /// A description of what was played, whatever the passes; for a
    /// judgement, see [`Bar::judged`].
    #[must_use]
    pub fn level(&self) -> u8 {
        let clean = self.wrong == 0
            && self.gave_up == 0
            && self
                .pause_us
                .checked_div(u64::from(self.passes))
                .unwrap_or(0)
                < CLEAN_PAUSE_US;
        if clean {
            return 0;
        }
        let difficulty = self.difficulty();
        1 + LEVELS
            .iter()
            .map(|&at| u8::from(difficulty >= at))
            .sum::<u8>()
    }

    /// [`Bar::level`], when there is enough to judge by: at least two
    /// passes. `None` is "not enough yet".
    #[must_use]
    pub fn judged(&self) -> Option<u8> {
        (self.passes >= 2).then(|| self.level())
    }

    /// The tempo the bar was played at, in quarter notes per minute, if any
    /// of it was timed.
    #[must_use]
    pub fn tempo(&self) -> Option<u64> {
        let quarter_us = self
            .played_us
            .checked_mul(u64::from(TICKS_PER_QUARTER))?
            .checked_div(self.written_ticks)?;
        60_000_000_u64.checked_div(quarter_us)
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
        self.played_us += other.played_us;
        self.written_ticks += other.written_ticks;
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
        Self::from_trace(&Trace::of(observed, reference, steps, gave_up))
    }

    /// The stats of the attempt `trace` follows.
    #[must_use]
    pub fn from_trace(trace: &Trace) -> Self {
        let mut bars = BTreeMap::<MeasureNumber, Bar>::new();

        if let Some(measure) = trace.gave_up_in {
            bars.entry(measure).or_default().gave_up += 1;
        }

        // A pass begins wherever the bar changes or the steps jump.
        let mut previous: Option<&StepTrace> = None;
        for step in &trace.steps {
            let continues = previous.is_some_and(|before| {
                before.step + 1 == step.step && before.measure == step.measure
            });
            if !continues {
                bars.entry(step.measure).or_default().passes += 1;
            }
            previous = Some(step);
        }

        for step in &trace.steps {
            let bar = bars.entry(step.measure).or_default();
            bar.wrong += u32::try_from(step.wrong.len())
                .unwrap_or(u32::MAX)
                .min(SEARCHING);
            let Some(timing) = step.timing else {
                continue;
            };
            bar.timed += 1;
            if timing.is_break {
                bar.breaks += 1;
                continue;
            }
            bar.played_us += timing.taken_us;
            bar.written_ticks += timing.written_ticks;
            let pause = timing.pause_us();
            bar.pause_us += pause;
            bar.longest_pause_us = bar.longest_pause_us.max(pause);
            if step.opens_bar {
                bar.into_bar_pause_us += pause;
            }
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

    /// Every bar added into one: overall tempo, wrong notes, passes.
    #[must_use]
    pub fn total(&self) -> Bar {
        let mut total = Bar::default();
        for bar in self.0.values() {
            total.add(bar);
        }
        total
    }

    /// How even the tempo was from bar to bar, in percent: 100 less the
    /// spread of the bars' tempos as a percentage of their average. Bars
    /// with fewer than two timed steps say too little to count; with fewer
    /// than two such bars, `None`.
    #[must_use]
    pub fn steadiness(&self) -> Option<u32> {
        let tempos: Vec<u64> = self
            .0
            .values()
            .filter(|bar| bar.timed >= 2)
            .filter_map(Bar::tempo)
            .collect();
        if tempos.len() < 2 {
            return None;
        }
        let n = u64::try_from(tempos.len()).ok()?;
        let mean = tempos.iter().sum::<u64>() / n;
        let variance = tempos
            .iter()
            .map(|&tempo| tempo.abs_diff(mean).pow(2))
            .sum::<u64>()
            / n;
        let spread = variance.isqrt() * 100 / mean.max(1);
        u32::try_from(100_u64.saturating_sub(spread)).ok()
    }

    /// Every bar reached, in bar order.
    pub fn iter(&self) -> impl Iterator<Item = (MeasureNumber, &Bar)> {
        self.0.iter().map(|(measure, bar)| (*measure, bar))
    }

    /// Where each bar stands now, from per-session stats oldest first:
    /// each bar's own last three sessions in which it was played, added up.
    /// A bar left alone lately keeps its last three, however long ago.
    #[must_use]
    pub fn now(per_session: &[Self]) -> Self {
        let mut now = BTreeMap::<MeasureNumber, (u8, Bar)>::new();
        for session in per_session.iter().rev() {
            for (measure, bar) in session.iter().filter(|(_, bar)| bar.passes > 0) {
                let (sessions, total) = now.entry(measure).or_default();
                if *sessions < RECENT_SESSIONS {
                    *sessions += 1;
                    total.add(bar);
                }
            }
        }
        Self(
            now.into_iter()
                .map(|(measure, (_, bar))| (measure, bar))
                .collect(),
        )
    }
}

/// Under this much pause per pass, with no wrong notes and no giving up, a
/// bar went fine: level 0.
const CLEAN_PAUSE_US: u64 = 500_000;

/// The [`Bar::difficulty`] (thousandths per pass) at which levels 2, 3 and
/// 4 begin. Provisional, calibrated 26 Oct 2026 on real history.
const LEVELS: [u64; 3] = [2_000, 5_000, 10_000];

/// How many of a bar's sessions say where it stands now.
const RECENT_SESSIONS: u8 = 3;

/// One attempt, step by step: what the coach and the bar stats are both
/// worked out from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Trace {
    /// Every step the performer was ever waiting at, in order.
    pub steps: Vec<StepTrace>,
    /// The bar the attempt was given up in (restart or stop), if it was
    /// given up after something was played there.
    pub gave_up_in: Option<MeasureNumber>,
}

/// One step of an attempt.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StepTrace {
    pub step: usize,
    pub measure: MeasureNumber,
    /// The first step of its bar, coming from the bar before.
    pub opens_bar: bool,
    /// How long it took to reach, when that could be timed.
    pub timing: Option<Timing>,
    /// Each wrong key played while the cursor waited here, as how far it
    /// was from the nearest written note, in keys (semitones): -1 is the key
    /// just below. Grace notes and ornaments are not wrong.
    pub wrong: Vec<i8>,
}

/// How long one step took against what the music asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timing {
    /// From the step before to this one.
    pub taken_us: u64,
    /// What the written rhythm asks for, at the performer's pace.
    pub expected_us: u64,
    /// The written gap, in ticks.
    pub written_ticks: u64,
    /// Long enough to be a break, not a pause.
    pub is_break: bool,
}

impl Timing {
    /// Time beyond what the music asks: half as long again is still just
    /// playing.
    #[must_use]
    pub const fn pause_us(&self) -> u64 {
        if self.is_break {
            0
        } else {
            self.taken_us.saturating_sub(self.expected_us * 3 / 2)
        }
    }
}

impl Trace {
    /// Follows one attempt that began at `steps.start` and could go no
    /// further than `steps.end`, with the same Cursor as Analysis and the
    /// screen. `gave_up` is whether it ended by restarting or stopping.
    #[must_use]
    pub fn of(
        observed: &ObservedPerformance,
        reference: &ReferencePerformance,
        steps: Range<usize>,
        gave_up: bool,
    ) -> Self {
        let measure_of = |step: usize| reference.at(step).map(ExpectedNotes::measure);

        // When each step was completed, and which steps the performer was
        // ever waiting at.
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
        let gave_up_in = (gave_up && visited.contains(&stopped))
            .then(|| measure_of(stopped))
            .flatten();

        let timings: BTreeMap<usize, Timing> = gaps(reference, &completed, steps.start)
            .into_iter()
            .map(|gap| {
                let is_break = gap.taken > BREAK_FLOOR_US.max(BREAK_FACTOR * gap.expected);
                let timing = Timing {
                    taken_us: gap.taken,
                    expected_us: gap.expected,
                    written_ticks: gap.written,
                    is_break,
                };
                (gap.step, timing)
            })
            .collect();

        let mut wrong: BTreeMap<usize, Vec<i8>> = BTreeMap::new();
        for finding in Analysis::over(observed, reference, steps).findings() {
            if let Finding::IncorrectPitch {
                at,
                played,
                expected,
            } = finding
            {
                let offset = expected
                    .iter()
                    .map(|note| i16::from(u8::from(*played)) - i16::from(u8::from(note.sounding())))
                    .min_by_key(|offset| offset.unsigned_abs())
                    .and_then(|offset| i8::try_from(offset).ok())
                    .unwrap_or(i8::MAX);
                wrong.entry(at.position()).or_default().push(offset);
            }
        }

        let steps = visited
            .iter()
            .chain(timings.keys())
            .copied()
            .collect::<BTreeSet<usize>>()
            .into_iter()
            .filter_map(|step| {
                let measure = measure_of(step)?;
                Some(StepTrace {
                    step,
                    measure,
                    opens_bar: step
                        .checked_sub(1)
                        .is_some_and(|before| measure_of(before) != Some(measure)),
                    timing: timings.get(&step).copied(),
                    wrong: wrong.remove(&step).unwrap_or_default(),
                })
            })
            .collect();

        Self { steps, gave_up_in }
    }
}

/// One timed step: its gap since the step before and what the music asks
/// of that gap, in ticks and in µs at the performer's pace.
struct Gap {
    step: usize,
    taken: u64,
    written: u64,
    expected: u64,
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
) -> Vec<Gap> {
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
        .map(|(step, taken, written)| Gap {
            step,
            taken,
            written,
            expected: written * pace / u64::from(TICKS_PER_QUARTER),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use hogwash_domain::MeasureNumber;

    use hogwash_notation::ExpectedNotes;

    use crate::bars::{Bar, Bars};
    use crate::fixtures::{minuet_in_time, played_in_time};

    const QUARTER_US: u64 = 600_000;

    fn bar(number: u32) -> MeasureNumber {
        MeasureNumber::new(number).expect("a bar number")
    }

    // Two runs; the second stalls 12 s into one step. The repeat plays
    // that bar twice a run, so it is 3 s of pause per pass: level 2. Every
    // other bar went fine.
    #[test]
    fn a_bar_stalled_in_is_the_weakest_and_the_rest_went_fine() {
        let reference = minuet_in_time();
        let all = 0..reference.len();
        let stalled = all.end / 2;
        let mut bars = Bars::default();
        for late in [&[][..], &[(stalled, 12_000_000)]] {
            let observed = played_in_time(&reference, all.clone(), QUARTER_US, late);
            bars.add(&Bars::of(&observed, &reference, all.clone(), false));
        }
        let weak = reference.at(stalled).expect("a step").measure();

        assert_eq!(
            bars.get(weak).map(|bar| (bar.passes, bar.judged())),
            Some((4, Some(2)))
        );
        assert!(
            bars.iter()
                .all(|(measure, bar)| measure == weak || bar.judged() == Some(0))
        );
    }

    #[test]
    fn a_bar_played_once_has_a_level_but_is_not_judged() {
        let reference = minuet_in_time();
        let all = 0..reference.len();
        let steady = Bars::of(
            &played_in_time(&reference, all.clone(), QUARTER_US, &[]),
            &reference,
            all.clone(),
            false,
        );
        let (once, _) = steady
            .iter()
            .find(|(_, bar)| bar.passes == 1)
            .expect("a bar played once");
        let stall = all
            .clone()
            .find(|&step| reference.at(step).map(ExpectedNotes::measure) == Some(once))
            .expect("its first step");
        let observed = played_in_time(&reference, all.clone(), QUARTER_US, &[(stall, 3_000_000)]);

        let bars = Bars::of(&observed, &reference, all, false);

        let bar = bars.get(once).expect("played");
        assert!(bar.level() > 0);
        assert_eq!(bar.judged(), None);
    }

    fn played(passes: u32, pause_ms: u64, wrong: u32, gave_up: u32) -> Bar {
        Bar {
            passes,
            pause_us: pause_ms * 1000,
            wrong,
            gave_up,
            ..Bar::default()
        }
    }

    #[test]
    fn levels_are_one_scale_for_every_bar() {
        assert_eq!(played(1, 499, 0, 0).level(), 0);
        assert_eq!(played(1, 500, 0, 0).level(), 1);
        // One wrong note in four passes is little, but not clean.
        assert_eq!(played(4, 0, 1, 0).level(), 1);
        assert_eq!(played(1, 0, 0, 1).level(), 2);
        for (pause_ms, level) in [
            (1_999, 1),
            (2_000, 2),
            (4_999, 2),
            (5_000, 3),
            (9_999, 3),
            (10_000, 4),
        ] {
            assert_eq!(played(1, pause_ms, 0, 0).level(), level, "{pause_ms} ms");
        }
        // Per pass: 12 s over 4 passes is 3 s each.
        assert_eq!(played(4, 12_000, 0, 0).level(), 2);
        assert_eq!(Bar::default().level(), 0);
    }

    // Bar 1 is played in sessions 1, 2 and 4; bar 2 in all five. Each
    // bar's own last three count.
    #[test]
    fn now_is_each_bars_own_last_three_sessions() {
        let session = |first: Option<Bar>, second: Bar| {
            Bars(
                first
                    .map(|first| (bar(1), first))
                    .into_iter()
                    .chain([(bar(2), second)])
                    .collect(),
            )
        };
        let sessions = [
            session(Some(played(1, 1_000, 0, 0)), played(1, 1, 0, 0)),
            session(Some(played(1, 2_000, 0, 0)), played(1, 10, 0, 0)),
            session(None, played(1, 100, 0, 0)),
            session(Some(played(1, 4_000, 0, 0)), played(1, 1_000, 0, 0)),
            session(None, played(1, 10_000, 0, 0)),
        ];

        let now = Bars::now(&sessions);

        let first = now.get(bar(1)).expect("bar 1");
        assert_eq!((first.passes, first.pause_us), (3, 7_000_000));
        let second = now.get(bar(2)).expect("bar 2");
        assert_eq!((second.passes, second.pause_us), (3, 11_100_000));
        assert_eq!(Bars::now(&[]), Bars::default());
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

    // A quarter every 600 ms is 100 quarters a minute, in every bar.
    #[test]
    fn each_bar_knows_the_tempo_it_was_played_at() {
        let reference = minuet_in_time();
        let observed = played_in_time(&reference, 0..reference.len(), QUARTER_US, &[]);

        let bars = Bars::of(&observed, &reference, 0..reference.len(), false);

        assert!(bars.iter().all(|(_, bar)| bar.tempo() == Some(100)));
    }

    #[test]
    fn an_even_tempo_is_fully_steady_and_a_hesitation_is_not() {
        let reference = minuet_in_time();
        let even = played_in_time(&reference, 0..reference.len(), QUARTER_US, &[]);
        let halting = played_in_time(
            &reference,
            0..reference.len(),
            QUARTER_US,
            &[(16, 2_000_000)],
        );

        let even = Bars::of(&even, &reference, 0..reference.len(), false);
        let halting = Bars::of(&halting, &reference, 0..reference.len(), false);

        assert_eq!(even.steadiness(), Some(100));
        assert!(
            halting
                .steadiness()
                .is_some_and(|steadiness| steadiness < 80)
        );
        assert_eq!(Bars::default().steadiness(), None);
        assert_eq!(even.total().tempo(), Some(100));
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
