//! `replay` and `expected`: real scores through the real practice path, no
//! keyboard needed.
//!
//! ```text
//! segno-cli replay   <score.musicxml> <events.json> [--from-bar <n> | --loop <a>-<b>]
//! segno-cli expected <score.musicxml>
//! segno-cli bars     <score.musicxml> <data folder> <piece id>
//! ```
//!
//! An events file is a JSON array of played notes, in the order they were
//! played: `[{"pitch": 60, "time_us": 0}, {"pitch": 64}]`. `pitch` is a MIDI
//! note number; `time_us` is optional and only printed. Any other field is
//! ignored, which is what lets `expected` tag each entry with its bar.

use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

use segno_analysis::{Analysis, Bar, Bars};
use segno_domain::{MeasureNumber, PieceId, Pitch, PitchSpelling, Step};
use segno_engraving::{EngravedScore, Engraver as _};
use segno_engraving_verovio::VerovioEngraver;
use segno_feedback::Feedback;
use segno_notation::{ExpectedNotes, ReferencePerformance};
use segno_practice::{Attempt, CursorMovement, Loop};
use segno_records::{Ended, Records};
use serde::Deserialize;

/// One note from an events file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct Event {
    pitch: u8,
    #[serde(default)]
    time_us: Option<u64>,
}

/// Reads an events file's contents.
pub(crate) fn events_from(json: &str) -> Result<Vec<(Pitch, Option<u64>)>, String> {
    let events: Vec<Event> =
        serde_json::from_str(json).map_err(|failure| format!("not an events file: {failure}"))?;

    events
        .into_iter()
        .enumerate()
        .map(|(index, event)| {
            Pitch::new(event.pitch)
                .map(|pitch| (pitch, event.time_us))
                .ok_or_else(|| format!("event {index}: pitch {} is outside 0..=127", event.pitch))
        })
        .collect()
}

/// Where a replayed attempt starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Start {
    Beginning,
    /// At this bar, as clicking it in the app does.
    Bar(u32),
    /// Looping these bars, as Shift+clicking does: each pass is its own
    /// attempt, and the count of clean passes is printed at each wrap.
    Loop(u32, u32),
}

