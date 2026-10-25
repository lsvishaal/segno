//! An Attempt — one run at a Piece.

use std::sync::Arc;

use hogwash_capture::{ObservedPerformance, Touch};
use hogwash_domain::Pitch;
use hogwash_notation::ReferencePerformance;

use crate::cursor::{Cursor, CursorMovement};

/// One run at a Piece, in progress.
///
/// The unit of practice: it is what gets evaluated, what produces Feedback,
/// and what accumulates into Practice History.
///
/// An Attempt does not evaluate itself — the design assigns that to Analysis,
/// which runs after the Attempt has ended, over the whole performance. What
/// happens here is only the two things the design describes: the
/// note is recorded, and the Cursor is told about it.
#[derive(Debug, Clone)]
pub struct Attempt {
    cursor: Cursor,
    observed: ObservedPerformance,
    from: usize,
}

impl Attempt {
    /// Begins an Attempt at the Piece `reference` describes.
    ///
    /// The design Lifecycle — the Observed Performance begins empty and the
    /// Cursor rests on the first expected position.
    #[must_use]
    pub fn begin(reference: Arc<ReferencePerformance>) -> Self {
        Self::begin_at(reference, 0)
    }

    /// Begins an Attempt at step `from` of the Piece: the Cursor rests
    /// there, waiting also for any note tied into that step's measure.
    #[must_use]
    pub fn begin_at(reference: Arc<ReferencePerformance>, from: usize) -> Self {
        Self {
            cursor: Cursor::starting_at(reference, from),
            observed: ObservedPerformance::new(),
            from,
        }
    }

    /// The step this Attempt began at.
    #[must_use]
    pub const fn from(&self) -> usize {
        self.from
    }

    /// The performer played `pitch`.
    ///
    /// Two independent things happen, in this order, and the order matters:
    /// the note is **recorded unconditionally**, and only then is the Cursor
    /// given a chance to move. The design requires a note the Score never
    /// asked for to be recorded "exactly as readily as one it did", so a note
    /// the Cursor ignores is still in the Observed Performance and
    /// still reaches the attempt summary.
    pub fn play(&mut self, pitch: Pitch) -> CursorMovement {
        self.observed.record(pitch);
        self.cursor.observe(pitch)
    }

    /// The performer played `pitch`, struck as `touch` says. Exactly
    /// [`play`](Self::play), with how the key was struck recorded beside it.
    pub fn play_touched(&mut self, pitch: Pitch, touch: Touch) -> CursorMovement {
        self.observed.record_touched(pitch, touch);
        self.cursor.observe(pitch)
    }

    /// The key for `pitch` came up. Recorded; the Cursor never hears of it.
    pub fn release(&mut self, pitch: Pitch, at_us: u64) {
        self.observed.release(pitch, at_us);
    }

    /// The sustain pedal is `down` (or up). Recorded; the Cursor never hears
    /// of it.
    pub fn pedal(&mut self, at_us: u64, down: bool) {
        self.observed.pedal(at_us, down);
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
    /// *"Once ended, an Attempt does not change."* Carried by
    /// the type rather than by discipline: this consumes the Attempt, and
    /// [`EndedAttempt`] exposes no way to alter anything.
    #[must_use]
    pub fn end(self) -> EndedAttempt {
        EndedAttempt {
            from: self.from,
            reached: self.cursor.position(),
            completed: self.cursor.has_reached_the_end(),
            observed: self.observed,
        }
    }
}

/// An Attempt that has ended.
///
/// Fixed. The design — *"Ended things do not change."*
///
/// It deliberately does **not** hold the Reference Performance. The design
/// says an Attempt has an Observed Performance; the expected notes belong to
/// the Piece, and Analysis is given both separately. Keeping the
/// borrow out of this type is also what lets a Session record an ended
/// Attempt while still lending its Reference Performance to the next one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EndedAttempt {
    observed: ObservedPerformance,
    from: usize,
    reached: usize,
    completed: bool,
}

impl EndedAttempt {
    /// What was played during the Attempt.
    #[must_use]
    pub const fn observed(&self) -> &ObservedPerformance {
        &self.observed
    }

    /// The step the Attempt began at.
    #[must_use]
    pub const fn from(&self) -> usize {
        self.from
    }

