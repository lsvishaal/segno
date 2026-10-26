//! A piece's history: every attempt ever kept for it, bar by bar and
//! session by session, to say which bars are hard for this performer now.

use hogwash_analysis::{Bar, Bars};
use hogwash_domain::PieceId;
use hogwash_notation::ReferencePerformance;
use hogwash_records::{Ended, Records, SessionRecord};
use serde::Serialize;

/// Which bars of a piece are hard now (D10, see [`Bar::level`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeakBars {
    /// How many attempts (runs and loop passes) this is from.
    pub runs: usize,
    /// Each bar with enough of its last three sessions to judge (two
    /// passes), and its level: 0 went fine, 1–4 lighter to deeper.
    pub bars: Vec<WeakBar>,
}

impl WeakBars {
    /// Up to two of the hardest bars (level 3 or 4), hardest first.
    #[must_use]
    pub fn weakest(&self) -> Vec<u32> {
        let mut hard: Vec<&WeakBar> = self.bars.iter().filter(|bar| bar.level >= 3).collect();
        hard.sort_by_key(|bar| (std::cmp::Reverse((bar.level, bar.difficulty)), bar.measure));
        hard.iter().take(2).map(|bar| bar.measure).collect()
    }
}

/// When `piece` was last practised, in Unix milliseconds.
#[must_use]
pub fn last_practised(sessions: &[SessionRecord], piece: &PieceId) -> Option<u64> {
    sessions
        .iter()
        .filter(|session| &session.piece == piece)
        .filter_map(|session| session.started.parse().ok())
        .max()
}

/// One bar's place on the scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeakBar {
    pub measure: u32,
    pub level: u8,
    /// Only to order bars of one level, hardest first.
    #[serde(skip)]
    pub difficulty: u64,
}

/// A piece's sessions, oldest first, each bar by bar, and where each bar
/// stands now. One replay of every attempt.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// How many attempts (runs and loop passes) this is from.
    pub runs: usize,
    pub sessions: Vec<SessionBars>,
    /// Each bar's own last three sessions in which it was played, added up.
    #[serde(skip)]
    pub now: Bars,
}

/// One session, bar by bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionBars {
    /// Unix milliseconds; 0 if the record doesn't say.
    pub started_ms: u64,
    /// Time spent on the notes, breaks left out.
    pub played_us: u64,
    /// Only the bars played; a bar not played is no cell at all.
    pub bars: Vec<Cell>,
}

/// One bar in one session: a record of what happened, so its level stands
/// even after one pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub measure: u32,
    pub passes: u32,
    pub pause_us: u64,
    pub wrong: u32,
    pub stops: u32,
    pub level: u8,
}

impl Progress {
    /// The bars there is enough to judge, with their levels now.
    #[must_use]
    pub fn weak_bars(&self) -> WeakBars {
        WeakBars {
            runs: self.runs,
            bars: self
                .now
                .iter()
                .filter_map(|(measure, bar)| {
                    Some(WeakBar {
                        measure: measure.get(),
                        level: bar.judged()?,
                        difficulty: bar.difficulty(),
                    })
                })
                .collect(),
        }
    }
}

/// Replays every attempt kept for `piece` against `reference`. Sessions
/// that cannot be read are skipped, as the library does.
#[must_use]
pub fn weak_bars(records: &Records, piece: &PieceId, reference: &ReferencePerformance) -> WeakBars {
    weak_bars_in(&sessions(records), piece, reference)
}

/// Every session that can be read; none if the history folder can't be.
#[must_use]
pub fn sessions(records: &Records) -> Vec<SessionRecord> {
    records
        .sessions()
        .map(|(sessions, _unreadable)| sessions)
        .unwrap_or_default()
}

/// [`weak_bars`] over sessions already read, so a whole library reads them once.
#[must_use]
pub fn weak_bars_in(
    sessions: &[SessionRecord],
    piece: &PieceId,
    reference: &ReferencePerformance,
) -> WeakBars {
    progress_in(sessions, piece, reference).weak_bars()
}

