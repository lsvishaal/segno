//! The coach: what to work on after a run, why it is likely happening, and
//! where — at most three things, the most important first.
//!
//! Every rule here comes from how teachers diagnose a stumble: a pause going
//! into a bar is a transition not yet learnt; a slip in fast notes is a hand
//! that hasn't found the shape; a wrong key far from the written one is a
//! reading slip. The words come later, from `hogwash-feedback`.

use std::collections::BTreeMap;

use hogwash_domain::MeasureNumber;
use hogwash_notation::{ExpectedNotes, ReferencePerformance, TICKS_PER_QUARTER};

use crate::bars::{Bars, Trace};

/// A second of pause going into a bar, per time through, is worth a card.
const INTO_BAR_US: u64 = 1_000_000;
/// Half a second of pause, or a wrong key, right after a jump.
const JUMP_PAUSE_US: u64 = 500_000;
/// A note further than this from every note before it is a jump (keys).
const LEAP: u16 = 12;
/// Notes this close or closer are fast: an eighth note.
const FAST_TICKS: u64 = TICKS_PER_QUARTER as u64 / 2;
/// Pause in fast notes worth a card, per time through.
const FAST_PAUSE_US: u64 = 800_000;
/// A wrong key this close to the written one is a slip, further a misread.
const SLIP: u8 = 2;
/// A note at least this long (a half note) that was cut short…
const LONG_TICKS: u64 = 2 * TICKS_PER_QUARTER as u64;
/// …to under this share of its length, in tenths.
const CUT_SHORT_TENTHS: u64 = 6;
/// A bar under this share of your usual tempo, in tenths, slows you down.
const SLOW_TENTHS: u64 = 7;
/// Most cards on one run.
const MOST_CARDS: usize = 3;
/// A fast-notes spot takes in at most this many bars.
const MOST_BARS: u32 = 4;

/// What kind of trouble a spot is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The change into the bar: a pause on its first note.
    IntoBar,
    /// A jump: a note far from every note before it.
    Jump,
    /// Fast notes; `broken_chord` when they spell out a chord.
    FastNotes { broken_chord: bool },
    /// Wrong keys: misread when far from the written one, mis-hit when
    /// right next to it (the offset in the facts says which).
    WrongNotes,
    /// Long notes or rests cut short.
    RushedLongNotes,
    /// Bars played well under your usual tempo.
    SlowBars,
    /// Restarted or stopped in the same bar more than once.
    KeepsStopping,
    /// Trouble nearly everywhere: the piece is a big step up.
    BigStep,
}

/// One spot worth working on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Spot {
    pub kind: Kind,
    /// The bars it is in.
    pub first: MeasureNumber,
    pub last: MeasureNumber,
    /// The bars to loop to practise it: playable in a row.
    pub drill: (MeasureNumber, MeasureNumber),
    /// How much it matters; only compared with other spots.
    pub weight: u64,
    pub facts: Facts,
}

/// What happened at a spot, for the words.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Facts {
    /// Times through the spot's bars.
    pub passes: u32,
    /// The pause that matters for this kind, over the run.
    pub pause_us: u64,
    /// Wrong keys that matter for this kind.
    pub wrong: u32,
    /// The most common wrong key, relative to the written one (keys).
    pub offset: Option<i8>,
    /// Long notes cut short.
    pub rushed: u32,
    /// Restarts and stops.
    pub stops: u32,
    /// The spot's tempo and your usual one, quarters per minute.
    pub tempo: Option<u64>,
    pub usual_tempo: Option<u64>,
}

/// A card: one spot, and maybe a second thing on the same bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub main: Spot,
    pub also: Option<Spot>,
}

/// Something that went well.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WentWell {
    /// Bars with no wrong key and no pause.
    Clean {
        first: MeasureNumber,
        last: MeasureNumber,
    },
    /// Bars at an even tempo, close to your usual.
    Even {
        first: MeasureNumber,
        last: MeasureNumber,
    },
}