/// `segno-cli replay <score> <events>`: every event, step by step, then the
/// analysis and feedback the performer would get, from where `start` says.
pub(crate) fn replay(score: &Path, events: &Path, start: Start) -> Result<String, String> {
    let engraved = engrave(score)?;
    let text = std::fs::read_to_string(events)
        .map_err(|failure| format!("cannot read {}: {failure}", events.display()))?;
    let events = events_from(&text)?;

    let reference = Arc::new(ReferencePerformance::derived_from(engraved.score()));
    let bar = |number: u32| {
        MeasureNumber::new(number).ok_or_else(|| format!("`{number}` is not a bar number"))
    };
    let steps = match start {
        Start::Beginning => 0..reference.len(),
        Start::Bar(number) => reference
            .first_position_of(bar(number)?)
            .map(|from| from..reference.len())
            .ok_or_else(|| format!("bar {number} has nothing to start at"))?,
        Start::Loop(first, last) => reference
            .range_of(bar(first)?, bar(last)?)
            .ok_or_else(|| format!("bars {first}-{last} are not played in a row"))?,
    };
    let mut looping = matches!(start, Start::Loop(..)).then(|| Loop::new(steps.clone(), None));
    let mut attempt = Attempt::begin_at(Arc::clone(&reference), steps.start);
    let mut passes = 0;

    let mut out = header(&engraved, &reference);

    for (pitch, time_us) in events {
        let at = attempt.cursor().position();
        let expected = attempt.cursor().expecting().cloned();
        let tolerated = attempt.cursor().tolerates(pitch);
        if let Some(looping) = looping.as_mut() {
            looping.heard(attempt.cursor(), pitch);
        }
        let movement = attempt.play(pitch);

        let time = time_us.map_or_else(|| "        ".to_owned(), seconds);
        let (bar, wanted) = expected.as_ref().map_or_else(
            || ("-".to_owned(), "(end of piece)".to_owned()),
            |expected| (expected.measure().get().to_string(), names(expected)),
        );
        let outcome = if tolerated {
            "tolerated (grace note or ornament), held".to_owned()
        } else {
            outcome(&attempt, expected.as_ref(), pitch, movement)
        };

        let _ = writeln!(
            out,
            "{time} bar {bar:>3}  step {at:>4}  expected {wanted:<18} played {:>3} {:<4} {outcome}",
            u8::from(pitch),
            sharp_name(pitch),
        );

        if let Some(looping) = looping.as_mut()
            && looping.is_at_the_end(attempt.cursor())
        {
            passes += 1;
            let pass = std::mem::replace(
                &mut attempt,
                Attempt::begin_at(Arc::clone(&reference), steps.start),
            )
            .end();
            let summary = Analysis::over(pass.observed(), &reference, steps.clone()).summary();
            let _ = writeln!(
                out,
                "--- pass {passes}: {} correct, {} wrong, {} unexpected · {:?}",
                summary.correct(),
                summary.incorrect(),
                summary.unexpected(),
                looping.wrapped(),
            );
        }
    }

    let ended = attempt.end();
    let analysis = Analysis::over(ended.observed(), &reference, steps);

    out.push('\n');
    for line in crate::summarise(analysis.summary()) {
        let _ = writeln!(out, "{line}");
    }
    out.push_str("\nfeedback:\n");
    for line in Feedback::of(&analysis).lines() {
        if line.is_empty() {
            out.push('\n');
        } else {
            let _ = writeln!(out, "  {line}");
        }
    }
    Ok(out)
}

/// `segno-cli expected <score>`: the Reference Performance as an events
/// file — a perfect performance, one entry per required pitch.
pub(crate) fn expected(score: &Path) -> Result<String, String> {
    let engraved = engrave(score)?;
    let reference = ReferencePerformance::derived_from(engraved.score());

    let entries: Vec<String> = reference
        .iter()
        .flat_map(|step| {
            step.sounding().map(move |pitch| {
                format!(
                    "  {{\"pitch\": {}, \"bar\": {}}}",
                    u8::from(pitch),
                    step.measure().get()
                )
            })
        })
        .collect();

    Ok(format!("[\n{}\n]\n", entries.join(",\n")))
}

