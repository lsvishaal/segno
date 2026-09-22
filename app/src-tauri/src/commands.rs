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

use hogwash_engraving::{EngravedScore, Engraver, EngravingFailure};
use hogwash_notation::ReferencePerformance;
use serde::Serialize;
use tauri::{Manager as _, State};
use tauri_plugin_dialog::DialogExt as _;

use crate::engraving::Engraving;
use crate::practice::AttemptSummary;
use crate::startup::Unavailable;
use crate::state::{Failed, Hogwash, Shared};

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

    opened_piece(id, hogwash_engraving_verovio::Layout::default(), &hogwash)
}

/// Re-engraves a Piece at a different staff size (`BFR-4`).
///
/// Stateless with respect to `Practice`'s session: a Piece's `Attempt` is
/// keyed on `Location`/pitch, never on an element id, so re-engraving at a
/// new scale never disturbs one in progress. The frontend gets a full fresh
/// `OpenedPiece` — new `pages` and new `marks` together, atomically — rather
/// than being asked to patch anything in place.
///
/// # Errors
///
/// As [`open_piece`].
#[tauri::command]
pub fn rescale_piece(
    id: String,
    scale: u32,
    state: State<'_, Shared>,
) -> Result<OpenedPiece, Refused> {
    let hogwash = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    let layout = hogwash_engraving_verovio::Layout {
        scale: clamp_scale(scale),
        ..hogwash_engraving_verovio::Layout::default()
    };

    opened_piece(id, layout, &hogwash)
}

/// Keeps a requested staff size within what is actually usable — not trusting
/// the frontend's arithmetic (a rapid Ctrl+scroll, or a slider dragged past
/// its own visual ends) any more than any other boundary input.
const fn clamp_scale(requested: u32) -> u32 {
    if requested < 50 {
        50
    } else if requested > 400 {
        400
    } else {
        requested
    }
}

/// The body [`open_piece`] and [`rescale_piece`] share: everything about
/// engraving a Piece except which [`Layout`](hogwash_engraving_verovio::Layout)
/// to engrave it at.
fn opened_piece(
    id: String,
    layout: hogwash_engraving_verovio::Layout,
    hogwash: &crate::state::Hogwash,
) -> Result<OpenedPiece, Refused> {
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
        .engrave(&source, layout)
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

/// Saves a Piece's original source to a location the performer chooses
/// (`BFR-1`, the tractable slice of it — `MusicXML` only, this round).
///
/// The dialog is a **blocking** call, so the state lock is released before
/// showing it: holding it would freeze every other command (the library
/// listing, the practice thread's commands) for as long as the performer is
/// looking at a file picker.
///
/// # Errors
///
/// If the piece is not in the library, its source could not be found, or the
/// chosen location could not be written to. A cancelled dialog is **not** an
/// error — it resolves successfully, having done nothing.
#[tauri::command]
pub fn export_musicxml(
    id: String,
    app: tauri::AppHandle,
    state: State<'_, Shared>,
) -> Result<(), Refused> {
    let (title, source) = {
        let hogwash = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

        let piece_id = hogwash_domain::PieceId::new(&id)
            .ok_or_else(|| format!("`{id}` is not a usable piece name."))?;

        let piece = hogwash
            .library()
            .pieces()
            .into_iter()
            .find(|piece| *piece.id() == piece_id)
            .ok_or_else(|| format!("There is no piece called `{id}` in the library."))?;

        let source = hogwash.source_of(piece.id()).ok_or_else(|| {
            "The music for this piece could not be found, so it cannot be exported.".to_owned()
        })?;

        (piece.title().to_owned(), source)
    };

    let extension = extension_of(&source);
    let suggested = format!("{title}.{extension}");

    let Some(destination) = app
        .dialog()
        .file()
        .set_file_name(&suggested)
        .add_filter("MusicXML", &[extension])
        .blocking_save_file()
    else {
        return Ok(());
    };

    let path = destination
        .into_path()
        .map_err(|error| format!("Could not use that location: {error}"))?;

    std::fs::write(&path, &source).map_err(|error| format!("Could not save the file: {error}"))
}

/// Whether a piece's source is compressed `MusicXML` (`.mxl`, a zip) or plain
/// (`.musicxml`) — the same signature `hogwash-acquisition` and the engraver
/// already sniff for (see `hogwash_engraving_verovio::engraver`), sniffed
/// again here only to suggest the right extension in the save dialog.
fn extension_of(source: &[u8]) -> &'static str {
    if source.starts_with(b"PK") {
        "mxl"
    } else {
        "musicxml"
    }
}

