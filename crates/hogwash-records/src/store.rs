//! Keeping practice history, and the scores it refers to (`004 §8.9`).

use std::fs;
use std::path::{Path, PathBuf};

use hogwash_capture::ObservedPerformance;
use hogwash_domain::{PieceId, Pitch};

use crate::atomic;
use crate::stored::{SCHEMA, StoredAttempt, StoredPiece, StoredSession, read_id, stored_pitch};

/// One session's worth of history, in the terms the application deals in.
///
/// Deliberately not `hogwash_practice::PracticeSession`. That type carries a
/// borrowed Reference Performance and a typestate lifecycle, neither of which
/// survives a restart; this is what is *kept*, which is a smaller thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    /// What this session is called.
    pub id: String,
    /// Which Piece was practised (`003 §7.2` — exactly one).
    pub piece: PieceId,
    /// When it began, as the caller reported it.
    ///
    /// Supplied, never read from a clock. `004 §6.7` keeps the clock out of
    /// the domain, and storage is not the place to smuggle one back in: a
    /// store that timestamped its own writes would make two runs of the same
    /// test produce different records.
    pub started: String,
    /// Whether the performer closed it, or simply stopped (FR-702).
    pub closed: bool,
    /// Every attempt, in the order they were made.
    pub attempts: Vec<AttemptRecord>,
}

/// One attempt, in the terms the application deals in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptRecord {
    /// What the performer played, in order.
    pub played: ObservedPerformance,
    /// How far they got.
    pub reached: usize,
}

/// A piece kept on disk, and where its score is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PieceRecord {
    /// Its durable name.
    pub id: PieceId,
    /// What to call it (FR-201).
    pub title: String,
    /// Who wrote it, where the source said.
    pub composer: Option<String>,
    /// The score file, beside the record.
    pub source: PathBuf,
    /// What the performer's own file was called, where known.
    pub file_name: Option<String>,
    /// Whether the performer renamed it: `title` is then theirs.
    pub renamed: bool,
}

/// Why a record could not be kept or read back.
#[derive(Debug)]
pub enum RecordsFailure {
    /// The filesystem said no.
    Storage(std::io::Error),

    /// A record exists and could not be understood.
    ///
    /// Distinct from a missing one: a record that is *there* and unreadable
    /// is a performer's history that has gone wrong, and telling them the
    /// difference is the least that can be done about it.
    Unreadable {
        /// Which record.
        path: PathBuf,
        /// What was wrong with it.
        detail: String,
    },

    /// A record written by a newer version of Hogwash.
    ///
    /// Refused rather than read as best it can be: a half-understood record
    /// reported as understood tells a performer the wrong thing about their
    /// own practice, which is worse than telling them it cannot be opened.
    FromTheFuture {
        /// Which record.
        path: PathBuf,
        /// The shape it claims.
        schema: u32,
    },
}

impl From<std::io::Error> for RecordsFailure {
    fn from(failure: std::io::Error) -> Self {
        Self::Storage(failure)
    }
}

/// Where everything is kept.
///
/// # Two directories, because FR-703 asks for two
///
/// *"A clear separation between imported piece content and user-generated
/// data such as practice history."* It is a real separation and not a tidiness
/// preference: imported scores are the performer's own material, held
/// read-only (FR-701) and replaceable from wherever they came from, while
/// history is the thing that cannot be reconstructed if it is lost. Backing
/// up one is not the same job as backing up the other.
#[derive(Debug, Clone)]
pub struct Records {
    root: PathBuf,
}

