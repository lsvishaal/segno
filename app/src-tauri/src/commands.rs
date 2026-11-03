//! What the interface may ask for.
//!
//! # The shape of everything here
//!
//! Each command takes the shared state, does one thing, and returns a plain
//! serialisable value. The types crossing this boundary are **not** the
//! domain's types — they are the same deliberate translation the design made
//! for storage, for the same reason: an interface that received
//! `segno_notation::Score` directly would break the moment that type was
//! refactored, and nothing in TypeScript would notice until it ran.
//!
//! # What is not here
//!
//! Any judgement about a performance. The design says Presentation
//! "never originates a finding of its own", and the way that is kept true is
//! that every sentence about playing arrives already written, from
//! `segno-feedback`. There is no command that returns a fragment for the
//! frontend to assemble into a verdict.

// Tauri's calling convention takes `State` and `AppHandle` by value: they are
// cheap handles, and the macro generates the call. Restructuring the
// signatures to satisfy the lint would mean fighting the framework over
// something that costs a pointer copy.
#![expect(
    clippy::needless_pass_by_value,
    reason = "tauri commands receive State and AppHandle by value; both are handles"
)]

use segno_engraving::{EngravedScore, Engraver, EngravingFailure};
use segno_notation::ReferencePerformance;
use serde::Serialize;
use tauri::{Manager as _, State};
use tauri_plugin_dialog::DialogExt as _;

use crate::engraving::Engraving;
use crate::practice::{AttemptSummary, Ending, LoopOf, LoopRange, NoStart, StartPoint};
use crate::startup::Unavailable;
use crate::state::{Failed, Segno, Shared};

/// The longest title a piece can be given.
const LONGEST_TITLE: usize = 200;

/// Renames an imported Piece; the new title is kept across restarts.
///
/// # Errors
///
/// If the title is blank or too long, the piece is not in the library, it
/// shipped with Segno (those keep their own titles), or the record could
/// not be written.
#[tauri::command]
pub fn rename_piece(
    id: String,
    title: String,
    state: State<'_, Shared>,
) -> Result<PieceSummary, Refused> {
    let mut segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    retitle(&mut segno, &id, &title)
}

/// The body of [`rename_piece`], public so an integration test can drive it.
///
/// # Errors
///
/// As [`rename_piece`].
pub fn retitle(segno: &mut Segno, id: &str, title: &str) -> Result<PieceSummary, Refused> {
    let title = title.trim();
    if title.is_empty() {
        return Err("A title cannot be blank.".to_owned());
    }
    if title.chars().count() > LONGEST_TITLE {
        return Err(format!(
            "A title can be at most {LONGEST_TITLE} characters."
        ));
    }

    let piece_id = segno_domain::PieceId::new(id)
        .ok_or_else(|| format!("`{id}` is not a usable piece name."))?;

    if segno.records().piece_record(&piece_id).is_none() {
        return Err("Pieces that come with Segno keep their own titles.".to_owned());
    }
    segno
        .records()
        .rename_piece(&piece_id, title)
        .map_err(|failure| format!("Could not save the new title: {failure:?}"))?;
    segno.retitle_piece(&piece_id, title);

    segno
        .library()
        .pieces()
        .into_iter()
        .find(|piece| *piece.id() == piece_id)
        .map(|piece| summarise(piece, segno, &crate::history::sessions(segno.records())))
        .ok_or_else(|| format!("There is no piece called `{id}` in the library."))
}

/// How many practice sessions are kept for a piece: what deleting it would
/// take with it.
///
/// # Errors
///
/// If the state is unreadable, or `id` is not a usable name.
#[tauri::command]
pub fn history_count(id: String, state: State<'_, Shared>) -> Result<usize, Refused> {
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;
    let piece_id = segno_domain::PieceId::new(&id)
        .ok_or_else(|| format!("`{id}` is not a usable piece name."))?;

    Ok(segno.records().sessions_of(&piece_id))
}

