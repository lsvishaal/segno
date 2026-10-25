//! Which work is being practised.

/// A durable name for a Piece.
///
/// The design fixes a Practice Session to exactly one Piece, and a Practice
/// Record that outlives the program has to say which one. That is the whole
/// job of this type.
///
/// # Why it is opaque
///
/// What the name is *made of* is a persistence question, answered by
/// The design, and nothing outside storage should depend on the answer. A
/// title would collide and would change if a score were retitled; a file path
/// would break the moment a performer tidied their folders; a number assigned
/// in sequence would mean two machines disagreed about the same score. All
/// three are visible mistakes only in hindsight, which is why this type shows
/// nothing about which was chosen.
///
/// It orders and compares, so records can be found and grouped, and that is
/// all it does.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PieceId(String);

impl PieceId {
    /// The Piece this name refers to.
    ///
    /// Returns `None` for an empty name, or one containing anything but
    /// letters, digits and dashes. That is not fussiness about tidiness: a
    /// name reaches a filesystem in `hogwash-records`, and a name containing
    /// a separator or a `..` would be a path rather than a name.
    #[must_use]
    pub fn new(name: &str) -> Option<Self> {
        let usable = !name.is_empty()
            && name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-');

        usable.then(|| Self(name.to_owned()))
    }

    /// The name, for storage to use.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::piece::PieceId;

    #[test]
    fn a_piece_is_named_by_its_identifier() {
        assert_eq!(
            PieceId::new("a1b2c3").map(|id| id.as_str().to_owned()),
            Some("a1b2c3".to_owned())
        );
    }

    #[test]
    fn a_piece_with_no_name_is_not_a_piece() {
        assert!(PieceId::new("").is_none());
    }

    // An identifier becomes a filename in hogwash-records. One containing a
    // separator, or a parent-directory hop, would be a path -- and a record
    // could then be written wherever the name said.
    #[test]
    fn an_identifier_cannot_smuggle_a_path_into_storage() {
        for name in ["../escape", "a/b", "a\\b", "..", "a b", "a.b", "a\0b"] {
            assert!(PieceId::new(name).is_none(), "`{name}` was accepted");
        }
    }

    #[test]
    fn identifiers_order_so_records_can_be_grouped() {
        let first = PieceId::new("aaa").expect("valid");
        let second = PieceId::new("bbb").expect("valid");

        assert!(first < second);
    }
}