/// What the coach has to say about a run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Coach {
    pub cards: Vec<Card>,
    pub went_well: Vec<WentWell>,
}

/// Per bar, over a run.
#[derive(Debug, Default)]
struct Tally {
    into_us: u64,
    pause_us: u64,
    jump_us: u64,
    jump_wrong: u32,
    fast_slips: u32,
    fast_us: u64,
    /// Wrong keys outside fast notes, and misreads anywhere.
    far: u32,
    offsets: Vec<i8>,
    rushed: u32,
}

impl Coach {
    /// The coach's view of a run made of the attempts `traces` follow, over
    /// `reference`. `accuracy` is the run's, in percent.
    #[must_use]
    pub fn of(reference: &ReferencePerformance, traces: &[Trace], accuracy: Option<u32>) -> Self {
        let mut bars = Bars::default();
        for trace in traces {
            bars.add(&Bars::from_trace(trace));
        }
        let tallies = tally(reference, traces);
        let passes = |measure: MeasureNumber| bars.get(measure).map_or(1, |bar| bar.passes.max(1));
        let usual = usual_tempo(&bars);

        let mut spots: Vec<Spot> = tallies
            .iter()
            .flat_map(|(&measure, tally)| {
                spots_in(
                    reference,
                    measure,
                    tally,
                    passes(measure),
                    bars.get(measure),
                    usual,
                )
            })
            .collect();
        for (measure, bar) in bars.iter() {
            if bar.gave_up >= 2 {
                spots.push(Spot {
                    kind: Kind::KeepsStopping,
                    first: measure,
                    last: measure,
                    drill: practise_range(reference, measure, measure),
                    weight: 1500 * u64::from(bar.gave_up),
                    facts: Facts {
                        passes: bar.passes,
                        stops: bar.gave_up,
                        ..Facts::default()
                    },
                });
            }
        }

        let spots = join_neighbours(reference, spots);
        let mut cards = choose(spots);
        if let Some(big) = big_step(reference, &bars, accuracy) {
            cards.insert(
                0,
                Card {
                    main: big,
                    also: None,
                },
            );
            cards.truncate(MOST_CARDS);
        }

        Self {
            cards,
            went_well: went_well(&bars, usual),
        }
    }
}

/// The spots in bar `measure`, played `n` times through.
fn spots_in(
    reference: &ReferencePerformance,
    measure: MeasureNumber,
    tally: &Tally,
    n: u32,
    bar: Option<&crate::bars::Bar>,
    usual: Option<u64>,
) -> Vec<Spot> {
    let mut spots = Vec::new();
    let n64 = u64::from(n);
    let facts = Facts {
        passes: n,
        ..Facts::default()
    };
    let spot = |kind, weight, facts| Spot {
        kind,
        first: measure,
        last: measure,
        drill: practise_range(reference, measure, measure),
        weight,
        facts,
    };

    if tally.into_us >= INTO_BAR_US * n64 && 2 * tally.into_us >= tally.pause_us {
        let into = spot(
            Kind::IntoBar,
            tally.into_us / 1000 / n64,
            Facts {
                pause_us: tally.into_us,
                ..facts
            },
        );
        spots.push(into);
    }
    if tally.jump_us >= JUMP_PAUSE_US * n64 || tally.jump_wrong >= n {
        spots.push(spot(
            Kind::Jump,
            (tally.jump_us / 1000 + 1000 * u64::from(tally.jump_wrong)) / n64,
            Facts {
                pause_us: tally.jump_us,
                wrong: tally.jump_wrong,
                ..facts
            },
        ));
    }
    if tally.fast_slips >= n || tally.fast_us >= FAST_PAUSE_US * n64 {
        spots.push(spot(
            Kind::FastNotes {
                broken_chord: broken_chord(reference, measure),
            },
            (1000 * u64::from(tally.fast_slips) + tally.fast_us / 1000) / n64,
            Facts {
                pause_us: tally.fast_us,
                wrong: tally.fast_slips,
                offset: most_common(&tally.offsets, |offset| offset.unsigned_abs() <= SLIP),
                ..facts
            },
        ));
    }
    if tally.far >= n {
        spots.push(spot(
            Kind::WrongNotes,
            1000 * u64::from(tally.far) / n64,
            Facts {
                wrong: tally.far,
                offset: most_common(&tally.offsets, |_| true),
                ..facts
            },
        ));
    }
    if tally.rushed >= n {
        spots.push(spot(
            Kind::RushedLongNotes,
            500 * u64::from(tally.rushed) / n64,
            Facts {
                rushed: tally.rushed,
                ..facts
            },
        ));
    }
    let into_led = 2 * tally.into_us >= tally.pause_us && tally.into_us > 0;
    if let Some(bar) = bar
        && let (Some(tempo), Some(usual)) = (bar.tempo(), usual)
        && bar.timed >= 2
        && tempo * 10 < usual * SLOW_TENTHS
        && !into_led
    {
        spots.push(spot(
            Kind::SlowBars,
            (usual - tempo) * 20,
            Facts {
                tempo: Some(tempo),
                usual_tempo: Some(usual),
                ..facts
            },
        ));
    }
    spots
}