/// The settings kept, or `None` when there are none yet (the front end then
/// brings over what it kept itself before). A file that can't be read, or
/// one from a newer Segno, gives the defaults and a line in the log.
///
/// # Errors
///
/// If the state is unreadable.
#[tauri::command]
pub fn settings(state: State<'_, Shared>) -> Result<Option<segno_records::Settings>, Refused> {
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;
    Ok(match segno.records().settings() {
        Ok(kept) => kept,
        Err(failure) => {
            crate::log::warn(&format!(
                "settings not read, using the defaults: {failure:?}"
            ));
            Some(segno_records::Settings::default())
        }
    })
}

/// Keeps `settings`, beside the history.
///
/// # Errors
///
/// If the state is unreadable or the file could not be written (logged too).
#[tauri::command]
pub fn save_settings(
    settings: segno_records::Settings,
    state: State<'_, Shared>,
) -> Result<(), Refused> {
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;
    segno.records().keep_settings(&settings).map_err(|failure| {
        let line = format!("settings not saved: {failure:?}");
        crate::log::warn(&line);
        line
    })
}

/// A line from the front end for the log: an error it caught.
#[tauri::command]
pub fn log_line(line: String) {
    crate::log::error(&format!("window: {line}"));
}

/// A piece's Progress page: its sessions bar by bar, where each bar stands
/// now, and what to work on next.
///
/// # Errors
///
/// If the state is unreadable, or there is no such piece.
#[tauri::command]
pub fn progress(
    id: String,
    state: State<'_, Shared>,
) -> Result<crate::history::ProgressView, Refused> {
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;
    let pieces = segno.library().pieces();
    let piece = pieces
        .iter()
        .find(|piece| piece.id().as_str() == id)
        .ok_or_else(|| format!("There is no piece called `{id}` in the library."))?;
    let sessions = crate::history::sessions(segno.records());

    Ok(
        crate::history::progress_in(&sessions, piece.id(), piece.reference())
            .view(piece, crate::history::last_practised(&sessions, piece.id())),
    )
}

/// Deletes an imported Piece: its copy of the file, and every practice
/// session kept for it.
///
/// # Errors
///
/// As [`forget`].
#[tauri::command]
pub fn delete_piece(id: String, state: State<'_, Shared>) -> Result<usize, Refused> {
    let mut segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    forget(&mut segno, &id)
}

/// The body of [`delete_piece`], public so an integration test can drive it.
/// Returns how many practice sessions went with the piece.
///
/// # Errors
///
/// If the piece shipped with Segno (those cannot be deleted), or its
/// files could not be removed.
pub fn forget(segno: &mut Segno, id: &str) -> Result<usize, Refused> {
    let piece_id = segno_domain::PieceId::new(id)
        .ok_or_else(|| format!("`{id}` is not a usable piece name."))?;

    if segno.records().piece_record(&piece_id).is_none() {
        return Err("Pieces that come with Segno cannot be deleted.".to_owned());
    }

    // An attempt still running would otherwise save history for a piece
    // that no longer exists. Nothing running is the ordinary case.
    drop(segno.practice().end_attempt(Ending::Left));

    let forgotten = segno
        .records()
        .forget_piece(&piece_id)
        .map_err(|failure| format!("Could not delete that piece: {failure:?}"))?;
    segno.remove_piece(&piece_id);

    Ok(forgotten)
}

/// A Piece, as the library screen needs it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PieceSummary {
    /// Its durable name, used to ask for it again.
    pub id: String,
    /// What to call it. Never empty.
    pub title: String,
    /// Who wrote it, where the source said.
    pub composer: Option<String>,
    /// How many bars, so the interface can say something about its size
    /// without inventing a difficulty rating (the design rules those out).
    pub bars: usize,
    /// How many notes are expected in total.
    pub notes: usize,
    /// What the performer's file was called, for an imported piece: shown
    /// only to tell apart two pieces with the same title.
    pub file_name: Option<String>,
    /// Whether the performer imported it, rather than it shipping with
    /// Segno: only imported pieces can be renamed or deleted.
    pub imported: bool,
    /// Which starter shelf it goes on; `None` for imported pieces.
    pub level: Option<crate::provided::Level>,
    /// When it was last practised, in Unix milliseconds.
    pub last_practised_ms: Option<u64>,
    /// Up to two of its hardest bars over time, hardest first.
    pub weakest: Vec<u32>,
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
    /// in a webview the score is real elements, so marking a note
    /// is a class on one of them rather than a rectangle computed from a
    /// bounding box.
    pub pages: Vec<String>,
    /// Where every expected note was drawn, in reading order.
    pub marks: Vec<Mark>,
    /// Which drawn element is which bar, so a click can say which bar.
    pub bars: Vec<Bar>,
}

