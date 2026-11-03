//! A piece's history survives an edit to its score (Phase 2.5, DECISIONS
//! 3 Nov).
//!
//! The running app has the bundled Minuet and Prelude compiled in, so "an
//! update edited the score" is set up the other way round: the edited copy is
//! the *older* version, kept in the data folder with a session played on it,
//! and the file compiled in is today's. Each session must be judged against
//! the notes it was played on, and stay under the piece's permanent name.

#![expect(
    clippy::expect_used,
    reason = "test helpers; clippy's allow-expect-in-tests cannot see a tests/ helper fn"
)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use segno_app_lib::history::{Progress, Versions, progress_in, progress_of};
use segno_app_lib::state::Segno;
use segno_capture::{ObservedPerformance, Touch};
use segno_domain::PieceId;
use segno_engraving::Engraver as _;
use segno_notation::{ExpectedNotes, ReferencePerformance, TICKS_PER_QUARTER};
use segno_records::{AttemptRecord, Ended, Records, SessionRecord};

fn temporary(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("segno-versions-test-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn segno(root: &std::path::Path) -> Segno {
    let engraving = segno_app_lib::engraving::Engraving::start().expect("the engine starts");
    let records = Records::under(root);
    let practice = segno_app_lib::practice::Practice::start(records.clone(), |_| {})
        .expect("the practice thread starts");
    Segno::start(engraving, records, practice).expect("the provided material prepares")
}

fn library_file(name: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("library")
            .join(name),
    )
    .expect("ships with the application")
}

fn reference(source: &str) -> Arc<ReferencePerformance> {
    let drawn = segno_engraving_verovio::VerovioEngraver::from_environment()
        .expect("libverovio")
        .engrave(source.as_bytes())
        .expect("engraves");
    Arc::new(ReferencePerformance::derived_from(drawn.score()))
}

/// `source` with the first `<step>` of bar `measure` changed to `step`,
/// which must be a different note.
fn with_note(source: &str, measure: u32, step: char) -> String {
    let bar = source
        .find(&format!("<measure number=\"{measure}\""))
        .expect("the bar");
    let at = bar + source[bar..].find("<step>").expect("a note") + "<step>".len();
    assert_ne!(
        source[at..].chars().next(),
        Some(step),
        "an edit that changes nothing"
    );
    format!("{}{step}{}", &source[..at], &source[at + 1..])
}

/// `source` without bar `measure`.
fn without_bar(source: &str, measure: u32) -> String {
    let from = source
        .find(&format!("<measure number=\"{measure}\""))
        .expect("the bar");
    let to = from + source[from..].find("</measure>").expect("its end") + "</measure>".len();
    format!("{}{}", &source[..from], &source[to..])
}

/// The whole piece as `reference` writes it, a quarter every 600 ms: `late`
/// µs of hesitation before step `stall`, and `wrong` (a key) struck before
/// step `slip`.
fn played(
    reference: &ReferencePerformance,
    stall: usize,
    late: u64,
    slip: usize,
    wrong: u8,
) -> AttemptRecord {
    let mut played = ObservedPerformance::new();
    let mut at_us = 1_000_000;
    let touch = |at_us| Touch {
        at_us,
        velocity: 64,
        released_us: None,
    };
    for step in 0..reference.len() {
        if step > 0 {
            let written = u64::from(reference.written_gap(step).unwrap_or(TICKS_PER_QUARTER));
            at_us += written * 600_000 / u64::from(TICKS_PER_QUARTER);
        }
        if step == stall {
            at_us += late;
        }
        if step == slip {
            played.record_touched(
                segno_domain::Pitch::new(wrong).expect("a key"),
                touch(at_us),
            );
            at_us += 100_000;
        }
        for pitch in reference
            .at(step)
            .into_iter()
            .flat_map(ExpectedNotes::sounding)
        {
            played.record_touched(pitch, touch(at_us));
        }
    }
    AttemptRecord {
        began_ms: Some(0),
        from: 0,
        to: None,
        played,
        reached: reference.len(),
        ended: Some(Ended::Finished),
    }
}

/// The first step of bar `measure`.
fn first_step(reference: &ReferencePerformance, measure: u32) -> usize {
    reference
        .iter()
        .position(|step| step.measure().get() == measure)
        .expect("the bar is played")
}

