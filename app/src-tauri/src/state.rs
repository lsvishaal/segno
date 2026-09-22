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

use std::sync::Mutex;

use hogwash_domain::PieceId;
use hogwash_engraving_verovio::Layout;
use hogwash_library::{Library, Piece};
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
