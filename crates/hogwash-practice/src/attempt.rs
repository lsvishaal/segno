//! An Attempt — one run at a Piece (`003 §7.3`).

use std::sync::Arc;

use hogwash_capture::ObservedPerformance;
use hogwash_domain::Pitch;
use hogwash_notation::ReferencePerformance;

use crate::cursor::{Cursor, CursorMovement};

/// One run at a Piece, in progress (`003 §7.3`).
///
/// The unit of practice: it is what gets evaluated, what produces Feedback,
/// and what accumulates into Practice History.
///
/// An Attempt does not evaluate itself — `003 §7.3` assigns that to Analysis,
/// which runs after the Attempt has ended, over the whole performance. What
/// happens here is only the two things `004 §8.4` and `§8.5` describe: the
/// note is recorded, and the Cursor is told about it.
#[derive(Debug, Clone)]
pub struct Attempt {
    cursor: Cursor,
    observed: ObservedPerformance,
}

impl Attempt {
    /// Begins an Attempt at the Piece `reference` describes.
    ///
    /// `003 §7.3` Lifecycle — the Observed Performance begins empty and the
    /// Cursor rests on the first expected position.
    #[must_use]
    pub fn begin(reference: Arc<ReferencePerformance>) -> Self {
        Self {
            cursor: Cursor::at_the_beginning(reference),
            observed: ObservedPerformance::new(),
        }
    }

    /// The performer played `pitch`.
    ///
    /// Two independent things happen, in this order, and the order matters:
    /// the note is **recorded unconditionally**, and only then is the Cursor
    /// given a chance to move. `003 §7.5` requires a note the Score never
    /// asked for to be recorded "exactly as readily as one it did", so a note
    /// the Cursor ignores (ADR-0002) is still in the Observed Performance and
    /// still reaches the attempt summary.
    pub fn play(&mut self, pitch: Pitch) -> CursorMovement {
        self.observed.record(pitch);
        self.cursor.observe(pitch)
    }

    /// Where the performer is expected to be.
    #[must_use]
    pub const fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// What has been played so far.
    #[must_use]
    pub const fn observed(&self) -> &ObservedPerformance {
        &self.observed
    }

    /// Ends the Attempt, whether or not the end of the Piece was reached.
    ///
    /// `003 §7.3` — *"Once ended, an Attempt does not change."* Carried by
    /// the type rather than by discipline: this consumes the Attempt, and
    /// [`EndedAttempt`] exposes no way to alter anything.
    #[must_use]
    pub fn end(self) -> EndedAttempt {
        EndedAttempt {
            reached: self.cursor.position(),
            completed: self.cursor.has_reached_the_end(),
            observed: self.observed,
        }
    }
}

/// An Attempt that has ended (`003 §7.3`).
///
/// Fixed. `003 §9.1` — *"Ended things do not change."*
///
/// It deliberately does **not** hold the Reference Performance. `003 §7.3`
/// says an Attempt has an Observed Performance; the expected notes belong to
/// the Piece, and Analysis is given both separately (`004 §8.6`). Keeping the
/// borrow out of this type is also what lets a Session record an ended
/// Attempt while still lending its Reference Performance to the next one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EndedAttempt {
    observed: ObservedPerformance,
    reached: usize,
    completed: bool,
}

impl EndedAttempt {
    /// What was played during the Attempt.
    #[must_use]
    pub const fn observed(&self) -> &ObservedPerformance {
        &self.observed
    }

    /// How far through the Piece the Cursor got.
    #[must_use]
    pub const fn reached(&self) -> usize {
        self.reached
    }

    /// Whether the Attempt reached the end of the Piece.
    ///
    /// Not a judgement of quality — `003 §7.3` is explicit that a very short
    /// or abandoned Attempt is still an Attempt, and the domain does not
    /// distinguish a "real" one from a false start.
    #[must_use]
    pub const fn was_completed(&self) -> bool {
        self.completed
    }
}

#[cfg(test)]
mod tests {
    use hogwash_domain::Step;

    use crate::attempt::Attempt;
    use crate::cursor::CursorMovement;
    use crate::fixtures::{one_bar, pitch, unexpected};

    // 003 §7.3 Lifecycle -- an Attempt begins, and its Observed Performance
    // begins empty with it (003 §7.5).
    #[test]
    fn an_attempt_begins_with_nothing_played_and_the_cursor_at_the_start() {
        let reference = one_bar(&[Step::C, Step::D]);
        let attempt = Attempt::begin(reference);

        assert!(attempt.observed().is_empty());
        assert_eq!(attempt.cursor().position(), 0);
    }

    // 003 §7.5 -- the Observed Performance "records notes as played,
    // including notes the Score never asked for". The Cursor ignoring a note
    // (ADR-0002) must not stop it being recorded; those are two different
    // components doing two different jobs (004 §8.4, §8.5).
    #[test]
    fn every_played_note_is_recorded_including_unexpected_ones() {
        let reference = one_bar(&[Step::C]);
        let mut attempt = Attempt::begin(reference);

        attempt.play(unexpected());
        attempt.play(pitch(Step::C));

        assert_eq!(attempt.observed().played(), &[unexpected(), pitch(Step::C)]);
    }

    #[test]
    fn playing_the_expected_note_advances_the_attempts_cursor() {
        let reference = one_bar(&[Step::C, Step::D]);
        let mut attempt = Attempt::begin(reference);

        assert_eq!(attempt.play(pitch(Step::C)), CursorMovement::Advanced);

        assert_eq!(attempt.cursor().position(), 1);
    }

    // 003 §7.3 -- "It may reach the end of the Piece or stop before it." A
    // very short or abandoned Attempt is still an Attempt; the domain does
    // not distinguish a "real" attempt from a false start.
    #[test]
    fn an_attempt_may_end_before_the_end_of_the_piece() {
        let reference = one_bar(&[Step::C, Step::D, Step::E]);
        let mut attempt = Attempt::begin(reference);

        attempt.play(pitch(Step::C));
        let ended = attempt.end();

        assert_eq!(ended.reached(), 1);
        assert!(!ended.was_completed());
    }

    #[test]
    fn an_attempt_that_reaches_the_end_says_so() {
        let reference = one_bar(&[Step::C]);
        let mut attempt = Attempt::begin(reference);

        attempt.play(pitch(Step::C));

        assert!(attempt.end().was_completed());
    }

    // 003 §7.2 Notes -- a performer may begin and be interrupted before
    // playing anything. That is a legitimate Attempt, not an error.
    #[test]
    fn an_attempt_in_which_nothing_was_played_is_still_an_attempt() {
        let reference = one_bar(&[Step::C]);

        let ended = Attempt::begin(reference).end();

        assert!(ended.observed().is_empty());
        assert_eq!(ended.reached(), 0);
    }

    // 003 §7.3 -- "Once ended, an Attempt does not change." Carried by the
    // type: `end` consumes the Attempt and returns a value with no mutating
    // method at all, so this is a compile-time fact. The test records what
    // survives the transition.
    #[test]
    fn an_ended_attempt_keeps_what_was_played() {
        let reference = one_bar(&[Step::C, Step::D]);
        let mut attempt = Attempt::begin(reference);

        attempt.play(pitch(Step::C));
        attempt.play(unexpected());

        let ended = attempt.end();

        assert_eq!(ended.observed().played(), &[pitch(Step::C), unexpected()]);
    }
}