/// A bar as drawn: its element in the SVG, and its number.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bar {
    /// The identifier of the bar's `g.measure` element.
    pub element_id: String,
    /// Its number, as the performer counts bars.
    pub measure: u32,
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
    /// Every note drawn here. Several for a chord.
    ///
    /// Each carries what it sounds as, which is what lets the interface place
    /// a wrong note: relative to a note that was drawn, in the same staff.
    pub notes: Vec<DrawnNote>,
}

/// One drawn notehead, and the note it sounds.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawnNote {
    /// The identifier Verovio gave this notehead.
    pub element_id: String,
    /// The letter name — `"C"` through `"B"`.
    pub step: String,
    /// The octave it sounds in (under an 8va line, not the one it is drawn
    /// in), in scientific pitch notation: middle C is octave 4.
    pub octave: i8,
    /// The MIDI number it sounds as.
    pub pitch: u8,
}

/// Something a command could not do, in words.
///
/// A `String` rather than a type, because everything reaching here has
/// already been turned into an explanation by the crate that knew what went
/// wrong. Re-wrapping it would only add a layer with less information.
pub type Refused = String;

/// The Pieces available to practise.
///
/// # Errors
///
/// If the state is unreadable, which means a previous command panicked while
/// holding it. Reported rather than propagated: a poisoned lock is a defect
/// in Segno, and the performer should be told that rather than shown a
/// crash.
#[tauri::command]
pub fn library(state: State<'_, Shared>) -> Result<Vec<PieceSummary>, Refused> {
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    // Read once for every card, not once per card.
    let sessions = crate::history::sessions(segno.records());
    Ok(segno
        .library()
        .pieces()
        .into_iter()
        .map(|piece| summarise(piece, &segno, &sessions))
        .collect())
}

/// Engraves a Piece and hands over everything needed to draw it.
///
/// `async`, like every command that engraves: Tauri then runs it off the
/// main thread, so the window stays responsive while Verovio works.
///
/// # Errors
///
/// If the piece is not in the library, or the engraver refused it. Both are
/// reported in words the interface can show.
#[tauri::command]
pub async fn open_piece(id: String, state: State<'_, Shared>) -> Result<OpenedPiece, Refused> {
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    opened_piece(id, segno_engraving_verovio::Layout::default(), &segno)
}

/// Re-engraves a Piece to fit the score's width on screen, `width` pixels,
/// at `zoom` percent: the music reflows, so zooming in means fewer bars per
/// line. Zoom is clamped to the supported range here, not trusted from the
/// interface.
///
/// A practice session is keyed on position and pitch, never on an element
/// id, so re-engraving mid-attempt disturbs nothing. The interface gets new
/// `pages` and `marks` together and replaces both.
///
/// # Errors
///
/// As [`open_piece`].
#[tauri::command]
pub async fn engrave_piece(
    id: String,
    zoom: u32,
    width: u32,
    state: State<'_, Shared>,
) -> Result<OpenedPiece, Refused> {
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    opened_piece(
        id,
        segno_engraving_verovio::Layout::fitting(width, zoom),
        &segno,
    )
}

/// The body [`open_piece`] and [`engrave_piece`] share: everything about
/// engraving a Piece except which [`Layout`](segno_engraving_verovio::Layout)
/// to engrave it at.
fn opened_piece(
    id: String,
    layout: segno_engraving_verovio::Layout,
    segno: &crate::state::Segno,
) -> Result<OpenedPiece, Refused> {
    let piece = segno
        .library()
        .pieces()
        .into_iter()
        .find(|piece| piece.id().as_str() == id)
        .ok_or_else(|| format!("There is no piece called `{id}` in the library."))?;

    // Engraved on demand rather than kept from startup. A score's SVG is
    // large and only one is ever on screen; holding every page of every piece
    // for the life of the process would be paying for a piece nobody opened.
    let source = segno.source_of(piece.id()).ok_or_else(|| {
        "The music for this piece could not be found, so it cannot be shown.".to_owned()
    })?;

    let engraved = segno
        .engraving()
        .engrave(&source, layout)
        .map_err(|failure| format!("{failure:?}"))?;

    // Derived again here rather than reused from `Piece::reference()`: the
    // one just engraved is what these element ids actually correspond to,
    // and the design makes the derivation deterministic, so the two are the
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
                            .map(segno_notation::ExpectedNotes::notes)
                            .unwrap_or_default(),
                    )
                    .map(|(element, note)| DrawnNote {
                        element_id: element.as_str().to_owned(),
                        step: format!("{:?}", note.spelling().step()),
                        octave: note.spelling().octave(),
                        pitch: u8::from(note.sounding()),
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
        bars: engraved
            .bars()
            .iter()
            .map(|(element, number)| Bar {
                element_id: element.as_str().to_owned(),
                measure: number.get(),
            })
            .collect(),
    })
}

