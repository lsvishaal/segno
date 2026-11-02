//! One thing Feedback says.

use segno_domain::{Location, MeasureNumber, PitchSpelling, Step};
use segno_notation::Note;

/// One statement about an Attempt, located in the Piece.
///
/// Structured rather than prose, because the design sends Feedback to
/// Practice Record Keeping as well as to Presentation: storing the words
/// would mean that rewording them later makes old records inconsistent with
/// new ones. [`describe`](Self::describe) produces the English on demand.
///
/// There is no statement for a correct note. The design says Feedback exists
/// to make an Analysis understandable, and the overall result already reports
/// how many notes were right; a line per correct note would bury the
/// deviations the performer is looking for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Statement {
    /// The Score wrote this note here, and something else was played.
    ///
    /// What was played is deliberately not carried. The design forbids
    /// reconstructing a spelling from a sounding pitch, and the design records
    /// the consequence: Feedback names what was written and reports that a
    /// different note was played, without inventing a name for it.
    NotPlayedAsWritten {
        /// Where in the music to look.
        at: Location,
        /// The note the Score wrote, as written.
        written: Note,
    },

    /// The Score wrote this note and it never arrived.
    NeverPlayed {
        /// Where in the music to look.
        at: Location,
        /// The note the Score wrote, as written.
        written: Note,
    },

    /// Notes were played after the end of the Piece.
    ///
    /// Counted rather than located, because the design gives these findings no
    /// position — nothing in the Score corresponds to them. Counting findings
    /// the Analysis already produced is presentation, not a new finding.
    PlayedAfterTheEnd {
        /// How many.
        count: usize,
    },
}

impl Statement {
    /// Where in the music this statement concerns, if it concerns anywhere.
    ///
    /// `None` only for [`PlayedAfterTheEnd`](Statement::PlayedAfterTheEnd),
    /// which belongs to no place in the music.
    ///
    /// A full [`Location`] rather than a bar number, because the design locates
    /// Feedback *"by reference to the Piece — to the Notes and Measures
    /// concerned"*, and the design shows a deviation **on the score**, which
    /// needs the note and not only the bar. This does not reopen the design: the
    /// words still say only the bar, because a position index is not something
    /// a performer says aloud.
    #[must_use]
    pub const fn location(&self) -> Option<Location> {
        match self {
            Self::NotPlayedAsWritten { at, .. } | Self::NeverPlayed { at, .. } => Some(*at),
            Self::PlayedAfterTheEnd { .. } => None,
        }
    }

    /// Which bar this statement concerns, if it concerns one.
    #[must_use]
    pub const fn measure(&self) -> Option<MeasureNumber> {
        match self.location() {
            Some(at) => Some(at.measure()),
            None => None,
        }
    }

    /// This statement in words.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::NotPlayedAsWritten { at, written } => format!(
                "Bar {}: {} was written, and a different note was played.",
                at.measure().get(),
                name_of(written.spelling())
            ),
            Self::NeverPlayed { at, written } => format!(
                "Bar {}: {} was written, and was never played.",
                at.measure().get(),
                name_of(written.spelling())
            ),
            Self::PlayedAfterTheEnd { count } => {
                let notes = if *count == 1 { "one note" } else { "notes" };
                let number = if *count == 1 {
                    String::new()
                } else {
                    format!("{count} ")
                };
                format!("After the end of the piece, {number}{notes} were played.")
            }
        }
    }
}

/// A written note in words — "F", "F sharp", "B flat".
///
/// The requirement requires feedback understandable without music theory beyond the
/// target user's baseline, so the accidental is spelled out rather than given
/// as `#` or `♯`.
///
/// This names a **written spelling**, which the Score supplies. It never
/// names a sounding pitch, because that would require choosing between two
/// readings of one key.
fn name_of(spelling: PitchSpelling) -> String {
    let letter = match spelling.step() {
        Step::C => 'C',
        Step::D => 'D',
        Step::E => 'E',
        Step::F => 'F',
        Step::G => 'G',
        Step::A => 'A',
        Step::B => 'B',
    };

    match spelling.alter() {
        0 => letter.to_string(),
        1 => format!("{letter} sharp"),
        -1 => format!("{letter} flat"),
        2 => format!("{letter} double sharp"),
        -2 => format!("{letter} double flat"),
        // Notation beyond a double accidental is vanishingly rare and has no
        // settled plain-English name. Saying it numerically is ugly; saying
        // the wrong thing would be worse, and silently dropping the
        // alteration would name a different note entirely.
        other => format!("{letter} altered by {other} semitones"),
    }
}

#[cfg(test)]
mod tests {
    use segno_domain::{Location, MeasureNumber, Step};

    use crate::fixtures::{note, note_letters_in};
    use crate::statement::Statement;

    fn bar(number: u32) -> MeasureNumber {
        MeasureNumber::new(number).expect("fixture measures are numbered from one")
    }

    // - "expressed in musical terms rather than measurements". The
    // performer reads "F", not 65.
    #[test]
    fn a_written_note_is_named_by_its_letter() {
        let statement = Statement::NotPlayedAsWritten {
            at: Location::new(bar(1), 0),
            written: note(Step::F, 0),
        };

        assert!(statement.describe().contains('F'));
        assert!(!statement.describe().contains("65"));
    }

    // - understandable without music theory expertise beyond the
    // target user's baseline. "F sharp", not "F#" or "F♯".
    #[test]
    fn accidentals_are_named_in_words() {
        let sharp = Statement::NotPlayedAsWritten {
            at: Location::new(bar(1), 0),
            written: note(Step::F, 1),
        };
        let flat = Statement::NotPlayedAsWritten {
            at: Location::new(bar(1), 0),
            written: note(Step::B, -1),
        };

        assert!(sharp.describe().contains("F sharp"));
        assert!(flat.describe().contains("B flat"));
    }

    #[test]
    fn double_accidentals_are_named_too() {
        let statement = Statement::NotPlayedAsWritten {
            at: Location::new(bar(1), 0),
            written: note(Step::C, 2),
        };

        assert!(statement.describe().contains("C double sharp"));
    }

    // - the played note is never named, because naming
    // it would mean choosing between F sharp and G flat, which is a guess
    // about notation the instrument cannot support.
    //
    // Asserted by absence: a statement about a wrong note names the written
    // note and no other note letter.
    #[test]
    fn a_statement_names_only_the_note_that_was_written() {
        let statement = Statement::NotPlayedAsWritten {
            at: Location::new(bar(4), 0),
            written: note(Step::F, 0),
        };

        assert_eq!(note_letters_in(&statement.describe()), vec!['F']);
    }

    // - Feedback locates its findings "by reference to the Piece --
    // to the Notes and Measures concerned".
    #[test]
    fn a_statement_says_which_bar_it_concerns() {
        let statement = Statement::NeverPlayed {
            at: Location::new(bar(7), 0),
            written: note(Step::D, 0),
        };

        assert!(statement.describe().contains('7'));
    }

    // Notes played after the piece has ended have no location, so
    // they are counted rather than placed.
    #[test]
    fn notes_played_after_the_end_are_reported_with_their_number() {
        assert!(
            Statement::PlayedAfterTheEnd { count: 3 }
                .describe()
                .contains('3')
        );
    }

    #[test]
    fn a_single_note_after_the_end_reads_naturally() {
        let described = Statement::PlayedAfterTheEnd { count: 1 }.describe();

        assert!(!described.contains("1 notes"));
    }
}
