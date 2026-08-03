//! Determinism harness. **Not a product surface.**
//!
//! `FR-406`, `NFR-300` and `NFR-301` require identical results across
//! separate executions, and `MVP §9.2` makes that release-blocking. A test
//! inside one process cannot demonstrate it: the failure this guards against
//! — a hash-ordered container, a pointer address, an uninitialised read —
//! shows up precisely *between* runs. So `just determinism` runs this binary
//! twice and diffs the bytes.
//!
//! It reads **no file format**, deliberately. How practice material and
//! records are stored is an unmade decision (`004 §15`), and inventing a
//! format in order to test determinism would be exactly the speculative work
//! this project avoids. The score is built in code; the performance arrives
//! as MIDI note numbers on the command line.
//!
//! The formatting lives here rather than on `Analysis`, because `003 §7.6`
//! says an Analysis "does not present anything; Feedback does". This is a
//! raw dump for a diff, not the human-readable output FR-500 asks for.
//!
//! ```text
//! hogwash-cli 60 62 64 65 67
//! ```

use std::process::ExitCode;

use hogwash_analysis::{Analysis, Finding, Summary};
use hogwash_capture::ObservedPerformance;
use hogwash_domain::{MeasureNumber, Pitch, PitchSpelling, Step};
use hogwash_feedback::Feedback;
use hogwash_notation::{Measure, Note, Position, ReferencePerformance, Score};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    let observed = match performance_from(&arguments) {
        Ok(observed) => observed,
        Err(complaint) => {
            eprintln!("hogwash-cli: {complaint}");
            eprintln!("usage: hogwash-cli <midi-note-number>...");
            return ExitCode::from(2);
        }
    };

    let Some(reference) = exercise() else {
        eprintln!("hogwash-cli: the built-in exercise is malformed");
        return ExitCode::from(3);
    };

    print!(
        "{}",
        report(&Analysis::of(&observed, &reference), &reference)
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
fn report(analysis: &Analysis, reference: &ReferencePerformance) -> String {
    let mut lines = vec![
        "piece: built-in exercise".to_owned(),
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
        Finding::Correct {
            measure,
            position,
            pitch,
        } => format!(
            "correct     bar {:<3} position {:<3} played {}",
            measure.get(),
            position,
            u8::from(*pitch)
        ),
        Finding::IncorrectPitch {
            measure,
            position,
            played,
            expected,
        } => format!(
            "incorrect   bar {:<3} position {:<3} played {}  expected {}",
            measure.get(),
            position,
            u8::from(*played),
            numbers(expected)
        ),
        Finding::Omitted {
            measure,
            position,
            written,
        } => format!(
            "omitted     bar {:<3} position {:<3} wanted {}",
            measure.get(),
            position,
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

/// FR-408 — one overall result. Counts only: no grade, no percentage, no
/// verdict (`003 §7.6`, FR-503).
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
