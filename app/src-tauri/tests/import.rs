//! Importing a `MusicXML` file, against a real engraving engine.
//!
//! `#[ignore]`d and run with `just provided-test`'s sibling recipe, for the
//! same reason as `tests/provided.rs`: Verovio is loaded at run
//! time, so these need `HOGWASH_VEROVIO_LIB`/`HOGWASH_VEROVIO_DATA` set.

#![expect(
    clippy::expect_used,
    reason = "test helpers; clippy's allow-expect-in-tests cannot see a tests/ helper fn"
)]

use std::path::PathBuf;

use hogwash_app_lib::commands::{accept_offered, forget, retitle};

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

// importing a well-formed, supported MusicXML file produces a
// corresponding Piece, and it joins the pieces already on the shelf rather
// than replacing them.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_supported_file_joins_the_library() {
    let root = temporary("joins");
    let mut hogwash = hogwash(&root);
    let before = hogwash.library().count();

    let imported =
        accept_offered(&fixture(), None, &mut hogwash).expect("the fixture is supported");

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

    let imported = accept_offered(&material, None, &mut hogwash).expect("the fixture is supported");

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
    let mut running = hogwash(&root);
    let material = fixture();

    accept_offered(&material, None, &mut running).expect("the fixture is supported");
    let before = running.library().count();

    let second = accept_offered(&material, None, &mut running).expect("re-import is not a refusal");

    assert_eq!(running.library().count(), before);
    assert_eq!(second.lines, ["This piece is already in your library."]);
    assert!(second.already);

    // Still one piece after a restart: identical bytes are stored once.
    assert_eq!(hogwash(&root).library().count(), before);
}

// content in an unsupported format is refused, not partially
// accepted, and the refusal is something a performer can read: it names the
// format and says what to do instead, verbatim from `hogwash-acquisition`.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn an_unsupported_format_is_refused_with_a_route() {
    let root = temporary("refused");
    let mut hogwash = hogwash(&root);
    let before = hogwash.library().count();

    let refusal = accept_offered(b"%PDF-1.7 not music", None, &mut hogwash)
        .expect_err("a PDF is not supported");

    assert!(refusal.to_lowercase().contains("pdf"));
    assert_eq!(hogwash.library().count(), before);
    assert!(
        hogwash.records().imported().is_empty(),
        "a refused file is never stored"
    );
}

// A file that looks like MusicXML but cannot be read (the performer's real
// Canon in D, cut off half way, as a failed download would be) is refused
// before anything is copied into the data folder.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_broken_file_is_refused_and_never_stored() {
    let root = temporary("broken");
    let mut running = hogwash(&root);
    let canon = library_file("Canon_in_D.mxl");

    let refusal = accept_offered(
        &canon[..canon.len() / 2],
        Some("Canon_in_D.mxl"),
        &mut running,
    )
    .expect_err("half a file is not music");

    assert!(refusal.contains("incomplete or damaged"), "{refusal}");
    assert!(
        running.records().imported().is_empty(),
        "nothing was stored"
    );

    // And the engine is still fine for the next file.
    accept_offered(&canon, Some("Canon_in_D.mxl"), &mut running).expect("the whole file imports");
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

    let imported = accept_offered(&material, None, &mut hogwash).expect("a real .mxl is supported");

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
        let imported = accept_offered(&library_file(file), None, &mut hogwash).expect("supported");

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
        None,
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

// A file identical to a bundled piece is that piece: nothing is added and
// nothing is stored.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_file_identical_to_a_bundled_piece_adds_nothing() {
    let root = temporary("bundled-twin");
    let mut hogwash = hogwash(&root);
    let before = hogwash.library().count();

    let imported = accept_offered(&library_file("minuet-in-g.musicxml"), None, &mut hogwash)
        .expect("the Minuet is supported");

    assert!(imported.already);
    assert_eq!(imported.piece.title, "Minuet in G Major");
    assert_eq!(hogwash.library().count(), before);
    assert!(
        hogwash.records().imported().is_empty(),
        "nothing was stored"
    );
}

// Identical files that got into the data folder under two names (copied by
// hand, say) are one piece after a restart, and the spare copy is removed.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn identical_stored_copies_collapse_at_startup() {
    let root = temporary("collapse");
    let before = hogwash(&root).library().count();
    let imported = accept_offered(&library_file("Canon_in_D.mxl"), None, &mut hogwash(&root))
        .expect("supported");
    let scores = root.join("scores");
    std::fs::copy(
        scores.join(format!("{}.musicxml", imported.piece.id)),
        scores.join("hand-copied.musicxml"),
    )
    .expect("copies the file");

    let restarted = hogwash(&root);

    assert_eq!(restarted.library().count(), before + 1);
    assert!(!scores.join("hand-copied.musicxml").exists());
}

