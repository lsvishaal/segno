//! Two jobs, both for developers rather than performers.
//!
//! **Determinism harness** (no subcommand). The same performance must give
//! byte-identical analysis across separate processes, and a test inside one
//! process cannot show that: a hash-ordered container or a pointer address
//! only shows up *between* runs. So `just determinism` runs this twice and
//! diffs the bytes. The score is built in code and the performance arrives as
//! MIDI note numbers:
//!
//! ```text
//! segno-cli 60 62 64 65 67
//! ```
//!
//! **Replay** (`replay`, `expected`, `bars`): a real `MusicXML` file through
//! the real practice and analysis path, with no keyboard. See [`replay`].

mod replay;

use std::path::Path;
use std::process::ExitCode;

use replay::Start;

use segno_analysis::{Analysis, Finding, Summary};
use segno_capture::ObservedPerformance;
use segno_domain::{MeasureNumber, Pitch, PitchSpelling, Step};
use segno_feedback::Feedback;
use segno_notation::{Measure, Note, Position, ReferencePerformance, Score};

const USAGE: &str = "replay <score.musicxml> <events.json> [--from-bar <n> | --loop <a>-<b>] | expected <score.musicxml> | bars <score.musicxml> <data folder> <piece id>";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    let replayed = match arguments.as_slice() {
        [command, score, events] if command == "replay" => Some(replay::replay(
            Path::new(score),
            Path::new(events),
            Start::Beginning,
        )),
        [command, score, events, flag, bar] if command == "replay" && flag == "--from-bar" => Some(
            bar.parse()
                .map_err(|_| format!("`{bar}` is not a bar number"))
                .and_then(|bar| {
                    replay::replay(Path::new(score), Path::new(events), Start::Bar(bar))
                }),
        ),
        [command, score, events, flag, bars] if command == "replay" && flag == "--loop" => Some(
            bars.split_once('-')
                .and_then(|(first, last)| Some((first.parse().ok()?, last.parse().ok()?)))
                .ok_or_else(|| format!("`{bars}` is not a range of bars, like 9-12"))
                .and_then(|(first, last)| {
                    replay::replay(
                        Path::new(score),
                        Path::new(events),
                        Start::Loop(first, last),
                    )
                }),
        ),
        [command, score] if command == "expected" => Some(replay::expected(Path::new(score))),
        [command, score, data, piece] if command == "bars" => {
            Some(replay::bars(Path::new(score), Path::new(data), piece))
        }
        [command, ..] if ["replay", "expected", "bars"].contains(&command.as_str()) => {
            Some(Err(format!("usage: segno-cli {USAGE}")))
        }
        _ => None,
    };

    if let Some(result) = replayed {
        return match result {
            Ok(output) => {
                print!("{output}");
                ExitCode::SUCCESS
            }
            Err(complaint) => {
                eprintln!("segno-cli: {complaint}");
                ExitCode::from(2)
            }
        };
    }

    let observed = match performance_from(&arguments) {
        Ok(observed) => observed,
        Err(complaint) => {
            eprintln!("segno-cli: {complaint}");
            eprintln!("usage: segno-cli <midi-note-number>...");
            eprintln!("       segno-cli {USAGE}");
            return ExitCode::from(2);
        }
    };

    let Some(reference) = exercise() else {
        eprintln!("segno-cli: the built-in exercise is malformed");
        return ExitCode::from(3);
    };

    print!(
        "{}",
        report(
            &Analysis::of(&observed, &reference),
            &reference,
            "built-in exercise"
        )
    );

    ExitCode::SUCCESS
}

/// Parses MIDI note numbers into a performance.
fn performance_from(arguments: &[String]) -> Result<ObservedPerformance, String> {
    let mut observed = ObservedPerformance::new();

    for argument in arguments {
        let number: u8 = argument
            .parse()
            .map_err(|_| format!("`{argument}` is not a MIDI note number (0..=127)"))?;

        let pitch = Pitch::new(number).ok_or_else(|| format!("`{number}` is outside 0..=127"))?;

        observed.record(pitch);
    }

    Ok(observed)
}

