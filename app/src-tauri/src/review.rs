//! The review after an attempt: what the screen draws once playing stops.
//!
//! Every number and sentence is worked out here, from the per-bar stats and
//! the analysis; the screen only draws them.

use std::collections::BTreeMap;
use std::ops::Range;

use hogwash_analysis::{Bar, Bars, Finding, Suggestion};
use hogwash_capture::ObservedPerformance;
use hogwash_domain::MeasureNumber;
use hogwash_feedback::review::{bar_line, headline, label, loop_headline};
use hogwash_notation::{ExpectedNotes, ReferencePerformance};
use serde::Serialize;

/// What the screen shows after an attempt.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    /// The one line of text.
    pub headline: String,
    /// One per bar, in bar-number order, from where the attempt began to the
    /// end of the piece (or of the loop).
    pub bars: Vec<ReviewBar>,
    /// What happened at each step played: drawn on the score.
    pub marks: Vec<ReviewMark>,
    /// The one thing worth doing next, if anything.
    pub suggestion: Option<Suggested>,
}

/// One bar's column in the strip.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewBar {
    pub measure: u32,
    pub state: BarState,
    pub pause_ms: u64,
    pub into_ms: u64,
    pub wrong: u32,
    pub breaks: u32,
    /// Said when pointing at the column.
    pub line: String,
}

/// Whether a bar was played this time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BarState {
    Played,
    /// After where the attempt stopped.
    NotReached,
    /// Nothing to strike in it: rests, or only held notes.
    Empty,
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

/// A loop being practised: every pass so far, added up.
#[derive(Debug, Clone)]
pub struct LoopRun {
    pub first: u32,
    pub last: u32,
    pub bars: Bars,
    pub passes: u32,
    pub clean: u32,
}

impl LoopRun {
    #[must_use]
    pub fn new(first: u32, last: u32) -> Self {
        Self {
            first,
            last,
            bars: Bars::default(),
            passes: 0,
            clean: 0,
        }
    }
}

/// The review of an attempt over `steps` that has just ended. `run` is the
/// loop it was a pass of, if any: the review then covers every pass of it.
pub fn of(
    reference: &ReferencePerformance,
    observed: &ObservedPerformance,
    findings: &[Finding],
    steps: Range<usize>,
    bars: &Bars,
    run: Option<&LoopRun>,
) -> Review {
    let bar_at = |step: usize| reference.at(step).map(ExpectedNotes::measure);
    let start = bar_at(steps.start).filter(|_| !observed.is_empty() || run.is_some());
    let Some(start) = start else {
        return Review {
            headline: "Nothing played.".to_owned(),
            bars: Vec::new(),
            marks: Vec::new(),
            suggestion: None,
        };
    };

    let (bars, start, end) = if let Some(run) = run {
        let mut all = run.bars.clone();
        all.add(bars);
        let first = bar(run.first).unwrap_or(start);
        (all, first, bar(run.last).unwrap_or(first))
    } else {
        let end = (steps.start..steps.end.min(reference.len()))
            .filter_map(bar_at)
            .max()
            .unwrap_or(start);
        (bars.clone(), start, end)
    };
    let suggestion = Suggestion::after(&bars, start, reference);
    let headline = if let Some(run) = run {
        loop_headline(start, end, run.passes, run.clean)
    } else {
        let reached = bars
            .iter()
            .map(|(measure, _)| measure)
            .max()
            .unwrap_or(start);
        headline(start, reached, duration(observed), &bars, &suggestion)
    };

    let columns = (start.get()..=end.get())
        .filter_map(bar)
        .map(|measure| column(measure, bars.get(measure), reference))
        .collect();

    Review {
        headline,
        bars: columns,
        marks: marks(findings),
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

/// From the first note to the last, if they were timed.
fn duration(observed: &ObservedPerformance) -> Option<u64> {
    let mut times = observed.touches().iter().flatten().map(|touch| touch.at_us);
    let first = times.next()?;
    Some(times.next_back().unwrap_or(first).saturating_sub(first))
}

fn bar(number: u32) -> Option<MeasureNumber> {
    MeasureNumber::new(number)
}
