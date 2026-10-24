//! The results after playing: what the screen draws once a run stops.
//!
//! A run is everything from pressing Start to stopping: restarts, jumps and
//! loop passes along the way are parts of it. Every number and sentence is
//! worked out here, from the per-bar stats and the analysis; the screen only
//! draws them.

use std::collections::BTreeMap;

use hogwash_analysis::{Analysis, Bar, Bars, Finding, Suggestion};
use hogwash_capture::ObservedPerformance;
use hogwash_domain::MeasureNumber;
use hogwash_feedback::review::{bar_line, headline, label, loop_headline, pass_line};
use hogwash_notation::{ExpectedNotes, ReferencePerformance};
use serde::Serialize;

/// What the screen shows after a run.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    /// The one line of text.
    pub headline: String,
    /// The big numbers.
    pub numbers: Numbers,
    /// One per bar, in bar-number order, from where the run began to the
    /// end of the piece (or of the loop).
    pub bars: Vec<ReviewBar>,
    /// One per completed pass, when the run ended in a loop.
    pub passes: Vec<ReviewPass>,
    /// What happened at each step played: drawn on the score.
    pub marks: Vec<ReviewMark>,
    /// The one thing worth doing next, if anything.
    pub suggestion: Option<Suggested>,
}

/// The numbers at a glance.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Numbers {
    /// Right notes out of every note played, in percent.
    pub accuracy: Option<u32>,
    /// Quarter notes per minute, pauses included.
    pub tempo: Option<u64>,
    /// Whether `tempo` follows the written rhythm, or counts every step as
    /// a quarter because the score's rhythm is unknown.
    pub rhythm: bool,
    /// Evenness of tempo across bars, in percent.
    pub steadiness: Option<u32>,
    pub right: usize,
    pub wrong: usize,
    /// Notes after where the run stopped.
    pub missed: usize,
    pub longest_pause_ms: u64,
    pub longest_pause_bar: Option<u32>,
    /// From the first note to the last, over every part of the run.
    pub time_ms: Option<u64>,
    /// Times begun again (Restart) during the run.
    pub restarts: u32,
}

/// One bar's point on the chart.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewBar {
    pub measure: u32,
    pub state: BarState,
    pub tempo: Option<u64>,
    pub pause_ms: u64,
    pub into_ms: u64,
    pub wrong: u32,
    pub breaks: u32,
    /// Said when pointing at it.
    pub line: String,
}

/// Whether a bar was played in the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BarState {
    Played,
    /// After where the run stopped.
    NotReached,
    /// Nothing to strike in it: rests, or only held notes.
    Empty,
}

/// One completed pass of a loop.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPass {
    pub number: usize,
    pub tempo: Option<u64>,
    pub wrong: u32,
    pub clean: bool,
    pub line: String,
}

/// What happened at one step: reached and played (`correct`), and how many
/// wrong notes came while the cursor waited there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewMark {
    pub position: usize,
    pub correct: bool,
    pub wrong: u32,
}

/// The suggestion's button.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggested {
    pub label: String,
    /// Bars to loop; absent for "play it again".
    pub first: Option<u32>,
    pub last: Option<u32>,
}

/// How an attempt ended, as far as the results care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// A completed pass of a loop.
    Pass,
    /// Begun again by Restart.
    Restart,
    /// Anything else: a jump, Stop, the end, the keyboard lost.
    Other,
}

/// One part of a run: one attempt, as analysed when it ended.
#[derive(Debug, Clone)]
pub struct Segment {
    /// The bar it began in.
    start: MeasureNumber,
    bars: Bars,
    ending: Ending,
    right: usize,
    wrong: usize,
    missed: usize,
    marks: Vec<ReviewMark>,
    span_us: Option<u64>,
}

