//! The practice material that ships with the application.
//!
//! Compiled in rather than read from disk. The requirements are material
//! available "without any import having been performed", and a Library that
//! is never empty — both of which a file on disk cannot
//! guarantee, because a file on disk can be missing.
//!
//! `Library::opened_with` takes its first Piece **by value**, so emptiness is
//! not a state the type has. This module is the one place that could fail to
//! supply that first Piece, and it cannot: the bytes are in the binary.

use std::fmt::Write as _;

use segno_domain::PieceId;
use sha2::{Digest, Sha256};

/// One piece that ships with Segno.
#[derive(Debug, Clone, Copy)]
pub struct Provided {
    /// What the file is called, for diagnostics.
    pub name: &'static str,
    /// The `MusicXML` itself.
    pub source: &'static [u8],
    /// Which shelf of the library it goes on.
    pub level: Level,
}

/// How far along a player a provided piece suits: a description, not a grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    /// Single lines and first two-hand pieces.
    Beginner,
    /// Around Grades 1–4.
    Intermediate,
}

/// The level of the provided piece called `id`; `None` for anything imported.
#[must_use]
pub fn level_of(id: &PieceId) -> Option<Level> {
    PROVIDED
        .iter()
        .find(|provided| &identify(provided.source) == id)
        .map(|provided| provided.level)
}

/// Everything in `library/`, in the order a first-time performer meets it.
///
/// Ordered deliberately: the shortest and simplest first, then the first
/// piece with a second staff, then two real published pieces (a Minuet with
/// repeats, a Prelude full of held notes). `library/PROVENANCE.md` records
/// where each one came from and under what licence.
pub const PROVIDED: [Provided; 5] = [
    Provided {
        name: "lightly-row.musicxml",
        source: include_bytes!("../library/lightly-row.musicxml"),
        level: Level::Beginner,
    },
    Provided {
        name: "ode-to-joy.musicxml",
        source: include_bytes!("../library/ode-to-joy.musicxml"),
        level: Level::Beginner,
    },
    Provided {
        name: "fur-elise.musicxml",
        source: include_bytes!("../library/fur-elise.musicxml"),
        level: Level::Beginner,
    },
    Provided {
        name: "minuet-in-g.musicxml",
        source: include_bytes!("../library/minuet-in-g.musicxml"),
        level: Level::Intermediate,
    },
    Provided {
        name: "prelude-in-c.musicxml",
        source: include_bytes!("../library/prelude-in-c.musicxml"),
        level: Level::Intermediate,
    },
];

/// The durable name for a piece with these bytes.
///
/// # Why the content, and not the title or a counter
///
/// The design left what a [`PieceId`] is made of to storage, and this is
/// storage's answer. Hashing the source gives three properties that matter
/// and are hard to get otherwise:
///
/// - **Idempotent.** Importing the same file twice yields the same Piece
///   rather than a duplicate.
/// - **Stable across machines and restarts**, so a Practice Record written on
///   one still names the right piece on another.
/// - **Independent of metadata.** A retitled score is the same score; two
///   different arrangements sharing a title are not the same piece.
///
/// Truncated to 16 hex characters. That is 64 bits — far more than enough
/// when the population is one person's sheet music, and short enough to read
/// in a filename.
#[must_use]
pub fn identify(source: &[u8]) -> PieceId {
    let digest = Sha256::digest(source);

    let name = digest.iter().take(8).fold(String::new(), |mut name, byte| {
        // `write!` into a String cannot fail. Ignored rather than unwrapped
        // so this stays panic-free without pretending the Result matters.
        let _ = write!(name, "{byte:02x}");
        name
    });

    // `PieceId::new` refuses anything that is not letters, digits and dashes,
    // which hexadecimal always is. The fallback exists so this function is
    // total rather than because it can happen.
    PieceId::new(&name).unwrap_or_else(|| {
        PieceId::new("unnamed").unwrap_or_else(|| unreachable!("`unnamed` is a usable name"))
    })
}

#[cfg(test)]
mod tests {
    use crate::provided::{Level, PROVIDED, identify, level_of};

    #[test]
    fn each_provided_piece_is_on_a_shelf_and_imports_are_not() {
        for piece in PROVIDED {
            assert_eq!(
                level_of(&identify(piece.source)),
                Some(piece.level),
                "{}",
                piece.name
            );
        }
        assert_eq!(level_of(&identify(b"<score-partwise/>")), None);
        let minuet = PROVIDED
            .iter()
            .find(|piece| piece.name == "minuet-in-g.musicxml");
        assert_eq!(minuet.map(|piece| piece.level), Some(Level::Intermediate));
    }

    // a first session requires no externally obtained
    // file. Compiled in, so this cannot become false by someone moving a
    // directory.
    #[test]
    fn practice_material_ships_with_the_application() {
        assert!(!PROVIDED.is_empty());

        for piece in PROVIDED {
            assert!(!piece.source.is_empty(), "{} is empty", piece.name);
        }
    }

    // Every provided file must actually be MusicXML, or the Library would be
    // empty at the first run despite the requirement — and the failure would appear on
    // a performer's screen rather than in a test.
    #[test]
    fn every_provided_piece_is_musicxml() {
        for piece in PROVIDED {
            let text = std::str::from_utf8(piece.source).expect("provided pieces are text");

            assert!(
                text.contains("score-partwise"),
                "{} is not MusicXML",
                piece.name
            );
        }
    }

    // the same file is the same piece, so re-importing one does
    // not accumulate duplicates.
    #[test]
    fn the_same_bytes_always_name_the_same_piece() {
        assert_eq!(
            identify(b"<score-partwise/>"),
            identify(b"<score-partwise/>")
        );
    }

    #[test]
    fn different_music_is_a_different_piece() {
        assert_ne!(
            identify(b"<score-partwise/>"),
            identify(b"<score-timewise/>")
        );
    }

    // The identifier becomes a filename, and PieceId refuses anything
    // path-shaped. Hexadecimal always satisfies that; the test is here so a
    // future change of encoding has to notice.
    #[test]
    fn an_identifier_is_usable_as_a_name() {
        for piece in PROVIDED {
            let id = identify(piece.source);

            assert_eq!(id.as_str().len(), 16);
            assert!(id.as_str().chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    // Distinct pieces, distinct names. A collision here would
    // silently merge two pieces in the Library.
    #[test]
    fn the_provided_pieces_are_all_distinct() {
        let mut names: Vec<String> = PROVIDED
            .iter()
            .map(|piece| identify(piece.source).as_str().to_owned())
            .collect();

        names.sort();
        names.dedup();

        assert_eq!(names.len(), PROVIDED.len());
    }
}
