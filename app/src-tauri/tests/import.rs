//! Importing a `MusicXML` file, against a real engraving engine.
//!
//! `#[ignore]`d and run with `just provided-test`'s sibling recipe, for the
//! same reason as `tests/provided.rs` (`ADR-0005`): Verovio is loaded at run
//! time, so these need `HOGWASH_VEROVIO_LIB`/`HOGWASH_VEROVIO_DATA` set.

#![expect(
    clippy::expect_used,
    reason = "test helpers; clippy's allow-expect-in-tests cannot see a tests/ helper fn"
)]

use std::path::PathBuf;

use hogwash_app_lib::commands::accept_offered;

/// A fresh directory for one test. Emptied first: imported files are kept
/// read-only, so a previous run's copy would refuse to be written again.
fn temporary(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("hogwash-import-test-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn hogwash(root: &std::path::Path) -> hogwash_app_lib::state::Hogwash {
    let engraving = hogwash_app_lib::engraving::Engraving::start().expect("the engine starts");
    let records = hogwash_records::Records::under(root);
    let practice = hogwash_app_lib::practice::Practice::start(records.clone(), |_| {})
        .expect("the practice thread starts");

    hogwash_app_lib::state::Hogwash::start(engraving, records, practice)
        .expect("the provided material prepares")
}

fn fixture() -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/hogwash-acquisition/tests/fixtures/two-bars.musicxml"),
    )
    .expect("the acquisition fixture ships in the repo")
}

// FR-100 — importing a well-formed, supported MusicXML file produces a
// corresponding Piece, and it joins the pieces already on the shelf rather
// than replacing them.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_supported_file_joins_the_library() {
    let root = temporary("joins");
    let mut hogwash = hogwash(&root);
    let before = hogwash.library().count();

    let imported = accept_offered(&fixture(), &mut hogwash).expect("the fixture is supported");

    assert_eq!(hogwash.library().count(), before + 1);
    assert!(
        hogwash
            .library()
            .pieces()
            .into_iter()
            .any(|piece| piece.id().as_str() == imported.piece.id)
    );
}

// The imported piece's source round-trips through `Records`: what `source_of`
// returns for its id is what was actually imported, which is the seam
// `open_piece` and `export_musicxml` both rely on afterwards.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn the_imported_source_can_be_read_back() {
    let root = temporary("round-trips");
    let mut hogwash = hogwash(&root);
    let material = fixture();

    let imported = accept_offered(&material, &mut hogwash).expect("the fixture is supported");

    let id = hogwash_domain::PieceId::new(&imported.piece.id).expect("a usable name");
    assert_eq!(hogwash.source_of(&id), Some(material));
}

// Re-offering the same bytes does not accumulate a duplicate: `PieceId` is a
// lookup key, and two Pieces under one id would make `open_piece`'s lookup
// ambiguous.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn importing_the_same_bytes_twice_does_not_duplicate() {
    let root = temporary("dedup");
    let mut hogwash = hogwash(&root);
    let material = fixture();

    accept_offered(&material, &mut hogwash).expect("the fixture is supported");
    let before = hogwash.library().count();

    let second = accept_offered(&material, &mut hogwash).expect("re-import is not a refusal");

    assert_eq!(hogwash.library().count(), before);
    assert_eq!(second.lines, ["This piece is already in your library."]);
}

// FR-104 — content in an unsupported format is refused, not partially
// accepted, and the refusal is something a performer can read: it names the
// format and says what to do instead, verbatim from `hogwash-acquisition`.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn an_unsupported_format_is_refused_with_a_route() {
    let root = temporary("refused");
    let mut hogwash = hogwash(&root);
    let before = hogwash.library().count();

    let refusal =
        accept_offered(b"%PDF-1.7 not music", &mut hogwash).expect_err("a PDF is not supported");

    assert!(refusal.to_lowercase().contains("pdf"));
    assert_eq!(hogwash.library().count(), before);
}

// A real compressed MusicXML (.mxl) file as MuseScore exports it: the
// performer's own Für Elise (easy), imported the way the dialog imports it.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_real_compressed_musicxml_file_joins_the_library() {
    let root = temporary("real-mxl");
    let mut hogwash = hogwash(&root);
    let before = hogwash.library().count();
    let material =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/Fur_Elise_Easy.mxl"))
            .expect("the file is in the library folder");

    let imported = accept_offered(&material, &mut hogwash).expect("a real .mxl is supported");

    assert_eq!(hogwash.library().count(), before + 1);
    assert_eq!(imported.piece.title, "Fur Elise");
}

// Real files with no <work-title>, only the title and composer printed on the
// page (MuseScore 1/2 exports): read the way the page reads.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_title_printed_only_on_the_page_is_read() {
    let root = temporary("printed-titles");
    let mut hogwash = hogwash(&root);

    for (file, title, composer) in [
        (
            "Erik_Satie_-_Gymnopedie_No.1.mxl",
            "Gymnopédie No.1",
            "Erik Satie",
        ),
        (
            "Ode_to_Joy_Easy_variation.mxl",
            "Ode to Joy",
            "Ludwig Van Beethoven",
        ),
    ] {
        let imported = accept_offered(&library_file(file), &mut hogwash).expect("supported");

        assert_eq!(imported.piece.title, title);
        assert_eq!(imported.piece.composer.as_deref(), Some(composer));
    }
}

// An imported piece is still there after a restart: a fresh Hogwash on the
// same data folder finds it on disk.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn an_imported_piece_survives_a_restart() {
    let root = temporary("restart");
    let imported = accept_offered(
        &library_file("Erik_Satie_-_Gymnopedie_No.1.mxl"),
        &mut hogwash(&root),
    )
    .expect("supported");

    let restarted = hogwash(&root);

    assert!(restarted.library().pieces().into_iter().any(|piece| {
        piece.id().as_str() == imported.piece.id && piece.title() == "Gymnopédie No.1"
    }));
}

fn library_file(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("library")
            .join(name),
    )
    .expect("the file is in the library folder")
}