/// `segno-cli bars <score> <data> <piece>`: every session of the piece
/// kept under the data folder `data`, added up bar by bar, with each bar's
/// level now (its own last three sessions, as the app judges it). Only reads.
pub(crate) fn bars(score: &Path, data: &Path, piece: &str) -> Result<String, String> {
    let engraved = engrave(score)?;
    let reference = ReferencePerformance::derived_from(engraved.score());
    let piece = PieceId::new(piece).ok_or_else(|| format!("`{piece}` is not a piece id"))?;
    let (sessions, unreadable) = Records::under(data)
        .sessions()
        .map_err(|failure| format!("cannot read sessions: {failure:?}"))?;

    let mut out = header(&engraved, &reference);
    let mut mine: Vec<_> = sessions
        .iter()
        .filter(|session| session.piece == piece)
        .collect();
    mine.sort_by_key(|session| session.started.parse::<u64>().unwrap_or(0));
    let mut total = Bars::default();
    let mut per_session = Vec::new();
    let mut attempts = 0;
    for session in mine {
        let mut bars = Bars::default();
        for attempt in &session.attempts {
            let to = attempt.to.unwrap_or(reference.len()).min(reference.len());
            let gave_up = matches!(attempt.ended, Some(Ended::Restart | Ended::Stop));
            bars.add(&Bars::of(
                &attempt.played,
                &reference,
                attempt.from.min(to)..to,
                gave_up,
            ));
            attempts += 1;
        }
        total.add(&bars);
        per_session.push(bars);
    }
    let now = Bars::now(&per_session);
    let level = |measure| now.get(measure).and_then(Bar::judged);
    let _ = writeln!(
        out,
        "{attempts} attempts in {} sessions; {} session files unreadable\n",
        per_session.len(),
        unreadable.len()
    );

    out.push_str(
        "bar  passes  timed  pause s  into s  longest s  breaks  wrong  gave up  difficulty  level now\n",
    );
    let tenths = |us: u64| format!("{}.{}", us / 1_000_000, us / 100_000 % 10);
    for (measure, bar) in total.iter() {
        let _ = writeln!(
            out,
            "{:>3}  {:>6}  {:>5}  {:>7}  {:>6}  {:>9}  {:>6}  {:>5}  {:>7}  {:>10}  {:>9}",
            measure.get(),
            bar.passes,
            bar.timed,
            tenths(bar.pause_us),
            tenths(bar.into_bar_pause_us),
            tenths(bar.longest_pause_us),
            bar.breaks,
            bar.wrong,
            bar.gave_up,
            bar.difficulty(),
            level(measure).map_or_else(|| "·".to_owned(), |level| level.to_string()),
        );
    }

    let mut hardest: Vec<_> = now
        .iter()
        .filter_map(|(measure, bar)| Some((measure, bar.judged()?, bar.difficulty())))
        .filter(|&(_, level, _)| level >= 3)
        .collect();
    hardest.sort_by_key(|&(measure, level, difficulty)| {
        (std::cmp::Reverse((level, difficulty)), measure)
    });
    let names: Vec<String> = hardest
        .iter()
        .take(2)
        .map(|(measure, _, _)| measure.get().to_string())
        .collect();
    let _ = writeln!(out, "\nweakest (level 3+ now): bars {}", names.join(", "));
    Ok(out)
}

/// Imports `score` into the data folder `data`, as the app's import keeps a
/// piece (the same `segno-records` calls), and answers its new id. `title`
/// is kept as a title the performer chose. For seeding a test's data folder
/// without a file dialog; it doesn't check for a second copy, as the app does.
pub(crate) fn import(data: &Path, score: &Path, title: Option<&str>) -> Result<String, String> {
    let source = std::fs::read(score)
        .map_err(|failure| format!("cannot read {}: {failure}", score.display()))?;
    let engraved = engrave(score)?;
    let details = engraved.details();
    let file_name = score.file_name().map(|name| name.to_string_lossy());
    let records = Records::under(data);
    let id = records.new_import_id(&segno_records::fingerprint(&source));

    records
        .keep_piece(
            &id,
            details.title.as_deref().unwrap_or("Untitled"),
            details.composer.as_deref(),
            file_name.as_deref(),
            &source,
        )
        .map_err(|failure| format!("cannot keep it: {failure:?}"))?;
    if let Some(title) = title {
        records
            .rename_piece(&id, title)
            .map_err(|failure| format!("cannot title it: {failure:?}"))?;
    }
    Ok(format!("{}\n", id.as_str()))
}

fn engrave(score: &Path) -> Result<EngravedScore, String> {
    let source = std::fs::read(score)
        .map_err(|failure| format!("cannot read {}: {failure}", score.display()))?;

    VerovioEngraver::from_environment()
        .and_then(|mut engraver| engraver.engrave(&source))
        .map_err(|failure| format!("cannot engrave {}: {failure:?}", score.display()))
}

fn header(engraved: &EngravedScore, reference: &ReferencePerformance) -> String {
    let details = engraved.details();
    format!(
        "piece: {}{}\nsteps: {} across {} bars\n\n",
        details.title.as_deref().unwrap_or("(untitled)"),
        details
            .composer
            .as_deref()
            .map_or_else(String::new, |composer| format!(" — {composer}")),
        reference.len(),
        engraved.score().measures().len(),
    )
}

