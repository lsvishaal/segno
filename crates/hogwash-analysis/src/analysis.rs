//! Analysis — the comparison itself.

use std::ops::Range;
use std::sync::Arc;

use hogwash_capture::ObservedPerformance;
use hogwash_domain::Location;
use hogwash_notation::ReferencePerformance;
use hogwash_practice::Cursor;

use crate::finding::Finding;

/// The comparison of an Observed Performance against the Reference
/// Performance of the Piece it was played from.
///
/// Determined entirely by its two inputs. The design Domain Classification —
/// *"the same inputs are the same Analysis. Treating it as having independent
/// identity would suggest it could differ between runs, which is precisely
/// what the domain forbids."*
///
/// It does not present itself. The design assigns that to Feedback, so there
/// is deliberately no `Display` here.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Analysis {
    findings: Vec<Finding>,
    summary: Summary,
}

impl Analysis {
    /// Compares `observed` against `reference`.
    ///
    /// The comparison **replays the Cursor**. That is what makes the account
    /// agree with what the performer saw: the same rule that advanced the
    /// cursor on screen decides here which played note answered which written
    /// one (the design, one source of truth).
    ///
    /// Both inputs are taken by shared reference, so the design's *"never
    /// alters what it examines"* is carried by the signature.
    #[must_use]
    pub fn of(observed: &ObservedPerformance, reference: &ReferencePerformance) -> Self {
        Self::starting_at(observed, reference, 0)
    }

    /// Compares `observed`, an attempt that began at step `from`, against
    /// `reference`. The Cursor is replayed from `from`, as it ran on screen;
    /// what lies before `from` was never asked for, so it is not missed.
    #[must_use]
    pub fn starting_at(
        observed: &ObservedPerformance,
        reference: &ReferencePerformance,
        from: usize,
    ) -> Self {
        Self::over(observed, reference, from..reference.len())
    }

    /// Compares `observed`, one pass of a loop over the steps in `steps`,
    /// against `reference`. As [`starting_at`](Self::starting_at), except
    /// that nothing after the loop's end was asked for either, and reaching
    /// that end is reaching the end.
    #[must_use]
    pub fn over(
        observed: &ObservedPerformance,
        reference: &ReferencePerformance,
        steps: Range<usize>,
    ) -> Self {
        let Range { start: from, end } = steps;
        // A Cursor holds a share of what it indexes into, so one is made here.
        // The copy is deliberate: the design says an Analysis is given an
        // Observed Performance and a Reference Performance, and taking an
        // `Arc` instead would push the Cursor's storage choice out into every
        // caller's signature. It happens once per Attempt, not once per note.
        let mut cursor = Cursor::starting_at(Arc::new(reference.clone()), from);
        let mut findings = Vec::new();

        for &played in observed.played() {
            // Copied out rather than borrowed: `expecting` borrows the whole
            // Cursor, and `observe` below needs it mutably.
            let Some(expected) = cursor.expecting().cloned() else {
                // Nothing corresponds to it, so it is unexpected rather than
                // incorrect, and it has no location -- unless it is the tail
                // of a trill on the final note.
                if !cursor.tolerates(played) {
                    findings.push(Finding::Unexpected { played });
                }
                continue;
            };

            let at = Location::new(expected.measure(), cursor.position());

            if expected.sounding().any(|wanted| wanted == played) {
                findings.push(Finding::Correct { at, pitch: played });
            } else if !cursor.tolerates(played) {
                // A tolerated pitch -- a grace note, or part of an ornament --
                // is neither required nor wrong: no finding, no count moves.
                findings.push(Finding::IncorrectPitch {
                    at,
                    played,
                    expected: expected.notes().to_vec(),
                });
            }

            cursor.observe(played);
        }

        // Whatever the performer never reached. Under wait-for-me
        // this is the tail and nothing else, because a note cannot be skipped
        // mid-piece — the Cursor will not pass it.
        let stopped_at = cursor.position();
        for position in stopped_at..end.min(reference.len()) {
            // Where it stopped, what the Cursor was asking for: on the step
            // an attempt began at, that includes any note held into it.
            let expected = if position == stopped_at {
                cursor.expecting()
            } else {
                reference.at(position)
            };
            let Some(expected) = expected else {
                continue;
            };

            for note in expected.notes() {
                // At the position the performer stopped on, some notes of a
                // chord may already be down. Those were played, and reporting
                // them as never arriving would contradict the record.
                if position == stopped_at && cursor.has_been_played(note.sounding()) {
                    continue;
                }

                findings.push(Finding::Omitted {
                    at: Location::new(expected.measure(), position),
                    written: *note,
                });
            }
        }

        let summary = Summary::over(&findings, stopped_at >= end);

        Self { findings, summary }
    }