/// Saves a Piece's original source to a location the performer chooses
/// (the requirement, the tractable slice of it — `MusicXML` only, this round).
///
/// `async` so Tauri runs it off the main thread, where the blocking save
/// dialog would freeze the window (see [`import_musicxml`]). The state lock
/// is released before the dialog shows, so nothing else waits on it.
///
/// # Errors
///
/// If the piece is not in the library, its source could not be found, or the
/// chosen location could not be written to. A cancelled dialog is **not** an
/// error — it resolves successfully, having done nothing.
#[tauri::command]
pub async fn export_musicxml(
    id: String,
    app: tauri::AppHandle,
    state: State<'_, Shared>,
) -> Result<(), Refused> {
    let (title, source) = {
        let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

        let piece_id = segno_domain::PieceId::new(&id)
            .ok_or_else(|| format!("`{id}` is not a usable piece name."))?;

        let piece = segno
            .library()
            .pieces()
            .into_iter()
            .find(|piece| *piece.id() == piece_id)
            .ok_or_else(|| format!("There is no piece called `{id}` in the library."))?;

        let source = segno.source_of(piece.id()).ok_or_else(|| {
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
/// (`.musicxml`) — the same signature `segno-acquisition` and the engraver
/// already sniff for (see `segno_engraving_verovio::engraver`), sniffed
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
    /// The file was identical to a piece already in the library, bundled or
    /// imported: nothing was added, and `piece` is that one.
    pub already: bool,
}

/// Adapts [`Engraving`]'s channel handle to the [`Engraver`] port
/// `segno_acquisition::assess` is written against.
///
/// The mismatch is only in shape, not substance: `Engraving::engrave` takes
/// `&self` and a `Layout` because it just posts to a channel and the engine
/// on the other end holds the layout state; `Engraver::engrave` takes
/// `&mut self` and no layout because a port assumes an engine that holds the
/// document it is working on. Import has no layout of its own —
/// the Library does not keep one — so [`Layout::default`] is the same choice
/// every other command already makes when opening a piece for the first
/// time, not a placeholder invented for this.
struct ImportEngraver<'a>(&'a Engraving);

impl Engraver for ImportEngraver<'_> {
    fn engrave(&mut self, source: &[u8]) -> Result<EngravedScore, EngravingFailure> {
        self.0
            .engrave(source, segno_engraving_verovio::Layout::default())
    }
}

/// Imports a `MusicXML` file the performer chooses, joining it to the
/// library.
///
/// `async` so Tauri runs it off the main thread: the file dialog blocks
/// until answered, and its answer arrives through the main thread, so
/// waiting for it there would freeze the window. The state lock is only
/// taken after the dialog returns.
///
/// # Errors
///
/// If the chosen file could not be read, or [`segno_acquisition::assess`]
/// refused it — in which case the error is the refusal's reason and route,
/// verbatim: this command adds no wording of its own to an
/// explanation that is already complete.
#[tauri::command]
pub async fn import_musicxml(
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

    let mut segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());

    accept_offered(&material, file_name.as_deref(), &mut segno).map(Some)
}

