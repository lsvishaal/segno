//! What the application holds while it is running.
//!
//! # One lock, at the boundary
//!
//! Tauri runs commands on a pool of threads, so the state they share needs a
//! `Mutex`. `ADR-0009` said the domain would be single-threaded and lock-free,
//! and it still is: the lock is around the *application's* state, taken and
//! released inside one command, and nothing behind it knows it exists.
//!
//! The distinction matters because it is the one that decision was about. A
//! lock inside `hogwash-practice` would mean every test of the cursor had to
//! think about contention. A lock here means one command at a time touches
//! the session, which is exactly the "one owner" the ADR asked for — arrived
//! at by a different route, since a webview has no frame loop to be the owner.

use std::collections::BTreeSet;
use std::sync::Mutex;

use hogwash_domain::PieceId;
use hogwash_engraving_verovio::Layout;
use hogwash_library::{Library, Piece, title_from};
use hogwash_records::Records;

use crate::engraving::Engraving;
use crate::practice::Practice;
use crate::provided::{PROVIDED, identify};
use crate::startup::Unavailable;

/// Everything the application needs while it is running.
#[derive(Debug)]
pub struct Hogwash {
    engraving: Engraving,
    library: Library,
    records: Records,
    practice: Practice,
}

impl Hogwash {
    /// Engraves the provided material and opens the performer's records.
    ///
    /// # Why the provided pieces are engraved at startup
    ///
    /// Because a Piece needs a Score, and a Score comes from the engraver
    /// (`004 §6.5` — one parse, so a note and the glyph drawn for it cannot
    /// drift apart). Three short pieces take milliseconds.
    ///
    /// # Errors
    ///
    /// [`Unavailable`] if **every** provided piece fails to engrave. One that
    /// fails is skipped and reported, because losing a piece is a smaller
    /// harm than refusing to start; losing all of them means the engraver is
    /// not working, and FR-207 cannot be satisfied by pretending otherwise.
    pub fn start(
        engraving: Engraving,
        records: Records,
        practice: Practice,
    ) -> Result<Self, Unavailable> {
        let mut pieces = Vec::new();
        let mut refused = Vec::new();

        for provided in PROVIDED {
            match engraving.engrave(provided.source, Layout::default()) {
                Ok(engraved) => {
                    let details = engraved.details();

                    pieces.push(Piece::new(
                        identify(provided.source),
                        details.title.as_deref().unwrap_or_default(),
                        details.composer.clone(),
                        engraved.score().clone(),
                    ));
                }
                Err(failure) => refused.push(format!("{}: {failure:?}", provided.name)),
            }
        }

        // What the performer imported in earlier runs, read back from disk,
        // with identical copies collapsed first. A saved file that no longer
        // reads costs only that piece.
        let stored: Vec<(PieceId, Vec<u8>)> = records
            .imported()
            .into_iter()
            .filter_map(|id| records.source_of(&id).map(|source| (id, source)))
            .collect();
        let practised: BTreeSet<PieceId> = records
            .sessions()
            .map(|(sessions, _)| sessions.into_iter().map(|session| session.piece).collect())
            .unwrap_or_default();
        let bundled: BTreeSet<PieceId> = pieces.iter().map(|piece| piece.id().clone()).collect();
        let (kept, surplus) = collapse(
            stored
                .iter()
                .map(|(id, source)| (id.clone(), identify(source))),
            &bundled,
            &practised,
        );

        for id in surplus {
            if let Err(failure) = records.discard_score(&id) {
                eprintln!(
                    "hogwash: a duplicate of {} could not be removed: {failure:?}",
                    id.as_str()
                );
            }
        }

        for (id, source) in stored.into_iter().filter(|(id, _)| kept.contains(id)) {
            match engraving.engrave(&source, Layout::default()) {
                Ok(engraved) => {
                    let details = engraved.details();

                    // A title the performer chose wins; otherwise the file's
                    // own, then its name.
                    let record = records.piece_record(&id);
                    let title = match &record {
                        Some(record) if record.renamed => record.title.clone(),
                        _ => title_from(
                            details.title.as_deref(),
                            record
                                .as_ref()
                                .and_then(|record| record.file_name.as_deref()),
                        ),
                    };

                    pieces.push(Piece::new(
                        id,
                        &title,
                        details.composer.clone(),
                        engraved.score().clone(),
                    ));
                }
                Err(failure) => {
                    eprintln!(
                        "hogwash: imported piece {} could not be read: {failure:?}",
                        id.as_str()
                    );
                }
            }
        }

        let mut pieces = pieces.into_iter();

        // FR-207 expressed as a signature: `opened_with` needs a first Piece
        // by value, so a Library that exists is a Library with music in it.
        let Some(first) = pieces.next() else {
            return Err(Unavailable {
                summary: "No practice material could be prepared.".to_owned(),
                detail: "Hogwash ships with music to practise, and none of it could be \
                         read. That points at the engraving engine rather than at the \
                         music, since these files travel with the application."
                    .to_owned(),
                looked_in: refused,
            });
        };

        Ok(Self {
            engraving,
            library: Library::opened_with(first, pieces.collect()),
            records,
            practice,
        })
    }