fn outcome(
    attempt: &Attempt,
    expected: Option<&ExpectedNotes>,
    played: Pitch,
    movement: CursorMovement,
) -> String {
    let Some(expected) = expected else {
        return "after the end".to_owned();
    };

    let correct = expected.sounding().any(|wanted| wanted == played);

    match (correct, movement) {
        (true, CursorMovement::Advanced) => "correct, advanced".to_owned(),
        (true, CursorMovement::Held) => {
            let waiting: Vec<String> = expected
                .notes()
                .iter()
                .filter(|note| !attempt.cursor().has_been_played(note.sounding()))
                .map(|note| spelled(note.spelling()))
                .collect();
            format!("correct, waiting for {}", waiting.join("+"))
        }
        (false, _) => "WRONG, held".to_owned(),
    }
}

fn names(expected: &ExpectedNotes) -> String {
    expected
        .notes()
        .iter()
        .map(|note| spelled(note.spelling()))
        .collect::<Vec<_>>()
        .join("+")
}

fn seconds(time_us: u64) -> String {
    format!(
        "{:>3}.{:03}s",
        time_us / 1_000_000,
        time_us % 1_000_000 / 1_000
    )
}

fn letter(step: Step) -> &'static str {
    match step {
        Step::C => "C",
        Step::D => "D",
        Step::E => "E",
        Step::F => "F",
        Step::G => "G",
        Step::A => "A",
        Step::B => "B",
    }
}

/// A written note as written: `F#4`, `Bb3`.
fn spelled(spelling: PitchSpelling) -> String {
    let accidental = match spelling.alter() {
        0 => String::new(),
        alter if alter > 0 => "#".repeat(usize::from(alter.unsigned_abs())),
        alter => "b".repeat(usize::from(alter.unsigned_abs())),
    };
    format!(
        "{}{accidental}{}",
        letter(spelling.step()),
        spelling.octave()
    )
}

/// A played key, named with sharps. A keyboard reports no spelling; this is
/// only a reading aid for a debugging tool.
fn sharp_name(pitch: Pitch) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let number = u8::from(pitch);
    let name = NAMES.get(usize::from(number % 12)).copied().unwrap_or("?");
    format!("{name}{}", i16::from(number / 12) - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_carry_pitch_and_an_optional_time() {
        let events = events_from(r#"[{"pitch": 60, "time_us": 500000}, {"pitch": 64}]"#).unwrap();

        assert_eq!(
            events,
            vec![
                (Pitch::new(60).unwrap(), Some(500_000)),
                (Pitch::new(64).unwrap(), None),
            ]
        );
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let events = events_from(r#"[{"pitch": 60, "bar": 3}]"#).unwrap();

        assert_eq!(events, vec![(Pitch::new(60).unwrap(), None)]);
    }

    #[test]
    fn an_event_without_a_pitch_is_refused() {
        assert!(events_from(r#"[{"time_us": 0}]"#).is_err());
    }

    #[test]
    fn a_pitch_outside_the_midi_range_is_refused() {
        let refused = events_from(r#"[{"pitch": 60}, {"pitch": 128}]"#).unwrap_err();

        assert!(refused.contains("event 1"), "{refused}");
    }

    #[test]
    fn times_print_as_seconds() {
        assert_eq!(seconds(1_234_567), "  1.234s");
    }

    #[test]
    fn played_keys_are_named_with_sharps_and_middle_c_is_c4() {
        assert_eq!(sharp_name(Pitch::new(60).unwrap()), "C4");
        assert_eq!(sharp_name(Pitch::new(66).unwrap()), "F#4");
        assert_eq!(sharp_name(Pitch::new(21).unwrap()), "A0");
    }

    #[test]
    fn written_notes_keep_their_spelling() {
        assert_eq!(spelled(PitchSpelling::new(Step::G, -1, 4)), "Gb4");
        assert_eq!(spelled(PitchSpelling::new(Step::F, 2, 5)), "F##5");
    }
}
