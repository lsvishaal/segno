//! The session file format, pinned.
//!
//! A performer's history outlives any one version of Hogwash, so what is on
//! disk is checked against files kept here: one written by schema 1, which
//! must always load, and one in today's shape, which must load to exactly
//! the record it describes and be written back byte for byte. A change to
//! the format that was not meant fails here first.
//!
//! The notes are the opening of the Minuet in G: D5 G4 A4 B4 as written,
//! and in schema 2 a wrong F#4 before the A4.

#![expect(
    clippy::expect_used,
    reason = "test helpers; clippy's allow-expect-in-tests cannot see a tests/ helper fn"
)]

use std::path::Path;

use hogwash_capture::{ObservedPerformance, Touch};
use hogwash_domain::{PieceId, Pitch};
use hogwash_records::{AttemptRecord, Records, RecordsFailure, SessionRecord};

const SCHEMA_1: &str = include_str!("fixtures/session-schema-1.json");
const SCHEMA_2: &str = include_str!("fixtures/session-schema-2.json");

fn key(number: u8) -> Pitch {
    Pitch::new(number).expect("a piano key")
}

const fn touch(at_us: u64, velocity: u8, released_us: Option<u64>) -> Touch {
    Touch {
        at_us,
        velocity,
        released_us,
    }
}

/// Reads `text` back as the only session in a fresh history.
fn read(text: &str, id: &str) -> Result<SessionRecord, RecordsFailure> {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let history = directory.path().join("history");
    std::fs::create_dir_all(&history).expect("a history folder");
    std::fs::write(history.join(format!("{id}.json")), text).expect("writes");

    let (mut sessions, mut unreadable) = Records::under(directory.path())
        .sessions()
        .expect("the history is readable");
    assert_eq!(
        sessions.len() + unreadable.len(),
        1,
        "expected one session or one failure"
    );
    unreadable
        .pop()
        .map_or_else(|| Ok(sessions.pop().expect("one session")), Err)
}

fn written(session: &SessionRecord, root: &Path) -> String {
    Records::under(root).keep(session).expect("keeps");
    std::fs::read_to_string(root.join("history").join(format!("{}.json", session.id)))
        .expect("reads back")
}

fn schema_2_record() -> SessionRecord {
    let mut played = ObservedPerformance::new();
    played.record_touched(key(74), touch(0, 80, Some(410_000)));
    played.pedal(300_000, true);
    played.record_touched(key(67), touch(512_000, 72, Some(990_000)));
    played.record_touched(key(66), touch(1_040_000, 58, Some(1_100_000)));
    played.pedal(1_200_000, false);
    played.record_touched(key(69), touch(1_350_000, 90, None));

    SessionRecord {
        id: "minuet-1792270830806".to_owned(),
        piece: PieceId::new("minuet").expect("a plain name"),
        started: "1792270830806".to_owned(),
        closed: false,
        attempts: vec![AttemptRecord {
            began_ms: Some(1_792_270_831_002),
            played,
            reached: 3,
        }],
    }
}

#[test]
fn a_schema_1_session_still_loads_with_its_notes_untimed() {
    let session = read(SCHEMA_1, "minuet-1790435530700").expect("schema 1 loads");

    let attempt = &session.attempts[0];
    assert_eq!(
        attempt.played.played(),
        &[key(74), key(67), key(69), key(71)]
    );
    assert!(attempt.played.touches().iter().all(Option::is_none));
    assert!(attempt.played.pedal_changes().is_empty());
    assert_eq!(attempt.began_ms, None);
    assert_eq!(attempt.reached, 4);
}

#[test]
fn a_schema_2_session_loads_to_exactly_what_it_describes() {
    let session = read(SCHEMA_2, "minuet-1792270830806").expect("schema 2 loads");

    assert_eq!(session, schema_2_record());
}

#[test]
fn todays_format_is_written_byte_for_byte() {
    let directory = tempfile::tempdir().expect("a temporary directory");

    assert_eq!(
        written(&schema_2_record(), directory.path()).trim_end(),
        SCHEMA_2.trim_end()
    );
}

// A schema 1 session practised again is rewritten in today's shape, its old
// notes kept, untimed.
#[test]
fn an_old_session_is_written_back_in_todays_shape_without_losing_a_note() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let session = read(SCHEMA_1, "minuet-1790435530700").expect("schema 1 loads");

    let text = written(&session, directory.path());

    assert!(text.contains("\"schema\": 2"));
    assert!(!text.contains("at_us"));
    assert_eq!(
        read(&text, "minuet-1790435530700").expect("reloads"),
        session
    );
}

#[test]
fn a_note_whose_timing_is_half_there_is_refused() {
    for broken in [
        // a release with no press time
        r#"{ "key": 74, "released_us": 410000 }"#,
        // a press time with no velocity
        r#"{ "key": 74, "at_us": 0 }"#,
        // a velocity no press has
        r#"{ "key": 74, "at_us": 0, "velocity": 0 }"#,
        r#"{ "key": 74, "at_us": 0, "velocity": 128 }"#,
        // a key no piano has
        r#"{ "key": 128 }"#,
    ] {
        let text = format!(
            r#"{{ "schema": 2, "id": "broken", "piece": "minuet", "started": "0",
                 "closed": false, "attempts": [ {{ "reached": 0, "notes": [ {broken} ] }} ] }}"#
        );

        assert!(
            matches!(
                read(&text, "broken"),
                Err(RecordsFailure::Unreadable { .. })
            ),
            "{broken} was accepted"
        );
    }
}

#[test]
fn a_session_from_a_later_schema_is_still_refused() {
    let text = SCHEMA_2.replace("\"schema\": 2", "\"schema\": 3");

    assert!(matches!(
        read(&text, "minuet-1792270830806"),
        Err(RecordsFailure::FromTheFuture { schema: 3, .. })
    ));
}