/// Whether a note of step `step` lands further than an octave from every
/// note of the step before it: a hand had to jump there.
#[must_use]
pub fn leap_into(reference: &ReferencePerformance, step: usize) -> bool {
    let (Some(before), Some(here)) = (
        step.checked_sub(1).and_then(|before| reference.at(before)),
        reference.at(step),
    ) else {
        return false;
    };
    here.sounding().any(|note| {
        before
            .sounding()
            .all(|earlier| u16::from(u8::from(note).abs_diff(u8::from(earlier))) > LEAP)
    })
}

/// Whether the fast single notes in bar `measure` spell out a chord: three
/// or more steps in a row of a third or wider, the same way.
#[must_use]
pub fn broken_chord(reference: &ReferencePerformance, measure: MeasureNumber) -> bool {
    let Some(start) = reference.first_position_of(measure) else {
        return false;
    };
    let mut run = 0;
    let mut direction = 0_i16;
    let mut previous: Option<i16> = None;
    for step in start.. {
        let Some(expected) = reference
            .at(step)
            .filter(|expected| expected.measure() == measure)
        else {
            break;
        };
        // The top note: the line the hand traces through the chord.
        let Some(top) = expected
            .sounding()
            .max()
            .map(|pitch| i16::from(u8::from(pitch)))
        else {
            continue;
        };
        if let Some(before) = previous {
            let interval = top - before;
            let same_way = interval.signum() == direction || run == 0;
            if interval.abs() >= 3 && same_way {
                run += 1;
                direction = interval.signum();
                if run >= 3 {
                    return true;
                }
            } else if interval.abs() >= 3 {
                run = 1;
                direction = interval.signum();
            } else {
                run = 0;
            }
        }
        previous = Some(top);
    }
    false
}