    /// Everything found, in the order it happened.
    ///
    /// Played notes appear in the order they were played; omissions follow,
    /// in the order the Score writes them. The ordering is fixed by the
    /// inputs alone, which is what the requirement requires.
    #[must_use]
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// The single overall result for the Attempt.
    #[must_use]
    pub const fn summary(&self) -> Summary {
        self.summary
    }
}

/// One overall result for an Attempt.
///
/// **Counts, and nothing else.** No percentage, no grade, no pass or fail. A
/// figure like "82%" is an interpretation dressed as a measurement, and it
/// invites exactly the grading this project rules out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Summary {
    pub(crate) correct: usize,
    pub(crate) incorrect: usize,
    pub(crate) omitted: usize,
    pub(crate) unexpected: usize,
    pub(crate) reached_the_end: bool,
}

impl Summary {
    fn over(findings: &[Finding], reached_the_end: bool) -> Self {
        let mut summary = Self {
            correct: 0,
            incorrect: 0,
            omitted: 0,
            unexpected: 0,
            reached_the_end,
        };

        for finding in findings {
            match finding {
                Finding::Correct { .. } => summary.correct += 1,
                Finding::IncorrectPitch { .. } => summary.incorrect += 1,
                Finding::Omitted { .. } => summary.omitted += 1,
                Finding::Unexpected { .. } => summary.unexpected += 1,
            }
        }

        summary
    }

    /// How many played notes the Score asked for where they were played.
    #[must_use]
    pub const fn correct(&self) -> usize {
        self.correct
    }

    /// How many played notes differed from what was written there.
    #[must_use]
    pub const fn incorrect(&self) -> usize {
        self.incorrect
    }

    /// How many written notes never arrived.
    #[must_use]
    pub const fn omitted(&self) -> usize {
        self.omitted
    }

    /// How many notes were played when nothing was expected.
    #[must_use]
    pub const fn unexpected(&self) -> usize {
        self.unexpected
    }

    /// Whether the performer reached the end of the Piece.
    ///
    /// Not a judgement: the design is explicit that a short or abandoned
    /// Attempt is still an Attempt.
    #[must_use]
    pub const fn reached_the_end(&self) -> bool {
        self.reached_the_end
    }
}

#[cfg(test)]
mod tests {
    use hogwash_domain::{Location, MeasureNumber, Step};

    use crate::analysis::{Analysis, Summary};
    use crate::finding::Finding;
    use crate::fixtures::{
        note, one_bar, one_bar_tolerating, one_chord, ornament_on, pitch, played, played_steps,
        two_bars, unexpected,
    };

    fn bar(number: u32) -> MeasureNumber {
        MeasureNumber::new(number).expect("fixture measures are numbered from one")
    }

    // - "compare the observed performance of an attempt against the
    // reference performance of the associated piece".
    #[test]
    fn a_faultless_performance_is_all_correct() {
        let reference = one_bar(&[Step::C, Step::D, Step::E]);
        let observed = played_steps(&[Step::C, Step::D, Step::E]);

        let analysis = Analysis::of(&observed, &reference);

        assert!(analysis.findings().iter().all(Finding::is_correct));
        assert_eq!(analysis.summary().correct(), 3);
    }

    // - "identify notes in the observed performance whose pitch
    // differs from the corresponding note in the reference performance".
    //
    // The corresponding note is the one expected where the Cursor was
    // waiting, so the finding can say both what was played and what was
    // wanted, and where.
    #[test]
    fn a_note_played_where_another_was_written_is_an_incorrect_pitch() {
        let reference = one_bar(&[Step::C, Step::D]);
        let observed = played_steps(&[Step::G, Step::C, Step::D]);

        let analysis = Analysis::of(&observed, &reference);

        assert_eq!(
            analysis.findings()[0],
            Finding::IncorrectPitch {
                at: Location::new(bar(1), 0),
                played: pitch(Step::G),
                expected: vec![note(Step::C)],
            }
        );
    }

    // - the Cursor holds on a wrong note, so the position is not
    // consumed. The performer plays the right note next and it is correct.
    // One position, two findings, which is a faithful account.
    #[test]
    fn a_wrong_note_does_not_consume_the_position_it_was_played_at() {
        let reference = one_bar(&[Step::C]);
        let observed = played_steps(&[Step::G, Step::C]);

        let analysis = Analysis::of(&observed, &reference);

        assert_eq!(analysis.summary().incorrect(), 1);
        assert_eq!(analysis.summary().correct(), 1);
        assert_eq!(analysis.summary().omitted(), 0);
    }