/// The body [`import_musicxml`] runs once it has bytes and a locked
/// [`Segno`] — separated out, and public, so an integration test can drive
/// it without a real dialog or `AppHandle` (see `lib.rs`'s module doc).
///
/// # Errors
///
/// As [`import_musicxml`]: the offered material was refused, verbatim from
/// [`segno_acquisition::assess`].
pub fn accept_offered(
    material: &[u8],
    file_name: Option<&str>,
    segno: &mut Segno,
) -> Result<ImportedPiece, Refused> {
    let id = crate::provided::identify(material);
    // A bundled piece's bytes are known by its tag, not their fingerprint.
    let known = crate::provided::PROVIDED
        .iter()
        .find(|provided| crate::provided::identify(provided.source) == id)
        .map_or_else(|| id.clone(), crate::provided::Provided::id);

    // Re-offering the same bytes is not a second piece: `PieceId` is the
    // lookup key `source_of` and `open_piece` use, and a duplicate under one
    // id would make that lookup ambiguous rather than merely wasteful.
    if let Some(already) = segno
        .library()
        .pieces()
        .into_iter()
        .find(|piece| *piece.id() == known)
    {
        return Ok(ImportedPiece {
            piece: summarise(already, segno, &crate::history::sessions(segno.records())),
            lines: vec!["This piece is already in your library.".to_owned()],
            already: true,
        });
    }

    let mut engraver = ImportEngraver(segno.engraving());

    match segno_acquisition::assess(material, id.clone(), file_name, &mut engraver) {
        segno_acquisition::Outcome::NotAccepted(refused) => {
            Err(format!("{}\n\n{}", refused.reason(), refused.route()))
        }
        segno_acquisition::Outcome::Accepted { piece, report } => {
            segno
                .records()
                .keep_piece(&id, piece.title(), piece.composer(), file_name, material)
                .map_err(|failure| format!("Could not keep that piece: {failure:?}"))?;

            let summary = summarise(&piece, segno, &[]);
            segno.accept_piece(*piece);

            Ok(ImportedPiece {
                piece: summary,
                lines: report.lines(),
                already: false,
            })
        }
    }
}