impl Segment {
    /// The part of a run an attempt that began at step `from` was, given
    /// its analysis and per-bar stats. `None` if nothing was played.
    #[must_use]
    pub fn of(
        reference: &ReferencePerformance,
        from: usize,
        observed: &ObservedPerformance,
        analysis: &Analysis,
        bars: Bars,
        ending: Ending,
    ) -> Option<Self> {
        if observed.is_empty() {
            return None;
        }
        let summary = analysis.summary();
        Some(Self {
            start: reference.at(from).map(ExpectedNotes::measure)?,
            bars,
            ending,
            right: summary.correct(),
            wrong: summary.incorrect() + summary.unexpected(),
            missed: summary.omitted(),
            marks: marks(analysis.findings()),
            span_us: span(observed),
        })
    }
}

/// Everything played since Start, part by part.
#[derive(Debug, Clone, Default)]
pub struct Run {
    pub segments: Vec<Segment>,
}

/// A loop running when the run stopped: its bars, and the part of the run
/// it began at.
#[derive(Debug, Clone, Copy)]
pub struct LoopScope {
    pub first: u32,
    pub last: u32,
    pub since: usize,
}

/// The results of `run`, which stopped with `looping` running, if it did.
#[must_use]
pub fn of(reference: &ReferencePerformance, run: &Run, looping: Option<LoopScope>) -> Review {
    let segments = looping.map_or(&run.segments[..], |scope| {
        run.segments.get(scope.since..).unwrap_or_default()
    });
    let Some(first) = segments.first() else {
        return Review {
            headline: "Nothing played.".to_owned(),
            numbers: Numbers::default(),
            bars: Vec::new(),
            passes: Vec::new(),
            marks: Vec::new(),
            suggestion: None,
        };
    };

    let mut bars = Bars::default();
    for segment in segments {
        bars.add(&segment.bars);
    }
    let start = segments
        .iter()
        .map(|segment| segment.start)
        .min()
        .unwrap_or(first.start);
    let loop_bars = looping.and_then(|scope| Some((bar(scope.first)?, bar(scope.last)?)));
    let (start, end) = loop_bars.unwrap_or_else(|| {
        let end = reference
            .iter()
            .map(ExpectedNotes::measure)
            .max()
            .unwrap_or(start);
        (start, end)
    });

    let passes: Vec<ReviewPass> = segments
        .iter()
        .filter(|segment| looping.is_some() && segment.ending == Ending::Pass)
        .enumerate()
        .map(|(index, segment)| {
            let total = segment.bars.total();
            let clean = segment.wrong == 0;
            ReviewPass {
                number: index + 1,
                tempo: total.tempo(),
                wrong: total.wrong,
                clean,
                line: pass_line(index + 1, &total, clean),
            }
        })
        .collect();

    let suggestion = Suggestion::after(&bars, start, reference);
    let headline = if looping.is_some() {
        let clean = passes.iter().filter(|pass| pass.clean).count();
        loop_headline(start, end, count(passes.len()), count(clean))
    } else {
        let reached = bars
            .iter()
            .map(|(measure, _)| measure)
            .max()
            .unwrap_or(start);
        headline(start, reached, time(segments), &bars, &suggestion)
    };

    Review {
        headline,
        numbers: numbers(reference, segments, &bars, looping.is_none()),
        bars: (start.get()..=end.get())
            .filter_map(bar)
            .map(|measure| column(measure, bars.get(measure), reference))
            .collect(),
        passes,
        marks: merged(segments),
        suggestion: suggested(&suggestion),
    }
}

/// The button after a loop reached its goal: the loop a bar wider each side.
#[must_use]
pub fn widened(first: u32, last: u32, reference: &ReferencePerformance) -> Option<Suggested> {
    suggested(&Suggestion::after_the_goal(
        bar(first)?,
        bar(last)?,
        reference,
    ))
}

