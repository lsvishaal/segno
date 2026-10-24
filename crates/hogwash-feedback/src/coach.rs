//! The coach's words: for each spot, what happened, why it is likely
//! happening, and how to practise it, in plain words a beginner can act on.

use hogwash_analysis::coach::{Card, Kind, Spot, WentWell};

use crate::review::{plural, seconds, span};

/// Everything a card says.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CardText {
    pub title: String,
    /// What happened: facts.
    pub what: String,
    /// Why it is likely happening.
    pub why: String,
    /// How to fix it, in order.
    pub steps: Vec<String>,
    /// The button that starts practising it.
    pub button: String,
    /// A second thing on the same bars, in one line.
    pub also: Option<String>,
}

/// The words for `card`.
#[must_use]
pub fn card(card: &Card) -> CardText {
    let spot = &card.main;
    let (title, what, why, steps) = words(spot);
    CardText {
        title,
        what,
        why,
        steps,
        button: button(spot),
        also: card
            .also
            .map(|also| format!("Also here: {}.", lower(&words(&also).0))),
    }
}

/// What a clean run says, with a next step: an even tempo first, then
/// faster, then from memory.
#[must_use]
pub fn clean_run(steadiness: Option<u32>) -> CardText {
    let steps = if steadiness.is_some_and(|steadiness| steadiness < 85) {
        vec!["Play it again, keeping the beat even all the way through.".to_owned()]
    } else {
        vec![
            "Play it a little faster next time.".to_owned(),
            "Or try it from memory, starting from any bar.".to_owned(),
        ]
    };
    CardText {
        title: "A clean run".to_owned(),
        what: "No wrong keys and no stops worth mentioning.".to_owned(),
        why: String::new(),
        steps,
        button: String::new(),
        also: None,
    }
}

/// What too short a run says.
#[must_use]
pub fn too_short() -> String {
    "Play a few bars more to get feedback.".to_owned()
}

/// One thing that went well, in a line.
#[must_use]
pub fn went_well(item: &WentWell) -> String {
    match item {
        WentWell::Clean { first, last } => {
            format!("{} with no mistakes and no stops", span(*first, *last))
        }
        WentWell::Even { first, last } => {
            format!("An even tempo in {}", lower(&span(*first, *last)))
        }
    }
}

/// A card's title, what happened, why, and how to fix it.
type Words = (String, String, String, Vec<String>);

fn words(spot: &Spot) -> Words {
    let facts = spot.facts;
    let bars = lower(&span(spot.first, spot.last));
    let n = spot.first.get();
    let each = if facts.passes > 1 {
        format!(", over {} times through", facts.passes)
    } else {
        String::new()
    };
    let steps = |lines: &[&str]| lines.iter().map(|&line| line.to_owned()).collect();

    match spot.kind {
        Kind::IntoBar => into_bar(spot, &each),
        Kind::Jump => jump(spot, &each),
        Kind::FastNotes { broken_chord } => fast_notes(spot, broken_chord, &bars, &each),
        Kind::WrongNotes => wrong_notes(spot, &each),
        Kind::RushedLongNotes => (
            format!("Long notes in bar {n}"),
            format!(
                "You moved on early from a long note {}{each}.",
                plural(facts.rushed, "time", "times")
            ),
            "Long notes and rests are easy to cut short when you're thinking ahead.".to_owned(),
            vec![
                format!("Count the beats out loud as you play bar {n}."),
                "Hold each long note until you've counted all of it.".to_owned(),
                "Then play it again, counting in your head.".to_owned(),
            ],
        ),
        Kind::SlowBars => (
            format!("{} slow you down", span(spot.first, spot.last)),
            match (facts.tempo, facts.usual_tempo) {
                (Some(tempo), Some(usual)) => {
                    format!("You played {bars} at ♩ = {tempo}; the rest went at about ♩ = {usual}.")
                }
                _ => format!("You played {bars} much slower than the rest."),
            },
            "They're harder than the rest, so the rest runs ahead of them.".to_owned(),
            steps(&[
                "Practise these bars on their own until they feel easy.",
                "Then play the whole piece at the speed of these bars.",
                "Speed up only while it stays even.",
            ]),
        ),
        Kind::KeepsStopping => (
            format!("Stopping at bar {n}"),
            format!(
                "You stopped or started again in bar {n} {}.",
                plural(facts.stops, "time", "times")
            ),
            "Starting from the top each time means the hard part gets the least practice."
                .to_owned(),
            vec![
                format!("Start your practice at bar {n} instead of the beginning."),
                format!("Play from bar {n} to the end of the next bar, 3 times."),
                "Then start a few bars earlier and play through it.".to_owned(),
            ],
        ),
        Kind::BigStep => (
            "This piece is a big step".to_owned(),
            format!(
                "{} of the {} bars you played had trouble.",
                facts.wrong, facts.passes
            ),
            "That's normal for a piece a level or two up; small sections make it manageable."
                .to_owned(),
            steps(&[
                "Work on 4 bars at a time.",
                "Add the next 4 only when these feel easy.",
                "An easier piece with the same kind of music is also a fine choice.",
            ]),
        ),
    }
}