    // - "identify notes present in the reference performance but
    // absent from the observed performance".
    //
    // Under wait-for-me these are exactly the tail the performer never
    // reached: a note cannot be skipped mid-piece, because the Cursor will
    // not pass it.
    #[test]
    fn notes_after_the_point_the_performer_stopped_are_omitted() {
        let reference = two_bars(&[Step::C, Step::D], &[Step::E]);
        let observed = played_steps(&[Step::C]);

        let analysis = Analysis::of(&observed, &reference);

        assert_eq!(
            analysis.findings()[1..],
            [
                Finding::Omitted {
                    at: Location::new(bar(1), 1),
                    written: note(Step::D),
                },
                Finding::Omitted {
                    at: Location::new(bar(2), 2),
                    written: note(Step::E),
                },
            ]
        );
    }

    // A chord the performer got half way through: the notes played are
    // correct, and only the ones that never arrived are omitted. Reporting
    // all three as omitted would contradict the record of what was played.
    #[test]
    fn only_the_unplayed_notes_of_a_part_played_chord_are_omitted() {
        let reference = one_chord(&[Step::C, Step::E, Step::G]);
        let observed = played_steps(&[Step::C, Step::E]);

        let analysis = Analysis::of(&observed, &reference);

        assert_eq!(analysis.summary().correct(), 2);
        assert_eq!(analysis.summary().omitted(), 1);
        assert_eq!(
            analysis.findings()[2],
            Finding::Omitted {
                at: Location::new(bar(1), 0),
                written: note(Step::G),
            }
        );
    }

    // - "identify notes present in the observed performance but
    // absent from the reference performance".
    //
    // Nothing corresponds to a note played when nothing is expected, which
    // is what distinguishes this from the requirement. It has no location for the
    // same reason.
    #[test]
    fn a_note_played_after_the_piece_has_ended_is_unexpected() {
        let reference = one_bar(&[Step::C]);
        let observed = played(&[pitch(Step::C), unexpected()]);

        let analysis = Analysis::of(&observed, &reference);

        assert_eq!(
            analysis.findings()[1],
            Finding::Unexpected {
                played: unexpected(),
            }
        );
        assert_eq!(analysis.findings()[1].measure(), None);
    }

    // The design Business Invariant -- "Identical inputs always produce an
    // identical Analysis." the requirement; and the design Notes: "A comparison that
    // could differ between runs would not be a comparison of these two
    // things."
    #[test]
    fn identical_inputs_always_produce_an_identical_analysis() {
        let reference = two_bars(&[Step::C, Step::D], &[Step::E, Step::F]);
        let observed = played(&[pitch(Step::C), unexpected(), pitch(Step::D), pitch(Step::E)]);

        assert_eq!(
            Analysis::of(&observed, &reference),
            Analysis::of(&observed, &reference)
        );
    }

    // - "It never alters what it examines." The inputs are taken
    // by shared reference, so mutation is not expressible; this records that
    // they survive unchanged.
    #[test]
    fn analysis_never_alters_what_it_examines() {
        let reference = one_bar(&[Step::C]);
        let observed = played_steps(&[Step::G]);

        let before = (observed.clone(), reference.clone());
        let _ = Analysis::of(&observed, &reference);

        assert_eq!((observed, reference), before);
    }

    // - "produce a single overall analysis summary for each attempt".
    // Exactly one, and it is counts.
    #[test]
    fn an_attempt_has_exactly_one_overall_result() {
        let reference = one_bar(&[Step::C, Step::D]);
        let observed = played(&[unexpected(), pitch(Step::C)]);

        let summary = Analysis::of(&observed, &reference).summary();

        assert_eq!(summary.correct(), 1);
        assert_eq!(summary.incorrect(), 1);
        assert_eq!(summary.omitted(), 1);
        assert_eq!(summary.unexpected(), 0);
        assert!(!summary.reached_the_end());
    }

    #[test]
    fn a_summary_records_whether_the_performer_reached_the_end() {
        let reference = one_bar(&[Step::C]);

        assert!(
            Analysis::of(&played_steps(&[Step::C]), &reference)
                .summary()
                .reached_the_end()
        );
    }

    // The summary is the four counts and whether the end was reached -- no
    // grade, no percentage, no pass or fail.
    //
    // Asserted by round-trip: a Summary is fully reconstructible from those
    // five values. Add a score field and the round trip cannot carry it.
    #[test]
    fn the_summary_is_five_measurements_and_no_verdict() {
        let reference = one_bar(&[Step::C, Step::D]);
        let observed = played(&[unexpected(), pitch(Step::C)]);
        let summary = Analysis::of(&observed, &reference).summary();

        let rebuilt = Summary {
            correct: summary.correct(),
            incorrect: summary.incorrect(),
            omitted: summary.omitted(),
            unexpected: summary.unexpected(),
            reached_the_end: summary.reached_the_end(),
        };

        assert_eq!(rebuilt, summary);
    }

