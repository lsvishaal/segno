//! A Practice Session — one sitting on one Piece.

use std::sync::Arc;

use hogwash_notation::ReferencePerformance;

use crate::attempt::{Attempt, EndedAttempt};

/// A continuous period during which a performer practises one Piece
///, still open.
///
/// **One Piece, fixed for life.** The design states it as an invariant, and
/// it is carried by the type: the Reference Performance is supplied once at
/// [`open`](Self::open) and there is no way to replace it, so
/// [`begin_attempt`](Self::begin_attempt) needs no Piece argument — there is
/// nothing else an Attempt could be at.
///
/// A Session evaluates nothing and judges nothing (the design
/// Non-Responsibilities). It groups Attempts; that is all.
///
/// **No timestamps.** The design calls a Session "bounded in time", but
/// The design forbids a clock here. When Practice Records arrive the caller
/// supplies the time; the domain never reads one.
#[derive(Debug, Clone)]
pub struct PracticeSession {
    reference: Arc<ReferencePerformance>,
    attempts: Vec<EndedAttempt>,
}

impl PracticeSession {
    /// Opens a Session on the Piece `reference` describes.
    ///
    /// It begins with no Attempts, which the design notes is legitimate — a
    /// performer may open one and be interrupted before playing.
    #[must_use]
    pub fn open(reference: Arc<ReferencePerformance>) -> Self {
        Self {
            reference,
            attempts: Vec::new(),
        }
    }

    /// What the Piece expects.
    #[must_use]
    pub fn reference(&self) -> &ReferencePerformance {
        &self.reference
    }

    /// A share of what the Piece expects, for whoever needs to hold one.
    ///
    /// The application keeps this alongside the Session, which is why the
    /// Reference Performance is shared rather than borrowed at all — see the
    /// note on [`Cursor`](crate::Cursor).
    #[must_use]
    pub fn shared_reference(&self) -> Arc<ReferencePerformance> {
        Arc::clone(&self.reference)
    }

    /// Begins an Attempt at this Session's Piece.
    #[must_use]
    pub fn begin_attempt(&self) -> Attempt {
        Attempt::begin(Arc::clone(&self.reference))
    }

    /// Begins an Attempt at step `from` of this Session's Piece.
    #[must_use]
    pub fn begin_attempt_at(&self, from: usize) -> Attempt {
        Attempt::begin_at(Arc::clone(&self.reference), from)
    }

    /// Adds an ended Attempt to this Session's history.
    pub fn record(&mut self, attempt: EndedAttempt) {
        self.attempts.push(attempt);
    }

    /// The Attempts made so far, in the order they were made.
    #[must_use]
    pub fn attempts(&self) -> &[EndedAttempt] {
        &self.attempts
    }

    /// Closes the Session.
    ///
    /// *"A closed Session accepts no further Attempts."* Carried
    /// by the type: this consumes the Session, and [`ClosedSession`] has
    /// neither [`begin_attempt`](Self::begin_attempt) nor
    /// [`record`](Self::record). A closed Session gaining an Attempt is not a
    /// bug that can be written.
    #[must_use]
    pub fn close(self) -> ClosedSession {
        ClosedSession {
            attempts: self.attempts,
        }
    }
}

/// A Practice Session that has been closed.
///
/// It keeps its history and accepts nothing further. The design — *"Ended
/// things do not change."*
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClosedSession {
    attempts: Vec<EndedAttempt>,
}

impl ClosedSession {
    /// The Attempts made during the Session, in the order they were made.
    #[must_use]
    pub fn attempts(&self) -> &[EndedAttempt] {
        &self.attempts
    }
}

#[cfg(test)]
mod tests {
    use hogwash_domain::Step;

    use crate::fixtures::{one_bar, pitch};
    use crate::session::PracticeSession;

    // The design Notes -- "A Session containing no Attempts is legitimate -- a
    // performer may open one and be interrupted before playing. Nothing in
    // the domain requires an Attempt to have occurred."
    #[test]
    fn a_session_with_no_attempts_is_legitimate() {
        let session = PracticeSession::open(one_bar(&[Step::C]));

        assert!(session.attempts().is_empty());
        assert!(session.close().attempts().is_empty());
    }

    // - "It holds the Attempts made within it, in the order they
    // were made."
    #[test]
    fn a_session_holds_its_attempts_in_the_order_they_were_made() {
        let mut session = PracticeSession::open(one_bar(&[Step::C, Step::D]));

        let mut first = session.begin_attempt();
        first.play(pitch(Step::C));
        let first = first.end();
        session.record(first);

        let mut second = session.begin_attempt();
        second.play(pitch(Step::C));
        second.play(pitch(Step::D));
        let second = second.end();
        session.record(second);

        let reached: Vec<_> = session
            .attempts()
            .iter()
            .map(super::EndedAttempt::reached)
            .collect();

        assert_eq!(reached, vec![1, 2]);
    }

    // - "A Practice Session concerns exactly one Piece, fixed for
    // the Session's lifetime." Every Attempt is at that Piece, because
    // `begin_attempt` takes no piece argument -- there is nothing else it
    // could be at.
    #[test]
    fn every_attempt_in_a_session_is_at_the_sessions_piece() {
        let session = PracticeSession::open(one_bar(&[Step::C, Step::D]));

        let attempt = session.begin_attempt();

        assert_eq!(attempt.cursor().expecting(), session.reference().at(0));
    }

    // The design Lifecycle -- "A closed Session accepts no further Attempts."
    // Carried by the type: `close` consumes the Session, and ClosedSession
    // has no `begin_attempt` or `record`. This is a compile-time fact; the
    // test records that closing preserves the history rather than discarding
    // it (the design, "Ended things do not change").
    #[test]
    fn a_closed_session_keeps_the_attempts_it_already_held() {
        let mut session = PracticeSession::open(one_bar(&[Step::C]));

        let mut attempt = session.begin_attempt();
        attempt.play(pitch(Step::C));
        let attempt = attempt.end();
        session.record(attempt);

        assert_eq!(session.close().attempts().len(), 1);
    }
}
