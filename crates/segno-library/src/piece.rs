//! A practisable work.

use segno_domain::PieceId;
use segno_notation::{ReferencePerformance, Score};

/// The best title there is: the score's own, or failing that the name of the
/// file it came from, tidied (`Canon_in_D.mxl` becomes "Canon in D"). Empty
/// when there is neither, which [`Piece::new`] turns into "Untitled".
#[must_use]
pub fn title_from(score_title: Option<&str>, file_name: Option<&str>) -> String {
    if let Some(title) = score_title.map(str::trim).filter(|title| !title.is_empty()) {
        return title.to_owned();
    }

    let Some(name) = file_name else {
        return String::new();
    };
    let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let stem = match name.rsplit_once('.') {
        Some((stem, extension))
            if ["mxl", "musicxml", "xml"].contains(&extension.to_ascii_lowercase().as_str()) =>
        {
            stem
        }
        _ => name,
    };

    stem.replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A musical work the performer can practise.
///
/// Holds its notation, a durable name, and enough about itself to be told
/// apart from another.
///
/// The name is a [`PieceId`] rather than a title, because the design fixes a
/// Practice Session to exactly one Piece and a Practice Record has to say
/// which — across restarts, and after a retitling. What the name is made of
/// is the design's business and deliberately invisible here.
///
/// Immutable. The requirement treats imported content as read-only after import, and
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
    /// The requirement requires *"at minimum a title and, where available, a
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

    /// The same Piece under another title, as the performer chose to call
    /// it. A blank title becomes "Untitled", as in [`new`](Self::new).
    #[must_use]
    pub fn retitled(self, title: &str) -> Self {
        Self {
            title: if title.trim().is_empty() {
                "Untitled".to_owned()
            } else {
                title.trim().to_owned()
            },
            ..self
        }
    }

    /// Its durable name, for a Practice Record to refer to.
    #[must_use]
    pub const fn id(&self) -> &PieceId {
        &self.id
    }

    /// What to call it. Never empty.
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

    /// What playing it is expected to produce.
    ///
    /// Derived once, at construction, rather than on each request. The design
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
    use segno_domain::{MeasureNumber, PieceId, PitchSpelling, Step};
    use segno_notation::{Measure, Note, Position, Score};

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

    // "at least a title is available".
    #[test]
    fn a_piece_has_a_title() {
        let piece = Piece::new(named(), "Minuet in G", None, score());

        assert_eq!(piece.title(), "Minuet in G");
    }

    // A score with no title is still music. Refusing it would be refusing
    // music over metadata, and the requirement only requires that a title *is
    // available* — not that the source supplied one.
    #[test]
    fn a_piece_with_no_title_still_has_one() {
        assert_eq!(
            Piece::new(named(), "   ", None, score()).title(),
            "Untitled"
        );
    }

    #[test]
    fn a_score_title_wins_over_the_file_name() {
        assert_eq!(
            super::title_from(Some(" Canon in D "), Some("x.mxl")),
            "Canon in D"
        );
    }

    #[test]
    fn with_no_score_title_the_file_name_is_tidied_into_one() {
        for (file, title) in [
            ("Canon_in_D.mxl", "Canon in D"),
            (
                "Erik_Satie_-_Gymnopedie_No.1.mxl",
                "Erik Satie - Gymnopedie No.1",
            ),
            ("/home/me/Downloads/Fur_Elise.musicxml", "Fur Elise"),
            ("minuet  in   g.XML", "minuet in g"),
            ("Sonata No. 14", "Sonata No. 14"),
        ] {
            assert_eq!(super::title_from(Some("  "), Some(file)), title, "{file}");
        }
    }

    #[test]
    fn with_neither_there_is_no_title_to_give() {
        assert_eq!(super::title_from(None, None), "");
    }

    // the composer is "where available", so its absence is ordinary.
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

    // "given an imported piece, when its reference performance is
    // requested, then a sequence of expected musical events is returned".
    #[test]
    fn a_piece_knows_what_playing_it_should_produce() {
        let piece = Piece::new(named(), "T", None, score());

        assert_eq!(piece.reference().len(), 1);
    }

    // the same Score always yields the same Reference Performance.
    #[test]
    fn the_same_score_always_gives_the_same_piece() {
        assert_eq!(
            Piece::new(named(), "T", None, score()),
            Piece::new(named(), "T", None, score())
        );
    }
}
