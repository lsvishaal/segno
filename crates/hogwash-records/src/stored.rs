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

use hogwash_capture::{ObservedPerformance, Touch};
use hogwash_domain::{PieceId, Pitch};
use serde::{Deserialize, Serialize};

/// The version of the session shape below.
///
/// Bumped whenever the stored shape changes in a way an older reader would
/// misunderstand. A reader that meets a version it does not know **refuses**
/// rather than guessing: a half-understood record read as if it were
/// understood is worse than one that could not be opened, because the first
/// silently reports the wrong thing about a performer's practice.
///
/// - **1**: each attempt's notes as bare key numbers, `"played": [74, 67]`.
///   Still read; never written.
/// - **2**: each note an object, with when and how hard it was struck and
///   when it came up, plus the sustain pedal and when the attempt began:
///
/// ```json
/// { "began_ms": 1792270830806, "reached": 2,
///   "notes": [ { "key": 74, "at_us": 0, "velocity": 80, "released_us": 410000 },
///              { "key": 67, "at_us": 512000, "velocity": 72 } ],
///   "pedal": [ { "at_us": 300000, "down": true } ] }
/// ```
///
/// Times are microseconds since the attempt began; `began_ms` is Unix time in
/// milliseconds, like a session's `started`. A note's timing fields are
/// absent where they were never known (it came from a schema 1 file), and
/// `released_us` is absent if the key was still down when the attempt ended.
pub(crate) const SESSION_SCHEMA: u32 = 2;

/// The version of the piece shape below. Separate from the session's: the
/// two change for different reasons, and bumping one must not make an older
/// version refuse the other.
pub(crate) const PIECE_SCHEMA: u32 = 1;

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
    /// When the attempt began, in Unix milliseconds. Absent in schema 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) began_ms: Option<u64>,
    /// How far the performer got, as a position in the Reference Performance.
    pub(crate) reached: usize,
    /// Every note played, in order.
    ///
    /// The Observed Performance and **nothing derived from it**. An Analysis
    /// is a total function of this and the Piece's Reference Performance, so
    /// storing findings as well would mean keeping two accounts that could
    /// disagree — and if they ever did, there would be no way to tell which
    /// was right. The results are recomputed, identically, every time.
    #[serde(default)]
    pub(crate) notes: Vec<StoredNote>,
    /// Every change of the sustain pedal, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) pedal: Vec<StoredPedal>,
    /// Schema 1's notes: bare key numbers. Read, never written.
    #[serde(default, skip_serializing)]
    pub(crate) played: Vec<u8>,
}

/// One played note, as stored.
///
/// The key is a bare MIDI number, which is all an instrument reports. No
/// spelling is kept because none was ever known: the performer pressed a
/// key, and which of its two readings they had in mind is not something the
/// instrument said.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoredNote {
    pub(crate) key: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) at_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) velocity: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) released_us: Option<u64>,
}

/// A change of the sustain pedal, as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoredPedal {
    pub(crate) at_us: u64,
    pub(crate) down: bool,
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
    /// What the performer's own file was called when imported. Absent from
    /// records written before it was kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) file_name: Option<String>,
    /// Whether `title` is one the performer chose, which then wins over
    /// whatever the file says when it is read again.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) renamed: bool,
}

/// An Observed Performance's notes and pedal, as stored.
pub(crate) fn stored_performance(
    observed: &ObservedPerformance,
) -> (Vec<StoredNote>, Vec<StoredPedal>) {
    let notes = observed
        .played()
        .iter()
        .zip(observed.touches())
        .map(|(&pitch, touch)| StoredNote {
            key: u8::from(pitch),
            at_us: touch.map(|touch| touch.at_us),
            velocity: touch.map(|touch| touch.velocity),
            released_us: touch.and_then(|touch| touch.released_us),
        })
        .collect();

    let pedal = observed
        .pedal_changes()
        .iter()
        .map(|change| StoredPedal {
            at_us: change.at_us,
            down: change.down,
        })
        .collect();

    (notes, pedal)
}

/// A stored attempt's notes and pedal, read back as an Observed Performance.
///
/// Schema 1 kept bare key numbers in `played`; schema 2 keeps `notes`.
///
/// # Errors
///
/// What is wrong, if anything stored is not something a keyboard could
/// have reported: a key or velocity out of range, or timing that is only
/// partly there. Refused rather than repaired, for the same reason a record
/// from a newer version is.
pub(crate) fn read_performance(
    schema: u32,
    attempt: &StoredAttempt,
) -> Result<ObservedPerformance, String> {
    let mut observed = ObservedPerformance::new();
    let key =
        |note: u8| Pitch::new(note).ok_or_else(|| format!("{note} is not a note any key produces"));

    if schema == 1 {
        for &note in &attempt.played {
            observed.record(key(note)?);
        }
        return Ok(observed);
    }

    for note in &attempt.notes {
        let pitch = key(note.key)?;

        match (note.at_us, note.velocity, note.released_us) {
            (None, None, None) => observed.record(pitch),
            (Some(at_us), Some(velocity @ 1..=127), released_us) => observed.record_touched(
                pitch,
                Touch {
                    at_us,
                    velocity,
                    released_us,
                },
            ),
            _ => {
                return Err(format!(
                    "key {}: its timing is incomplete or out of range",
                    note.key
                ));
            }
        }
    }

    for change in &attempt.pedal {
        observed.pedal(change.at_us, change.down);
    }

    Ok(observed)
}

/// A durable name, read back.
///
/// Returns `None` for a name that is not one — which, since the name reaches
/// a filesystem, is also the check that a tampered record cannot make storage
/// write outside its own directory.
pub(crate) fn read_id(name: &str) -> Option<PieceId> {
    PieceId::new(name)
}