impl Records {
    /// Records kept under `root`.
    ///
    /// The path is given rather than discovered. Where an installed
    /// application puts its data is a packaging question, and a library that
    /// decided for itself could not be tested twice over without the two runs
    /// meeting.
    #[must_use]
    pub fn under(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// Where practice history lives.
    #[must_use]
    pub fn history(&self) -> PathBuf {
        self.root.join("history")
    }

    /// Where imported scores live (FR-701, FR-703).
    #[must_use]
    pub fn scores(&self) -> PathBuf {
        self.root.join("scores")
    }

    /// Keeps `session`, replacing any earlier version of it (FR-700).
    ///
    /// # Errors
    ///
    /// [`RecordsFailure::Storage`] if it could not be written. The previous
    /// version is untouched in that case — see [`crate::atomic`].
    pub fn keep(&self, session: &SessionRecord) -> Result<(), RecordsFailure> {
        let stored = StoredSession {
            schema: SCHEMA,
            id: session.id.clone(),
            piece: session.piece.as_str().to_owned(),
            started: session.started.clone(),
            closed: session.closed,
            attempts: session
                .attempts
                .iter()
                .map(|attempt| StoredAttempt {
                    played: attempt
                        .played
                        .played()
                        .iter()
                        .copied()
                        .map(stored_pitch)
                        .collect(),
                    reached: attempt.reached,
                })
                .collect(),
        };

        // Pretty-printed. A practice record is a few kilobytes and a
        // performer's own history; being able to read it with `cat` is worth
        // more than the bytes it costs, and `001 §7` is local-first, which
        // means the files are theirs.
        let written = serde_json::to_string_pretty(&stored).map_err(|failure| {
            RecordsFailure::Unreadable {
                path: self.session_path(&session.id),
                detail: failure.to_string(),
            }
        })?;

        atomic::write(&self.session_path(&session.id), &written)?;

        Ok(())
    }

    /// Every session kept, oldest name first (FR-600, FR-702).
    ///
    /// A record that cannot be read is **skipped, not fatal**. One corrupt
    /// file must not make the rest of a performer's history unreachable —
    /// `004 §8.9`'s failure boundary is precisely that prior records remain
    /// intact — so the unreadable ones are returned alongside rather than
    /// instead.
    ///
    /// # Errors
    ///
    /// [`RecordsFailure::Storage`] only if the directory itself cannot be
    /// read. A missing directory is not an error: it means nothing has been
    /// practised yet.
    pub fn sessions(&self) -> Result<(Vec<SessionRecord>, Vec<RecordsFailure>), RecordsFailure> {
        let directory = self.history();

        if !directory.exists() {
            return Ok((Vec::new(), Vec::new()));
        }

        let mut paths: Vec<PathBuf> = fs::read_dir(&directory)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|kind| kind == "json"))
            .collect();

        // Sorted, so two runs list them the same way (FR-406 in spirit: a
        // directory's order is the filesystem's business and no one else's).
        paths.sort();

        let mut sessions = Vec::new();
        let mut unreadable = Vec::new();

        for path in paths {
            match Self::read_session(&path) {
                Ok(session) => sessions.push(session),
                Err(failure) => unreadable.push(failure),
            }
        }