/// A piece a performer offered, resolved into what the library screen needs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedPiece {
    /// The Piece now available to practise.
    pub piece: PieceSummary,
    /// What was noticed while reading it (`ValidationReport::lines`),
    /// verbatim — "The whole score was read." when nothing was lost, one
    /// sentence per loss otherwise. Carried here because nothing later can
    /// reconstruct it: this is the one reading of the file that produced it.
    pub lines: Vec<String>,
}

/// Adapts [`Engraving`]'s channel handle to the [`Engraver`] port
/// `hogwash_acquisition::assess` is written against.
///
/// The mismatch is only in shape, not substance: `Engraving::engrave` takes
/// `&self` and a `Layout` because it just posts to a channel and the engine
/// on the other end holds the layout state; `Engraver::engrave` takes
/// `&mut self` and no layout because a port assumes an engine that holds the
/// document it is working on (`ADR-0008`). Import has no layout of its own —
/// the Library does not keep one — so [`Layout::default`] is the same choice
/// every other command already makes when opening a piece for the first
/// time, not a placeholder invented for this.
struct ImportEngraver<'a>(&'a Engraving);

impl Engraver for ImportEngraver<'_> {
    fn engrave(&mut self, source: &[u8]) -> Result<EngravedScore, EngravingFailure> {
        self.0
            .engrave(source, hogwash_engraving_verovio::Layout::default())
    }
}

