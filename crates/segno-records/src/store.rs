//! Keeping practice history, and the scores it refers to.

use std::fs;
use std::path::{Path, PathBuf};

use segno_capture::ObservedPerformance;
use segno_domain::PieceId;

use crate::atomic;
pub use crate::stored::Ended;
use crate::stored::{
    PIECE_SCHEMA, SESSION_SCHEMA, Schema, StoredAttempt, StoredPiece, StoredSession, read_id,
    read_performance, stored_performance,
};

/// The fingerprint of a score's bytes: the first 16 hex characters of their
/// SHA-256. It names a *version* of a score and spots a second import of the
/// same file; it is never a piece's identity, or any edit would orphan the
/// piece's history (DECISIONS, 3 Nov).
#[must_use]
pub fn fingerprint(source: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    Sha256::digest(source)
        .iter()
        .take(8)
        .fold(String::new(), |mut name, byte| {
            // `write!` into a String cannot fail.
            let _ = write!(name, "{byte:02x}");
            name
        })
}

/// Removes a file Segno kept read-only. Already gone is not an error.
fn remove(path: &Path) -> Result<(), RecordsFailure> {
    if let Ok(metadata) = fs::metadata(path) {
        let mut permissions = metadata.permissions();
        #[expect(
            clippy::permissions_set_readonly_false,
            reason = "about to delete the file, on the performer's behalf"
        )]
        permissions.set_readonly(false);
        drop(fs::set_permissions(path, permissions));
    }
    match fs::remove_file(path) {
        Err(failure) if failure.kind() != std::io::ErrorKind::NotFound => Err(failure.into()),
        _ => Ok(()),
    }
}

/// One session's worth of history, in the terms the application deals in.
///
/// Deliberately not `segno_practice::PracticeSession`. That type carries a
/// borrowed Reference Performance and a typestate lifecycle, neither of which
/// survives a restart; this is what is *kept*, which is a smaller thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    /// What this session is called.
    pub id: String,
    /// Which Piece was practised (the design — exactly one).
    pub piece: PieceId,
    /// The fingerprint of the score it was played on, kept in
    /// [`Records::versions`]: its attempts are judged against that version.
    pub version: String,
    /// Which analysis rules it was played under (`segno_analysis::RULES`).
    pub rules: u32,
    /// When it began, as the caller reported it.
    ///
    /// Supplied, never read from a clock. The design keeps the clock out of
    /// the domain, and storage is not the place to smuggle one back in: a
    /// store that timestamped its own writes would make two runs of the same
    /// test produce different records.
    pub started: String,
    /// Whether the performer closed it, or simply stopped.
    pub closed: bool,
    /// Every attempt, in the order they were made.
    pub attempts: Vec<AttemptRecord>,
}

/// One attempt, in the terms the application deals in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptRecord {
    /// When it began, in Unix milliseconds, as the caller reported it.
    /// `None` for attempts kept before this was recorded.
    pub began_ms: Option<u64>,
    /// Where it started: a position in the Reference Performance (a step in
    /// played order), not a bar number. 0 for attempts kept before this was.
    pub from: usize,
    /// For a pass of a loop, where the loop ends (exclusive): a position,
    /// like `from`. `None` otherwise.
    pub to: Option<usize>,
    /// What the performer played, in order.
    pub played: ObservedPerformance,
    /// How far they got.
    pub reached: usize,
    /// How it ended. `None` for attempts kept before this was recorded.
    pub ended: Option<Ended>,
}

