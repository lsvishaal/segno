//! The Observed Performance (`003 §7.5`).

use hogwash_domain::Pitch;

/// What the performer actually played during an Attempt (`003 §7.5`).
///
/// A record, not a judgement — nothing in it says whether a note was right.
/// It holds sounding pitches because that is all an instrument reports: a
/// performer presses a key, and a key has no spelling (ADR-0001).
///
/// **It carries no timing.** `003 §15` does not model when anything happened,
/// so order is the only structure here. That is enough: `003 §7.4` scopes the
/// Reference Performance it will be compared against to an ordered sequence
/// with no timing significance either.
///
/// **It never filters.** There is no expected-note parameter anywhere on this
/// type, so a note the Score never asked for cannot be dropped — the absence
/// of the ability is a stronger guarantee than the discipline not to use it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ObservedPerformance {
    played: Vec<Pitch>,
}

impl ObservedPerformance {
    /// An Observed Performance that has recorded nothing.
    ///
    /// `003 §7.5` — an Attempt begins with one of these, empty.
    #[must_use]
    pub const fn new() -> Self {
        Self { played: Vec::new() }
    }

    /// Records that the performer played `pitch`.
    ///
    /// Takes no view on whether it was expected. Doing so would require
    /// knowing what was expected, which `004 §8.5` forbids this component.
    pub fn record(&mut self, pitch: Pitch) {
        self.played.push(pitch);
    }

    /// What was played, in the order it was played.
    #[must_use]
    pub fn played(&self) -> &[Pitch] {
        &self.played
    }

    /// How many notes were played.
    #[must_use]
    pub fn len(&self) -> usize {
        self.played.len()
    }

    /// Whether the performer played nothing at all.
    ///
    /// Legitimate: `003 §7.3` allows an Attempt that stops before anything
    /// was played, and `003 §7.2` allows a Session with no Attempts.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.played.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use hogwash_domain::Pitch;

    use crate::ObservedPerformance;

    fn pitch(number: u8) -> Pitch {
        Pitch::new(number).unwrap()
    }

    // 003 §7.5 Lifecycle — "Begins empty when an Attempt begins".
    #[test]
    fn an_observed_performance_begins_empty() {
        assert!(ObservedPerformance::new().played().is_empty());
    }

    // 003 §7.5 — "It accumulates as the performer plays." Order is the only
    // structure it has; 003 §15 does not model when anything happened.
    #[test]
    fn notes_are_recorded_in_the_order_they_were_played() {
        let mut observed = ObservedPerformance::new();

        observed.record(pitch(60));
        observed.record(pitch(64));
        observed.record(pitch(62));

        assert_eq!(observed.played(), &[pitch(60), pitch(64), pitch(62)]);
    }

    // 003 §7.5 Non-Responsibilities -- "does not correct or filter what was
    // played. A note the Score never contained is recorded exactly as readily
    // as one it did." There is no expected-note parameter anywhere here to
    // filter against, which is the strongest form of that guarantee.
    #[test]
    fn a_note_the_score_never_asked_for_is_recorded_just_the_same() {
        let mut observed = ObservedPerformance::new();

        observed.record(Pitch::LOWEST);
        observed.record(Pitch::HIGHEST);

        assert_eq!(observed.played(), &[Pitch::LOWEST, Pitch::HIGHEST]);
    }

    // A performer who strikes the same key twice played two notes. Collapsing
    // them would be interpretation, and 003 §7.5 says this is "a record, not
    // a judgement".
    #[test]
    fn a_repeated_note_is_recorded_each_time_it_is_played() {
        let mut observed = ObservedPerformance::new();

        observed.record(Pitch::MIDDLE_C);
        observed.record(Pitch::MIDDLE_C);

        assert_eq!(observed.played(), &[Pitch::MIDDLE_C, Pitch::MIDDLE_C]);
    }

    // 003 §7.5 Domain Classification -- "Defined by its content". Two
    // performances of the same notes in the same order are the same thing.
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
}
