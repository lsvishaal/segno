//! `replay` and `expected`: real scores through the real practice path, no
//! keyboard needed.
//!
//! ```text
//! hogwash-cli replay   <score.musicxml> <events.json>
//! hogwash-cli expected <score.musicxml>
//! ```
//!
//! An events file is a JSON array of played notes, in the order they were
//! played: `[{"pitch": 60, "time_us": 0}, {"pitch": 64}]`. `pitch` is a MIDI
//! note number; `time_us` is optional and only printed. Any other field is
//! ignored, which is what lets `expected` tag each entry with its bar.

use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

use hogwash_analysis::Analysis;
use hogwash_domain::{Pitch, PitchSpelling, Step};
use hogwash_engraving::{EngravedScore, Engraver as _};
use hogwash_engraving_verovio::VerovioEngraver;
use hogwash_feedback::Feedback;
use hogwash_notation::{ExpectedNotes, ReferencePerformance};
use hogwash_practice::{Attempt, CursorMovement};
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

/// `hogwash-cli replay <score> <events>`: every event, step by step, then the
/// analysis and feedback the performer would get.
pub(crate) fn replay(score: &Path, events: &Path) -> Result<String, String> {
    let engraved = engrave(score)?;
    let text = std::fs::read_to_string(events)
        .map_err(|failure| format!("cannot read {}: {failure}", events.display()))?;
    let events = events_from(&text)?;

    let reference = Arc::new(ReferencePerformance::derived_from(engraved.score()));
    let mut attempt = Attempt::begin(Arc::clone(&reference));

    let mut out = header(&engraved, &reference);

    for (pitch, time_us) in events {
        let at = attempt.cursor().position();
        let expected = attempt.cursor().expecting().cloned();
        let tolerated = attempt.cursor().tolerates(pitch);
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
    }

    let ended = attempt.end();
    let analysis = Analysis::of(ended.observed(), &reference);

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

/// `hogwash-cli expected <score>`: the Reference Performance as an events
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