    // The requirement (timing) and the requirement (duration) are deferred, for the
    // reason the design derives: with practice waiting for the performer, the
    // tempo is the performer's speed and there is no reference to measure
    // against.
    //
    // Guarded structurally: an Analysis is fully determined by the two
    // sequences it compares, neither of which carries time. A
    // deviation figure would have to come from somewhere, and there is
    // nowhere for it to come from.
    #[test]
    fn no_finding_reports_timing_or_duration() {
        let reference = one_bar(&[Step::C, Step::D]);

        // The same notes are the same analysis however they were "played" --
        // there is no third input that could distinguish two renditions.
        assert_eq!(
            Analysis::of(&played_steps(&[Step::C, Step::D]), &reference),
            Analysis::of(&played_steps(&[Step::C, Step::D]), &reference)
        );
    }

    // The requirement (aggregate per measure) is deferred. Findings carry a
    // MeasureNumber because Feedback locates findings by Measure,
    // and location is not aggregation: there is no per-measure result, only
    // findings that know where they are.
    #[test]
    fn findings_are_located_by_measure_but_not_aggregated_by_it() {
        let reference = two_bars(&[Step::C], &[Step::D]);
        let observed = played_steps(&[Step::C, Step::D]);

        let analysis = Analysis::of(&observed, &reference);

        assert_eq!(analysis.findings()[0].measure(), Some(bar(1)));
        assert_eq!(analysis.findings()[1].measure(), Some(bar(2)));

        // The whole result is one flat ordered sequence plus one overall
        // summary -- the requirement's "single overall result", and nothing between.
        assert_eq!(analysis.findings().len(), 2);
    }

    // The design allows an Attempt in which nothing was played. Every expected
    // note is then omitted, and nothing else is reported.
    #[test]
    fn an_attempt_in_which_nothing_was_played_omits_everything() {
        let reference = one_bar(&[Step::C, Step::D]);

        let analysis = Analysis::of(&played(&[]), &reference);

        assert_eq!(analysis.summary().omitted(), 2);
        assert_eq!(analysis.summary().correct(), 0);
        assert_eq!(analysis.summary().incorrect(), 0);
    }

    // - comparison is on sounding pitch. A performer plays a key,
    // and F sharp and G flat are one key; the analysis must agree with what
    // the Cursor did on screen.
    #[test]
    fn correctness_is_decided_on_the_sound_not_the_spelling() {
        let reference = one_bar(&[Step::F]);
        let observed = played(&[pitch(Step::F)]);

        assert_eq!(Analysis::of(&observed, &reference).summary().correct(), 1);
    }

    // A trill on E, resolving down to D, played as upper-note trill: F E F E
    // F E, then D. The extra notes are tolerated at the trilled note and the
    // one after it, so nothing is wrong and nothing is unexpected.
    #[test]
    fn a_trill_played_correctly_has_no_wrong_notes() {
        let reference = one_bar_tolerating(&[Step::C, Step::E, Step::D], 1, &ornament_on(Step::E));
        let trill = [
            Step::C,
            Step::F,
            Step::E,
            Step::F,
            Step::E,
            Step::F,
            Step::E,
            Step::D,
        ];

        let summary = Analysis::of(&played_steps(&trill), &reference).summary();

        assert_eq!(summary.incorrect(), 0);
        assert_eq!(summary.unexpected(), 0);
        assert_eq!(summary.correct(), 3);
        assert!(summary.reached_the_end());
    }

    // An A is nowhere near the trilled E. Played during the trill, it is
    // still a wrong note.
    #[test]
    fn a_clearly_wrong_note_during_a_trill_is_still_wrong() {
        let reference = one_bar_tolerating(&[Step::C, Step::E, Step::D], 1, &ornament_on(Step::E));

        let summary = Analysis::of(
            &played_steps(&[Step::C, Step::E, Step::F, Step::A, Step::E, Step::D]),
            &reference,
        )
        .summary();

        assert_eq!(summary.incorrect(), 1);
    }