/// `piece`'s [`Progress`] over sessions already read.
#[must_use]
pub fn progress_in(
    sessions: &[SessionRecord],
    piece: &PieceId,
    reference: &ReferencePerformance,
) -> Progress {
    let mut mine: Vec<&SessionRecord> = sessions
        .iter()
        .filter(|session| &session.piece == piece)
        .collect();
    let started = |session: &SessionRecord| session.started.parse::<u64>().unwrap_or(0);
    mine.sort_by_key(|session| started(session));

    let mut runs = 0;
    let per_session: Vec<Bars> = mine
        .iter()
        .map(|session| {
            let mut bars = Bars::default();
            for attempt in &session.attempts {
                // A score re-imported shorter leaves old attempts past its end.
                let to = attempt.to.unwrap_or(reference.len()).min(reference.len());
                let gave_up = matches!(attempt.ended, Some(Ended::Restart | Ended::Stop));
                bars.add(&Bars::of(
                    &attempt.played,
                    reference,
                    attempt.from.min(to)..to,
                    gave_up,
                ));
                runs += 1;
            }
            bars
        })
        .collect();

    Progress {
        runs,
        sessions: mine
            .iter()
            .zip(&per_session)
            .map(|(session, bars)| SessionBars {
                started_ms: started(session),
                played_us: bars.total().played_us,
                bars: bars
                    .iter()
                    .filter(|(_, bar)| bar.passes > 0)
                    .map(|(measure, bar)| cell(measure.get(), bar))
                    .collect(),
            })
            .collect(),
        now: Bars::now(&per_session),
    }
}