/// What each bar's steps came to, over every attempt of the run.
fn tally(reference: &ReferencePerformance, traces: &[Trace]) -> BTreeMap<MeasureNumber, Tally> {
    let measure_of = |step: usize| reference.at(step).map(ExpectedNotes::measure);
    let mut tallies: BTreeMap<MeasureNumber, Tally> = BTreeMap::new();
    for step in traces.iter().flat_map(|trace| &trace.steps) {
        let tally = tallies.entry(step.measure).or_default();
        let pause = step.timing.map_or(0, |timing| timing.pause_us());
        let written = step
            .timing
            .map(|timing| timing.written_ticks)
            .or_else(|| reference.written_gap(step.step).map(u64::from));
        let wrong = u32::try_from(step.wrong.len().min(3)).unwrap_or(3);
        let slips = step
            .wrong
            .iter()
            .filter(|offset| offset.unsigned_abs() <= SLIP)
            .count();
        let slips = u32::try_from(slips.min(3)).unwrap_or(3);

        let fast = written.is_some_and(|ticks| ticks <= FAST_TICKS);
        tally.pause_us += pause;
        tally.offsets.extend(&step.wrong);
        // A slip in fast notes belongs to the fast notes; anywhere else,
        // every wrong key is a wrong note.
        tally.far += if fast { wrong - slips } else { wrong };
        if step.opens_bar {
            tally.into_us += pause;
        }
        if leap_into(reference, step.step) {
            tally.jump_us += pause;
            tally.jump_wrong += wrong;
        }
        if fast {
            tally.fast_slips += slips;
            tally.fast_us += pause;
        }
        // A long note before this step, left too soon: it belongs to the
        // bar the long note is written in.
        if let Some(timing) = step.timing
            && !timing.is_break
            && timing.written_ticks >= LONG_TICKS
            && timing.taken_us * 10 < timing.expected_us * CUT_SHORT_TENTHS
            && let Some(long) = step.step.checked_sub(1).and_then(measure_of)
        {
            tallies.entry(long).or_default().rushed += 1;
        }
    }
    tallies
}

/// Fast-note spots in neighbouring bars become one spot, up to four bars.
fn join_neighbours(reference: &ReferencePerformance, spots: Vec<Spot>) -> Vec<Spot> {
    let mut joined: Vec<Spot> = Vec::new();
    for spot in spots {
        let neighbour = joined.iter_mut().find(|other| {
            matches!(other.kind, Kind::FastNotes { .. } | Kind::SlowBars)
                && std::mem::discriminant(&other.kind) == std::mem::discriminant(&spot.kind)
                && after(other.last) == Some(spot.first)
                && other.last.get() - other.first.get() + 1 < MOST_BARS
        });
        if let Some(other) = neighbour {
            other.last = spot.first;
            other.weight += spot.weight;
            other.facts.pause_us += spot.facts.pause_us;
            other.facts.wrong += spot.facts.wrong;
            other.facts.tempo = other.facts.tempo.min(spot.facts.tempo).or(spot.facts.tempo);
            if let Kind::FastNotes { broken_chord } = spot.kind {
                other.kind = match other.kind {
                    Kind::FastNotes { broken_chord: was } => Kind::FastNotes {
                        broken_chord: was || broken_chord,
                    },
                    kind => kind,
                };
            }
            other.drill = practise_range(reference, other.first, other.last);
        } else {
            joined.push(spot);
        }
    }
    joined
}

/// The three most important cards; a spot on bars a card already covers
/// joins that card instead (once).
fn choose(mut spots: Vec<Spot>) -> Vec<Card> {
    spots.sort_by_key(|spot| (std::cmp::Reverse(spot.weight), spot.first));
    let mut cards: Vec<Card> = Vec::new();
    for spot in spots {
        let overlaps = |card: &Card| spot.first <= card.main.last && card.main.first <= spot.last;
        if let Some(card) = cards.iter_mut().find(|card| overlaps(card)) {
            if card.also.is_none() && card.main.kind != spot.kind {
                card.also = Some(spot);
            }
        } else if cards.len() < MOST_CARDS {
            cards.push(Card {
                main: spot,
                also: None,
            });
        }
    }
    cards
}

