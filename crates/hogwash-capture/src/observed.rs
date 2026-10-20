//! The Observed Performance.

use hogwash_domain::Pitch;

/// What the performer actually played during an Attempt.
///
/// A record, not a judgement — nothing in it says whether a note was right.
/// It holds sounding pitches because that is all an instrument reports: a
/// performer presses a key, and a key has no spelling.
///
/// **Order is the structure the comparison uses.** [`played`](Self::played)
/// is what analysis reads, and it carries no timing. Beside it, each note may
/// have a [`Touch`] (when, how hard, how long), and the sustain pedal's
/// changes are kept: facts nothing reads yet, recorded because a moment not
/// captured when it happened can never be recovered for a later question.
///
/// **It never filters.** There is no expected-note parameter anywhere on this
/// type, so a note the Score never asked for cannot be dropped — the absence
/// of the ability is a stronger guarantee than the discipline not to use it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ObservedPerformance {
    played: Vec<Pitch>,
    /// One per entry in `played`, always the same length.
    touches: Vec<Option<Touch>>,
    pedal: Vec<PedalChange>,
}

/// How one note was struck.
///
/// Times are microseconds since the Attempt began, as the adapter measured
/// them. Nothing here reads a clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Touch {
    /// When the key went down.
    pub at_us: u64,
    /// How hard, 1–127, as the instrument reported it.
    pub velocity: u8,
    /// When the key came up. `None` if it was still down when the Attempt
    /// ended, or the instrument went away first.
    pub released_us: Option<u64>,
}

/// The sustain pedal going down or coming up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PedalChange {
    /// When, in microseconds since the Attempt began.
    pub at_us: u64,
    /// Whether it went down (rather than up).
    pub down: bool,
}