fn session(piece: &PieceId, version: &str, day: u64, attempt: AttemptRecord) -> SessionRecord {
    SessionRecord {
        id: format!("{}-{day}", piece.as_str()),
        piece: piece.clone(),
        version: version.to_owned(),
        rules: segno_analysis::RULES,
        started: (1_793_000_000_000 + 86_400_000 * day).to_string(),
        closed: true,
        attempts: vec![attempt],
    }
}

/// A session's cell for bar `measure`: (passes, pause µs, wrong notes).
fn bar(progress: &Progress, session: usize, measure: u32) -> (u32, u64, u32) {
    let cell = progress
        .sessions
        .get(session)
        .expect("the session")
        .bars
        .iter()
        .find(|cell| cell.measure == measure)
        .expect("the bar was played");
    (cell.passes, cell.pause_us, cell.wrong)
}

fn none(_: &str) -> Option<Arc<ReferencePerformance>> {
    None
}

// Edit one note in bar 6 of the Minuet. A session played on the old notes
// keeps exactly its results and stays under `minuet-in-g`; a session on
// today's notes is judged against today's; the two are combined by bar,
// since the bars themselves didn't change.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn editing_a_note_never_changes_past_results_and_history_stays_attached() {
    let root = temporary("minuet");
    let segno = segno(&root);
    let minuet = PieceId::new("minuet-in-g").expect("a tag");
    let piece = segno
        .library()
        .pieces()
        .into_iter()
        .find(|piece| *piece.id() == minuet)
        .expect("the Minuet is bundled");
    let today = piece.reference();

    // The old version: bar 6's first note a G where today's file has a B.
    let old_source = with_note(&library_file("minuet-in-g.musicxml"), 6, 'G');
    let old_version = segno
        .records()
        .keep_version(old_source.as_bytes())
        .expect("kept");
    let old = reference(&old_source);
    let six = first_step(&old, 6);
    assert_ne!(
        old.at(six).expect("bar 6").sounding().collect::<Vec<_>>(),
        today.at(six).expect("bar 6").sounding().collect::<Vec<_>>(),
        "the edit changed bar 6's first note"
    );

    // Played on the old version: its notes exactly, hesitating 3 s before
    // bar 6, a wrong F#4 (66) before bar 3.
    let on_old = session(
        &minuet,
        &old_version,
        0,
        played(&old, six, 3_000_000, first_step(&old, 3), 66),
    );
    let before = progress_in(
        std::slice::from_ref(&on_old),
        &minuet,
        &Versions {
            now: old_version.clone(),
            reference: &old,
            now_kept_ms: None,
            older: &none,
        },
    );
    let old_bar_3 = bar(&before, 0, 3);
    let old_bar_6 = bar(&before, 0, 6);
    assert_eq!(old_bar_3.2, 1, "the slip in bar 3 is one wrong note");
    assert_eq!(old_bar_6.2, 0, "bar 6 was played as that version wrote it");
    assert!(
        // Most of the 3 s: analysis allows for the note's own written gap.
        old_bar_6.1 >= 2_000_000,
        "the hesitation before bar 6: {old_bar_6:?}"
    );

    // Then today's: the same playing, the old note in bar 6 included.
    let today_version = segno.version_now(&minuet).expect("today's version");
    assert_ne!(today_version, old_version);
    // Then on today's: the old G first, by habit, then today's B.
    let habit = old
        .at(six)
        .expect("bar 6")
        .sounding()
        .find(|pitch| {
            !today
                .at(six)
                .expect("bar 6")
                .sounding()
                .any(|now| now == *pitch)
        })
        .expect("the edited note");
    let on_today = session(
        &minuet,
        &today_version,
        1,
        played(today, six, 0, six, u8::from(habit)),
    );
    segno.records().keep(&on_old).expect("kept");
    segno.records().keep(&on_today).expect("kept");

    let (sessions, unreadable) = segno.records().sessions().expect("the history reads");
    assert!(unreadable.is_empty(), "{unreadable:?}");
    let after = progress_of(&segno, &sessions, piece);

    // Both sessions are still the Minuet's, combined, with nothing left out.
    assert_eq!(segno.records().sessions_of(&minuet), 2);
    assert_eq!(after.sessions.len(), 2);
    assert_eq!(after.earlier, 0);
    assert_eq!(after.updated_ms, None, "the bars didn't change: no line");
    // The old session's results, exactly as before the edit.
    assert_eq!(after.sessions.first(), before.sessions.first());
    // Today's session is judged against today's note: the old G is one
    // wrong note in bar 6.
    assert_eq!(bar(&after, 1, 6).2, 1, "the old G, against today's B");

    // And the test can tell: judged against today's notes, the old session's
    // bar 6 would be wrong (the cursor waits at the B, so every note after
    // the G counts against bar 6).
    let rejudged = progress_in(
        std::slice::from_ref(&on_old),
        &minuet,
        &Versions {
            now: old_version.clone(),
            reference: today,
            now_kept_ms: None,
            older: &none,
        },
    );
    assert!(bar(&rejudged, 0, 6).2 > 0, "{:?}", bar(&rejudged, 0, 6));
}