/// Trouble in more than half of three or more bars played, with under 70% of
/// the notes right: the piece is a big step up from where you are.
fn big_step(reference: &ReferencePerformance, bars: &Bars, accuracy: Option<u32>) -> Option<Spot> {
    let played: Vec<_> = bars.iter().collect();
    let trouble: Vec<_> = played
        .iter()
        .filter(|(_, bar)| bar.difficulty() >= 1000)
        .map(|(measure, _)| *measure)
        .collect();
    if played.len() < 3 || trouble.len() * 2 <= played.len() || accuracy.is_none_or(|a| a >= 70) {
        return None;
    }
    let first = *trouble.first()?;
    let last = (0..MOST_BARS - 1)
        .try_fold(first, |bar, _| {
            after(bar)
                .filter(|next| bars.get(*next).is_some())
                .ok_or(bar)
        })
        .unwrap_or_else(|bar| bar);
    Some(Spot {
        kind: Kind::BigStep,
        first,
        last,
        drill: practise_range(reference, first, last),
        weight: u64::MAX,
        facts: Facts {
            wrong: u32::try_from(trouble.len()).unwrap_or(u32::MAX),
            passes: u32::try_from(played.len()).unwrap_or(u32::MAX),
            ..Facts::default()
        },
    })
}

/// Runs of two or more bars with no wrong key, no pause and no stop, and
/// one run of three or more at an even tempo.
fn went_well(bars: &Bars, usual: Option<u64>) -> Vec<WentWell> {
    let ranges = |keep: &dyn Fn(&crate::bars::Bar) -> bool, least: u32| {
        let mut found: Vec<(MeasureNumber, MeasureNumber)> = Vec::new();
        for (measure, bar) in bars.iter() {
            if !keep(bar) {
                continue;
            }
            match found.last_mut() {
                Some((_, last)) if after(*last) == Some(measure) => *last = measure,
                _ => found.push((measure, measure)),
            }
        }
        found.retain(|(first, last)| last.get() - first.get() + 1 >= least);
        found.sort_by_key(|(first, last)| (std::cmp::Reverse(last.get() - first.get()), *first));
        found
    };
    let clean = ranges(
        &|bar| bar.wrong == 0 && bar.gave_up == 0 && bar.pause_us < 300_000 * u64::from(bar.passes),
        2,
    );
    let even = usual.map_or_else(Vec::new, |usual| {
        ranges(
            &|bar| {
                bar.timed >= 2
                    && bar
                        .tempo()
                        .is_some_and(|tempo| tempo.abs_diff(usual) * 100 <= usual * 15)
            },
            3,
        )
    });
    clean
        .into_iter()
        .take(2)
        .map(|(first, last)| WentWell::Clean { first, last })
        .chain(
            even.into_iter()
                .take(1)
                .map(|(first, last)| WentWell::Even { first, last }),
        )
        .collect()
}

/// The median tempo of the bars with enough timed steps to say.
fn usual_tempo(bars: &Bars) -> Option<u64> {
    let mut tempos: Vec<u64> = bars
        .iter()
        .filter(|(_, bar)| bar.timed >= 2)
        .filter_map(|(_, bar)| bar.tempo())
        .collect();
    tempos.sort_unstable();
    tempos.get(tempos.len() / 2).copied()
}

/// The most common offset among those `keep` accepts.
fn most_common(offsets: &[i8], keep: impl Fn(&i8) -> bool) -> Option<i8> {
    let mut counts: BTreeMap<i8, u32> = BTreeMap::new();
    for offset in offsets.iter().filter(|offset| keep(offset)) {
        *counts.entry(*offset).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|&(offset, count)| (count, std::cmp::Reverse(offset.unsigned_abs())))
        .map(|(offset, _)| offset)
}

/// The bars to loop to practise trouble in `first` to `last`: from one bar
/// before it (practise from a bar before the error, Duke, Simmons & Cash
/// 2009), clamped at bar 1. If that can't be looped in a row (a repeat in
/// the way), `first` to `last`; failing that, `first` alone.
#[must_use]
pub fn practise_range(
    reference: &ReferencePerformance,
    first: MeasureNumber,
    last: MeasureNumber,
) -> (MeasureNumber, MeasureNumber) {
    let loopable = |from: MeasureNumber| reference.range_of(from, last).is_some();
    [before(first).unwrap_or(first), first]
        .into_iter()
        .find(|&from| loopable(from))
        .map_or((first, first), |from| (from, last))
}