/// Imports a `MusicXML` file the performer chooses, joining it to the
/// library (FR-100, FR-102, FR-104).
///
/// The dialog is a **blocking** call and happens before anything is looked
/// up, so nothing needs the state lock while the performer is looking at a
/// file picker — the same reasoning [`export_musicxml`] documents, applied
/// the other way around because import has nothing to look up until the
/// dialog returns.
///
/// # Errors
///
/// If the chosen file could not be read, or [`hogwash_acquisition::assess`]
/// refused it — in which case the error is the refusal's reason and route,
/// verbatim (`004 §8.2`): this command adds no wording of its own to an
/// explanation that is already complete.
#[tauri::command]
pub fn import_musicxml(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
) -> Result<Option<ImportedPiece>, Refused> {
    let Some(chosen) = app
        .dialog()
        .file()
        .add_filter("MusicXML", &["musicxml", "mxl"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };

    let path = chosen
        .into_path()
        .map_err(|error| format!("Could not use that location: {error}"))?;

    let material =
        std::fs::read(&path).map_err(|error| format!("Could not read that file: {error}"))?;

    let mut hogwash = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    accept_offered(&material, &mut hogwash).map(Some)
}

/// The body [`import_musicxml`] runs once it has bytes and a locked
/// [`Hogwash`] — separated out, and public, so an integration test can drive
/// it without a real dialog or `AppHandle` (see `lib.rs`'s module doc).
///
/// # Errors
///
/// As [`import_musicxml`]: the offered material was refused, verbatim from
/// [`hogwash_acquisition::assess`].
pub fn accept_offered(material: &[u8], hogwash: &mut Hogwash) -> Result<ImportedPiece, Refused> {
    let id = crate::provided::identify(material);

    // Re-offering the same bytes is not a second piece: `PieceId` is the
    // lookup key `source_of` and `open_piece` use, and a duplicate under one
    // id would make that lookup ambiguous rather than merely wasteful.
    if let Some(already) = hogwash
        .library()
        .pieces()
        .into_iter()
        .find(|piece| *piece.id() == id)
    {
        return Ok(ImportedPiece {
            piece: summarise(already),
            lines: vec!["This piece is already in your library.".to_owned()],
        });
    }

    let mut engraver = ImportEngraver(hogwash.engraving());

    match hogwash_acquisition::assess(material, id.clone(), &mut engraver) {
        hogwash_acquisition::Outcome::NotAccepted(refused) => {
            Err(format!("{}\n\n{}", refused.reason(), refused.route()))
        }
        hogwash_acquisition::Outcome::Accepted { piece, report } => {
            hogwash
                .records()
                .keep_piece(&id, piece.title(), piece.composer(), material)
                .map_err(|failure| format!("Could not keep that piece: {failure:?}"))?;

            let summary = summarise(&piece);
            hogwash.accept_piece(*piece);

            Ok(ImportedPiece {
                piece: summary,
                lines: report.lines(),
            })
        }
    }
}

/// A Piece, as the library screen needs it — the same shape [`library`]
/// produces, so an import and a startup listing agree on what a summary is.
fn summarise(piece: &hogwash_library::Piece) -> PieceSummary {
    PieceSummary {
        id: piece.id().as_str().to_owned(),
        title: piece.title().to_owned(),
        composer: piece.composer().map(str::to_owned),
        bars: piece.score().measures().len(),
        notes: piece.reference().len(),
    }
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

/// A MIDI output device, as the picker needs it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputPortSummary {
    /// What to show the performer, and what `set_thru` names it by.
    pub name: String,
}

/// The MIDI output devices currently available, for routing Thru (`BFR`,
/// "remove the sound, allow modification of input and output sources").
///
/// # Errors
///
/// If none are available, or the practice thread has gone.
#[tauri::command]
pub fn output_ports(state: State<'_, Shared>) -> Result<Vec<OutputPortSummary>, Refused> {
    let hogwash = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    hogwash
        .practice()
        .output_ports()
        .map(|found| {
            found
                .into_iter()
                .map(|port| OutputPortSummary {
                    name: port.name().to_owned(),
                })
                .collect()
        })
        .map_err(|absent| absent.describe())
}

/// Sets or clears the MIDI Thru route: everything `input` reports gets
/// forwarded, byte for byte, to `output` — instead of Hogwash making its own
/// sound. Independent of `start_attempt`: routing works whether or not a
/// practice attempt is running.
///
/// Passing `null` for either side clears whatever route was set.
///
/// # Errors
///
/// If either device could not be opened, or the practice thread has gone.
#[tauri::command]
pub fn set_thru(
    input: Option<String>,
    output: Option<String>,
    state: State<'_, Shared>,
) -> Result<(), Refused> {
    let hogwash = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    hogwash
        .practice()
        .set_thru(
            input.map(hogwash_capture::Instrument::new),
            output.map(hogwash_midi::OutputPort::new),
        )
        .map_err(|absent| absent.describe())
}

/// What to say when the state cannot be read.
const POISONED: &str = "Hogwash got into a state it cannot recover from. Restarting it should clear this; \
     your practice history is on disk and is unaffected.";

#[cfg(test)]
mod tests {
    use super::{clamp_scale, extension_of};

    #[test]
    fn plain_musicxml_has_no_zip_signature() {
        assert_eq!(
            extension_of(b"<?xml version=\"1.0\"?><score-partwise/>"),
            "musicxml"
        );
    }

    #[test]
    fn a_zip_signature_means_compressed_musicxml() {
        assert_eq!(extension_of(b"PK\x03\x04rest of the archive"), "mxl");
    }

    #[test]
    fn a_usable_scale_passes_through_unchanged() {
        assert_eq!(clamp_scale(150), 150);
    }

    #[test]
    fn too_small_is_clamped_up() {
        assert_eq!(clamp_scale(0), 50);
    }

    #[test]
    fn too_large_is_clamped_down() {
        assert_eq!(clamp_scale(10_000), 400);
    }
}