    /// How far through the Piece the Cursor got.
    #[must_use]
    pub const fn reached(&self) -> usize {
        self.reached
    }

    /// Whether the Attempt reached the end of the Piece.
    ///
    /// Not a judgement of quality — the design is explicit that a very short
    /// or abandoned Attempt is still an Attempt, and the domain does not
    /// distinguish a "real" one from a false start.
    #[must_use]
    pub const fn was_completed(&self) -> bool {
        self.completed
    }
}

#[cfg(test)]
mod tests {
    use hogwash_capture::Touch;
    use hogwash_domain::Step;

    use crate::attempt::Attempt;
    use crate::cursor::CursorMovement;
    use crate::fixtures::{one_bar, pitch, unexpected};

    // The design Lifecycle -- an Attempt begins, and its Observed Performance
    // begins empty with it.
    #[test]
    fn an_attempt_begins_with_nothing_played_and_the_cursor_at_the_start() {
        let reference = one_bar(&[Step::C, Step::D]);
        let attempt = Attempt::begin(reference);

        assert!(attempt.observed().is_empty());
        assert_eq!(attempt.cursor().position(), 0);
    }

    // - the Observed Performance "records notes as played,
    // including notes the Score never asked for". The Cursor ignoring a note
    // must not stop it being recorded; those are two different
    // components doing two different jobs.
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

    // - "It may reach the end of the Piece or stop before it." A
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

    // The design Notes -- a performer may begin and be interrupted before
    // playing anything. That is a legitimate Attempt, not an error.
    #[test]
    fn an_attempt_in_which_nothing_was_played_is_still_an_attempt() {
        let reference = one_bar(&[Step::C]);

        let ended = Attempt::begin(reference).end();

        assert!(ended.observed().is_empty());
        assert_eq!(ended.reached(), 0);
    }

    // The exact computation a live interface performs for on-screen
    // correctness (`app/src-tauri/src/practice.rs`'s `drain`, the design):
    // since `CursorMovement` gives no verdict, "was that note
    // right" is answered by checking membership in `cursor().expecting()`
    // *before* calling `play`, which may advance the Cursor or clear its
    // progress. Pinned here because nothing exercised this boolean before —
    // `play`'s return value was tested, but not the read that has to happen
    // ahead of it.
    #[test]
    fn whether_a_note_is_expected_is_readable_before_playing_it() {
        // A plain function rather than a closure: a closure capturing
        // `attempt` would still borrow it immutably at the `attempt.play`
        // call below, which needs `&mut`.
        fn expects(attempt: &Attempt, pitch: hogwash_domain::Pitch) -> bool {
            attempt
                .cursor()
                .expecting()
                .is_some_and(|expected| expected.sounding().any(|note| note == pitch))
        }

        let reference = one_bar(&[Step::C, Step::D]);
        let mut attempt = Attempt::begin(reference);

        assert!(expects(&attempt, pitch(Step::C)));
        assert!(!expects(&attempt, unexpected()));

        // The read is unaffected by whether the note is then actually played.
        attempt.play(unexpected());
        assert!(expects(&attempt, pitch(Step::C)));
    }

    // - "Once ended, an Attempt does not change." Carried by the
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

    // Releases and the pedal are facts about the performance, not notes:
    // the Cursor waits for the same position before and after them. The
    // notes are the Minuet's first two, D and G.
    #[test]
    fn a_release_or_the_pedal_never_moves_the_cursor() {
        let mut attempt = Attempt::begin(one_bar(&[Step::D, Step::G]));
        let touch = Touch {
            at_us: 0,
            velocity: 80,
            released_us: None,
        };

        assert_eq!(
            attempt.play_touched(pitch(Step::D), touch),
            CursorMovement::Advanced
        );
        let position = attempt.cursor().position();

        attempt.release(pitch(Step::D), 300_000);
        attempt.pedal(350_000, true);
        attempt.pedal(900_000, false);

        assert_eq!(attempt.cursor().position(), position);
        assert_eq!(attempt.observed().played(), &[pitch(Step::D)]);
        assert_eq!(
            attempt.observed().touches()[0].and_then(|touch| touch.released_us),
            Some(300_000)
        );
        assert_eq!(attempt.observed().pedal_changes().len(), 2);
    }
}
