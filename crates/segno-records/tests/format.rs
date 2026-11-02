//! The session file format, pinned.
//!
//! A performer's history outlives any one version of Hogwash, so what is on
//! disk is checked against files kept here: one per schema ever written.
//! Old ones must always load; today's must load to exactly the record it
//! describes and be written back byte for byte. A change to the format that
//! was not meant fails here first.
//!
//! The notes are the Minuet in G: its opening D5 G4 A4 B4 (in schemas 2 and
//! 3 with a wrong F#4 before the A4), and in schemas 3 and 4 a second
//! attempt started at bar 9, which is step 36 of the Minuet's played order.
//! Schema 4 adds one pass of a loop over bar 9: steps 36 to 41.

#![expect(
    clippy::expect_used,
    reason = "test helpers; clippy's allow-expect-in-tests cannot see a tests/ helper fn"
)]

use std::path::Path;

use segno_capture::{ObservedPerformance, Touch};
use segno_domain::{PieceId, Pitch};
use segno_records::{AttemptRecord, Ended, Records, RecordsFailure, SessionRecord};

const SCHEMA_1: &str = include_str!("fixtures/session-schema-1.json");
const SCHEMA_2: &str = include_str!("fixtures/session-schema-2.json");
const SCHEMA_3: &str = include_str!("fixtures/session-schema-3.json");
const SCHEMA_4: &str = include_str!("fixtures/session-schema-4.json");

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

/// The schema 2 fixture as read today: it says nothing of where its attempt
/// started (so the start) or how it ended (unknown).
fn schema_2_record() -> SessionRecord {
    SessionRecord {
        id: "minuet-1792270830806".to_owned(),
        piece: PieceId::new("minuet").expect("a plain name"),
        started: "1792270830806".to_owned(),
        closed: false,
        attempts: vec![AttemptRecord {
            began_ms: Some(1_792_270_831_002),
            from: 0,
            to: None,
            played: opening(),
            reached: 3,
            ended: None,
        }],
    }
}

fn schema_3_record() -> SessionRecord {
    // Bar 9 of the Minuet: D5 over B3, then G4.
    let mut bar_9 = ObservedPerformance::new();
    bar_9.record_touched(key(74), touch(0, 64, Some(380_000)));
    bar_9.record_touched(key(59), touch(21_000, 52, Some(395_000)));
    bar_9.record_touched(key(67), touch(498_000, 60, Some(801_000)));

    SessionRecord {
        attempts: vec![
            AttemptRecord {
                ended: Some(Ended::Restart),
                ..schema_2_record().attempts.remove(0)
            },
            AttemptRecord {
                began_ms: Some(1_792_270_845_310),
                from: 36,
                to: None,
                played: bar_9,
                reached: 38,
                ended: Some(Ended::Jump),
            },
        ],
        ..schema_2_record()
    }
}

fn schema_4_record() -> SessionRecord {
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

    let mut session = schema_3_record();
    session.attempts.push(AttemptRecord {
        began_ms: Some(1_792_270_852_120),
        from: 36,
        to: Some(41),
        played: pass,
        reached: 41,
        ended: Some(Ended::Pass),
    });
    session
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
    assert_eq!(attempt.from, 0);
    assert_eq!(attempt.reached, 4);
    assert_eq!(attempt.ended, None);
}

#[test]
fn a_schema_2_session_still_loads_starting_at_the_start_and_ending_unknown() {
    let session = read(SCHEMA_2, "minuet-1792270830806").expect("schema 2 loads");

    assert_eq!(session, schema_2_record());
}

#[test]
fn a_schema_3_session_loads_to_exactly_what_it_describes() {
    let session = read(SCHEMA_3, "minuet-1792270830806").expect("schema 3 loads");

    assert_eq!(session, schema_3_record());
}

// `from` is a step in played order. Bar 9 of the Minuet is step 36; storing
// the bar number instead would replay the attempt from the wrong place.
#[test]
fn from_is_the_step_an_attempt_started_at_not_its_bar_number() {
    let session = read(SCHEMA_3, "minuet-1792270830806").expect("schema 3 loads");

    assert_eq!(session.attempts[1].from, 36);
}

#[test]
fn a_schema_4_session_loads_to_exactly_what_it_describes() {
    let session = read(SCHEMA_4, "minuet-1792270830806").expect("schema 4 loads");

    assert_eq!(session, schema_4_record());
}

#[test]
fn todays_format_is_written_byte_for_byte() {
    let directory = tempfile::tempdir().expect("a temporary directory");

    assert_eq!(
        written(&schema_4_record(), directory.path()).trim_end(),
        SCHEMA_4.trim_end()
    );
}

#[test]
fn a_pass_that_went_past_its_loops_end_is_refused() {
    let text = SCHEMA_4.replace("\"to\": 41", "\"to\": 40");

    assert!(matches!(
        read(&text, "minuet-1792270830806"),
        Err(RecordsFailure::Unreadable { .. })
    ));
}

// A schema 1 session practised again is rewritten in today's shape, its old
// notes kept, untimed.
#[test]
fn an_old_session_is_written_back_in_todays_shape_without_losing_a_note() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let session = read(SCHEMA_1, "minuet-1790435530700").expect("schema 1 loads");

    let text = written(&session, directory.path());

    assert!(text.contains("\"schema\": 4"));
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
    let text = SCHEMA_4.replace("\"schema\": 4", "\"schema\": 5");

    assert!(matches!(
        read(&text, "minuet-1792270830806"),
        Err(RecordsFailure::FromTheFuture { schema: 5, .. })
    ));
}

// An ending this version does not know is refused, not guessed at, and costs
// only its own file: the rest of the history still reads.
#[test]
fn an_unknown_ending_makes_only_that_session_unreadable() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let history = directory.path().join("history");
    std::fs::create_dir_all(&history).expect("a history folder");
    std::fs::write(history.join("minuet-1792270830806.json"), SCHEMA_3).expect("writes");
    std::fs::write(
        history.join("minuet-1792270899999.json"),
        SCHEMA_3
            .replace("minuet-1792270830806", "minuet-1792270899999")
            .replace("\"jump\"", "\"teleported\""),
    )
    .expect("writes");

    let (sessions, unreadable) = Records::under(directory.path())
        .sessions()
        .expect("the history is readable");

    assert_eq!(sessions, [schema_3_record()]);
    assert!(
        matches!(unreadable.as_slice(), [RecordsFailure::Unreadable { detail, .. }] if detail.contains("teleported")),
        "{unreadable:?}"
    );
}

#[test]
fn an_attempt_that_starts_after_where_it_reached_is_refused() {
    let text = SCHEMA_3.replace("\"from\": 36", "\"from\": 40");

    assert!(matches!(
        read(&text, "minuet-1792270830806"),
        Err(RecordsFailure::Unreadable { .. })
    ));
}