// Remove a bar from the Prelude. A session on the old version still counts in
// the totals, but the bars show only today's, with the line saying so.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn a_changed_bar_structure_shows_only_todays_bars_and_says_so() {
    let root = temporary("prelude");
    let segno = segno(&root);
    let prelude = PieceId::new("prelude-in-c").expect("a tag");
    let piece = segno
        .library()
        .pieces()
        .into_iter()
        .find(|piece| *piece.id() == prelude)
        .expect("the Prelude is bundled");

    let old_source = without_bar(&library_file("prelude-in-c.musicxml"), 10);
    let old_version = segno
        .records()
        .keep_version(old_source.as_bytes())
        .expect("kept");
    let old = reference(&old_source);
    let today_version = segno.version_now(&prelude).expect("today's version");
    segno
        .records()
        .keep(&session(
            &prelude,
            &old_version,
            0,
            played(&old, 0, 0, usize::MAX, 0),
        ))
        .expect("kept");
    segno
        .records()
        .keep(&session(
            &prelude,
            &today_version,
            1,
            played(piece.reference(), 0, 0, usize::MAX, 0),
        ))
        .expect("kept");

    let (sessions, _) = segno.records().sessions().expect("the history reads");
    let progress = progress_of(&segno, &sessions, piece);
    let view = progress.view(piece, None);

    assert_eq!(progress.sessions.len(), 1, "only today's bars");
    assert_eq!(progress.earlier, 1);
    assert_eq!(view.practised, 2, "the header still counts both");
    assert!(
        view.played_us > progress.sessions.first().expect("today's").played_us,
        "and both sessions' time"
    );
    assert!(view.updated_ms.is_some(), "the line has a date");
    assert_eq!(
        view.updated_ms,
        segno.records().version_kept_ms(&today_version)
    );
}

// How long the Prelude's Progress takes to work out with an older version
// to engrave (cold) and once it is cached. Printed for the report; run with
// `--nocapture`.
#[test]
#[ignore = "needs a real libverovio; run `just provided-test`"]
fn the_preludes_progress_cold_and_cached() {
    let mut cold = Vec::new();
    let mut cached = Vec::new();
    for run in 0..5 {
        let root = temporary(&format!("timing-{run}"));
        let segno = segno(&root);
        let prelude = PieceId::new("prelude-in-c").expect("a tag");
        let piece = segno
            .library()
            .pieces()
            .into_iter()
            .find(|piece| *piece.id() == prelude)
            .expect("bundled");
        let old_source = with_note(&library_file("prelude-in-c.musicxml"), 10, 'E');
        let old_version = segno
            .records()
            .keep_version(old_source.as_bytes())
            .expect("kept");
        let old = reference(&old_source);
        let today_version = segno.version_now(&prelude).expect("today's");
        assert_ne!(old_version, today_version, "the edit made a new version");
        let sessions = vec![
            session(&prelude, &old_version, 0, played(&old, 0, 0, usize::MAX, 0)),
            session(
                &prelude,
                &today_version,
                1,
                played(piece.reference(), 0, 0, usize::MAX, 0),
            ),
        ];

        let started = Instant::now();
        let first = progress_of(&segno, &sessions, piece);
        cold.push(started.elapsed());
        let started = Instant::now();
        let second = progress_of(&segno, &sessions, piece);
        cached.push(started.elapsed());
        assert_eq!(first, second);
        assert_eq!(first.sessions.len(), 2);
    }
    println!("Prelude Progress, 1 old version + today's, 2 sessions:");
    println!("  cold   (old version engraved): {cold:?}");
    println!("  cached (old version reused):   {cached:?}");
}
