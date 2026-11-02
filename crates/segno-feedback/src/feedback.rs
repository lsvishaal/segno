//! Feedback — an Analysis a performer can read.

use std::collections::BTreeSet;

use segno_analysis::{Analysis, Finding, Summary};
use segno_domain::MeasureNumber;

use crate::statement::Statement;

/// The presentation of an Analysis in terms the performer can act on.
///
/// **Derived entirely from one Analysis, adding nothing.** The design —
/// *"Feedback introduces no findings of its own"*. The same Analysis always
/// yields the same Feedback, so revisiting an Attempt later reads the same
/// way it did on the day.
///
/// The requirement (recurring mistakes across attempts) is deferred, and is
/// not merely unimplemented: [`of`](Self::of) takes one Analysis, and one
/// Analysis concerns one Attempt, so recurrence across attempts
/// is not expressible here.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Feedback {
    statements: Vec<Statement>,
    overall: Summary,
}

impl Feedback {
    /// Presents `analysis`.
    #[must_use]
    pub fn of(analysis: &Analysis) -> Self {
        let mut statements = Vec::new();
        let mut after_the_end = 0;

        for finding in analysis.findings() {
            match finding {
                Finding::Correct { .. } => {}
                Finding::IncorrectPitch { at, expected, .. } => {
                    // One statement per written note the performer missed at
                    // this position. A chord wants several notes, and naming
                    // them individually is what tells the performer which.
                    statements.extend(expected.iter().map(|written| {
                        Statement::NotPlayedAsWritten {
                            at: *at,
                            written: *written,
                        }
                    }));
                }
                Finding::Omitted { at, written } => statements.push(Statement::NeverPlayed {
                    at: *at,
                    written: *written,
                }),
                Finding::Unexpected { .. } => after_the_end += 1,
            }
        }

        if after_the_end > 0 {
            statements.push(Statement::PlayedAfterTheEnd {
                count: after_the_end,
            });
        }

        Self {
            statements,
            overall: analysis.summary(),
        }
    }

    /// Everything Feedback has to say, in the order it happened.
    #[must_use]
    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }

    /// The overall result for the Attempt.
    ///
    /// The Analysis's summary unchanged — counts, with no grade or
    /// percentage. The design says Feedback adds nothing.
    #[must_use]
    pub const fn overall(&self) -> Summary {
        self.overall
    }

    /// The bars containing something to look at.
    ///
    /// Sorted and without repeats. This is the requirement read through its acceptance
    /// criterion — *"the areas of the piece associated with those deviations
    /// are identified"* — which asks **where**, not what to do.
    ///
    /// It is not the requirement, which asks for analysis results *aggregated* per
    /// measure and stays deferred: a list of bars is not a per-bar
    /// result. Nothing here says how many notes went wrong in bar four.
    ///
    /// A `BTreeSet` rather than a `HashSet`, so the order is the Score's
    /// order on every run.
    #[must_use]
    pub fn measures_with_deviations(&self) -> Vec<MeasureNumber> {
        self.statements
            .iter()
            .filter_map(Statement::measure)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Feedback in words, one line per statement, then the overall result.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self.statements.iter().map(Statement::describe).collect();

        if lines.is_empty() {
            lines.push("Every written note was played.".to_owned());
        }

        lines.push(String::new());
        lines.push(format!(
            "{} notes played as written, {} not as written, {} never played.",
            self.overall.correct(),
            self.overall.incorrect(),
            self.overall.omitted()
        ));
        lines.push(
            if self.overall.reached_the_end() {
                "The attempt reached the end of the piece."
            } else {
                "The attempt stopped before the end of the piece."
            }
            .to_owned(),
        );

        lines
    }
}

#[cfg(test)]
mod tests {
    use segno_domain::{Location, MeasureNumber, Step};

    use crate::feedback::Feedback;
    use crate::fixtures::{analyse, bars, note, note_letters_in, pitch, unexpected};
    use crate::statement::Statement;

    fn bar(number: u32) -> MeasureNumber {
        MeasureNumber::new(number).expect("fixture measures are numbered from one")
    }

    fn rendered(feedback: &Feedback) -> String {
        feedback.lines().join("\n")
    }

    // - "present the results of analysis in a form comprehensible to
    // the target user without requiring interpretation of raw data".
    #[test]
    fn feedback_says_what_was_written_where_a_wrong_note_was_played() {
        let reference = bars(&[&[(Step::C, 0), (Step::F, 0)]]);
        let analysis = analyse(&[pitch(Step::C, 0), unexpected()], &reference);

        let feedback = Feedback::of(&analysis);

        assert_eq!(
            feedback.statements()[0],
            Statement::NotPlayedAsWritten {
                at: Location::new(bar(1), 1),
                written: note(Step::F, 0),
            }
        );
    }

    // The rendered text may name what the Score wrote and
    // must never name what the performer played -- naming it would choose
    // between two spellings of one key, which is a guess about notation.
    #[test]
    fn feedback_never_names_the_note_that_was_played() {
        // B flat written; the performer plays a G and stops. If the played
        // note were ever named, a 'G' would appear in the output.
        //
        // B appears twice, and both are true: a different note was played
        // there, and the written B flat never arrived. Feedback adds nothing
        // to the Analysis, and it subtracts nothing either.
        let reference = bars(&[&[(Step::B, -1)]]);
        let analysis = analyse(&[pitch(Step::G, 0)], &reference);

        let named = note_letters_in(&rendered(&Feedback::of(&analysis)));

        assert!(!named.is_empty(), "the written note should be named");
        assert!(
            named.iter().all(|letter| *letter == 'B'),
            "only the written note may be named, found {named:?}"
        );
    }