    /// The Pieces available to practise. Never empty (FR-207).
    #[must_use]
    pub const fn library(&self) -> &Library {
        &self.library
    }

    /// Joins an imported Piece to the session's Library.
    ///
    /// `Library::with` consumes and returns rather than mutating in place
    /// (`004 §8.1`), so an addition here is a clone-and-replace rather than a
    /// push. `Library` is cheap enough — one imported Piece is a title, a
    /// Score and a Reference Performance, and import is a performer clicking
    /// a button, not a hot path — that paying for the clone is the honest
    /// cost of that design, not a reason to work around it.
    pub fn accept_piece(&mut self, piece: Piece) {
        self.library = self.library.clone().with(piece);
    }

    /// Takes an imported Piece out of the session's Library.
    pub fn remove_piece(&mut self, id: &PieceId) {
        self.library = self.library.clone().without(id);
    }

    /// Gives a Piece in the session's Library a new title.
    pub fn retitle_piece(&mut self, id: &PieceId, title: &str) {
        self.library = self.library.clone().retitled(id, title);
    }

    /// The engraver, for reading a score the performer offers.
    #[must_use]
    pub const fn engraving(&self) -> &Engraving {
        &self.engraving
    }

    /// The music a Piece was read from.
    ///
    /// Provided pieces are compiled in; imported ones are on disk. Looking in
    /// both here means nothing above has to know which kind it is holding —
    /// FR-200 says a Piece behaves the same whatever it came from, and this
    /// is where that stays true.
    #[must_use]
    pub fn source_of(&self, id: &PieceId) -> Option<Vec<u8>> {
        PROVIDED
            .iter()
            .find(|provided| identify(provided.source) == *id)
            .map(|provided| provided.source.to_vec())
            .or_else(|| self.records.source_of(id))
    }

    /// Where practice history is kept.
    #[must_use]
    pub const fn records(&self) -> &Records {
        &self.records
    }

    /// The practice loop: MIDI in, the Cursor advanced, an attempt ended.
    #[must_use]
    pub const fn practice(&self) -> &Practice {
        &self.practice
    }
}

/// The application state, as Tauri hands it to a command.
///
/// A `Mutex` rather than a `RwLock`: reads and writes are both rare — a
/// performer clicks a few times a minute — and a `Mutex` has one way to be
/// used wrongly where a `RwLock` has several.
#[derive(Debug)]
pub struct Shared(pub Mutex<Hogwash>);

/// The application, when it could not start.
///
/// Managed *instead of* [`Shared`], so a command that needs a working
/// Hogwash simply does not find one. The window still opens: a startup
/// failure that closed the program would have no way to say what went wrong
/// (`004 §8.2`).
#[derive(Debug)]
pub struct Failed(pub Unavailable);