/// A piece kept on disk, and where its score is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PieceRecord {
    /// Its durable name.
    pub id: PieceId,
    /// What to call it.
    pub title: String,
    /// Who wrote it, where the source said.
    pub composer: Option<String>,
    /// The fingerprint of the score's current version, kept in
    /// [`Records::versions`].
    pub version: String,
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

    /// A record written by a newer version of Segno.
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
/// # Two directories, because the requirement asks for two
///
/// *"A clear separation between imported piece content and user-generated
/// data such as practice history."* It is a real separation and not a tidiness
/// preference: imported scores are the performer's own material, held
/// read-only and replaceable from wherever they came from, while
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

    /// The folder everything is kept under.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where practice history lives.
    #[must_use]
    pub fn history(&self) -> PathBuf {
        self.root.join("history")
    }

    /// Where imported pieces are recorded: one `<id>.json` each.
    #[must_use]
    pub fn scores(&self) -> PathBuf {
        self.root.join("scores")
    }

    /// Where every version of every score played or imported is kept, named
    /// by its [`fingerprint`]: written once, read-only, never changed, so a
    /// session can always be replayed against the notes it was played on.
    #[must_use]
    pub fn versions(&self) -> PathBuf {
        self.root.join("versions")
    }

    /// Keeps `source` as a version, if it isn't kept already, and answers its
    /// fingerprint.
    ///
    /// # Errors
    ///
    /// [`RecordsFailure::Storage`] if it could not be written.
    pub fn keep_version(&self, source: &[u8]) -> Result<String, RecordsFailure> {
        let version = fingerprint(source);
        let path = self.version_path(&version);
        if !path.exists() {
            fs::create_dir_all(self.versions())?;
            let partial = path.with_extension("partial");
            fs::write(&partial, source)?;
            fs::rename(&partial, &path)?;
            // Advisory: it keeps Segno from changing a version by accident.
            if let Ok(metadata) = fs::metadata(&path) {
                let mut permissions = metadata.permissions();
                permissions.set_readonly(true);
                drop(fs::set_permissions(&path, permissions));
            }
        }
        Ok(version)
    }

    /// When `version` was first kept, in Unix milliseconds: when Segno first
    /// had this version of a score.
    #[must_use]
    pub fn version_kept_ms(&self, version: &str) -> Option<u64> {
        let modified = fs::metadata(self.version_path(version))
            .ok()?
            .modified()
            .ok()?;
        let since = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
        u64::try_from(since.as_millis()).ok()
    }

    /// The score kept as `version`, if it is here.
    #[must_use]
    pub fn version(&self, version: &str) -> Option<Vec<u8>> {
        fs::read(self.version_path(version)).ok()
    }

    /// A name for a piece being imported now, from its fingerprint: given
    /// once and kept for good, whatever later happens to the file. Never one
    /// already in use.
    #[must_use]
    pub fn new_import_id(&self, version: &str) -> PieceId {
        let stem = format!("imp-{}", version.get(..12).unwrap_or(version));
        (1..=u32::MAX)
            .map(|n| {
                if n == 1 {
                    stem.clone()
                } else {
                    format!("{stem}-{n}")
                }
            })
            .filter_map(|name| PieceId::new(&name))
            .find(|id| !self.piece_path(id).exists())
            .unwrap_or_else(|| unreachable!("an unused name exists"))
    }

    /// The imported piece whose current version is `version`, if any: how a
    /// second import of the same file is spotted.
    #[must_use]
    pub fn imported_as(&self, version: &str) -> Option<PieceId> {
        self.imported().into_iter().find(|id| {
            self.piece_record(id)
                .is_some_and(|record| record.version == version)
        })
    }

    /// Keeps `session`, replacing any earlier version of it.
    ///
    /// # Errors
    ///
    /// [`RecordsFailure::Storage`] if it could not be written. The previous
    /// version is untouched in that case — see [`crate::atomic`].
    pub fn keep(&self, session: &SessionRecord) -> Result<(), RecordsFailure> {
        let stored = StoredSession {
            schema: SESSION_SCHEMA,
            id: session.id.clone(),
            piece: session.piece.as_str().to_owned(),
            version: session.version.clone(),
            rules: session.rules,
            started: session.started.clone(),
            closed: session.closed,
            attempts: session
                .attempts
                .iter()
                .map(|attempt| {
                    let (notes, pedal) = stored_performance(&attempt.played);
                    StoredAttempt {
                        began_ms: attempt.began_ms,
                        from: attempt.from,
                        to: attempt.to,
                        reached: attempt.reached,
                        ended: attempt.ended,
                        notes,
                        pedal,
                    }
                })
                .collect(),
        };

        // Pretty-printed. A practice record is a few kilobytes and a
        // performer's own history; being able to read it with `cat` is worth
        // more than the bytes it costs, and the design is local-first, which
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

    /// Every session kept, oldest name first.
    ///
    /// A record that cannot be read is **skipped, not fatal**. One corrupt
    /// file must not make the rest of a performer's history unreachable —
    /// The design's failure boundary is precisely that prior records remain
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

        // Sorted, so two runs list them the same way (the requirement in spirit: a
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

    /// Keeps an imported score and a record of it.
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
        let version = self.keep_version(source)?;

        let record = PieceRecord {
            id: id.clone(),
            title: title.to_owned(),
            composer: composer.map(str::to_owned),
            version: version.clone(),
            file_name: file_name.map(str::to_owned),
            renamed: false,
        };

        let stored = StoredPiece {
            schema: PIECE_SCHEMA,
            id: record.id.as_str().to_owned(),
            title: record.title.clone(),
            composer: record.composer.clone(),
            version,
            file_name: record.file_name.clone(),
            renamed: false,
        };

        let written = serde_json::to_string_pretty(&stored).map_err(|failure| {
            RecordsFailure::Unreadable {
                path: self.piece_path(id),
                detail: failure.to_string(),
            }
        })?;

        fs::create_dir_all(self.scores())?;
        atomic::write(&self.piece_path(id), &written)?;

        Ok(record)
    }

    /// What was recorded about an imported piece, if anything was.
    #[must_use]
    pub fn piece_record(&self, id: &PieceId) -> Option<PieceRecord> {
        let stored: StoredPiece =
            serde_json::from_str(&fs::read_to_string(self.piece_path(id)).ok()?).ok()?;
        if stored.schema != PIECE_SCHEMA {
            return None;
        }

        Some(PieceRecord {
            id: id.clone(),
            title: stored.title,
            composer: stored.composer,
            version: stored.version,
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
        self.change_piece(id, |stored| {
            title.clone_into(&mut stored.title);
            stored.renamed = true;
        })
    }

    /// Makes `source` the current version of an imported piece: same name,
    /// same title and history; earlier sessions keep the version they were
    /// played on (DECISIONS, 3 Nov, Q3).
    ///
    /// # Errors
    ///
    /// As [`Records::rename_piece`].
    pub fn update_piece(
        &self,
        id: &PieceId,
        source: &[u8],
        file_name: Option<&str>,
    ) -> Result<(), RecordsFailure> {
        if self.piece_record(id).is_none() {
            return Err(RecordsFailure::Unreadable {
                path: self.piece_path(id),
                detail: "no imported piece by that name".to_owned(),
            });
        }
        let version = self.keep_version(source)?;
        self.change_piece(id, |stored| {
            stored.version = version;
            if let Some(file_name) = file_name {
                stored.file_name = Some(file_name.to_owned());
            }
        })
    }

    fn change_piece(
        &self,
        id: &PieceId,
        change: impl FnOnce(&mut StoredPiece),
    ) -> Result<(), RecordsFailure> {
        let path = self.piece_path(id);
        let unreadable = |detail: String| RecordsFailure::Unreadable {
            path: path.clone(),
            detail,
        };

        let mut stored: StoredPiece = serde_json::from_str(&fs::read_to_string(&path)?)
            .map_err(|failure| unreadable(failure.to_string()))?;
        change(&mut stored);

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
                    .is_some_and(|extension| extension == "json")
            })
            .filter_map(|path| PieceId::new(path.file_stem()?.to_str()?))
            .collect();
        ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        ids
    }

    /// Removes an imported piece's record, and its version unless another
    /// piece or a kept session still needs it. Missing files are not an
    /// error.
    ///
    /// # Errors
    ///
    /// [`RecordsFailure::Storage`] if a file that is there could not be removed.
    pub fn discard_score(&self, id: &PieceId) -> Result<(), RecordsFailure> {
        let version = self.piece_record(id).map(|record| record.version);
        remove(&self.piece_path(id))?;

        if let Some(version) = version {
            let needed = self.imported_as(&version).is_some();
            if !needed {
                remove(&self.version_path(&version))?;
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

    /// The score a Piece was imported from, if it is here.
    ///
    /// Returns the bytes rather than the path, so a caller cannot be tempted
    /// to hold a path across a restart — the design copies imported material
    /// into the library precisely so that the performer's own file may move.
    #[must_use]
    pub fn source_of(&self, id: &PieceId) -> Option<Vec<u8>> {
        self.version(&self.piece_record(id)?.version)
    }

    fn version_path(&self, version: &str) -> PathBuf {
        self.versions().join(format!("{version}.musicxml"))
    }

    fn session_path(&self, id: &str) -> PathBuf {
        self.history().join(format!("{id}.json"))
    }

    fn piece_path(&self, id: &PieceId) -> PathBuf {
        self.scores().join(format!("{}.json", id.as_str()))
    }

    fn read_session(path: &Path) -> Result<SessionRecord, RecordsFailure> {
        let text = fs::read_to_string(path)?;
        let unreadable = |detail: String| RecordsFailure::Unreadable {
            path: path.to_path_buf(),
            detail,
        };

        // The schema first, so a file from another version is named as one
        // rather than as a missing field.
        let schema = serde_json::from_str::<Schema>(&text)
            .map_err(|failure| unreadable(failure.to_string()))?
            .schema;
        if schema > SESSION_SCHEMA {
            return Err(RecordsFailure::FromTheFuture {
                path: path.to_path_buf(),
                schema,
            });
        }
        if schema < SESSION_SCHEMA {
            return Err(unreadable(format!(
                "schema {schema} is from before Segno's history started fresh (3 Nov 2026), so it is not read"
            )));
        }

        let stored: StoredSession =
            serde_json::from_str(&text).map_err(|failure| unreadable(failure.to_string()))?;

        let piece = read_id(&stored.piece).ok_or_else(|| RecordsFailure::Unreadable {
            path: path.to_path_buf(),
            detail: format!("`{}` is not a piece name", stored.piece),
        })?;

        let mut attempts = Vec::new();

        for attempt in stored.attempts {
            let played =
                read_performance(&attempt).map_err(|detail| RecordsFailure::Unreadable {
                    path: path.to_path_buf(),
                    detail,
                })?;

            // An attempt cannot have got less far than where it started, nor
            // a loop's pass further than the loop's end.
            if attempt.from > attempt.reached {
                return Err(RecordsFailure::Unreadable {
                    path: path.to_path_buf(),
                    detail: format!(
                        "an attempt starts at {} but reached only {}",
                        attempt.from, attempt.reached
                    ),
                });
            }
            if let Some(to) = attempt.to
                && to < attempt.reached
            {
                return Err(RecordsFailure::Unreadable {
                    path: path.to_path_buf(),
                    detail: format!(
                        "a loop ends at {to} but its pass reached {}",
                        attempt.reached
                    ),
                });
            }

            attempts.push(AttemptRecord {
                began_ms: attempt.began_ms,
                from: attempt.from,
                to: attempt.to,
                played,
                reached: attempt.reached,
                ended: attempt.ended,
            });
        }

        Ok(SessionRecord {
            id: stored.id,
            piece,
            version: stored.version,
            rules: stored.rules,
            started: stored.started,
            closed: stored.closed,
            attempts,
        })
    }
}