    // The requirement, read through its acceptance criterion: "the areas of the piece
    // associated with those deviations are identified". Which bars to look
    // at -- not what to do about them.
    #[test]
    fn feedback_identifies_the_measures_containing_deviations() {
        let reference = bars(&[
            &[(Step::C, 0)],
            &[(Step::D, 0)],
            &[(Step::E, 0)],
            &[(Step::F, 0)],
        ]);
        // Bar 1 correct; a wrong note in bar 2; then stops, so bars 3 and 4
        // were never played.
        let analysis = analyse(
            &[pitch(Step::C, 0), unexpected(), pitch(Step::D, 0)],
            &reference,
        );

        let feedback = Feedback::of(&analysis);

        assert_eq!(
            feedback.measures_with_deviations(),
            vec![bar(2), bar(3), bar(4)]
        );
    }

    // Each bar appears once however many deviations it holds, and in score
    // order. Anything else would be a per-measure result, which is the requirement and
    // deferred.
    #[test]
    fn each_measure_is_identified_once_and_in_order() {
        let reference = bars(&[&[(Step::C, 0), (Step::D, 0)]]);
        let analysis = analyse(&[unexpected(), unexpected()], &reference);

        assert_eq!(
            Feedback::of(&analysis).measures_with_deviations(),
            vec![bar(1)]
        );
    }

    // The requirement reaches the performer as plain words.
    #[test]
    fn feedback_reports_notes_that_were_never_played() {
        let reference = bars(&[&[(Step::C, 0)], &[(Step::D, 0)]]);
        let analysis = analyse(&[pitch(Step::C, 0)], &reference);

        let feedback = Feedback::of(&analysis);

        assert_eq!(
            feedback.statements(),
            // Position 1: the index runs across the whole Reference
            // Performance, not restarting per bar. Bar 1's C is position 0.
            &[Statement::NeverPlayed {
                at: Location::new(bar(2), 1),
                written: note(Step::D, 0),
            }]
        );
    }

    // Notes played after the end have no location, so they are
    // counted in one statement rather than repeated without a place. Counting
    // findings the Analysis already produced is presentation, not a new
    // finding.
    #[test]
    fn notes_played_after_the_end_are_gathered_into_one_statement() {
        let reference = bars(&[&[(Step::C, 0)]]);
        let analysis = analyse(&[pitch(Step::C, 0), unexpected(), unexpected()], &reference);

        assert_eq!(
            Feedback::of(&analysis).statements(),
            &[Statement::PlayedAfterTheEnd { count: 2 }]
        );
    }

    // - Feedback holds nothing of its own. A faultless attempt has
    // nothing to report, and says so rather than inventing praise, which
    // would be a judgement.
    #[test]
    fn a_faultless_attempt_yields_feedback_with_no_deviations() {
        let reference = bars(&[&[(Step::C, 0), (Step::D, 0)]]);
        let analysis = analyse(&[pitch(Step::C, 0), pitch(Step::D, 0)], &reference);

        let feedback = Feedback::of(&analysis);

        assert!(feedback.statements().is_empty());
        assert!(feedback.measures_with_deviations().is_empty());
    }

    // - "Feedback presents exactly one Analysis and adds nothing to
    // it." Every deviation the Analysis found is accounted for, and none is
    // invented: the statements reconcile exactly with the Analysis's counts.
    #[test]
    fn feedback_adds_nothing_the_analysis_did_not_establish() {
        let reference = bars(&[&[(Step::C, 0), (Step::D, 0)], &[(Step::E, 0)]]);
        let analysis = analyse(&[unexpected(), pitch(Step::C, 0)], &reference);
        let summary = analysis.summary();

        let feedback = Feedback::of(&analysis);

        let wrong = feedback
            .statements()
            .iter()
            .filter(|statement| matches!(statement, Statement::NotPlayedAsWritten { .. }))
            .count();
        let missing = feedback
            .statements()
            .iter()
            .filter(|statement| matches!(statement, Statement::NeverPlayed { .. }))
            .count();

        assert_eq!(wrong, summary.incorrect());
        assert_eq!(missing, summary.omitted());
    }

    // The design Business Invariant -- "The same Analysis always yields the
    // same Feedback." It derives from a fixed Analysis, so revisiting an
    // Attempt later yields the same words.
    #[test]
    fn the_same_analysis_always_yields_the_same_feedback() {
        let reference = bars(&[&[(Step::C, 0), (Step::D, 0)], &[(Step::E, 0)]]);
        let analysis = analyse(&[unexpected(), pitch(Step::C, 0)], &reference);

        assert_eq!(Feedback::of(&analysis), Feedback::of(&analysis));
        assert_eq!(
            Feedback::of(&analysis).lines(),
            Feedback::of(&analysis).lines()
        );
    }

    // The overall result reaches the performer as counts. No grade, no
    // percentage.
    #[test]
    fn feedback_reports_the_overall_result_as_counts() {
        let reference = bars(&[&[(Step::C, 0), (Step::D, 0)]]);
        let analysis = analyse(&[pitch(Step::C, 0)], &reference);

        let described = rendered(&Feedback::of(&analysis));

        assert!(!described.contains('%'));
        assert_eq!(Feedback::of(&analysis).overall(), analysis.summary());
    }
}
