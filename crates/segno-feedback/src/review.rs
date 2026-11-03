//! The few sentences of the review after an attempt: facts, never grades.

use segno_analysis::{Bar, Bars, Suggestion};
use segno_domain::MeasureNumber;

/// The one line under the review: what was played, how long it took, and
/// the spot worth practising. `first` to `last` are the bars played;
/// `duration_us` is from the first note to the last, when timed.
#[must_use]
pub fn headline(
    first: MeasureNumber,
    last: MeasureNumber,
    duration_us: Option<u64>,
    bars: &Bars,
    suggestion: &Suggestion,
) -> String {
    let played = [Some(span(first, last)), duration_us.map(duration)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");

    match suggestion {
        Suggestion::TooShort => "Too short to say much yet.".to_owned(),
        Suggestion::Again | Suggestion::Widen { .. } => {
            format!("{played} · no trouble spots this time.")
        }
        Suggestion::Loop { trouble, into, .. } => {
            let place = if *into { "going into" } else { "in" };
            let details = bars.get(*trouble).map(details).unwrap_or_default();
            let details = if details.is_empty() {
                String::new()
            } else {
                format!(" ({})", details.join(", "))
            };
            format!(
                "{played} · most trouble {place} bar {}{details}.",
                trouble.get()
            )
        }
    }
}

/// What a run covered and how long it took, for the top of the results:
/// the cards below say what to work on.
#[must_use]
pub fn played(first: MeasureNumber, last: MeasureNumber, duration_us: Option<u64>) -> String {
    [Some(span(first, last)), duration_us.map(duration)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The line for a loop run that ended: which bars, how many passes, how
/// many of them clean.
#[must_use]
pub fn loop_headline(first: MeasureNumber, last: MeasureNumber, passes: u32, clean: u32) -> String {
    let passes = if passes == 1 {
        "1 pass".to_owned()
    } else {
        format!("{passes} passes")
    };
    format!("{} · {passes} · {clean} clean.", span(first, last))
}

/// What the suggestion's button says, if there is one.
#[must_use]
pub fn label(suggestion: &Suggestion) -> Option<String> {
    match suggestion {
        Suggestion::TooShort => None,
        Suggestion::Again => Some("Again".to_owned()),
        Suggestion::Loop { first, last, .. } | Suggestion::Widen { first, last } => {
            Some(format!("Loop {}", span(*first, *last).to_lowercase()))
        }
    }
}

/// One bar, in a line: shown when pointing at it.
#[must_use]
pub fn bar_line(measure: MeasureNumber, bar: &Bar) -> String {
    let mut parts = vec![span(measure, measure)];
    parts.extend(bar.tempo().map(|tempo| format!("♩ = {tempo}")));
    let details = details(bar);
    if details.is_empty() {
        parts.push("no trouble".to_owned());
    } else {
        parts.extend(details);
    }
    if bar.breaks > 0 {
        parts.push(plural(bar.breaks, "break", "breaks"));
    }
    parts.join(" · ")
}

/// One pass of a loop, in a line: shown when pointing at it.
#[must_use]
pub fn pass_line(number: usize, bar: &Bar, clean: bool) -> String {
    let mut parts = vec![format!("Pass {number}")];
    parts.extend(bar.tempo().map(|tempo| format!("♩ = {tempo}")));
    if clean {
        parts.push("clean".to_owned());
    } else if bar.wrong > 0 {
        parts.push(format!("{} wrong", bar.wrong));
    }
    parts.join(" · ")
}

fn details(bar: &Bar) -> Vec<String> {
    let mut details = Vec::new();
    // Under a tenth of a second is not worth saying.
    if bar.pause_us >= 100_000 {
        details.push(format!("{} pause", seconds(bar.pause_us)));
    }
    if bar.wrong > 0 {
        details.push(format!("{} wrong", bar.wrong));
    }
    if bar.gave_up > 0 {
        details.push(format!("restarted or stopped here {}×", bar.gave_up));
    }
    details
}

/// Bars as a label: "Bar 7", "Bars 6–7", "Pickup", "Pickup–bar 2". Never
/// "bar 0" (DECISIONS, 3 Nov, E2).
#[must_use]
pub fn span(first: MeasureNumber, last: MeasureNumber) -> String {
    match (first.is_pickup(), first == last) {
        (true, true) => "Pickup".to_owned(),
        (true, false) => format!("Pickup–bar {}", last.get()),
        (false, true) => format!("Bar {}", first.get()),
        (false, false) => format!("Bars {}–{}", first.get(), last.get()),
    }
}

/// One bar in a sentence: "bar 7", "the pickup".
#[must_use]
pub fn bar_name(bar: MeasureNumber) -> String {
    if bar.is_pickup() {
        "the pickup".to_owned()
    } else {
        format!("bar {}", bar.get())
    }
}

/// Bars in a sentence: "bar 7", "bars 6–7", "the pickup", "from the pickup
/// to bar 2".
#[must_use]
pub fn bars_named(first: MeasureNumber, last: MeasureNumber) -> String {
    match (first.is_pickup(), first == last) {
        (true, false) => format!("from the pickup to bar {}", last.get()),
        (_, true) => bar_name(first),
        (false, false) => format!("bars {}–{}", first.get(), last.get()),
    }
}

/// Where, after a noun: "in bar 7", "in bars 6–7", "in the pickup", "from
/// the pickup to bar 2".
#[must_use]
pub fn in_bars(first: MeasureNumber, last: MeasureNumber) -> String {
    if first.is_pickup() && first != last {
        bars_named(first, last)
    } else {
        format!("in {}", bars_named(first, last))
    }
}

pub(crate) fn seconds(us: u64) -> String {
    format!("{}.{} s", us / 1_000_000, us / 100_000 % 10)
}

fn duration(us: u64) -> String {
    let seconds = us / 1_000_000;
    if seconds < 60 {
        format!("{seconds} s")
    } else {
        format!("{} min {} s", seconds / 60, seconds % 60)
    }
}

pub(crate) fn plural(count: u32, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

#[cfg(test)]
mod tests {
    use segno_domain::MeasureNumber as M;

    use crate::review::{bar_name, bars_named, in_bars, span};

    // E2 (DECISIONS, 3 Nov): labels "Pickup–bar 2", sentences "from the
    // pickup to bar 2", alone "the pickup"; never "bar 0".
    #[test]
    fn the_pickup_has_its_own_words() {
        let (pickup, one, two) = (M::PICKUP, M::FIRST, M::printed(2));
        assert_eq!(span(pickup, pickup), "Pickup");
        assert_eq!(span(pickup, two), "Pickup–bar 2");
        assert_eq!(span(one, two), "Bars 1–2");
        assert_eq!(bar_name(pickup), "the pickup");
        assert_eq!(bars_named(pickup, two), "from the pickup to bar 2");
        assert_eq!(in_bars(pickup, two), "from the pickup to bar 2");
        assert_eq!(in_bars(pickup, pickup), "in the pickup");
        assert_eq!(in_bars(two, two), "in bar 2");
    }

    use segno_analysis::{Bar, Bars, Suggestion};
    use segno_domain::MeasureNumber;

    use crate::review::{bar_line, headline, label, loop_headline, pass_line, played};

    fn bar(number: u32) -> MeasureNumber {
        MeasureNumber::new(number).expect("a bar number")
    }

    #[test]
    fn the_headline_names_the_spot_and_why() {
        let trouble = Suggestion::Loop {
            first: bar(8),
            last: bar(9),
            trouble: bar(9),
            into: true,
        };

        assert_eq!(
            headline(
                bar(1),
                bar(18),
                Some(400_000_000),
                &Bars::default(),
                &trouble
            ),
            "Bars 1–18 · 6 min 40 s · most trouble going into bar 9."
        );
        assert_eq!(label(&trouble).as_deref(), Some("Loop bars 8–9"));
    }

    #[test]
    fn a_clean_run_says_so_and_a_short_one_says_little() {
        assert_eq!(
            headline(
                bar(1),
                bar(32),
                Some(42_000_000),
                &Bars::default(),
                &Suggestion::Again
            ),
            "Bars 1–32 · 42 s · no trouble spots this time."
        );
        assert_eq!(
            headline(
                bar(1),
                bar(1),
                None,
                &Bars::default(),
                &Suggestion::TooShort
            ),
            "Too short to say much yet."
        );
        assert_eq!(label(&Suggestion::TooShort), None);
    }

    #[test]
    fn a_bar_is_described_by_what_happened_in_it() {
        let hard = Bar {
            passes: 2,
            pause_us: 3_240_000,
            wrong: 2,
            breaks: 1,
            ..Bar::default()
        };

        assert_eq!(
            bar_line(bar(9), &hard),
            "Bar 9 · 3.2 s pause · 2 wrong · 1 break"
        );
        assert_eq!(bar_line(bar(4), &Bar::default()), "Bar 4 · no trouble");
        let timed = Bar {
            played_us: 1_200_000,
            written_ticks: 960,
            ..hard
        };
        assert_eq!(
            bar_line(bar(9), &timed),
            "Bar 9 · ♩ = 100 · 3.2 s pause · 2 wrong · 1 break"
        );
        assert_eq!(pass_line(3, &timed, false), "Pass 3 · ♩ = 100 · 2 wrong");
        assert_eq!(pass_line(4, &Bar::default(), true), "Pass 4 · clean");
    }

    #[test]
    fn what_a_run_covered_is_its_bars_and_its_time() {
        assert_eq!(played(bar(1), bar(8), Some(6_400_000)), "Bars 1–8 · 6 s");
        assert_eq!(played(bar(9), bar(9), None), "Bar 9");
    }

    #[test]
    fn a_loop_run_counts_its_passes() {
        assert_eq!(
            loop_headline(bar(9), bar(10), 7, 3),
            "Bars 9–10 · 7 passes · 3 clean."
        );
        assert_eq!(
            loop_headline(bar(9), bar(9), 1, 0),
            "Bar 9 · 1 pass · 0 clean."
        );
    }
}