/// A Piece, as the library screen needs it — the same shape [`library`]
/// produces, so an import and a startup listing agree on what a summary is.
fn summarise(
    piece: &segno_library::Piece,
    segno: &Segno,
    sessions: &[segno_records::SessionRecord],
) -> PieceSummary {
    let record = segno.records().piece_record(piece.id());

    PieceSummary {
        id: piece.id().as_str().to_owned(),
        title: piece.title().to_owned(),
        composer: piece.composer().map(str::to_owned),
        bars: piece.score().measures().len(),
        notes: piece.reference().len(),
        imported: record.is_some(),
        file_name: record.and_then(|record| record.file_name),
        level: crate::provided::level_of(piece.id()),
        last_practised_ms: crate::history::last_practised(sessions, piece.id()),
        weakest: crate::history::progress_in(sessions, piece.id(), piece.reference()).weakest(),
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

/// The MIDI instruments currently connected.
///
/// # Errors
///
/// If none are connected, or the practice thread has gone — either way, in
/// words a performer can read (`NoInstrument::describe`).
#[tauri::command]
pub fn instruments(state: State<'_, Shared>) -> Result<Vec<InstrumentSummary>, Refused> {
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    segno
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

/// Begins an attempt at `piece_id`, on `instrument`.
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
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    let id = segno_domain::PieceId::new(&piece_id)
        .ok_or_else(|| format!("`{piece_id}` is not a usable piece name."))?;

    let piece = segno
        .library()
        .pieces()
        .into_iter()
        .find(|piece| *piece.id() == id)
        .ok_or_else(|| format!("There is no piece called `{piece_id}` in the library."))?;

    let reference = std::sync::Arc::new(piece.reference().clone());
    let named = segno_capture::Instrument::new(instrument);

    segno
        .practice()
        .start_attempt(id, reference, named)
        .map_err(|absent| absent.describe())
}

/// Ends whatever attempt is running, analyses it, and keeps the record.
///
/// # Errors
///
/// If no attempt is running.
#[tauri::command]
pub fn end_attempt(ending: Ending, state: State<'_, Shared>) -> Result<AttemptSummary, Refused> {
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    segno
        .practice()
        .end_attempt(ending)
        .map_err(|_| "No attempt is in progress.".to_owned())
}

/// Moves where attempts at `piece_id` start to bar `measure`, its first
/// occurrence in played order. While practising, the running attempt is
/// ended and kept, and a new one begins there.
///
/// # Errors
///
/// If the piece or bar is unknown, there is nothing to start at, or the
/// keyboard is lost.
#[tauri::command]
pub fn start_at_bar(
    piece_id: String,
    measure: u32,
    state: State<'_, Shared>,
) -> Result<StartPoint, Refused> {
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    let (id, reference) = reference_of(&segno, &piece_id)?;
    let bar = segno_domain::MeasureNumber::new(measure)
        .ok_or_else(|| format!("There is no bar {measure}."))?;

    segno
        .practice()
        .start_at_bar(id, reference, bar)
        .map_err(|refused| match refused {
            NoStart::NothingThere => format!("There is nothing to start at from bar {measure}."),
            NoStart::InstrumentLost => LOST.to_owned(),
        })
}

/// Loops bars `first` to `last` (either order) of `piece_id`, until
/// `target` clean passes in a row, or for as long as the performer likes if
/// `None`. While practising, the running attempt is ended and kept, and the
/// first pass begins.
///
/// # Errors
///
/// If the piece or bars are unknown, the bars are never played in a row or
/// have nothing to strike, or the keyboard is lost.
#[tauri::command]
pub fn loop_bars(
    piece_id: String,
    first: u32,
    last: u32,
    target: Option<u32>,
    state: State<'_, Shared>,
) -> Result<LoopRange, Refused> {
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    let (id, reference) = reference_of(&segno, &piece_id)?;
    let bar = |number: u32| {
        segno_domain::MeasureNumber::new(number).ok_or_else(|| format!("There is no bar {number}."))
    };
    let wanted = LoopOf {
        first: bar(first)?,
        last: bar(last)?,
        target,
    };

    segno
        .practice()
        .loop_bars(id, reference, wanted)
        .map_err(|refused| match refused {
            NoStart::NothingThere if first == last => {
                format!("There is nothing to loop in bar {first}.")
            }
            NoStart::NothingThere => format!(
                "Bars {}–{} aren't played in a row.",
                first.min(last),
                first.max(last)
            ),
            NoStart::InstrumentLost => LOST.to_owned(),
        })
}

/// Stops looping; the start point stays at the loop's start.
///
/// # Errors
///
/// Only if the application state is unusable.
#[tauri::command]
pub fn clear_loop(state: State<'_, Shared>) -> Result<(), Refused> {
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;
    segno.practice().clear_loop();
    Ok(())
}

/// How many clean passes in a row end the loop from now on; `None` never.
///
/// # Errors
///
/// Only if the application state is unusable.
#[tauri::command]
pub fn loop_target(target: Option<u32>, state: State<'_, Shared>) -> Result<(), Refused> {
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;
    segno.practice().loop_target(target);
    Ok(())
}

/// Why the start cannot move while the keyboard is lost.
const LOST: &str = "The keyboard has stopped reporting; wait for it before moving.";

/// The piece called `piece_id`, and what playing it expects.
fn reference_of(
    segno: &Segno,
    piece_id: &str,
) -> Result<(segno_domain::PieceId, std::sync::Arc<ReferencePerformance>), Refused> {
    let id = segno_domain::PieceId::new(piece_id)
        .ok_or_else(|| format!("`{piece_id}` is not a usable piece name."))?;
    let piece = segno
        .library()
        .pieces()
        .into_iter()
        .find(|piece| *piece.id() == id)
        .ok_or_else(|| format!("There is no piece called `{piece_id}` in the library."))?;
    let reference = std::sync::Arc::new(piece.reference().clone());
    Ok((id, reference))
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
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    segno
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
/// forwarded, byte for byte, to `output` — instead of Segno making its own
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
    let segno = state.inner().0.lock().map_err(|_| POISONED.to_owned())?;

    segno
        .practice()
        .set_thru(
            input.map(segno_capture::Instrument::new),
            output.map(segno_midi::OutputPort::new),
        )
        .map_err(|absent| absent.describe())
}

/// What to say when the state cannot be read.
const POISONED: &str = "Segno got into a state it cannot recover from. Restarting it should clear this; \
     your practice history is on disk and is unaffected.";

#[cfg(test)]
mod tests {
    use super::extension_of;

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
}