fn before(bar: MeasureNumber) -> Option<MeasureNumber> {
    MeasureNumber::new(bar.get().checked_sub(1)?)
}

fn after(bar: MeasureNumber) -> Option<MeasureNumber> {
    MeasureNumber::new(bar.get().checked_add(1)?)
}

#[cfg(test)]
mod tests {
    use hogwash_capture::Touch;
    use hogwash_domain::{MeasureNumber, Pitch};

    use hogwash_notation::ExpectedNotes;

    use crate::bars::Trace;
    use crate::coach::{Coach, Kind, WentWell, broken_chord, practise_range};
    use crate::fixtures::{minuet_in_time, played_in_time};

    const QUARTER_US: u64 = 600_000;

    fn bar(number: u32) -> MeasureNumber {
        MeasureNumber::new(number).expect("a bar number")
    }

    fn whole(late: &[(usize, u64)]) -> Vec<Trace> {
        let reference = minuet_in_time();
        let observed = played_in_time(&reference, 0..reference.len(), QUARTER_US, late);
        vec![Trace::of(&observed, &reference, 0..reference.len(), false)]
    }

    #[test]
    fn a_clean_run_has_no_cards_and_says_what_went_well() {
        let coach = Coach::of(&minuet_in_time(), &whole(&[]), Some(100));

        assert!(coach.cards.is_empty());
        assert!(coach.went_well.contains(&WentWell::Clean {
            first: bar(1),
            last: bar(3),
        }));
    }

    // Played 1 2 1 2 3: every loop starts a bar before the trouble, bar 1
    // can't, and bars the piece doesn't have are left as asked.
    #[test]
    fn practise_starts_one_bar_before_the_trouble() {
        let reference = minuet_in_time();

        assert_eq!(practise_range(&reference, bar(1), bar(1)), (bar(1), bar(1)));
        assert_eq!(practise_range(&reference, bar(2), bar(2)), (bar(1), bar(2)));
        assert_eq!(practise_range(&reference, bar(3), bar(3)), (bar(2), bar(3)));
        assert_eq!(practise_range(&reference, bar(2), bar(3)), (bar(1), bar(3)));
        assert_eq!(practise_range(&reference, bar(9), bar(9)), (bar(9), bar(9)));
    }

    // Two wrong notes in bar 3, every run: the card loops bars 2–3.
    #[test]
    fn a_wrong_notes_card_loops_from_the_bar_before() {
        let reference = minuet_in_time();
        let wrong = Pitch::new(40).expect("a key");
        let mut observed = played_in_time(&reference, 0..16, QUARTER_US, &[]);
        observed.record(wrong);
        observed.record(wrong);
        for step in 16..reference.len() {
            for pitch in reference
                .at(step)
                .into_iter()
                .flat_map(ExpectedNotes::sounding)
            {
                observed.record(pitch);
            }
        }
        let traces = vec![Trace::of(&observed, &reference, 0..reference.len(), false)];

        let coach = Coach::of(&reference, &traces, Some(100));

        let card = coach
            .cards
            .iter()
            .find(|card| card.main.kind == Kind::WrongNotes)
            .expect("a wrong-notes card");
        assert_eq!(card.main.drill, (bar(2), bar(3)));
    }

    // Step 16 is bar 3's first note: three seconds late is the change into it.
    #[test]
    fn a_pause_going_into_a_bar_is_a_transition_to_practise_with_the_bar_before() {
        let coach = Coach::of(&minuet_in_time(), &whole(&[(16, 3_000_000)]), Some(100));

        let card = coach.cards.first().expect("a card");
        assert_eq!(card.main.kind, Kind::IntoBar);
        assert_eq!(
            (card.main.first, card.main.drill),
            (bar(3), (bar(2), bar(3)))
        );
    }

