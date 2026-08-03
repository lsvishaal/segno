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

use hogwash_notation::ReferencePerformance;
use serde::Serialize;
use tauri::{Manager as _, State};

use crate::practice::AttemptSummary;
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

/// A Piece, engraved and ready to practise from.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedPiece {
    /// Which piece this is.
    pub id: String,
    /// What to call it.
    pub title: String,
    /// Who wrote it.
    pub composer: Option<String>,
    /// The engraved pages, as SVG.
    ///
    /// Sent as markup rather than as an image. That is the whole of
    /// `ADR-0012`: in a webview the score is real elements, so marking a note
    /// is a class on one of them rather than a rectangle computed from a
    /// bounding box.
    pub pages: Vec<String>,
    /// Where every expected note was drawn, in reading order.
    pub marks: Vec<Mark>,
}

/// One place in the music, and the elements drawn for it.
///
/// `Location` flattened into three numbers, because a nested object buys
/// nothing on this side of the boundary and every consumer wants the parts.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mark {
    /// Which bar, as a performer counts them (from one).
    pub measure: u32,
    /// Which position in the piece, counting from zero across the whole
    /// score rather than restarting each bar.
    pub position: usize,
    /// Which page it was drawn on, counting from zero.
    pub page: usize,
    /// Every note drawn here, paired with what was written. Several for a
    /// chord.
    ///
    /// Carrying the written spelling — not just the element id — is what
    /// lets the interface measure real staff geometry from real points: a
    /// wrong note has no position of its own to draw at (a keyboard reports
    /// a sound, never a spelling, `ADR-0001`), so placing anything for it
    /// has to be computed from where notes that *were* written actually
    /// ended up on screen.
    pub notes: Vec<DrawnNote>,
}

/// One drawn notehead, and the spelling Verovio drew it from.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawnNote {
    /// The identifier Verovio gave this notehead.
    pub element_id: String,
    /// The written letter name — `"C"` through `"B"`.
    ///
    /// Not the accidental: `alter` only draws a glyph beside the note, it
    /// does not move it vertically on the staff, which is the only thing
    /// this field exists to let the interface reconstruct.
    pub step: String,
    /// Scientific pitch notation, where middle C is octave 4.
    pub octave: i8,
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

/// Engraves a Piece and hands over everything needed to draw it (FR-800).
///
/// # Errors
///
/// If the piece is not in the library, or the engraver refused it. Both are
/// reported in words the interface can show.
#[tauri::command]
pub fn open_piece(id: String, state: State<'_, Shared>) -> Result<OpenedPiece, Refused> {
    let hogwash = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    let piece = hogwash
        .library()
        .pieces()
        .into_iter()
        .find(|piece| piece.id().as_str() == id)
        .ok_or_else(|| format!("There is no piece called `{id}` in the library."))?;

    // Engraved on demand rather than kept from startup. A score's SVG is
    // large and only one is ever on screen; holding every page of every piece
    // for the life of the process would be paying for a piece nobody opened.
    let source = hogwash.source_of(piece.id()).ok_or_else(|| {
        "The music for this piece could not be found, so it cannot be shown.".to_owned()
    })?;

    let engraved = hogwash
        .engraving()
        .engrave(&source)
        .map_err(|failure| format!("{failure:?}"))?;

    // Derived again here rather than reused from `Piece::reference()`: the
    // one just engraved is what these element ids actually correspond to,
    // and `003 §7.4` makes the derivation deterministic, so the two are the
    // same performance either way.
    let reference = ReferencePerformance::derived_from(engraved.score());

    Ok(OpenedPiece {
        id,
        title: piece.title().to_owned(),
        composer: piece.composer().map(str::to_owned),
        pages: engraved.pages().to_vec(),
        marks: engraved
            .placements()
            .map(|(at, placement)| {
                let expected = reference.at(at.position());

                let notes = placement
                    .elements()
                    .iter()
                    .zip(
                        expected
                            .map(hogwash_notation::ExpectedNotes::notes)
                            .unwrap_or_default(),
                    )
                    .map(|(element, note)| DrawnNote {
                        element_id: element.as_str().to_owned(),
                        step: format!("{:?}", note.spelling().step()),
                        octave: note.spelling().octave(),
                    })
                    .collect();

                Mark {
                    measure: at.measure().get(),
                    position: at.position(),
                    page: placement.page().get(),
                    notes,
                }
            })
            .collect(),
    })
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

/// A MIDI instrument, as the picker needs it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentSummary {
    /// What to show the performer, and what `start_attempt` names it by.
    pub name: String,
}

/// The MIDI instruments currently connected (FR-307).
///
/// # Errors
///
/// If none are connected, or the practice thread has gone — either way, in
/// words a performer can read (`NoInstrument::describe`).
#[tauri::command]
pub fn instruments(state: State<'_, Shared>) -> Result<Vec<InstrumentSummary>, Refused> {
    let hogwash = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    hogwash
        .practice()
        .instruments()
        .map(|found| {
            found
                .into_iter()
                .map(|instrument| InstrumentSummary {
                    name: instrument.name().to_owned(),
                })
                .collect()
        })
        .map_err(|absent| absent.describe())
}

/// Begins an attempt at `piece_id`, on `instrument` (FR-801).
///
/// # Errors
///
/// If the piece is not in the library, or the instrument could not be
/// opened.
#[tauri::command]
pub fn start_attempt(
    piece_id: String,
    instrument: String,
    state: State<'_, Shared>,
) -> Result<(), Refused> {
    let hogwash = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    let id = hogwash_domain::PieceId::new(&piece_id)
        .ok_or_else(|| format!("`{piece_id}` is not a usable piece name."))?;

    let piece = hogwash
        .library()
        .pieces()
        .into_iter()
        .find(|piece| *piece.id() == id)
        .ok_or_else(|| format!("There is no piece called `{piece_id}` in the library."))?;

    let reference = std::sync::Arc::new(piece.reference().clone());
    let named = hogwash_capture::Instrument::new(instrument);

    hogwash
        .practice()
        .start_attempt(id, reference, named)
        .map_err(|absent| absent.describe())
}

/// Ends whatever attempt is running, analyses it, and keeps the record
/// (FR-408, ADR-0010).
///
/// # Errors
///
/// If no attempt is running.
#[tauri::command]
pub fn end_attempt(state: State<'_, Shared>) -> Result<AttemptSummary, Refused> {
    let hogwash = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    hogwash
        .practice()
        .end_attempt()
        .map_err(|_| "No attempt is in progress.".to_owned())
}

/// What to say when the state cannot be read.
const POISONED: &str = "Hogwash got into a state it cannot recover from. Restarting it should clear this; \
     your practice history is on disk and is unaffected.";
