//! A practisable work (`003 §7.9`).

use hogwash_domain::PieceId;
use hogwash_notation::{ReferencePerformance, Score};

/// A musical work the performer can practise.
///
/// Holds its notation, a durable name, and enough about itself to be told
/// apart from another (FR-201).
///
/// The name is a [`PieceId`] rather than a title, because `003 §7.2` fixes a
/// Practice Session to exactly one Piece and a Practice Record has to say
/// which — across restarts, and after a retitling. What the name is made of
/// is `ADR-0010`'s business and deliberately invisible here.
///
/// Immutable. FR-202 treats imported content as read-only after import, and
/// there is no operation here that changes anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    id: PieceId,
    title: String,
    composer: Option<String>,
    score: Score,
    reference: ReferencePerformance,
}

impl Piece {
    /// A Piece called `title`, written by `composer` where that is known.
    ///
    /// FR-201 requires *"at minimum a title and, where available, a
    /// composer"*, so the title is required and the composer is not. A
    /// missing or blank title becomes a stated placeholder rather than an
    /// error: a score with no title in it is perfectly practisable, and
    /// refusing it would be refusing music over metadata.
    #[must_use]
    pub fn new(id: PieceId, title: &str, composer: Option<String>, score: Score) -> Self {
        let reference = ReferencePerformance::derived_from(&score);

        Self {
            id,
            title: if title.trim().is_empty() {
                "Untitled".to_owned()
            } else {
                title.trim().to_owned()
            },
            composer: composer.filter(|name| !name.trim().is_empty()),
            score,
            reference,
        }
    }

    /// Its durable name, for a Practice Record to refer to.
    #[must_use]
    pub const fn id(&self) -> &PieceId {
        &self.id
    }

    /// What to call it. Never empty (FR-201).
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Who wrote it, where the source said.
    #[must_use]
    pub fn composer(&self) -> Option<&str> {
        self.composer.as_deref()
    }

    /// The written music.
    #[must_use]
    pub const fn score(&self) -> &Score {
        &self.score
    }

    /// What playing it is expected to produce (FR-204).
    ///
    /// Derived once, at construction, rather than on each request. `003 §7.4`
    /// makes the derivation total and deterministic — the same Score always
    /// yields the same Reference Performance — so there is nothing to gain by
    /// repeating it and one fewer place for the two to disagree.
    #[must_use]
    pub const fn reference(&self) -> &ReferencePerformance {
        &self.reference
    }
}

#[cfg(test)]
mod tests {
    use hogwash_domain::{MeasureNumber, PieceId, PitchSpelling, Step};
    use hogwash_notation::{Measure, Note, Position, Score};

    use crate::piece::Piece;

    fn named() -> PieceId {
        PieceId::new("fixture").expect("a plain name is usable")
    }

    fn score() -> Score {
        let note = Note::new(PitchSpelling::new(Step::C, 0, 4)).expect("middle C sounds");
        let position = Position::new(vec![note]).expect("one note is a position");
        let measure = Measure::new(
            MeasureNumber::new(1).expect("bars are numbered from one"),
            vec![position],
        );

        Score::new(vec![measure]).expect("one measure is a score")
    }

    // FR-201 — "at least a title is available".
    #[test]
    fn a_piece_has_a_title() {
        let piece = Piece::new(named(), "Minuet in G", None, score());

        assert_eq!(piece.title(), "Minuet in G");
    }

    // A score with no title is still music. Refusing it would be refusing
    // music over metadata, and FR-201 only requires that a title *is
    // available* — not that the source supplied one.
    #[test]
    fn a_piece_with_no_title_still_has_one() {
        assert_eq!(
            Piece::new(named(), "   ", None, score()).title(),
            "Untitled"
        );
    }

    // FR-201 — the composer is "where available", so its absence is ordinary.
    #[test]
    fn the_composer_is_optional_and_a_blank_one_is_no_composer() {
        assert_eq!(Piece::new(named(), "T", None, score()).composer(), None);
        assert_eq!(
            Piece::new(named(), "T", Some("  ".to_owned()), score()).composer(),
            None
        );
        assert_eq!(
            Piece::new(named(), "T", Some("Bach".to_owned()), score()).composer(),
            Some("Bach")
        );
    }

    // FR-204 — "given an imported piece, when its reference performance is
    // requested, then a sequence of expected musical events is returned".
    #[test]
    fn a_piece_knows_what_playing_it_should_produce() {
        let piece = Piece::new(named(), "T", None, score());

        assert_eq!(piece.reference().len(), 1);
    }

    // 003 §7.4 — the same Score always yields the same Reference Performance.
    #[test]
    fn the_same_score_always_gives_the_same_piece() {
        assert_eq!(
            Piece::new(named(), "T", None, score()),
            Piece::new(named(), "T", None, score())
        );
    }
}