    // Bar 3 played at half the tempo note by note, with no pause going in.
    #[test]
    fn bars_well_under_your_usual_tempo_slow_you_down() {
        let late = [(17, 300_000), (18, 300_000), (19, 300_000), (20, 300_000)];
        let coach = Coach::of(&minuet_in_time(), &whole(&late), Some(100));

        let card = coach.cards.first().expect("a card");
        assert_eq!(card.main.kind, Kind::SlowBars);
        assert_eq!(card.main.first, bar(3));
        assert!(card.main.facts.tempo < card.main.facts.usual_tempo);
    }

    // A4, the third note of bar 1 (an eighth), struck a key too low both
    // times through: G#4.
    #[test]
    fn a_slip_to_the_next_key_in_fast_notes_is_fast_notes() {
        let reference = minuet_in_time();
        let mut observed = hogwash_capture::ObservedPerformance::new();
        let mut at_us = 0;
        for step in 0..reference.len() {
            at_us += 300_000;
            if step == 2 || step == 10 {
                observed.record_touched(pitch(68), touch(at_us));
            }
            for note in reference
                .at(step)
                .into_iter()
                .flat_map(ExpectedNotes::sounding)
            {
                observed.record_touched(note, touch(at_us));
            }
        }
        let trace = Trace::of(&observed, &reference, 0..reference.len(), false);

        let coach = Coach::of(&reference, &[trace], Some(95));

        let card = coach.cards.first().expect("a card");
        assert_eq!(
            card.main.kind,
            Kind::FastNotes {
                broken_chord: false
            }
        );
        assert_eq!(card.main.facts.offset, Some(-1));
    }

    #[test]
    fn a_key_far_from_the_written_one_is_a_wrong_note_to_read() {
        let reference = minuet_in_time();
        let mut observed = played_in_time(&reference, 0..17, QUARTER_US, &[]);
        observed.record(Pitch::LOWEST);
        let trace = Trace::of(&observed, &reference, 0..reference.len(), false);

        let coach = Coach::of(&reference, &[trace], Some(95));

        assert!(
            coach
                .cards
                .iter()
                .any(|card| card.main.kind == Kind::WrongNotes && card.main.first == bar(3))
        );
    }

    #[test]
    fn stopping_in_the_same_bar_again_and_again_is_a_place_to_start() {
        let reference = minuet_in_time();
        // Into bar 3, then a wrong key there, then a restart.
        let mut observed = played_in_time(&reference, 0..18, QUARTER_US, &[]);
        observed.record(Pitch::LOWEST);
        let trace = Trace::of(&observed, &reference, 0..reference.len(), true);

        let coach = Coach::of(&reference, &[trace.clone(), trace], Some(100));

        assert!(
            coach
                .cards
                .iter()
                .any(|card| card.main.kind == Kind::KeepsStopping && card.main.first == bar(3))
        );
    }

    #[test]
    fn trouble_everywhere_says_the_piece_is_a_big_step() {
        let reference = minuet_in_time();
        let mut observed = hogwash_capture::ObservedPerformance::new();
        for step in 0..reference.len() {
            observed.record(Pitch::LOWEST);
            for note in reference
                .at(step)
                .into_iter()
                .flat_map(ExpectedNotes::sounding)
            {
                observed.record(note);
            }
        }
        let trace = Trace::of(&observed, &reference, 0..reference.len(), false);

        let coach = Coach::of(&reference, &[trace], Some(50));

        assert_eq!(
            coach.cards.first().map(|card| card.main.kind),
            Some(Kind::BigStep)
        );
        assert!(coach.cards.len() <= 3);
    }

    // The Minuet's bar 1 is a scale-like run (D5 G4 A4 B4 C5), not a chord.
    #[test]
    fn a_run_of_steps_is_not_a_broken_chord() {
        assert!(!broken_chord(&minuet_in_time(), bar(1)));
    }

    fn pitch(number: u8) -> Pitch {
        Pitch::new(number).expect("a key")
    }

    fn touch(at_us: u64) -> Touch {
        Touch {
            at_us,
            velocity: 64,
            released_us: None,
        }
    }
}
