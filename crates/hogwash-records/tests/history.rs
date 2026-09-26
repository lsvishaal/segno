//! Practice history, across a restart.
//!
//! Every test here writes to a real temporary directory and reads back
//! through a **fresh** [`Records`]. That is the point: NFR-100 and FR-704 are
//! about surviving a restart, and a test that kept anything in memory between
//! the writing and the reading would not be testing the thing they ask for.

#![expect(
    clippy::expect_used,
    reason = "test helpers; clippy's allow-expect-in-tests cannot see a tests/ helper fn"
)]

use std::path::Path;

use hogwash_capture::ObservedPerformance;
use hogwash_domain::{PieceId, Pitch};
use hogwash_records::{AttemptRecord, Records, RecordsFailure, SessionRecord};

fn piece() -> PieceId {
    PieceId::new("minuet-in-g").expect("a plain name is usable")
}

fn played(notes: &[u8]) -> ObservedPerformance {
    let mut observed = ObservedPerformance::new();

    for &note in notes {
        observed.record(Pitch::new(note).expect("fixture notes are in range"));
    }

    observed
}

fn session(id: &str) -> SessionRecord {
    SessionRecord {
        id: id.to_owned(),
        piece: piece(),
        started: "2026-08-03T19:00:00Z".to_owned(),
        closed: false,
        attempts: vec![
            AttemptRecord {
                played: played(&[60, 62, 64]),
                reached: 3,
            },
            AttemptRecord {
                played: played(&[60, 65]),
                reached: 1,
            },
        ],
    }
}

/// Reads back through a new `Records`, as a restart would.
fn reopened(root: &Path) -> Vec<SessionRecord> {
    let (sessions, unreadable) = Records::under(root)
        .sessions()
        .expect("the history is readable");

    assert!(
        unreadable.is_empty(),
        "records should have been readable: {unreadable:?}"
    );

    sessions
}

// FR-700, FR-702, NFR-100 — history survives a restart, intact.
#[test]
fn a_session_survives_being_written_and_read_back() {
    let directory = tempfile::tempdir().expect("a temporary directory");

    Records::under(directory.path())
        .keep(&session("one"))
        .expect("keeps");

    assert_eq!(reopened(directory.path()), [session("one")]);
}

// 003 §7.2 — a Session holds its Attempts in the order they were made. If
// storage reordered them, "your second attempt" would mean something
// different after a restart than before it.
#[test]
fn attempts_come_back_in_the_order_they_were_made() {
    let directory = tempfile::tempdir().expect("a temporary directory");

    Records::under(directory.path())
        .keep(&session("one"))
        .expect("keeps");

    let attempts = &reopened(directory.path())[0].attempts;

    assert_eq!(attempts[0].played.played().len(), 3);
    assert_eq!(attempts[1].played.played().len(), 2);
}

// FR-702 — "the state of any unclosed practice session" is recovered. A
// session the performer walked away from is exactly the one worth resuming,
// so whether it was closed has to survive.
#[test]
fn whether_a_session_was_closed_survives() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let records = Records::under(directory.path());

    let mut closed = session("closed");
    closed.closed = true;

    records.keep(&session("open")).expect("keeps");
    records.keep(&closed).expect("keeps");

    let states: Vec<bool> = reopened(directory.path())
        .iter()
        .map(|session| session.closed)
        .collect();

    // Sorted by name: "closed" before "open".
    assert_eq!(states, [true, false]);
}

// FR-704 — integrity through normal operation. Practising again replaces the
// session rather than accumulating copies of it.
#[test]
fn keeping_a_session_again_replaces_it() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let records = Records::under(directory.path());

    records.keep(&session("one")).expect("keeps");

    let mut later = session("one");
    later.attempts.push(AttemptRecord {
        played: played(&[67]),
        reached: 1,
    });
    records.keep(&later).expect("keeps again");

    let sessions = reopened(directory.path());

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].attempts.len(), 3);
}

// 004 §8.9 — prior records remain intact even when the most recent one does
// not. One corrupt file must not make a performer's whole history
// unreachable, so it is reported *alongside* the rest rather than instead.
#[test]
fn one_damaged_record_does_not_hide_the_others() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let records = Records::under(directory.path());

    records.keep(&session("good")).expect("keeps");
    std::fs::write(
        directory.path().join("history").join("bad.json"),
        "{ not json",
    )
    .expect("writes the damaged one");

    let (sessions, unreadable) = records.sessions().expect("the directory is readable");

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, "good");
    assert!(matches!(
        unreadable.as_slice(),
        [RecordsFailure::Unreadable { .. }]
    ));
}

// A record written by a later version is refused rather than read as best it
// can be. Reporting a half-understood record as understood tells a performer
// the wrong thing about their own practice.
#[test]
fn a_record_from_a_newer_version_is_refused_rather_than_guessed_at() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let records = Records::under(directory.path());

    records.keep(&session("one")).expect("keeps");

    let path = directory.path().join("history").join("one.json");
    let text = std::fs::read_to_string(&path).expect("reads");
    std::fs::write(&path, text.replace("\"schema\": 1", "\"schema\": 99")).expect("writes");

    let (sessions, unreadable) = records.sessions().expect("the directory is readable");

    assert!(sessions.is_empty());
    assert!(matches!(
        unreadable.as_slice(),
        [RecordsFailure::FromTheFuture { schema: 99, .. }]
    ));
}

