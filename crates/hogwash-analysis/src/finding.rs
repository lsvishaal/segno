//! One thing Analysis found.

use hogwash_domain::{Location, MeasureNumber, Pitch};
use hogwash_notation::Note;

/// A single observation about an Attempt.
///
/// The design names exactly four outcomes — *"which notes were correct, which
/// were wrong, which expected notes never arrived, and which unexpected ones
/// did"* — so there are four variants, no more and no fewer.
///
/// # Where the line between wrong and unexpected falls
///
/// Under wait-for-me the Cursor never moves past an unplayed note, so every
/// non-matching note is in some sense surplus, and the distinction between
/// The requirement is not obvious. The requirement wording draws it:
///
/// - the requirement concerns a note whose pitch differs from **the corresponding
///   note** in the reference performance. A corresponding note exists exactly
///   while the Cursor is still waiting on a position. → [`IncorrectPitch`],
///   which therefore has a location and can say what was wanted there.
/// - the requirement concerns a note **absent from the reference performance**.
///   Nothing corresponds to it exactly when nothing is expected — the
///   performer played on after the piece ended. → [`Unexpected`], which has
///   no location, because there is no position it belongs to.
///
/// [`IncorrectPitch`]: Finding::IncorrectPitch
/// [`Unexpected`]: Finding::Unexpected
///
/// # Locations, not aggregation
///
/// Findings carry a [`Location`] because Feedback locates its findings by
/// Measure and Note. That is *location*. The requirement, which asks for
/// results **aggregated** per measure, is deferred and no rollup
/// exists.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Finding {
    /// A played note that the Score asked for at the position it was played.
    Correct {
        /// Where in the music.
        at: Location,
        /// What was played, which is also what was wanted.
        pitch: Pitch,
    },

    /// A played note that differs from what the Score asked for here.
    ///
    /// It does **not** consume the position. The Cursor holds, so
    /// the performer usually goes on to play the right note, which is then
    /// [`Correct`](Finding::Correct). One position can yield several
    /// findings, and that is a faithful account of what happened.
    IncorrectPitch {
        /// Where in the music.
        at: Location,
        /// What the performer played.
        ///
        /// A bare `Pitch`, because that is genuinely all an instrument
        /// reports. The design forbids reconstructing a spelling from it.
        played: Pitch,
        /// What the Score asked for here, in written order.
        ///
        /// Carried as written `Note`s rather than sounding pitches, because
        /// The design gives Feedback an Analysis and nothing else, and
        /// Feedback must speak in musical terms — "F was
        /// written" cannot be said from the number 65.
        expected: Vec<Note>,
    },

    /// An expected note that never arrived.
    ///
    /// Under wait-for-me these can only be the tail the performer never
    /// reached, plus any unplayed members of a final part-played chord — a
    /// note cannot be skipped mid-piece, because the Cursor will not pass it.
    Omitted {
        /// Where in the music.
        at: Location,
        /// What the Score asked for and did not get, as written.
        written: Note,
    },

    /// A played note when the Score asked for nothing further.
    ///
    /// Carries no location: there is no position it belongs to, which is
    /// precisely what distinguishes it from
    /// [`IncorrectPitch`](Finding::IncorrectPitch).
    Unexpected {
        /// What the performer played.
        played: Pitch,
    },
}

impl Finding {
    /// Whether this finding reports something other than a correct note.
    ///
    /// A convenience for counting, not a judgement of the Attempt —
    /// The design is explicit that Analysis does not judge.
    #[must_use]
    pub const fn is_correct(&self) -> bool {
        matches!(self, Self::Correct { .. })
    }

    /// Where in the music this finding is, if it is anywhere.
    ///
    /// `None` only for [`Unexpected`](Finding::Unexpected), which by
    /// definition belongs to no position.
    #[must_use]
    pub const fn location(&self) -> Option<Location> {
        match self {
            Self::Correct { at, .. }
            | Self::IncorrectPitch { at, .. }
            | Self::Omitted { at, .. } => Some(*at),
            Self::Unexpected { .. } => None,
        }
    }

    /// Which bar this finding is in, if it is in one.
    #[must_use]
    pub const fn measure(&self) -> Option<MeasureNumber> {
        match self.location() {
            Some(at) => Some(at.measure()),
            None => None,
        }
    }
}