// The performer's two real Für Elise files are different music, so both are
// kept, each with what its file says about itself.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn two_real_fur_elise_files_are_two_pieces() {
    let root = temporary("two-fur-elise");
    let mut hogwash = hogwash(&root);
    let before = hogwash.library().count();

    let full = accept_offered(
        &library_file("Fur_Elise.mxl"),
        Some("Fur_Elise.mxl"),
        &mut hogwash,
    );
    let easy = accept_offered(
        &library_file("Fur_Elise_Easy.mxl"),
        Some("Fur_Elise_Easy.mxl"),
        &mut hogwash,
    )
    .expect("supported");

    // Its pickup bar is numbered 0, which is refused until pickup bars are
    // supported; flip this when they are.
    assert!(full.is_err_and(|refusal| refusal.contains("numbered `0`")));
    assert_eq!(hogwash.library().count(), before + 1);
    assert_eq!(easy.piece.title, "Fur Elise");
    assert_eq!(easy.piece.composer, None);
    assert_eq!(easy.piece.file_name.as_deref(), Some("Fur_Elise_Easy.mxl"));
}

// A score that names itself nowhere (the Minuet with its title, composer and
// printed credits taken out) is named after the performer's file.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_score_with_no_title_anywhere_is_named_after_its_file() {
    let root = temporary("file-title");
    let mut hogwash = hogwash(&root);
    let minuet = String::from_utf8(library_file("minuet-in-g.musicxml")).expect("text");
    let nameless = cut_all(
        &cut_all(
            &cut_all(&minuet, "<work>", "</work>"),
            "<credit ",
            "</credit>",
        ),
        "<identification>",
        "</identification>",
    );

    let imported = accept_offered(
        nameless.as_bytes(),
        Some("Petzold_Minuet_in_G.musicxml"),
        &mut hogwash,
    )
    .expect("still music");

    assert_eq!(imported.piece.title, "Petzold Minuet in G");
}

/// `text` with every span from `start` to `end` (inclusive) removed.
fn cut_all(text: &str, start: &str, end: &str) -> String {
    let mut out = text.to_owned();
    while let Some(from) = out.find(start) {
        let Some(length) = out[from..].find(end) else {
            break;
        };
        out.replace_range(from..from + length + end.len(), "");
    }
    out
}

// A title the performer chose is kept across a restart, and wins over the
// one in the file; a bundled piece keeps its own.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_renamed_piece_keeps_its_title_after_a_restart() {
    let root = temporary("rename");
    let mut running = hogwash(&root);
    let canon = accept_offered(
        &library_file("Canon_in_D.mxl"),
        Some("Canon_in_D.mxl"),
        &mut running,
    )
    .expect("supported")
    .piece;
    let minuet = running.library().pieces()[0].id().as_str().to_owned();

    let renamed = retitle(&mut running, &canon.id, "  Pachelbel's Canon ").expect("renames it");
    assert_eq!(renamed.title, "Pachelbel's Canon");
    assert!(
        retitle(&mut running, &canon.id, "   ").is_err(),
        "blank is refused"
    );
    assert!(
        retitle(&mut running, &minuet, "Mine now").is_err(),
        "bundled keeps its title"
    );

    let restarted = hogwash(&root);
    assert!(
        restarted
            .library()
            .pieces()
            .into_iter()
            .any(|piece| piece.id().as_str() == canon.id && piece.title() == "Pachelbel's Canon")
    );
}

// Deleting an imported piece removes it and its practice history for good;
// a bundled piece cannot be deleted.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_deleted_piece_and_its_history_are_gone_after_a_restart() {
    let root = temporary("delete");
    let mut running = hogwash(&root);
    let before = running.library().count();
    let canon = accept_offered(&library_file("Canon_in_D.mxl"), None, &mut running)
        .expect("supported")
        .piece;
    let canon_id = hogwash_domain::PieceId::new(&canon.id).expect("usable");
    running
        .records()
        .keep(&hogwash_records::SessionRecord {
            id: "practised-once".to_owned(),
            piece: canon_id.clone(),
            started: "1".to_owned(),
            closed: true,
            attempts: Vec::new(),
        })
        .expect("keeps a session");
    let minuet = running.library().pieces()[0].id().as_str().to_owned();

    assert!(
        forget(&mut running, &minuet).is_err(),
        "bundled pieces stay"
    );
    assert_eq!(forget(&mut running, &canon.id).expect("deletes it"), 1);
    assert_eq!(running.library().count(), before);

    let restarted = hogwash(&root);
    assert_eq!(restarted.library().count(), before);
    assert_eq!(restarted.records().sessions_of(&canon_id), 0);
}
