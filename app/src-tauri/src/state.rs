//! What the application holds while it is running.
//!
//! # One lock, at the boundary
//!
//! Tauri runs commands on a pool of threads, so the state they share needs a
//! `Mutex`. The design said the domain would be single-threaded and lock-free,
//! and it still is: the lock is around the *application's* state, taken and
//! released inside one command, and nothing behind it knows it exists.
//!
//! The distinction matters because it is the one that decision was about. A
//! lock inside `segno-practice` would mean every test of the cursor had to
//! think about contention. A lock here means one command at a time touches
//! the session, which is exactly the "one owner" the ADR asked for — arrived
//! at by a different route, since a webview has no frame loop to be the owner.

use std::sync::Mutex;

use segno_domain::PieceId;
use segno_engraving_verovio::Layout;
use segno_library::{Library, Piece, title_from};
use segno_records::Records;

use crate::engraving::Engraving;
use crate::practice::Practice;
use crate::provided::PROVIDED;
use crate::startup::Unavailable;

/// Everything the application needs while it is running.
#[derive(Debug)]
pub struct Segno {
    engraving: Engraving,
    library: Library,
    records: Records,
    practice: Practice,
}

impl Segno {
    /// Engraves the provided material and opens the performer's records.
    ///
    /// # Why the provided pieces are engraved at startup
    ///
    /// Because a Piece needs a Score, and a Score comes from the engraver
    /// (the design — one parse, so a note and the glyph drawn for it cannot
    /// drift apart). Three short pieces take milliseconds.
    ///
    /// # Errors
    ///
    /// [`Unavailable`] if **every** provided piece fails to engrave. One that
    /// fails is skipped and reported, because losing a piece is a smaller
    /// harm than refusing to start; losing all of them means the engraver is
    /// not working, and the requirement cannot be satisfied by pretending otherwise.
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
                        provided.id(),
                        details.title.as_deref().unwrap_or_default(),
                        details.composer.clone(),
                        engraved.score().clone(),
                    ));
                }
                Err(failure) => refused.push(format!("{}: {failure:?}", provided.name)),
            }
        }

        // What the performer imported in earlier runs, read back from disk.
        // A saved file that no longer reads costs only that piece. Nothing is
        // removed here: a second copy is caught at import, and a startup
        // that deleted files could lose one.
        let stored: Vec<(PieceId, Vec<u8>)> = records
            .imported()
            .into_iter()
            .filter_map(|id| records.source_of(&id).map(|source| (id, source)))
            .collect();
        // A session file that does not read costs only itself: the rest of
        // the history is used, and why that one was skipped is said.
        let (_, unreadable) = records.sessions().unwrap_or_default();
        for line in skipped(&unreadable) {
            crate::log::warn(&line);
        }

        for (id, source) in stored {
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
                    crate::log::warn(&format!(
                        "imported piece {} could not be read: {failure:?}",
                        id.as_str()
                    ));
                }
            }
        }

        let mut pieces = pieces.into_iter();

        // The requirement expressed as a signature: `opened_with` needs a first Piece
        // by value, so a Library that exists is a Library with music in it.
        let Some(first) = pieces.next() else {
            return Err(Unavailable {
                summary: "No practice material could be prepared.".to_owned(),
                detail: "Segno ships with music to practise, and none of it could be \
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

    /// The Pieces available to practise. Never empty.
    #[must_use]
    pub const fn library(&self) -> &Library {
        &self.library
    }

    /// Joins an imported Piece to the session's Library.
    ///
    /// `Library::with` consumes and returns rather than mutating in place
    ///, so an addition here is a clone-and-replace rather than a
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
    /// The requirement says a Piece behaves the same whatever it came from, and this
    /// is where that stays true.
    #[must_use]
    pub fn source_of(&self, id: &PieceId) -> Option<Vec<u8>> {
        crate::provided::provided(id)
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
pub struct Shared(pub Mutex<Segno>);

/// The application, when it could not start.
///
/// Managed *instead of* [`Shared`], so a command that needs a working
/// Segno simply does not find one. The window still opens: a startup
/// failure that closed the program would have no way to say what went wrong.
#[derive(Debug)]
pub struct Failed(pub Unavailable);

/// One line per practice record that could not be read, saying which file
/// and why. Written to the log; the file is left where it is.
fn skipped(unreadable: &[segno_records::RecordsFailure]) -> Vec<String> {
    unreadable
        .iter()
        .map(|failure| match failure {
            segno_records::RecordsFailure::Unreadable { path, detail } => {
                format!("skipped the practice record {}: {detail}", path.display())
            }
            segno_records::RecordsFailure::FromTheFuture { path, schema } => format!(
                "skipped the practice record {}: written by a newer Segno (schema {schema})",
                path.display()
            ),
            segno_records::RecordsFailure::Storage(failure) => {
                format!("a practice record could not be read: {failure}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::skipped;

    // One session file with an ending this version does not know: the app
    // still starts, the other session still counts, and the skipped one is
    // named with the reason.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_session_with_an_unknown_ending_is_skipped_and_the_rest_still_count() {
        use segno_records::{AttemptRecord, Ended, Records, SessionRecord};

        let directory = std::env::temp_dir().join("segno-state-unknown-ending");
        drop(std::fs::remove_dir_all(&directory));
        let records = Records::under(&directory);
        let minuet = crate::provided::PROVIDED
            .iter()
            .find(|piece| piece.name == "minuet-in-g.musicxml")
            .map(crate::provided::Provided::id)
            .expect("the Minuet is provided");
        let mut played = segno_capture::ObservedPerformance::new();
        played.record(segno_domain::Pitch::new(74).expect("D5"));
        let good = SessionRecord {
            id: format!("{}-1", minuet.as_str()),
            piece: minuet.clone(),
            started: "1".to_owned(),
            closed: false,
            attempts: vec![AttemptRecord {
                began_ms: None,
                from: 0,
                to: None,
                played,
                reached: 0,
                ended: Some(Ended::Stop),
            }],
        };
        records.keep(&good).expect("keeps");
        let history = directory.join("history");
        let text = std::fs::read_to_string(history.join(format!("{}-1.json", minuet.as_str())))
            .expect("reads");
        std::fs::write(
            history.join(format!("{}-2.json", minuet.as_str())),
            text.replace("-1\"", "-2\"")
                .replace("\"stop\"", "\"teleported\""),
        )
        .expect("writes");

        let engraving = crate::engraving::Engraving::start().expect("the engine starts");
        let practice =
            crate::practice::Practice::start(records.clone(), |_| {}).expect("the thread starts");
        let started = super::Segno::start(engraving, records.clone(), practice);

        assert!(started.is_ok(), "the app did not start");
        assert_eq!(records.sessions_of(&minuet), 1);
        let (_, unreadable) = records.sessions().expect("history reads");
        let lines = skipped(&unreadable);
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].contains("-2.json") && lines[0].contains("teleported"),
            "{}",
            lines[0]
        );
    }
}
