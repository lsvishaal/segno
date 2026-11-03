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

use segno_domain::PieceId;

/// One piece that ships with Segno.
#[derive(Debug, Clone, Copy)]
pub struct Provided {
    /// Its permanent name, and so its [`PieceId`]: fixed for good, whatever
    /// later happens to the file's bytes (DECISIONS, 3 Nov). Never reuse or
    /// rename one; history is filed under it.
    pub tag: &'static str,
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
    provided(id).map(|provided| provided.level)
}

/// The provided piece called `id`, if it is one.
#[must_use]
pub fn provided(id: &PieceId) -> Option<&'static Provided> {
    PROVIDED.iter().find(|provided| provided.tag == id.as_str())
}

impl Provided {
    /// Its permanent [`PieceId`].
    #[must_use]
    pub fn id(&self) -> PieceId {
        PieceId::new(self.tag).unwrap_or_else(|| unreachable!("tags are tested to be usable names"))
    }
}

/// Everything in `library/`, in the order a first-time performer meets it.
///
/// Ordered deliberately: the shortest and simplest first, then the first
/// piece with a second staff, then two real published pieces (a Minuet with
/// repeats, a Prelude full of held notes). `library/PROVENANCE.md` records
/// where each one came from and under what licence.
pub const PROVIDED: [Provided; 5] = [
    Provided {
        tag: "lightly-row",
        name: "lightly-row.musicxml",
        source: include_bytes!("../library/lightly-row.musicxml"),
        level: Level::Beginner,
    },
    Provided {
        tag: "ode-to-joy",
        name: "ode-to-joy.musicxml",
        source: include_bytes!("../library/ode-to-joy.musicxml"),
        level: Level::Beginner,
    },
    Provided {
        tag: "fur-elise",
        name: "fur-elise.musicxml",
        source: include_bytes!("../library/fur-elise.musicxml"),
        level: Level::Beginner,
    },
    Provided {
        tag: "minuet-in-g",
        name: "minuet-in-g.musicxml",
        source: include_bytes!("../library/minuet-in-g.musicxml"),
        level: Level::Intermediate,
    },
    Provided {
        tag: "prelude-in-c",
        name: "prelude-in-c.musicxml",
        source: include_bytes!("../library/prelude-in-c.musicxml"),
        level: Level::Intermediate,
    },
];

/// The fingerprint of these bytes: the first 16 hex characters of their
/// SHA-256. Names a *version* of a score and spots a duplicate import. It is
/// not a provided piece's identity (that is its `tag`), because then any edit
/// to a score would orphan its history (DECISIONS, 3 Nov).
#[must_use]
pub fn identify(source: &[u8]) -> PieceId {
    // Hexadecimal is always a usable name; the fallback keeps this total.
    PieceId::new(&segno_records::fingerprint(source)).unwrap_or_else(|| {
        PieceId::new("unnamed").unwrap_or_else(|| unreachable!("`unnamed` is a usable name"))
    })
}

#[cfg(test)]
mod tests {
    use segno_domain::PieceId;

    use crate::provided::{Level, PROVIDED, identify, level_of, provided};

    #[test]
    fn each_provided_piece_is_on_a_shelf_and_imports_are_not() {
        for piece in PROVIDED {
            assert_eq!(level_of(&piece.id()), Some(piece.level), "{}", piece.name);
        }
        assert_eq!(level_of(&identify(b"<score-partwise/>")), None);
        assert_eq!(level_of(&identify(PROVIDED[0].source)), None);
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

    // A tag becomes a filename, and PieceId refuses anything path-shaped.
    #[test]
    fn every_tag_is_a_usable_name() {
        for piece in PROVIDED {
            assert_eq!(
                PieceId::new(piece.tag).map(|id| id.as_str().to_owned()),
                Some(piece.tag.to_owned())
            );
        }
    }

    // Distinct pieces, distinct names. A collision here would silently merge
    // two pieces' history.
    #[test]
    fn the_provided_pieces_are_all_distinct() {
        let mut tags: Vec<&str> = PROVIDED.iter().map(|piece| piece.tag).collect();
        tags.sort_unstable();
        tags.dedup();
        assert_eq!(tags.len(), PROVIDED.len());

        let mut prints: Vec<PieceId> = PROVIDED
            .iter()
            .map(|piece| identify(piece.source))
            .collect();
        prints.sort();
        prints.dedup();
        assert_eq!(prints.len(), PROVIDED.len());
    }

    // The point of a tag: changing one byte of a bundled score changes its
    // fingerprint but not which piece it is.
    #[test]
    fn editing_a_bundled_score_keeps_its_name() {
        let minuet = provided(&PieceId::new("minuet-in-g").expect("a name")).expect("bundled");
        let mut edited = minuet.source.to_vec();
        let at = edited
            .iter()
            .position(|byte| *byte == b'D')
            .expect("a D somewhere");
        edited[at] = b'E';

        assert_ne!(identify(&edited), identify(minuet.source));
        assert_eq!(minuet.id().as_str(), "minuet-in-g");
    }

    // These are permanent: history is filed under them.
    #[test]
    fn the_tags_never_change() {
        let tags: Vec<&str> = PROVIDED.iter().map(|piece| piece.tag).collect();
        assert_eq!(
            tags,
            [
                "lightly-row",
                "ode-to-joy",
                "fur-elise",
                "minuet-in-g",
                "prelude-in-c"
            ]
        );
    }
}