/// Which stored imports to list and which are surplus copies, given each
/// one's stored name and the fingerprint of its bytes.
///
/// A stored file is normally named by its fingerprint, so copies cannot
/// happen, but a file copied into the data folder by hand, or a piece
/// imported before it was bundled, breaks that. Among identical copies every
/// one with practice history is kept (history is never lost to tidying);
/// if none has any, one is kept, unless a bundled piece already is that
/// music, in which case none is needed.
fn collapse(
    stored: impl Iterator<Item = (PieceId, PieceId)>,
    bundled: &BTreeSet<PieceId>,
    practised: &BTreeSet<PieceId>,
) -> (BTreeSet<PieceId>, Vec<PieceId>) {
    let mut groups: std::collections::BTreeMap<PieceId, Vec<PieceId>> =
        std::collections::BTreeMap::new();
    for (name, fingerprint) in stored {
        groups.entry(fingerprint).or_default().push(name);
    }

    let mut kept = BTreeSet::new();
    let mut surplus = Vec::new();

    for (fingerprint, mut copies) in groups {
        // The copy named by its own fingerprint is the canonical one.
        copies.sort_by_key(|name| *name != fingerprint);
        // A copy under a bundled piece's own name shares that piece's history
        // already, so it is never needed on the shelf a second time.
        let with_history: Vec<PieceId> = copies
            .iter()
            .filter(|name| practised.contains(*name) && !bundled.contains(*name))
            .cloned()
            .collect();

        let keep: Vec<PieceId> = if !with_history.is_empty() {
            with_history
        } else if bundled.contains(&fingerprint) {
            Vec::new()
        } else {
            copies.first().cloned().into_iter().collect()
        };

        for name in copies {
            if keep.contains(&name) {
                kept.insert(name);
            } else {
                surplus.push(name);
            }
        }
    }

    (kept, surplus)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use hogwash_domain::PieceId;

    use super::collapse;

    fn id(name: &str) -> PieceId {
        PieceId::new(name).expect("usable")
    }

    fn set(names: &[&str]) -> BTreeSet<PieceId> {
        names.iter().map(|name| id(name)).collect()
    }

    #[test]
    fn distinct_imports_are_all_kept() {
        let (kept, surplus) = collapse(
            [(id("aa"), id("aa")), (id("bb"), id("bb"))].into_iter(),
            &set(&[]),
            &set(&[]),
        );
        assert_eq!(kept, set(&["aa", "bb"]));
        assert!(surplus.is_empty());
    }

    #[test]
    fn identical_copies_without_history_collapse_to_the_canonical_one() {
        let (kept, surplus) = collapse(
            [(id("copy"), id("aa")), (id("aa"), id("aa"))].into_iter(),
            &set(&[]),
            &set(&[]),
        );
        assert_eq!(kept, set(&["aa"]));
        assert_eq!(surplus, [id("copy")]);
    }

    #[test]
    fn the_copy_with_history_is_the_one_kept() {
        let (kept, surplus) = collapse(
            [(id("copy"), id("aa")), (id("aa"), id("aa"))].into_iter(),
            &set(&[]),
            &set(&["copy"]),
        );
        assert_eq!(kept, set(&["copy"]));
        assert_eq!(surplus, [id("aa")]);
    }

    #[test]
    fn every_copy_with_history_is_kept() {
        let (kept, surplus) = collapse(
            [(id("copy"), id("aa")), (id("aa"), id("aa"))].into_iter(),
            &set(&[]),
            &set(&["copy", "aa"]),
        );
        assert_eq!(kept, set(&["aa", "copy"]));
        assert!(surplus.is_empty());
    }

    #[test]
    fn a_copy_under_a_bundled_name_is_surplus_even_with_history() {
        let (kept, surplus) = collapse(
            [(id("minuet"), id("minuet"))].into_iter(),
            &set(&["minuet"]),
            &set(&["minuet"]),
        );
        assert!(kept.is_empty());
        assert_eq!(surplus, [id("minuet")]);
    }

    #[test]
    fn a_copy_of_a_bundled_piece_is_not_listed_twice() {
        let (kept, surplus) = collapse(
            [(id("minuet"), id("minuet"))].into_iter(),
            &set(&["minuet"]),
            &set(&[]),
        );
        assert!(kept.is_empty());
        assert_eq!(surplus, [id("minuet")]);
    }
}