fn numbers(
    reference: &ReferencePerformance,
    segments: &[Segment],
    bars: &Bars,
    with_missed: bool,
) -> Numbers {
    let right: usize = segments.iter().map(|segment| segment.right).sum();
    let wrong: usize = segments.iter().map(|segment| segment.wrong).sum();
    let longest = bars
        .iter()
        .max_by_key(|(measure, bar)| (bar.longest_pause_us, std::cmp::Reverse(*measure)))
        .filter(|(_, bar)| bar.longest_pause_us > 0);

    Numbers {
        accuracy: percent(right, right + wrong),
        tempo: bars.total().tempo(),
        rhythm: reference.written_gap(1).is_some(),
        steadiness: bars.steadiness(),
        right,
        wrong,
        missed: if with_missed {
            segments.last().map_or(0, |segment| segment.missed)
        } else {
            0
        },
        longest_pause_ms: longest.map_or(0, |(_, bar)| bar.longest_pause_us / 1000),
        longest_pause_bar: longest.map(|(measure, _)| measure.get()),
        time_ms: time(segments).map(|us| us / 1000),
        restarts: count(
            segments
                .iter()
                .filter(|segment| segment.ending == Ending::Restart)
                .count(),
        ),
    }
}

fn column(
    measure: MeasureNumber,
    played: Option<&Bar>,
    reference: &ReferencePerformance,
) -> ReviewBar {
    let state = match played {
        Some(_) => BarState::Played,
        None if reference.first_position_of(measure).is_some() => BarState::NotReached,
        None => BarState::Empty,
    };
    let bar = played.copied().unwrap_or_default();
    ReviewBar {
        measure: measure.get(),
        state,
        tempo: bar.tempo(),
        pause_ms: bar.pause_us / 1000,
        into_ms: bar.into_bar_pause_us / 1000,
        wrong: bar.wrong,
        breaks: bar.breaks,
        line: bar_line(measure, &bar),
    }
}

fn marks(findings: &[Finding]) -> Vec<ReviewMark> {
    let mut at = BTreeMap::<usize, ReviewMark>::new();
    for finding in findings {
        let (position, correct) = match finding {
            Finding::Correct { at, .. } => (at.position(), true),
            Finding::IncorrectPitch { at, .. } => (at.position(), false),
            Finding::Omitted { .. } | Finding::Unexpected { .. } => continue,
        };
        let mark = at.entry(position).or_insert(ReviewMark {
            position,
            correct: false,
            wrong: 0,
        });
        if correct {
            mark.correct = true;
        } else {
            mark.wrong += 1;
        }
    }
    at.into_values().collect()
}

/// Every part's marks, a step played in several parts added up.
fn merged(segments: &[Segment]) -> Vec<ReviewMark> {
    let mut at = BTreeMap::<usize, ReviewMark>::new();
    for mark in segments.iter().flat_map(|segment| &segment.marks) {
        let merged = at.entry(mark.position).or_insert(ReviewMark {
            position: mark.position,
            correct: false,
            wrong: 0,
        });
        merged.correct |= mark.correct;
        merged.wrong += mark.wrong;
    }
    at.into_values().collect()
}

fn suggested(suggestion: &Suggestion) -> Option<Suggested> {
    let (first, last) = match suggestion {
        Suggestion::Loop { first, last, .. } | Suggestion::Widen { first, last } => {
            (Some(first.get()), Some(last.get()))
        }
        Suggestion::Again | Suggestion::TooShort => (None, None),
    };
    Some(Suggested {
        label: label(suggestion)?,
        first,
        last,
    })
}

/// From the first note to the last of each part, added up.
fn time(segments: &[Segment]) -> Option<u64> {
    segments
        .iter()
        .filter_map(|segment| segment.span_us)
        .reduce(|sum, span| sum + span)
}

/// From the first note to the last, if they were timed.
fn span(observed: &ObservedPerformance) -> Option<u64> {
    let mut times = observed.touches().iter().flatten().map(|touch| touch.at_us);
    let first = times.next()?;
    Some(times.next_back().unwrap_or(first).saturating_sub(first))
}

fn percent(part: usize, whole: usize) -> Option<u32> {
    let rounded = (part * 100 + whole / 2).checked_div(whole)?;
    u32::try_from(rounded).ok()
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn bar(number: u32) -> Option<MeasureNumber> {
    MeasureNumber::new(number)
}

#[cfg(test)]
mod tests {
    use crate::review::percent;

    #[test]
    fn accuracy_rounds_to_the_nearest_percent_and_needs_something_played() {
        assert_eq!(percent(112, 119), Some(94));
        assert_eq!(percent(0, 0), None);
    }
}
