//! The session file format, pinned.
//!
//! A performer's history outlives any one version of Segno, so what is on
//! disk is checked against a file kept here: today's schema must load to
//! exactly the record it describes and be written back byte for byte. A
//! change to the format that was not meant fails here first. Schemas 1–4
//! (before Segno's fresh start, 3 Nov 2026) are refused, with a reason.
//!
//! The notes are the Minuet in G (its fingerprint is the session's
//! `version`): its opening D5 G4 with a wrong F#4 before the A4, then an
//! attempt started at bar 9, which is step 36 of the Minuet's played order,
//! then one pass of a loop over bar 9: steps 36 to 41.

#![expect(
    clippy::expect_used,
    reason = "test helpers; clippy's allow-expect-in-tests cannot see a tests/ helper fn"
)]

use std::path::Path;

use segno_capture::{ObservedPerformance, Touch};
use segno_domain::{PieceId, Pitch};
use segno_records::{AttemptRecord, Ended, Records, RecordsFailure, SessionRecord};

const SCHEMA_5: &str = include_str!("fixtures/session-schema-5.json");
const ID: &str = "minuet-in-g-1792270830806";

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

fn opening() -> ObservedPerformance {
    let mut played = ObservedPerformance::new();
    played.record_touched(key(74), touch(0, 80, Some(410_000)));
    played.pedal(300_000, true);
    played.record_touched(key(67), touch(512_000, 72, Some(990_000)));
    played.record_touched(key(66), touch(1_040_000, 58, Some(1_100_000)));
    played.pedal(1_200_000, false);
    played.record_touched(key(69), touch(1_350_000, 90, None));
    played
}

/// The record the schema 5 fixture describes.
fn record() -> SessionRecord {
    // Bar 9 of the Minuet: D5 over B3, then G4.
    let mut bar_9 = ObservedPerformance::new();
    bar_9.record_touched(key(74), touch(0, 64, Some(380_000)));
    bar_9.record_touched(key(59), touch(21_000, 52, Some(395_000)));
    bar_9.record_touched(key(67), touch(498_000, 60, Some(801_000)));

    // Bar 9 of the Minuet, all of it: D5 over B3, G4, A4, B4 over A3, C5.
    let mut pass = ObservedPerformance::new();
    for (number, at_us, velocity) in [
        (74, 0, 66),
        (59, 18_000, 50),
        (67, 470_000, 58),
        (69, 940_000, 61),
        (71, 1_400_000, 63),
        (57, 1_420_000, 47),
        (72, 1_890_000, 60),
    ] {
        pass.record_touched(key(number), touch(at_us, velocity, Some(at_us + 380_000)));
    }

    SessionRecord {
        id: ID.to_owned(),
        piece: PieceId::new("minuet-in-g").expect("a plain name"),
        version: "53f88fc9b8889397".to_owned(),
        rules: 1,
        started: "1792270830806".to_owned(),
        closed: false,
        attempts: vec![
            AttemptRecord {
                began_ms: Some(1_792_270_831_002),
                from: 0,
                to: None,
                played: opening(),
                reached: 3,
                ended: Some(Ended::Restart),
            },
            AttemptRecord {
                began_ms: Some(1_792_270_845_310),
                from: 36,
                to: None,
                played: bar_9,
                reached: 38,
                ended: Some(Ended::Jump),
            },
            AttemptRecord {
                began_ms: Some(1_792_270_852_120),
                from: 36,
                to: Some(41),
                played: pass,
                reached: 41,
                ended: Some(Ended::Pass),
            },
        ],
    }
}

// The version is the real Minuet's fingerprint, so the fixture names the
// score its attempts were played on.
#[test]
fn the_fixtures_version_is_the_minuets() {
    let minuet = include_bytes!("../../../app/src-tauri/library/minuet-in-g.musicxml");

    assert_eq!(record().version, segno_records::fingerprint(minuet));
}

#[test]
fn todays_session_loads_to_exactly_what_it_describes() {
    assert_eq!(read(SCHEMA_5, ID).expect("loads"), record());
}

// `from` is a step in played order. Bar 9 of the Minuet is step 36; storing
// the bar number instead would replay the attempt from the wrong place.
#[test]
fn from_is_the_step_an_attempt_started_at_not_its_bar_number() {
    assert_eq!(read(SCHEMA_5, ID).expect("loads").attempts[1].from, 36);
}

#[test]
fn todays_format_is_written_byte_for_byte() {
    let directory = tempfile::tempdir().expect("a temporary directory");

    assert_eq!(
        written(&record(), directory.path()).trim_end(),
        SCHEMA_5.trim_end()
    );
}

#[test]
fn a_pass_that_went_past_its_loops_end_is_refused() {
    let text = SCHEMA_5.replace("\"to\": 41", "\"to\": 40");

    assert!(matches!(
        read(&text, ID),
        Err(RecordsFailure::Unreadable { .. })
    ));
}

// History from before the fresh start is refused with the reason, not
// misread.
#[test]
fn a_session_from_before_the_fresh_start_is_refused_with_the_reason() {
    for old in 1..=4 {
        let text = SCHEMA_5.replace("\"schema\": 5", &format!("\"schema\": {old}"));

        assert!(
            matches!(read(&text, ID), Err(RecordsFailure::Unreadable { detail, .. }) if detail.contains("fresh")),
            "schema {old}"
        );
    }
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
            r#"{{ "schema": 5, "id": "broken", "piece": "minuet-in-g", "version": "53f88fc9b8889397",
                 "rules": 1, "started": "0",
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
    let text = SCHEMA_5.replace("\"schema\": 5", "\"schema\": 6");

    assert!(matches!(
        read(&text, ID),
        Err(RecordsFailure::FromTheFuture { schema: 6, .. })
    ));
}

// An ending this version does not know is refused, not guessed at, and costs
// only its own file: the rest of the history still reads.
#[test]
fn an_unknown_ending_makes_only_that_session_unreadable() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let history = directory.path().join("history");
    std::fs::create_dir_all(&history).expect("a history folder");
    std::fs::write(history.join(format!("{ID}.json")), SCHEMA_5).expect("writes");
    std::fs::write(
        history.join("minuet-in-g-1792270899999.json"),
        SCHEMA_5
            .replace(ID, "minuet-in-g-1792270899999")
            .replace("\"jump\"", "\"teleported\""),
    )
    .expect("writes");

    let (sessions, unreadable) = Records::under(directory.path())
        .sessions()
        .expect("the history is readable");

    assert_eq!(sessions, [record()]);
    assert!(
        matches!(unreadable.as_slice(), [RecordsFailure::Unreadable { detail, .. }] if detail.contains("teleported")),
        "{unreadable:?}"
    );
}

#[test]
fn an_attempt_that_starts_after_where_it_reached_is_refused() {
    let text = SCHEMA_5.replacen("\"from\": 36", "\"from\": 40", 1);

    assert!(matches!(
        read(&text, ID),
        Err(RecordsFailure::Unreadable { .. })
    ));
}