fn into_bar(spot: &Spot, each: &str) -> Words {
    let (facts, n) = (spot.facts, spot.first.get());
    let previous = spot.drill.0.get();
    (
        format!("Going into bar {n}"),
        format!(
            "You paused {} before bar {n}{each}.",
            seconds(facts.pause_us)
        ),
        format!("The move from bar {previous} to bar {n} isn't automatic yet."),
        vec![
            format!(
                "Play the last beat of bar {previous} and the first of bar {n}, slowly, 5 times in a row without stopping."
            ),
            format!("Then play all of bars {previous}–{n}."),
            "Then add a bar on each side.".to_owned(),
        ],
    )
}

fn jump(spot: &Spot, each: &str) -> Words {
    let (facts, n) = (spot.facts, spot.first.get());
    let steps = |lines: &[&str]| lines.iter().map(|&line| line.to_owned()).collect();
    (
        format!("The jump in bar {n}"),
        joined(&[
            (facts.pause_us >= 100_000)
                .then(|| format!("You paused {} at the jump", seconds(facts.pause_us))),
            (facts.wrong > 0).then(|| {
                format!(
                    "landed on {}",
                    plural(facts.wrong, "wrong key", "wrong keys")
                )
            }),
        ]) + each
            + ".",
        "Your hand moves a long way there, and it isn't sure of the distance yet.".to_owned(),
        steps(&[
            "Play just the note before the jump and the note after it.",
            "Move fast and land early, then wait on the new note before going on.",
            "Make the wait shorter each time, until there's none.",
        ]),
    )
}

fn fast_notes(spot: &Spot, broken_chord: bool, bars: &str, each: &str) -> Words {
    let facts = spot.facts;
    let steps = |lines: &[&str]| lines.iter().map(|&line| line.to_owned()).collect();
    let what = joined(&[
        (facts.wrong > 0).then(|| {
            format!(
                "{} next to the written note{}",
                plural(facts.wrong, "slip onto the key", "slips onto the key"),
                facts.offset.map_or_else(String::new, |offset| {
                    format!(" (mostly {})", direction(offset))
                })
            )
        }),
        (facts.pause_us >= 100_000).then(|| format!("{} of pauses", seconds(facts.pause_us))),
    ]);
    if broken_chord {
        (
            format!("The fast notes in {bars}"),
            format!("{what}{each}."),
            "These notes spell out a chord, and your hand hasn't found its shape yet.".to_owned(),
            steps(&[
                "Play each group of notes as one chord, all together.",
                "Then spread them out again, slowly.",
                "Then at your own speed.",
            ]),
        )
    } else {
        (
            format!("The fast notes in {bars}"),
            format!("{what}{each}."),
            "Your fingers haven't settled into this run yet.".to_owned(),
            steps(&[
                "Play it slowly in a long-short rhythm, then short-long.",
                "Then evenly, still slowly.",
                "Then a little faster each time it comes out clean.",
            ]),
        )
    }
}

fn wrong_notes(spot: &Spot, each: &str) -> Words {
    let (facts, n) = (spot.facts, spot.first.get());
    let what = format!(
        "{} in bar {n}{}{each}.",
        capital(&plural(facts.wrong, "wrong key", "wrong keys")),
        facts.offset.map_or_else(String::new, |offset| {
            format!(", mostly {}", direction(offset))
        })
    );
    if facts
        .offset
        .is_some_and(|offset| offset.unsigned_abs() <= 2)
    {
        (
            format!("Wrong notes in bar {n}"),
            what,
            "Your finger landed next to the right key: your hand isn't sure of its place there yet.".to_owned(),
            vec![
                format!("Before playing bar {n}, put your hand over its first notes and look."),
                "Play it slowly, watching which finger plays which key.".to_owned(),
                "Play it 3 times without a mistake before going on.".to_owned(),
            ],
        )
    } else {
        (
            format!("Wrong notes in bar {n}"),
            what,
            "These look like reading slips: the note was misread, not mis-hit.".to_owned(),
            vec![
                format!("Read bar {n} away from the keys, saying each note's name."),
                "Turn on note names if it helps, then play it slowly.".to_owned(),
                "Play it 3 times without a mistake before going on.".to_owned(),
            ],
        )
    }
}

fn button(spot: &Spot) -> String {
    let (first, last) = spot.drill;
    match spot.kind {
        Kind::KeepsStopping => format!("Start at bar {}", spot.first.get()),
        _ => format!("Practise {}", lower(&span(first, last))),
    }
}