    // The tolerance belongs to the ornamented note. The same neighbour played
    // where nothing is ornamented is an ordinary wrong note.
    #[test]
    fn a_neighbour_note_where_nothing_is_ornamented_is_still_wrong() {
        let reference = one_bar_tolerating(&[Step::C, Step::E, Step::D], 1, &ornament_on(Step::E));

        let analysis = Analysis::of(&played_steps(&[Step::D, Step::C]), &reference);

        assert_eq!(analysis.summary().incorrect(), 1);
        assert!(matches!(
            analysis.findings()[0],
            Finding::IncorrectPitch { .. }
        ));
    }

    // A grace note is optional. Played, it is not a mistake of any kind --
    // it must never reach the counts later per-bar statistics are built on.
    #[test]
    fn a_played_grace_note_is_neither_wrong_nor_unexpected() {
        let reference = one_bar_tolerating(&[Step::C, Step::D], 1, &[pitch(Step::G)]);

        let analysis = Analysis::of(&played_steps(&[Step::C, Step::G, Step::D]), &reference);

        assert_eq!(analysis.summary().incorrect(), 0);
        assert_eq!(analysis.summary().unexpected(), 0);
        assert_eq!(analysis.summary().correct(), 2);
        assert_eq!(analysis.findings().len(), 2, "the grace note is no finding");
    }

    // Skipped, a grace note is not waited for and not reported missing.
    #[test]
    fn a_skipped_grace_note_is_not_waited_for() {
        let reference = one_bar_tolerating(&[Step::C, Step::D], 1, &[pitch(Step::G)]);

        let summary = Analysis::of(&played_steps(&[Step::C, Step::D]), &reference).summary();

        assert!(summary.reached_the_end());
        assert_eq!(summary.omitted(), 0);
    }

    // A trill on the final note keeps going once the cursor has reached the
    // end. Its tail is not "unexpected".
    #[test]
    fn a_trill_on_the_last_note_is_not_unexpected_after_the_end() {
        let reference = one_bar_tolerating(&[Step::C, Step::E], 1, &ornament_on(Step::E));

        let summary = Analysis::of(
            &played_steps(&[Step::C, Step::E, Step::F, Step::E, Step::F]),
            &reference,
        )
        .summary();

        assert_eq!(summary.unexpected(), 0);
        assert_eq!(summary.incorrect(), 0);
    }

    // Started at bar 2 and stopped after one note: bar 1 was never asked
    // for, so nothing in it is missed.
    #[test]
    fn an_attempt_started_at_a_later_bar_misses_nothing_before_it() {
        let reference = crate::fixtures::two_bars(&[Step::D, Step::G], &[Step::A, Step::B]);

        let analysis = Analysis::starting_at(&played_steps(&[Step::A]), &reference, 2);

        let bars: Vec<u32> = analysis
            .findings()
            .iter()
            .filter_map(Finding::measure)
            .map(MeasureNumber::get)
            .collect();
        assert_eq!(bars, [2, 2]);
        assert_eq!(analysis.summary().correct(), 1);
        assert_eq!(analysis.summary().omitted(), 1);
    }

    // The note tied into bar 2 was required, starting there; stopping
    // before it was played reports it missed with the bar's first note.
    #[test]
    fn a_held_note_required_at_the_start_is_missed_if_never_played() {
        let reference =
            crate::fixtures::tied_into_second(&[Step::D, Step::C], Step::C, &[Step::A, Step::B]);

        let analysis = Analysis::starting_at(&played_steps(&[Step::A]), &reference, 2);

        // A correct, then C (held) and B missed.
        assert_eq!(analysis.summary().correct(), 1);
        assert_eq!(analysis.summary().omitted(), 2);
    }

    #[test]
    fn starting_at_the_beginning_is_the_same_analysis_as_before() {
        let reference = crate::fixtures::two_bars(&[Step::D, Step::G], &[Step::A, Step::B]);
        let observed = played_steps(&[Step::D, Step::E, Step::G]);

        assert_eq!(
            Analysis::starting_at(&observed, &reference, 0),
            Analysis::of(&observed, &reference)
        );
    }

    // A pass of a loop over bar 1: bar 2 lies past the loop, so a clean
    // pass misses nothing and has reached its end.
    #[test]
    fn a_pass_of_a_loop_misses_nothing_after_the_loop() {
        let reference = crate::fixtures::two_bars(&[Step::D, Step::G], &[Step::A, Step::B]);

        let clean = Analysis::over(&played_steps(&[Step::D, Step::G]), &reference, 0..2).summary();
        assert_eq!((clean.correct(), clean.omitted()), (2, 0));
        assert!(clean.reached_the_end());

        let stopped = Analysis::over(&played_steps(&[Step::D]), &reference, 0..2).summary();
        assert_eq!(stopped.omitted(), 1);
        assert!(!stopped.reached_the_end());
    }
}