impl ObservedPerformance {
    /// An Observed Performance that has recorded nothing. An Attempt begins
    /// with one of these, empty.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            played: Vec::new(),
            touches: Vec::new(),
            pedal: Vec::new(),
        }
    }

    /// Records that the performer played `pitch`, with nothing known about
    /// when or how.
    ///
    /// Takes no view on whether it was expected. Doing so would require
    /// knowing what was expected, which this component never does.
    pub fn record(&mut self, pitch: Pitch) {
        self.played.push(pitch);
        self.touches.push(None);
    }

    /// Records that the performer played `pitch`, struck as `touch` says.
    pub fn record_touched(&mut self, pitch: Pitch, touch: Touch) {
        self.played.push(pitch);
        self.touches.push(Some(touch));
    }

    /// The key for `pitch` came up at `at_us`.
    ///
    /// Marks the most recent press of that key still down. A release with no
    /// such press (the key was already down when the Attempt began) has
    /// nothing to mark and is ignored.
    pub fn release(&mut self, pitch: Pitch, at_us: u64) {
        let held = self
            .played
            .iter()
            .zip(&mut self.touches)
            .rev()
            .filter(|(played, _)| **played == pitch)
            .find_map(|(_, touch)| touch.as_mut().filter(|touch| touch.released_us.is_none()));

        if let Some(touch) = held {
            touch.released_us = Some(at_us);
        }
    }

    /// The sustain pedal is `down` (or up) as of `at_us`.
    ///
    /// Only a change is kept. A keyboard with half-pedalling reports dozens
    /// of positions a second, and the pedal begins every Attempt up.
    pub fn pedal(&mut self, at_us: u64, down: bool) {
        let was_down = self.pedal.last().is_some_and(|change| change.down);

        if down != was_down {
            self.pedal.push(PedalChange { at_us, down });
        }
    }

    /// What was played, in the order it was played.
    #[must_use]
    pub fn played(&self) -> &[Pitch] {
        &self.played
    }

    /// How each note in [`played`](Self::played) was struck, index for
    /// index; `None` where that was not known.
    #[must_use]
    pub fn touches(&self) -> &[Option<Touch>] {
        &self.touches
    }

    /// Every change of the sustain pedal, in order.
    #[must_use]
    pub fn pedal_changes(&self) -> &[PedalChange] {
        &self.pedal
    }

    /// How many notes were played.
    #[must_use]
    pub fn len(&self) -> usize {
        self.played.len()
    }

    /// Whether the performer played nothing at all.
    ///
    /// Legitimate: an Attempt may stop before anything was played, and a
    /// Session may have no Attempts.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.played.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use hogwash_domain::Pitch;

    use crate::{ObservedPerformance, PedalChange, Touch};

    fn pitch(number: u8) -> Pitch {
        Pitch::new(number).unwrap()
    }

    #[test]
    fn an_observed_performance_begins_empty() {
        assert!(ObservedPerformance::new().played().is_empty());
    }

    #[test]
    fn notes_are_recorded_in_the_order_they_were_played() {
        let mut observed = ObservedPerformance::new();

        observed.record(pitch(60));
        observed.record(pitch(64));
        observed.record(pitch(62));

        assert_eq!(observed.played(), &[pitch(60), pitch(64), pitch(62)]);
    }

    // A note the Score never contained is recorded exactly as readily as one
    // it did.
    #[test]
    fn a_note_the_score_never_asked_for_is_recorded_just_the_same() {
        let mut observed = ObservedPerformance::new();

        observed.record(Pitch::LOWEST);
        observed.record(Pitch::HIGHEST);

        assert_eq!(observed.played(), &[Pitch::LOWEST, Pitch::HIGHEST]);
    }

    // A performer who strikes the same key twice played two notes. Collapsing
    // them would be interpretation, and this is a record, not a judgement.
    #[test]
    fn a_repeated_note_is_recorded_each_time_it_is_played() {
        let mut observed = ObservedPerformance::new();

        observed.record(Pitch::MIDDLE_C);
        observed.record(Pitch::MIDDLE_C);

        assert_eq!(observed.played(), &[Pitch::MIDDLE_C, Pitch::MIDDLE_C]);
    }

    // Defined by its content: two performances of the same notes in the same
    // order are the same thing.
    #[test]
    fn performances_of_the_same_notes_in_the_same_order_are_the_same() {
        let mut one = ObservedPerformance::new();
        let mut other = ObservedPerformance::new();

        for performance in [&mut one, &mut other] {
            performance.record(pitch(60));
            performance.record(pitch(62));
        }

        assert_eq!(one, other);
    }

    fn touch(at_us: u64, velocity: u8) -> Touch {
        Touch {
            at_us,
            velocity,
            released_us: None,
        }
    }

    // The opening of the Minuet in G, with the times and velocities a
    // keyboard would report.
    #[test]
    fn each_note_keeps_how_it_was_struck_beside_it() {
        let mut observed = ObservedPerformance::new();

        observed.record_touched(pitch(74), touch(0, 80));
        observed.record(pitch(67));
        observed.record_touched(pitch(69), touch(512_000, 64));

        assert_eq!(observed.played(), &[pitch(74), pitch(67), pitch(69)]);
        assert_eq!(
            observed.touches(),
            &[Some(touch(0, 80)), None, Some(touch(512_000, 64))]
        );
    }

    // Bar 2 of the Minuet's right hand plays G4 twice: a release belongs to
    // the press still down.
    #[test]
    fn a_release_marks_the_latest_press_of_that_key_still_down() {
        let mut observed = ObservedPerformance::new();

        observed.record_touched(pitch(67), touch(0, 70));
        observed.release(pitch(67), 400_000);
        observed.record_touched(pitch(67), touch(500_000, 70));
        observed.record_touched(pitch(71), touch(600_000, 70));
        observed.release(pitch(67), 900_000);

        let released: Vec<Option<u64>> = observed
            .touches()
            .iter()
            .map(|touch| touch.and_then(|touch| touch.released_us))
            .collect();
        assert_eq!(released, [Some(400_000), Some(900_000), None]);
    }

    #[test]
    fn a_release_with_no_press_still_down_changes_nothing() {
        let mut observed = ObservedPerformance::new();
        observed.record_touched(pitch(67), touch(0, 70));
        observed.release(pitch(67), 400_000);
        let before = observed.clone();

        observed.release(pitch(67), 800_000);
        observed.release(pitch(74), 800_000);

        assert_eq!(observed, before);
    }

    #[test]
    fn only_changes_of_the_pedal_are_kept() {
        let mut observed = ObservedPerformance::new();

        observed.pedal(100, false);
        observed.pedal(200, true);
        observed.pedal(210, true);
        observed.pedal(220, true);
        observed.pedal(900, false);
        observed.pedal(950, false);

        assert_eq!(
            observed.pedal_changes(),
            &[
                PedalChange {
                    at_us: 200,
                    down: true
                },
                PedalChange {
                    at_us: 900,
                    down: false
                },
            ]
        );
    }
}