/// Where a wrong key was, relative to the written note: a key has no
/// spelling, so it is never named.
fn direction(offset: i8) -> String {
    let keys = offset.unsigned_abs();
    let way = if offset < 0 { "low" } else { "high" };
    match keys {
        1 => format!("one key too {way}"),
        12 => format!("an octave too {way}"),
        _ => format!("{keys} keys too {way}"),
    }
}

/// The parts that apply, joined into one sentence's worth.
fn joined(parts: &[Option<String>]) -> String {
    let parts: Vec<&String> = parts.iter().flatten().collect();
    let mut text = parts
        .iter()
        .map(|part| part.as_str())
        .collect::<Vec<_>>()
        .join(" and ");
    if let Some(first) = text.get(..1) {
        text = first.to_uppercase() + text.get(1..).unwrap_or_default();
    }
    text
}

fn lower(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_lowercase().collect::<String>() + chars.as_str()
    })
}

fn capital(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + chars.as_str()
    })
}

#[cfg(test)]
mod tests {
    use hogwash_analysis::coach::{Card, Facts, Kind, Spot, WentWell};
    use hogwash_domain::MeasureNumber;

    use crate::coach::{card, clean_run, went_well};

    fn bar(number: u32) -> MeasureNumber {
        MeasureNumber::new(number).expect("a bar number")
    }

    fn spot(kind: Kind, first: u32, last: u32, facts: Facts) -> Spot {
        Spot {
            kind,
            first: bar(first),
            last: bar(last),
            drill: (bar(first), bar(last)),
            weight: 1,
            facts,
        }
    }

    #[test]
    fn going_into_a_bar_names_the_pause_the_join_and_the_drill() {
        let mut into = spot(
            Kind::IntoBar,
            6,
            6,
            Facts {
                passes: 1,
                pause_us: 4_020_000,
                ..Facts::default()
            },
        );
        into.drill = (bar(5), bar(6));

        let text = card(&Card {
            main: into,
            also: None,
        });

        assert_eq!(text.title, "Going into bar 6");
        assert_eq!(text.what, "You paused 4.0 s before bar 6.");
        assert_eq!(
            text.why,
            "The move from bar 5 to bar 6 isn't automatic yet."
        );
        assert_eq!(
            text.steps[0],
            "Play the last beat of bar 5 and the first of bar 6, slowly, 5 times in a row without stopping."
        );
        assert_eq!(text.button, "Practise bars 5–6");
    }

    #[test]
    fn fast_notes_say_how_the_keys_slipped_and_how_to_drill_a_chord_shape() {
        let fast = spot(
            Kind::FastNotes { broken_chord: true },
            1,
            2,
            Facts {
                passes: 2,
                wrong: 3,
                offset: Some(-1),
                ..Facts::default()
            },
        );

        let text = card(&Card {
            main: fast,
            also: None,
        });

        assert_eq!(text.title, "The fast notes in bars 1–2");
        assert_eq!(
            text.what,
            "3 slips onto the key next to the written note (mostly one key too low), over 2 times through."
        );
        assert_eq!(
            text.steps[0],
            "Play each group of notes as one chord, all together."
        );
    }

    #[test]
    fn a_second_thing_on_the_same_bars_is_one_line() {
        let jump = spot(
            Kind::Jump,
            9,
            9,
            Facts {
                passes: 1,
                pause_us: 2_000_000,
                ..Facts::default()
            },
        );
        let wrong = spot(
            Kind::WrongNotes,
            9,
            9,
            Facts {
                passes: 1,
                wrong: 2,
                ..Facts::default()
            },
        );

        let text = card(&Card {
            main: jump,
            also: Some(wrong),
        });

        assert_eq!(text.what, "You paused 2.0 s at the jump.");
        assert_eq!(
            text.also.as_deref(),
            Some("Also here: wrong notes in bar 9.")
        );
    }

    #[test]
    fn stopping_again_and_again_starts_there() {
        let text = card(&Card {
            main: spot(
                Kind::KeepsStopping,
                9,
                9,
                Facts {
                    passes: 2,
                    stops: 3,
                    ..Facts::default()
                },
            ),
            also: None,
        });

        assert_eq!(text.what, "You stopped or started again in bar 9 3 times.");
        assert_eq!(text.button, "Start at bar 9");
    }

    #[test]
    fn a_clean_run_gets_a_next_step_and_what_went_well_is_one_line() {
        assert_eq!(clean_run(Some(70)).steps.len(), 1);
        assert_eq!(
            clean_run(Some(95)).steps[0],
            "Play it a little faster next time."
        );
        assert_eq!(
            went_well(&WentWell::Clean {
                first: bar(1),
                last: bar(4)
            }),
            "Bars 1–4 with no mistakes and no stops"
        );
        assert_eq!(
            went_well(&WentWell::Even {
                first: bar(10),
                last: bar(12)
            }),
            "An even tempo in bars 10–12"
        );
    }
}