/// The built-in exercise: two bars of melody and a closing chord.
///
/// Fixed forever. If it changes, the diff this harness performs stops
/// comparing like with like.
fn exercise() -> Option<ReferencePerformance> {
    let single = |step: Step| -> Option<Position> {
        Position::new(vec![Note::new(PitchSpelling::new(step, 0, 4))?])
    };

    let mut first = Vec::new();
    for step in [Step::C, Step::D, Step::E, Step::F] {
        first.push(single(step)?);
    }

    let mut second = Vec::new();
    for step in [Step::G, Step::A, Step::B] {
        second.push(single(step)?);
    }

    let mut chord_notes = Vec::new();
    for step in [Step::C, Step::E, Step::G] {
        chord_notes.push(Note::new(PitchSpelling::new(step, 0, 4))?);
    }
    let chord = Position::new(chord_notes)?;

    let score = Score::new(vec![
        Measure::new(MeasureNumber::new(1)?, first),
        Measure::new(MeasureNumber::new(2)?, second),
        Measure::new(MeasureNumber::new(3)?, vec![chord]),
    ])?;

    Some(ReferencePerformance::derived_from(&score))
}

/// Renders the Analysis as a stable, diffable block of text.
///
/// Every line is derived from the ordered findings, so the output is fixed by
/// the inputs alone — which is the property under test.
fn report(analysis: &Analysis, reference: &ReferencePerformance, piece: &str) -> String {
    let mut lines = vec![
        format!("piece: {piece}"),
        format!("expected positions: {}", reference.len()),
        String::new(),
        "findings:".to_owned(),
    ];

    if analysis.findings().is_empty() {
        lines.push("  (none)".to_owned());
    } else {
        lines.extend(
            analysis
                .findings()
                .iter()
                .map(|finding| format!("  {}", describe(finding))),
        );
    }

    lines.push(String::new());
    lines.extend(summarise(analysis.summary()));

    // The rendered Feedback is included so `just determinism` compares the
    // words the performer actually reads, not only the raw findings. Wording
    // that varied between runs would otherwise go unnoticed.
    lines.push(String::new());
    lines.push("feedback:".to_owned());
    lines.extend(Feedback::of(analysis).lines().into_iter().map(|line| {
        if line.is_empty() {
            line
        } else {
            format!("  {line}")
        }
    }));

    // Trailing newline, so the file compared by `cmp` ends the way a text
    // file should and the diff on failure reads cleanly.
    lines.join("\n") + "\n"
}

fn describe(finding: &Finding) -> String {
    match finding {
        Finding::Correct { at, pitch } => format!(
            "correct     bar {:<3} position {:<3} played {}",
            at.measure().get(),
            at.position(),
            u8::from(*pitch)
        ),
        Finding::IncorrectPitch {
            at,
            played,
            expected,
        } => format!(
            "incorrect   bar {:<3} position {:<3} played {}  expected {}",
            at.measure().get(),
            at.position(),
            u8::from(*played),
            numbers(expected)
        ),
        Finding::Omitted { at, written } => format!(
            "omitted     bar {:<3} position {:<3} wanted {}",
            at.measure().get(),
            at.position(),
            u8::from(written.sounding())
        ),
        Finding::Unexpected { played } => {
            format!(
                "unexpected                            played {}",
                u8::from(*played)
            )
        }
    }
}

fn numbers(notes: &[Note]) -> String {
    notes
        .iter()
        .map(|note| u8::from(note.sounding()).to_string())
        .collect::<Vec<_>>()
        .join("+")
}

/// One overall result. Counts only: no grade, no percentage, no verdict.
fn summarise(summary: Summary) -> Vec<String> {
    vec![
        "summary:".to_owned(),
        format!("  correct        {}", summary.correct()),
        format!("  incorrect      {}", summary.incorrect()),
        format!("  omitted        {}", summary.omitted()),
        format!("  unexpected     {}", summary.unexpected()),
        format!(
            "  reached end    {}",
            if summary.reached_the_end() {
                "yes"
            } else {
                "no"
            }
        ),
    ]
}
