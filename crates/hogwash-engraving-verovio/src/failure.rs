//! Why the engine's output could not be turned into written music.
//!
//! Separate from [`ToolkitError`](crate::toolkit::ToolkitError), which is
//! about the boundary rather than about the music: the toolkit answering at
//! all, versus the answer being something a Score can be made from.

use std::fmt::{Display, Formatter, Result as Formatted};

/// A document the engine produced and this adapter could not read.
///
/// Every variant carries enough to say what was wrong, because `004 §8.2`
/// requires an assessment failure to resolve into an explanation rather than
/// a shrug — and `MVP §6.9` requires that explanation to reach the performer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TranslationFailure {
    /// The engine returned something that is not a parseable document.
    NotADocument(String),

    /// The document parsed and contains no music.
    NoMusic,

    /// A measure with no number, or numbered zero.
    ///
    /// `003 §7.8` numbers measures from one. Renumbering silently would put
    /// every later finding one bar out.
    UnnumberedMeasure(String),

    /// A note whose letter or octave could not be read.
    UnreadableNote(String),

    /// An accidental finer than a semitone.
    ///
    /// Refused rather than rounded — see `mei::alteration_of`.
    UnsupportedAccidental(String),

    /// The engine's list of what sounds when could not be read.
    UnreadableTiming(String),

    /// The music read, but does not form a Score.
    ///
    /// `hogwash-notation` rejects, for instance, a Score whose measures do
    /// not ascend. That is a rejection worth passing on rather than working
    /// around.
    NotAScore,
}

impl Display for TranslationFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Formatted {
        match self {
            Self::NotADocument(detail) => {
                write!(
                    formatter,
                    "the engraver returned no readable document: {detail}"
                )
            }
            Self::NoMusic => write!(formatter, "the file contains no music"),
            Self::UnnumberedMeasure(written) => write!(
                formatter,
                "a bar is numbered `{written}`, and bars are numbered from one"
            ),
            Self::UnreadableNote(detail) => write!(formatter, "a note could not be read: {detail}"),
            Self::UnsupportedAccidental(accidental) => write!(
                formatter,
                "the accidental `{accidental}` is finer than a semitone, which this program cannot represent"
            ),
            Self::UnreadableTiming(detail) => {
                write!(
                    formatter,
                    "the engraver's timing could not be read: {detail}"
                )
            }
            Self::NotAScore => write!(formatter, "the bars do not form a readable score"),
        }
    }
}
