//! What the interface may ask for.
//!
//! # The shape of everything here
//!
//! Each command takes the shared state, does one thing, and returns a plain
//! serialisable value. The types crossing this boundary are **not** the
//! domain's types — they are the same deliberate translation `ADR-0010` made
//! for storage, for the same reason: an interface that received
//! `hogwash_notation::Score` directly would break the moment that type was
//! refactored, and nothing in TypeScript would notice until it ran.
//!
//! # What is not here
//!
//! Any judgement about a performance. `004 §8.8` says Presentation
//! "never originates a finding of its own", and the way that is kept true is
//! that every sentence about playing arrives already written, from
//! `hogwash-feedback`. There is no command that returns a fragment for the
//! frontend to assemble into a verdict.

// Tauri's calling convention takes `State` and `AppHandle` by value: they are
// cheap handles, and the macro generates the call. Restructuring the
// signatures to satisfy the lint would mean fighting the framework over
// something that costs a pointer copy.
#![expect(
    clippy::needless_pass_by_value,
    reason = "tauri commands receive State and AppHandle by value; both are handles"
)]

use serde::Serialize;
use tauri::{Manager as _, State};

use crate::startup::Unavailable;
use crate::state::{Failed, Shared};

/// A Piece, as the library screen needs it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PieceSummary {
    /// Its durable name, used to ask for it again.
    pub id: String,
    /// What to call it (FR-201). Never empty.
    pub title: String,
    /// Who wrote it, where the source said.
    pub composer: Option<String>,
    /// How many bars, so the interface can say something about its size
    /// without inventing a difficulty rating (`001 §11` rules those out).
    pub bars: usize,
    /// How many notes are expected in total.
    pub notes: usize,
}

/// Something a command could not do, in words.
///
/// A `String` rather than a type, because everything reaching here has
/// already been turned into an explanation by the crate that knew what went
/// wrong. Re-wrapping it would only add a layer with less information.
pub type Refused = String;

/// The Pieces available to practise (FR-203, FR-206).
///
/// # Errors
///
/// If the state is unreadable, which means a previous command panicked while
/// holding it. Reported rather than propagated: a poisoned lock is a defect
/// in Hogwash, and the performer should be told that rather than shown a
/// crash.
#[tauri::command]
pub fn library(state: State<'_, Shared>) -> Result<Vec<PieceSummary>, Refused> {
    let hogwash = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    Ok(hogwash
        .library()
        .pieces()
        .into_iter()
        .map(|piece| PieceSummary {
            id: piece.id().as_str().to_owned(),
            title: piece.title().to_owned(),
            composer: piece.composer().map(str::to_owned),
            bars: piece.score().measures().len(),
            notes: piece.reference().len(),
        })
        .collect())
}

/// Why the application cannot work, if it cannot.
///
/// Returns `None` when everything started. The interface asks this first and
/// shows the answer as a screen — which is the whole reason a failed startup
/// still opens a window.
#[tauri::command]
#[must_use]
pub fn startup_failure(app: tauri::AppHandle) -> Option<Unavailable> {
    // `try_state` rather than `state`: not finding it is the ordinary case,
    // meaning everything started. `state` would panic on the happy path.
    app.try_state::<Failed>().map(|failed| failed.0.clone())
}

/// What to say when the state cannot be read.
const POISONED: &str = "Hogwash got into a state it cannot recover from. Restarting it should clear this; \
     your practice history is on disk and is unaffected.";