fn cell(measure: u32, bar: &Bar) -> Cell {
    Cell {
        measure,
        passes: bar.passes,
        pause_us: bar.pause_us,
        wrong: bar.wrong,
        stops: bar.gave_up,
        level: bar.level(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Instant;

    use hogwash_capture::{ObservedPerformance, Touch};
    use hogwash_notation::{ExpectedNotes, TICKS_PER_QUARTER};
    use hogwash_records::{AttemptRecord, SessionRecord};

    use super::*;

    /// The real Minuet, through the real engine, and an empty history.
    fn minuet(name: &str) -> (Records, PieceId, Arc<ReferencePerformance>) {
        provided(name, "Minuet in G Major")
    }

    /// A provided piece, through the real engine, and an empty history.
    fn provided(name: &str, title: &str) -> (Records, PieceId, Arc<ReferencePerformance>) {
        let directory = std::env::temp_dir().join(format!("hogwash-history-{name}"));
        drop(std::fs::remove_dir_all(&directory));
        let records = Records::under(&directory);
        let engraving = crate::engraving::Engraving::start().expect("the engine starts");
        let practice = crate::practice::Practice::start(records.clone(), |_| {})
            .expect("the practice thread starts");
        let hogwash = crate::state::Hogwash::start(engraving, records.clone(), practice)
            .expect("the provided material prepares");
        let piece = hogwash
            .library()
            .pieces()
            .into_iter()
            .find(|piece| piece.title().starts_with(title))
            .expect("the piece is on the shelf");
        (
            records,
            piece.id().clone(),
            Arc::new(piece.reference().clone()),
        )
    }

    /// The whole piece in its written rhythm, a quarter every 600 ms, with
    /// `late` µs more before step `stall`.
    fn run(reference: &ReferencePerformance, stall: usize, late: u64) -> AttemptRecord {
        let mut played = ObservedPerformance::new();
        let mut at_us = 1_000_000;
        for step in 0..reference.len() {
            if step > 0 {
                let written = u64::from(reference.written_gap(step).unwrap_or(TICKS_PER_QUARTER));
                at_us += written * 600_000 / u64::from(TICKS_PER_QUARTER);
            }
            if step == stall {
                at_us += late;
            }
            for pitch in reference
                .at(step)
                .into_iter()
                .flat_map(ExpectedNotes::sounding)
            {
                let touch = Touch {
                    at_us,
                    velocity: 64,
                    released_us: None,
                };
                played.record_touched(pitch, touch);
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

    fn keep(records: &Records, piece: &PieceId, id: &str, attempts: Vec<AttemptRecord>) {
        let session = SessionRecord {
            id: id.to_owned(),
            piece: piece.clone(),
            started: "2026-10-25T10:00:00Z".to_owned(),
            closed: true,
            attempts,
        };
        records.keep(&session).expect("kept");
    }

    // 12 s late in each of two sessions; the repeat plays the bar twice a
    // run, so 6 s of pause per pass: level 3. Every other bar went fine.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_bar_stalled_in_across_sessions_is_the_weakest() {
        let (records, piece, minuet) = minuet("stalled");
        let stall = minuet.len() / 2;
        keep(
            &records,
            &piece,
            "one",
            vec![run(&minuet, stall, 12_000_000)],
        );
        keep(
            &records,
            &piece,
            "two",
            vec![run(&minuet, stall, 12_000_000)],
        );
        let weak = minuet.at(stall).expect("a step").measure().get();

        let history = weak_bars(&records, &piece, &minuet);

        assert_eq!(history.runs, 2);
        let level = |measure| {
            history
                .bars
                .iter()
                .find(|bar| bar.measure == measure)
                .map(|bar| bar.level)
        };
        assert_eq!(level(weak), Some(3));
        assert!(
            history
                .bars
                .iter()
                .all(|bar| bar.measure == weak || bar.level == 0)
        );
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn the_library_card_knows_when_and_where() {
        let (records, piece, minuet) = minuet("card");
        let stall = minuet.len() / 2;
        let session = |id: &str, started: &str| SessionRecord {
            id: id.to_owned(),
            piece: piece.clone(),
            started: started.to_owned(),
            closed: true,
            attempts: vec![run(&minuet, stall, 12_000_000)],
        };
        let sessions = [
            session("one", "1761300000000"),
            session("two", "1761390000000"),
        ];
        let weak = minuet.at(stall).expect("a step").measure().get();

        assert_eq!(last_practised(&sessions, &piece), Some(1_761_390_000_000));
        assert_eq!(
            weak_bars_in(&sessions, &piece, &minuet).weakest(),
            vec![weak]
        );
        assert_eq!(last_practised(&[], &piece), None);
        drop(records);
    }

    // A cell records what happened, so it keeps its level after one pass;
    // judging the bar needs two.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_bar_played_once_has_a_cell_level_but_no_judgement() {
        let (records, piece, prelude) = provided("once", "Prelude in C Major");
        let session = |attempt| SessionRecord {
            id: "once".to_owned(),
            piece: piece.clone(),
            started: "1761300000000".to_owned(),
            closed: true,
            attempts: vec![attempt],
        };
        let steady = progress_in(&[session(run(&prelude, 0, 0))], &piece, &prelude);
        let once = steady.sessions[0]
            .bars
            .iter()
            .find(|cell| cell.passes == 1)
            .expect("the Prelude has no repeats")
            .measure;
        let stall = (1..prelude.len())
            .find(|&step| prelude.at(step).map(|notes| notes.measure().get()) == Some(once))
            .expect("its first step");

        let progress = progress_in(
            &[session(run(&prelude, stall, 12_000_000))],
            &piece,
            &prelude,
        );

        let cell = progress.sessions[0]
            .bars
            .iter()
            .find(|cell| cell.measure == once)
            .expect("played");
        assert_eq!((cell.passes, cell.level), (1, 4));
        assert!(
            progress
                .weak_bars()
                .bars
                .iter()
                .all(|bar| bar.measure != once)
        );
        drop(records);
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_piece_never_played_has_no_weak_bars() {
        let (records, piece, minuet) = minuet("empty");

        assert_eq!(weak_bars(&records, &piece, &minuet), WeakBars::default());
    }

    // Budget: opening a piece stays quick with a long history, on the
    // longest provided piece.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn fifty_sessions_add_up_in_well_under_a_tenth_of_a_second() {
        let (records, piece, prelude) = provided("fifty", "Prelude in C Major");
        for session in 0..50 {
            keep(
                &records,
                &piece,
                &format!("s{session}"),
                vec![run(&prelude, session, 1_000_000)],
            );
        }

        let began = Instant::now();
        let history = weak_bars(&records, &piece, &prelude);

        assert_eq!(history.runs, 50);
        // The budget is for the release build; a debug build is a few times slower.
        let budget = if cfg!(debug_assertions) { 500 } else { 100 };
        assert!(
            began.elapsed().as_millis() < budget,
            "{:?}",
            began.elapsed()
        );
    }
}
