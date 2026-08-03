//! One thing Analysis found (`003 §7.6`).

use hogwash_domain::{MeasureNumber, Pitch};

/// A single observation about an Attempt.
///
/// `003 §7.6` names exactly four outcomes — *"which notes were correct, which
/// were wrong, which expected notes never arrived, and which unexpected ones
/// did"* — so there are four variants, no more and no fewer.
///
/// # Where the line between wrong and unexpected falls
///
/// Under wait-for-me the Cursor never moves past an unplayed note, so every
/// non-matching note is in some sense surplus, and the distinction between
/// FR-401 and FR-403 is not obvious. The requirement wording draws it:
///
/// - FR-401 concerns a note whose pitch differs from **the corresponding
///   note** in the reference performance. A corresponding note exists exactly
///   while the Cursor is still waiting on a position. → [`IncorrectPitch`],
///   which therefore has a location and can say what was wanted there.
/// - FR-403 concerns a note **absent from the reference performance**.
///   Nothing corresponds to it exactly when nothing is expected — the
///   performer played on after the piece ended. → [`Unexpected`], which has
///   no location, because there is no position it belongs to.
///
/// [`IncorrectPitch`]: Finding::IncorrectPitch
/// [`Unexpected`]: Finding::Unexpected
///
/// # Locations, not aggregation
///
/// Findings carry a [`MeasureNumber`] because Feedback locates its findings
/// by Measure (`003 §7.7`). That is *location*. FR-407, which asks for
/// results **aggregated** per measure, is deferred (`MVP §8`) and no rollup
/// exists.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Finding {
    /// A played note that the Score asked for at the position it was played.
    Correct {
        /// Where in the music.
        measure: MeasureNumber,
        /// Which step of the Reference Performance.
        position: usize,
        /// What was played, which is also what was wanted.
        pitch: Pitch,
    },

    /// A played note that differs from what the Score asked for here
    /// (FR-401).
    ///
    /// It does **not** consume the position. The Cursor holds (`MVP §5`), so
    /// the performer usually goes on to play the right note, which is then
    /// [`Correct`](Finding::Correct). One position can yield several
    /// findings, and that is a faithful account of what happened.
    IncorrectPitch {
        /// Where in the music.
        measure: MeasureNumber,
        /// Which step of the Reference Performance.
        position: usize,
        /// What the performer played.
        played: Pitch,
        /// What the Score asked for here, in written order.
        expected: Vec<Pitch>,
    },

    /// An expected note that never arrived (FR-402).
    ///
    /// Under wait-for-me these can only be the tail the performer never
    /// reached, plus any unplayed members of a final part-played chord — a
    /// note cannot be skipped mid-piece, because the Cursor will not pass it
    /// (`MVP §5`).
    Omitted {
        /// Where in the music.
        measure: MeasureNumber,
        /// Which step of the Reference Performance.
        position: usize,
        /// What the Score asked for and did not get.
        pitch: Pitch,
    },

    /// A played note when the Score asked for nothing further (FR-403).
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
    /// `003 §7.6` is explicit that Analysis does not judge.
    #[must_use]
    pub const fn is_correct(&self) -> bool {
        matches!(self, Self::Correct { .. })
    }

    /// Where in the music this finding is, if it is anywhere.
    ///
    /// `None` only for [`Unexpected`](Finding::Unexpected), which by
    /// definition belongs to no position.
    #[must_use]
    pub const fn measure(&self) -> Option<MeasureNumber> {
        match self {
            Self::Correct { measure, .. }
            | Self::IncorrectPitch { measure, .. }
            | Self::Omitted { measure, .. } => Some(*measure),
            Self::Unexpected { .. } => None,
        }
    }
}