        Ok((sessions, unreadable))
    }

    /// Keeps an imported score and a record of it (FR-701, FR-703).
    ///
    /// The score is copied into the library, not referred to where it lies.
    /// A performer who tidies their downloads folder must not thereby lose a
    /// piece they have been practising for a month.
    ///
    /// # Errors
    ///
    /// [`RecordsFailure::Storage`] if either could not be written.
    pub fn keep_piece(
        &self,
        id: &PieceId,
        title: &str,
        composer: Option<&str>,
        file_name: Option<&str>,
        source: &[u8],
    ) -> Result<PieceRecord, RecordsFailure> {
        let score = self.scores().join(format!("{}.musicxml", id.as_str()));

        fs::create_dir_all(self.scores())?;
        fs::write(&score, source)?;

        // FR-701 — "in read-only form". Advisory rather than absolute; a
        // performer with a text editor can still change it. What it prevents
        // is Hogwash doing so by accident, which is the case FR-202 is about.
        if let Ok(metadata) = fs::metadata(&score) {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(true);
            drop(fs::set_permissions(&score, permissions));
        }

        let record = PieceRecord {
            id: id.clone(),
            title: title.to_owned(),
            composer: composer.map(str::to_owned),
            source: score,
            file_name: file_name.map(str::to_owned),
            renamed: false,
        };

        let stored = StoredPiece {
            schema: SCHEMA,
            id: record.id.as_str().to_owned(),
            title: record.title.clone(),
            composer: record.composer.clone(),
            source: record
                .source
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            file_name: record.file_name.clone(),
            renamed: false,
        };

        let written = serde_json::to_string_pretty(&stored).map_err(|failure| {
            RecordsFailure::Unreadable {
                path: self.piece_path(id),
                detail: failure.to_string(),
            }
        })?;

        atomic::write(&self.piece_path(id), &written)?;

        Ok(record)
    }

    /// What was recorded about an imported piece, if anything was.
    #[must_use]
    pub fn piece_record(&self, id: &PieceId) -> Option<PieceRecord> {
        let stored: StoredPiece =
            serde_json::from_str(&fs::read_to_string(self.piece_path(id)).ok()?).ok()?;

        Some(PieceRecord {
            id: id.clone(),
            title: stored.title,
            composer: stored.composer,
            source: self.scores().join(stored.source),
            file_name: stored.file_name,
            renamed: stored.renamed,
        })
    }

    /// Gives an imported piece the title the performer chose, kept across
    /// restarts.
    ///
    /// # Errors
    ///
    /// [`RecordsFailure::Unreadable`] if there is no record of that piece
    /// (bundled pieces have none), or [`RecordsFailure::Storage`] if it
    /// could not be written.
    pub fn rename_piece(&self, id: &PieceId, title: &str) -> Result<(), RecordsFailure> {
        let path = self.piece_path(id);
        let unreadable = |detail: String| RecordsFailure::Unreadable {
            path: path.clone(),
            detail,
        };

        let mut stored: StoredPiece = serde_json::from_str(&fs::read_to_string(&path)?)
            .map_err(|failure| unreadable(failure.to_string()))?;
        title.clone_into(&mut stored.title);
        stored.renamed = true;

        let written = serde_json::to_string_pretty(&stored)
            .map_err(|failure| unreadable(failure.to_string()))?;
        atomic::write(&path, &written)?;
        Ok(())
    }

    /// Every Piece imported so far, by name, in a stable order: what the
    /// Library has to be rebuilt with after a restart. An empty or missing
    /// folder is simply nothing imported yet.
    #[must_use]
    pub fn imported(&self) -> Vec<PieceId> {
        let Ok(entries) = fs::read_dir(self.scores()) else {
            return Vec::new();
        };

        let mut ids: Vec<PieceId> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "musicxml")
            })
            .filter_map(|path| PieceId::new(path.file_stem()?.to_str()?))
            .collect();
        ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        ids
    }

    /// Removes an imported score and its record, nothing else: practice
    /// history is kept. Missing files are not an error.
    ///
    /// # Errors
    ///
    /// [`RecordsFailure::Storage`] if a file that is there could not be removed.
    pub fn discard_score(&self, id: &PieceId) -> Result<(), RecordsFailure> {
        for path in [
            self.scores().join(format!("{}.musicxml", id.as_str())),
            self.piece_path(id),
        ] {
            // Scores are kept read-only; that guards against edits, not removal.
            if let Ok(metadata) = fs::metadata(&path) {
                let mut permissions = metadata.permissions();
                #[expect(
                    clippy::permissions_set_readonly_false,
                    reason = "about to delete the file, on the performer's behalf"
                )]
                permissions.set_readonly(false);
                drop(fs::set_permissions(&path, permissions));
            }
            match fs::remove_file(&path) {
                Err(failure) if failure.kind() != std::io::ErrorKind::NotFound => {
                    return Err(failure.into());
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// How many practice sessions are kept for a piece.
    #[must_use]
    pub fn sessions_of(&self, id: &PieceId) -> usize {
        self.sessions()
            .map(|(sessions, _)| {
                sessions
                    .iter()
                    .filter(|session| session.piece == *id)
                    .count()
            })
            .unwrap_or_default()
    }

    /// Forgets an imported piece entirely: its score, its record, and every
    /// practice session kept for it. Returns how many sessions went.
    ///
    /// # Errors
    ///
    /// [`RecordsFailure::Storage`] if a file could not be removed; anything
    /// removed before that stays removed.
    pub fn forget_piece(&self, id: &PieceId) -> Result<usize, RecordsFailure> {
        let (sessions, _) = self.sessions()?;
        let mut forgotten = 0;

        for session in sessions.iter().filter(|session| session.piece == *id) {
            fs::remove_file(self.session_path(&session.id))?;
            forgotten += 1;
        }

        self.discard_score(id)?;
        Ok(forgotten)
    }

    /// The score a Piece was imported from, if it is here (FR-701).
    ///
    /// Returns the bytes rather than the path, so a caller cannot be tempted
    /// to hold a path across a restart — `ADR-0010` copies imported material
    /// into the library precisely so that the performer's own file may move.
    #[must_use]
    pub fn source_of(&self, id: &PieceId) -> Option<Vec<u8>> {
        fs::read(self.scores().join(format!("{}.musicxml", id.as_str()))).ok()
    }

    fn session_path(&self, id: &str) -> PathBuf {
        self.history().join(format!("{id}.json"))
    }

    fn piece_path(&self, id: &PieceId) -> PathBuf {
        self.scores().join(format!("{}.json", id.as_str()))
    }

    fn read_session(path: &Path) -> Result<SessionRecord, RecordsFailure> {
        let text = fs::read_to_string(path)?;

        let stored: StoredSession =
            serde_json::from_str(&text).map_err(|failure| RecordsFailure::Unreadable {
                path: path.to_path_buf(),
                detail: failure.to_string(),
            })?;

        if stored.schema > SCHEMA {
            return Err(RecordsFailure::FromTheFuture {
                path: path.to_path_buf(),
                schema: stored.schema,
            });
        }

        let piece = read_id(&stored.piece).ok_or_else(|| RecordsFailure::Unreadable {
            path: path.to_path_buf(),
            detail: format!("`{}` is not a piece name", stored.piece),
        })?;

        let mut attempts = Vec::new();

        for attempt in stored.attempts {
            let mut played = ObservedPerformance::new();

            for note in attempt.played {
                let pitch = Pitch::new(note).ok_or_else(|| RecordsFailure::Unreadable {
                    path: path.to_path_buf(),
                    detail: format!("{note} is not a note any key produces"),
                })?;

                played.record(pitch);
            }

            attempts.push(AttemptRecord {
                played,
                reached: attempt.reached,
            });
        }

        Ok(SessionRecord {
            id: stored.id,
            piece,
            started: stored.started,
            closed: stored.closed,
            attempts,
        })
    }
}
