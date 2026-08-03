//! What a record looks like on disk.
//!
//! # These types are not the domain's types, and that is the point
//!
//! It would be shorter to put `serde` derives on `Attempt` and `Feedback` and
//! write those. It would also mean that renaming a field, or splitting a
//! type, silently invalidated every record ever written — because the wire
//! format would be whatever the domain happened to look like that week.
//!
//! So the shape on disk is declared here, separately, and mapped. The cost is
//! a translation nobody enjoys writing. What it buys is that
//! `hogwash-practice` can be refactored freely, and that when the stored shape
//! *does* have to change, changing it is a deliberate act with a version
//! number attached rather than an accident.
//!
//! It also keeps `serde` out of the pure crates, which `scripts/check-arch.sh`
//! enforces.

use hogwash_domain::{PieceId, Pitch};
use serde::{Deserialize, Serialize};

/// The version of the shape below.
///
/// Bumped whenever the stored shape changes in a way an older reader would
/// misunderstand. A reader that meets a version it does not know **refuses**
/// rather than guessing: a half-understood record read as if it were
/// understood is worse than one that could not be opened, because the first
/// silently reports the wrong thing about a performer's practice.
pub(crate) const SCHEMA: u32 = 1;

/// One practice session, as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoredSession {
    /// Which shape this is. First field so a partial read still finds it.
    pub(crate) schema: u32,
    /// What this session is called on disk, and in itself.
    pub(crate) id: String,
    /// Which Piece was practised (`003 §7.2` — exactly one).
    pub(crate) piece: String,
    /// When it began, as the caller reported it.
    ///
    /// A string in whatever form the caller chose, because `004 §6.7` keeps
    /// the clock out of the domain and this crate is not where that changes.
    /// Storage records what it is told the time was; it does not ask.
    pub(crate) started: String,
    /// Whether the performer closed it, or simply stopped (FR-702).
    pub(crate) closed: bool,
    /// Every attempt, in the order they were made (`003 §7.2`).
    pub(crate) attempts: Vec<StoredAttempt>,
}

/// One attempt at a piece, as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoredAttempt {
    /// Every note played, in order (`003 §7.5`).
    ///
    /// The Observed Performance and **nothing derived from it**. An Analysis
    /// is a total function of this and the Piece's Reference Performance
    /// (`003 §7.6`, FR-406), so storing findings as well would mean keeping
    /// two accounts that could disagree — and if they ever did, there would
    /// be no way to tell which was right.
    ///
    /// FR-601 wants attempt history "including their analysis results", and
    /// this satisfies it by keeping what those results are *made of*. The
    /// results themselves are recomputed, identically, every time.
    pub(crate) played: Vec<u8>,
    /// How far the performer got, as a position in the Reference Performance.
    pub(crate) reached: usize,
}

/// A piece in the library, as stored.
///
/// FR-703 keeps imported content separate from user-generated data, so this
/// records *about* a piece; the score itself is a file beside it, kept
/// read-only (FR-701).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoredPiece {
    /// Which shape this is.
    pub(crate) schema: u32,
    /// Its durable name.
    pub(crate) id: String,
    /// What to call it (FR-201).
    pub(crate) title: String,
    /// Who wrote it, where the source said.
    pub(crate) composer: Option<String>,
    /// The file the score was read from, beside this record.
    pub(crate) source: String,
}

/// A played note, as stored.
///
/// A bare MIDI number, which is all an instrument reports (ADR-0001). No
/// spelling is kept because none was ever known: the performer pressed a key,
/// and which of the two readings of that key they had in mind is not
/// something the instrument said.
pub(crate) fn stored_pitch(pitch: Pitch) -> u8 {
    u8::from(pitch)
}

/// A durable name, read back.
///
/// Returns `None` for a name that is not one — which, since the name reaches
/// a filesystem, is also the check that a tampered record cannot make storage
/// write outside its own directory.
pub(crate) fn read_id(name: &str) -> Option<PieceId> {
    PieceId::new(name)
}
