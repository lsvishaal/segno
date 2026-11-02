//! The set of Pieces available to practise.

use segno_domain::PieceId;

use crate::piece::Piece;

/// The Pieces the performer can choose from.
///
/// # Never empty, and why that is a type rather than a check
///
/// The design states it as an invariant and the design as a failure boundary:
/// *"a failure elsewhere in the system must never leave the Library in a
/// state where no Piece is available."* An empty Library is a dead end — there
/// is nothing to select, so there is nothing to do, and the first-run
/// experience the design exists to protect is exactly the case where it would
/// happen.
///
/// A runtime check would report the problem after it occurred. Instead the
/// only way to build a Library takes a first Piece **by value**
/// ([`opened_with`](Self::opened_with)), and nothing removes that one, so
/// emptiness is not a state this type has. The requirement is then a fact about the
/// program rather than a rule someone has to remember.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    // Not a Vec that might be empty: the first Piece is a separate field, so
    // the invariant is visible in the layout. `first` plus `rest` is uglier
    // than one Vec and it is the ugliness that makes the guarantee real.
    first: Piece,
    rest: Vec<Piece>,
}

impl Library {
    /// A Library holding the provided practice material.
    ///
    /// `first` is required, which is the requirement expressed as a signature: the
    /// application ships with practice material, so a Library can always be
    /// built without any import having happened.
    #[must_use]
    pub fn opened_with(first: Piece, rest: Vec<Piece>) -> Self {
        Self { first, rest }
    }

    /// Adds an imported Piece.
    ///
    /// Takes and returns `self`, so a Library is replaced rather than
    /// mutated. Nothing in the design needs a Library to change under
    /// anyone's feet, and a consumed receiver means an addition that fails
    /// part-way cannot leave one half-updated.
    #[must_use]
    pub fn with(mut self, piece: Piece) -> Self {
        self.rest.push(piece);
        self
    }

    /// The same Library with the Piece named `id` retitled; unchanged if
    /// there is no such Piece.
    #[must_use]
    pub fn retitled(self, id: &PieceId, title: &str) -> Self {
        let retitle = |piece: Piece| {
            if piece.id() == id {
                piece.retitled(title)
            } else {
                piece
            }
        };

        Self {
            first: retitle(self.first),
            rest: self.rest.into_iter().map(retitle).collect(),
        }
    }

    /// The same Library without the imported Piece named `id`.
    ///
    /// The first Piece is never removed, so the Library still cannot be
    /// empty; it is bundled material, which is not the performer's to
    /// delete.
    #[must_use]
    pub fn without(mut self, id: &PieceId) -> Self {
        self.rest.retain(|piece| piece.id() != id);
        self
    }

    /// Every Piece available, in the order they became available.
    ///
    /// Provided material first, then imports in the order they were
    /// accepted. Deterministic, and a Vec rather than a set because two
    /// Pieces may legitimately share a title.
    #[must_use]
    pub fn pieces(&self) -> Vec<&Piece> {
        std::iter::once(&self.first)
            .chain(self.rest.iter())
            .collect()
    }

    /// How many Pieces are available. **Never zero.**
    ///
    /// Called `count` rather than `len` deliberately. `len` invites an
    /// `is_empty` beside it, and an `is_empty` that can only ever answer
    /// `false` is a question worth not asking — it would suggest emptiness is
    /// a state this type has, which is exactly what the requirement says it does not.
    #[must_use]
    pub fn count(&self) -> usize {
        1 + self.rest.len()
    }

    /// The `index`-th Piece, if there is one.
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&Piece> {
        match index {
            0 => Some(&self.first),
            _ => self.rest.get(index - 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use segno_domain::{MeasureNumber, PieceId, PitchSpelling, Step};
    use segno_notation::{Measure, Note, Position, Score};

    use crate::{Library, Piece};

    fn piece(title: &str) -> Piece {
        let note = Note::new(PitchSpelling::new(Step::C, 0, 4)).expect("middle C sounds");
        let position = Position::new(vec![note]).expect("one note is a position");
        let measure = Measure::new(
            MeasureNumber::new(1).expect("bars are numbered from one"),
            vec![position],
        );
        let score = Score::new(vec![measure]).expect("one measure is a score");

        let id = PieceId::new(&title.replace(' ', "-")).expect("fixture titles are usable names");

        Piece::new(id, title, None, score)
    }

    // a first session requires no externally obtained
    // file. A Library exists before any import has happened.
    #[test]
    fn a_library_exists_before_anything_has_been_imported() {
        let library = Library::opened_with(piece("Provided"), vec![]);

        assert_eq!(library.count(), 1);
        assert_eq!(library.pieces()[0].title(), "Provided");
    }

    // never empty. This test cannot be written as
    // "assert the library is not empty after some sequence of failures",
    // because there is no way to construct an empty one; `len` returning at
    // least one is what the type guarantees.
    #[test]
    fn a_library_is_never_empty() {
        let library = Library::opened_with(piece("Provided"), vec![]);

        assert!(library.count() >= 1);
        assert!(library.at(0).is_some());
    }

    #[test]
    fn a_piece_can_be_retitled_and_nothing_else_changes() {
        let library = Library::opened_with(piece("Provided"), vec![piece("Sonate No 14")]);
        let id = PieceId::new("Sonate-No-14").expect("usable");

        let renamed = library.retitled(&id, "  Moonlight Sonata ");

        let titles: Vec<&str> = renamed.pieces().iter().map(|piece| piece.title()).collect();
        assert_eq!(titles, ["Provided", "Moonlight Sonata"]);
        assert_eq!(renamed.pieces()[1].id(), &id);
    }

    #[test]
    fn an_imported_piece_can_be_removed_but_the_first_never_is() {
        let library = Library::opened_with(piece("Provided"), vec![piece("Imported")]);

        let removed = library.without(&PieceId::new("Imported").expect("usable"));
        assert_eq!(removed.count(), 1);

        let still = removed.without(&PieceId::new("Provided").expect("usable"));
        assert_eq!(still.count(), 1, "never empty");
    }

    // "given two or more imported pieces, when the piece library is
    // accessed, then all imported pieces are individually retrievable".
    #[test]
    fn imported_pieces_join_the_provided_ones() {
        let library = Library::opened_with(piece("Provided"), vec![])
            .with(piece("Imported"))
            .with(piece("Also imported"));

        assert_eq!(
            library
                .pieces()
                .iter()
                .map(|piece| piece.title())
                .collect::<Vec<_>>(),
            ["Provided", "Imported", "Also imported"]
        );
    }

    // Two performers may own two arrangements of the same work. Silently
    // collapsing them would lose one.
    #[test]
    fn two_pieces_may_share_a_title() {
        let library = Library::opened_with(piece("Minuet"), vec![]).with(piece("Minuet"));

        assert_eq!(library.count(), 2);
    }

    #[test]
    fn a_piece_beyond_the_end_is_not_there() {
        assert!(
            Library::opened_with(piece("Provided"), vec![])
                .at(1)
                .is_none()
        );
    }
}