// Nothing practised yet is not an error. A first run must not look like a
// failure.
#[test]
fn a_history_that_does_not_exist_yet_is_empty_rather_than_broken() {
    let directory = tempfile::tempdir().expect("a temporary directory");

    let (sessions, unreadable) = Records::under(&directory.path().join("nothing-here"))
        .sessions()
        .expect("an absent history is readable");

    assert!(sessions.is_empty());
    assert!(unreadable.is_empty());
}

// FR-701, FR-703 — an imported score is kept locally, read-only, and apart
// from practice history. Kept where it was found, a piece would vanish the
// day the performer tidied their downloads folder.
#[test]
fn an_imported_score_is_copied_into_the_library_and_kept_apart_from_history() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let records = Records::under(directory.path());

    let kept = records
        .keep_piece(
            &piece(),
            "Minuet in G",
            Some("Petzold"),
            Some("Minuet_in_G.musicxml"),
            b"<score-partwise/>",
        )
        .expect("keeps the piece");

    assert!(kept.source.starts_with(records.scores()));
    assert!(!kept.source.starts_with(records.history()));
    assert_eq!(
        std::fs::read(&kept.source).expect("reads the score"),
        b"<score-partwise/>"
    );

    // What the performer's file was called survives a restart.
    assert_eq!(
        Records::under(directory.path())
            .piece_record(&piece())
            .and_then(|record| record.file_name)
            .as_deref(),
        Some("Minuet_in_G.musicxml")
    );

    // FR-701 — "in read-only form".
    assert!(
        std::fs::metadata(&kept.source)
            .expect("reads the metadata")
            .permissions()
            .readonly()
    );
}

// A restart has to find every imported piece again, from disk alone.
#[test]
fn imported_pieces_are_listed_after_a_restart() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let minuet = PieceId::new("b-minuet").expect("usable");
    let prelude = PieceId::new("a-prelude").expect("usable");

    let records = Records::under(directory.path());
    assert!(records.imported().is_empty(), "nothing imported yet");
    records
        .keep_piece(&minuet, "Minuet in G", None, None, b"<score-partwise/>")
        .expect("keeps the minuet");
    records
        .keep_piece(&prelude, "Prelude in C", None, None, b"<score-partwise/>")
        .expect("keeps the prelude");

    assert_eq!(
        Records::under(directory.path()).imported(),
        [prelude, minuet]
    );
}

// FR-406 in spirit: a directory's ordering is the filesystem's business, and
// two runs must not disagree about what order a performer's sessions are in.
#[test]
fn sessions_come_back_in_a_stable_order() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let records = Records::under(directory.path());

    for id in ["ccc", "aaa", "bbb"] {
        records.keep(&session(id)).expect("keeps");
    }

    let names: Vec<String> = reopened(directory.path())
        .iter()
        .map(|session| session.id.clone())
        .collect();

    assert_eq!(names, ["aaa", "bbb", "ccc"]);
}

// Grace notes and ornament notes are tolerated, never mistakes. What is kept
// is every note played, and an analysis of it read back after a restart must
// still count none of them as wrong or unexpected -- later per-bar statistics
// are built from exactly these counts.
#[test]
fn tolerated_notes_are_not_mistakes_in_a_kept_attempt() {
    use hogwash_analysis::Analysis;
    use hogwash_domain::{MeasureNumber, PitchSpelling, Step};
    use hogwash_notation::{Measure, Note, Position, ReferencePerformance, Score};

    let note = |step| Note::new(PitchSpelling::new(step, 0, 4)).expect("sounds");
    let pitch = |number| Pitch::new(number).expect("in range");
    // C, then D with a grace G before it, then E with a trill.
    let reference = ReferencePerformance::derived_from(
        &Score::new(vec![Measure::new(
            MeasureNumber::FIRST,
            vec![
                Position::new(vec![note(Step::C)]).expect("a note"),
                Position::new(vec![note(Step::D)])
                    .expect("a note")
                    .tolerating([pitch(67)]),
                Position::new(vec![note(Step::E)])
                    .expect("a note")
                    .tolerating((62..=66).map(pitch)),
            ],
        )])
        .expect("a score"),
    );

    let directory = tempfile::tempdir().expect("a temporary directory");
    let mut kept = session("tolerated");
    // C, grace G, D, trilled E (E F E F), and the trill's tail after the end.
    kept.attempts = vec![AttemptRecord {
        played: played(&[60, 67, 62, 64, 65, 64, 65]),
        reached: 3,
    }];
    Records::under(directory.path())
        .keep(&kept)
        .expect("the session is kept");

    let back = reopened(directory.path());
    let summary = Analysis::of(&back[0].attempts[0].played, &reference).summary();

    assert_eq!(summary.incorrect(), 0);
    assert_eq!(summary.unexpected(), 0);
    assert_eq!(summary.correct(), 3);
}
