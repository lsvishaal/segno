//! A piece's history: every attempt ever kept for it, bar by bar and
//! session by session, to say which bars are hard for this performer now.

use std::cmp::Reverse;
use std::collections::BTreeSet;

use hogwash_analysis::coach::practise_range;
use hogwash_analysis::{Bar, Bars};
use hogwash_domain::{MeasureNumber, PieceId};
use hogwash_notation::ReferencePerformance;
use hogwash_records::{Ended, Records, SessionRecord};
use serde::Serialize;

/// How many passages "Work on next" lists, at most.
const WORK_ON_NEXT: usize = 3;

/// How many of a bar's first sessions its trend starts from, at most.
const EARLIEST: usize = 3;

/// When `piece` was last practised, in Unix milliseconds.
#[must_use]
pub fn last_practised(sessions: &[SessionRecord], piece: &PieceId) -> Option<u64> {
    sessions
        .iter()
        .filter(|session| &session.piece == piece)
        .filter_map(|session| session.started.parse().ok())
        .max()
}

/// A piece's sessions, oldest first, each bar by bar, and where each bar
/// stands now. One replay of every attempt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    /// How many attempts (runs and loop passes) this is from.
    pub runs: usize,
    pub sessions: Vec<SessionBars>,
    /// Each bar's own last three sessions in which it was played, added up.
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

/// Bars next to each other worth practising together, and the loop that
/// practises them (from one bar before, [`practise_range`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Passage {
    pub first: u32,
    pub last: u32,
    pub from: u32,
    pub to: u32,
    /// "about 3.1 s of hesitation each time through", over the bars' Now
    /// ([`facts_per_pass`]).
    pub facts: String,
}

/// A bar's level now: `None` when it hasn't had the two passes a judgement needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NowBar {
    pub measure: u32,
    pub level: Option<u8>,
}

/// A bar's line on the Progress page, after "Bar N · ", and the loop its
/// Practise button plays ([`practise_range`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    pub measure: u32,
    pub line: String,
    pub from: u32,
    pub to: u32,
}

/// Everything the Progress page shows for one piece.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressView {
    pub id: String,
    pub title: String,
    pub composer: Option<String>,
    /// How many bars the piece has: the grid's columns.
    pub bars: usize,
    /// Newest first.
    pub sessions: Vec<SessionBars>,
    pub played_us: u64,
    pub last_practised_ms: Option<u64>,
    /// Every bar played, in bar order.
    pub now: Vec<NowBar>,
    pub work_on_next: Vec<Passage>,
    /// Every bar, in bar order.
    pub details: Vec<Detail>,
}

impl Progress {
    /// The bars judged (two passes) at level `min` or more, worst first:
    /// by level, then difficulty, then bar.
    fn worst(&self, min: u8) -> Vec<u32> {
        let mut bars: Vec<(u32, u8, u64)> = self
            .now
            .iter()
            .filter_map(|(measure, bar)| Some((measure.get(), bar.judged()?, bar.difficulty())))
            .filter(|&(_, level, _)| level >= min)
            .collect();
        bars.sort_by_key(|&(measure, level, difficulty)| (Reverse((level, difficulty)), measure));
        bars.into_iter().map(|(measure, ..)| measure).collect()
    }

    /// Up to two of the hardest bars (level 3 or 4), hardest first: the
    /// library card's "weakest bars".
    #[must_use]
    pub fn weakest(&self) -> Vec<u32> {
        self.worst(3).into_iter().take(2).collect()
    }

    /// "Work on next": the judged bars at level 1 or more, worst first,
    /// neighbours merged into one passage, up to three passages.
    #[must_use]
    pub fn work_on_next(&self, reference: &ReferencePerformance) -> Vec<Passage> {
        let worst = self.worst(1);
        let marked: BTreeSet<u32> = worst.iter().copied().collect();
        let mut passages: Vec<(u32, u32)> = Vec::new();
        for measure in worst {
            if passages.len() == WORK_ON_NEXT {
                break;
            }
            if passages
                .iter()
                .any(|&(first, last)| (first..=last).contains(&measure))
            {
                continue;
            }
            let mut first = measure;
            while first > 1 && marked.contains(&(first - 1)) {
                first -= 1;
            }
            let mut last = measure;
            while marked.contains(&(last + 1)) {
                last += 1;
            }
            passages.push((first, last));
        }
        passages
            .into_iter()
            .filter_map(|(first, last)| {
                let (from, to) = practise_range(
                    reference,
                    MeasureNumber::new(first)?,
                    MeasureNumber::new(last)?,
                );
                Some(Passage {
                    first,
                    last,
                    from: from.get(),
                    to: to.get(),
                    facts: self.facts(first..=last),
                })
            })
            .collect()
    }

    /// What happened in `bars` lately, per pass of each, added up.
    fn facts(&self, bars: std::ops::RangeInclusive<u32>) -> String {
        let (mut pause_us, mut wrong_milli, mut stops) = (0, 0, 0);
        for bar in bars.filter_map(|measure| self.now.get(MeasureNumber::new(measure)?)) {
            let passes = u64::from(bar.passes.max(1));
            pause_us += bar.pause_us / passes;
            wrong_milli += u64::from(bar.wrong) * 1000 / passes;
            stops += bar.gave_up;
        }
        facts_per_pass(pause_us, wrong_milli, stops)
    }

    /// Bar `measure`'s line: now against its earliest sessions.
    ///
    /// The earliest are its first sessions in which it was played, at most
    /// three and at most half of them; now is [`Bars::now`]. One session
    /// says only how it is now; one pass, not enough ([`ONCE`]).
    #[must_use]
    pub fn detail(&self, measure: u32) -> Option<String> {
        let played: Vec<&Cell> = self
            .sessions
            .iter()
            .filter_map(|session| session.bars.iter().find(|cell| cell.measure == measure))
            .collect();
        let now = self.now.get(MeasureNumber::new(measure)?)?;
        if now.passes < 2 {
            return Some(ONCE.to_owned());
        }
        let per_pass = |passes: u32, pause_us: u64, wrong: u32| {
            let passes = u64::from(passes.max(1));
            (pause_us / passes, u64::from(wrong) * 1000 / passes)
        };
        let (pause_now, wrong_now) = per_pass(now.passes, now.pause_us, now.wrong);
        let now_facts = facts_per_pass(pause_now, wrong_now, now.gave_up);
        if played.len() < 2 {
            return Some(format!("Now: {now_facts}."));
        }
        let earliest: Vec<&Cell> = played
            .iter()
            .take(EARLIEST.min(played.len() / 2))
            .copied()
            .collect();
        let (pause_then, wrong_then) = per_pass(
            earliest.iter().map(|cell| cell.passes).sum(),
            earliest.iter().map(|cell| cell.pause_us).sum(),
            earliest.iter().map(|cell| cell.wrong).sum(),
        );
        let then_facts = facts_per_pass(
            pause_then,
            wrong_then,
            earliest.iter().map(|cell| cell.stops).sum(),
        );
        Some(format!(
            "Now: {now_facts}. In your first sessions: {then_facts}."
        ))
    }

    /// The Progress page for `piece`.
    #[must_use]
    pub fn view(
        &self,
        piece: &hogwash_library::Piece,
        last_practised_ms: Option<u64>,
    ) -> ProgressView {
        let bars = piece.score().measures().len();
        ProgressView {
            id: piece.id().as_str().to_owned(),
            title: piece.title().to_owned(),
            composer: piece.composer().map(str::to_owned),
            bars,
            sessions: self.sessions.iter().rev().cloned().collect(),
            played_us: self.sessions.iter().map(|session| session.played_us).sum(),
            last_practised_ms,
            now: self
                .now
                .iter()
                .map(|(measure, bar)| NowBar {
                    measure: measure.get(),
                    level: bar.judged(),
                })
                .collect(),
            work_on_next: self.work_on_next(piece.reference()),
            details: (1..=u32::try_from(bars).unwrap_or(u32::MAX))
                .filter_map(|measure| {
                    let bar = MeasureNumber::new(measure)?;
                    let (from, to) = practise_range(piece.reference(), bar, bar);
                    Some(Detail {
                        measure,
                        line: self
                            .detail(measure)
                            .unwrap_or_else(|| "Not played yet.".to_owned()),
                        from: from.get(),
                        to: to.get(),
                    })
                })
                .collect(),
        }
    }
}

/// Microseconds as seconds to one decimal: "3.1".
fn seconds(us: u64) -> String {
    let tenths = (us + 50_000) / 100_000;
    format!("{}.{}", tenths / 10, tenths % 10)
}

/// Thousandths to one decimal, a whole number without one: "1", "1.5".
fn tenths(milli: u64) -> String {
    let tenths = (milli + 50) / 100;
    if tenths.is_multiple_of(10) {
        (tenths / 10).to_string()
    } else {
        format!("{}.{}", tenths / 10, tenths % 10)
    }
}

/// A bar's detail line when it has been played only once.
pub const ONCE: &str = "Played once so far. Play it again to see how it's going.";

/// What happened each time through, in words a pianist reads without help:
/// "about 3.1 s of hesitation each time through, a wrong note 1 time in 3,
/// you stopped here 2 times", or "clean". Pauses under 0.05 s aren't
/// mentioned.
fn facts_per_pass(pause_us: u64, wrong_milli: u64, stops: u32) -> String {
    let mut parts = Vec::new();
    if pause_us >= 50_000 {
        parts.push(format!(
            "about {} s of hesitation each time through",
            seconds(pause_us)
        ));
    }
    if let Some(twice) = 2000u64.checked_div(wrong_milli) {
        let one_in = twice.div_ceil(2);
        parts.push(if one_in >= 2 {
            format!("a wrong note 1 time in {one_in}")
        } else {
            let count = tenths(wrong_milli.max(1000));
            let notes = if count == "1" { "note" } else { "notes" };
            format!("{count} wrong {notes} each time through")
        });
    }
    if stops > 0 {
        let times = if stops == 1 { "time" } else { "times" };
        parts.push(format!("you stopped here {stops} {times}"));
    }
    if parts.is_empty() {
        "clean".to_owned()
    } else {
        parts.join(", ")
    }
}

/// Every session that can be read; none if the history folder can't be.
#[must_use]
pub fn sessions(records: &Records) -> Vec<SessionRecord> {
    records
        .sessions()
        .map(|(sessions, _unreadable)| sessions)
        .unwrap_or_default()
}

/// Replays every attempt kept for `piece` against `reference`, over
/// sessions already read (so a whole library reads them once).
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
        run_late(reference, &[(stall, late)])
    }

    /// [`run`], late before each of several steps.
    fn run_late(reference: &ReferencePerformance, stalls: &[(usize, u64)]) -> AttemptRecord {
        let mut played = ObservedPerformance::new();
        let mut at_us = 1_000_000;
        for step in 0..reference.len() {
            if step > 0 {
                let written = u64::from(reference.written_gap(step).unwrap_or(TICKS_PER_QUARTER));
                at_us += written * 600_000 / u64::from(TICKS_PER_QUARTER);
            }
            for &(stall, late) in stalls {
                if step == stall {
                    at_us += late;
                }
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

        let progress = progress_in(&sessions(&records), &piece, &minuet);

        assert_eq!(progress.runs, 2);
        let level = |measure: u32| progress.now.get(MeasureNumber::new(measure)?)?.judged();
        assert_eq!(level(weak), Some(3));
        assert!(
            progress
                .now
                .iter()
                .all(|(measure, bar)| measure.get() == weak || bar.judged() == Some(0))
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
            progress_in(&sessions, &piece, &minuet).weakest(),
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
        let judged = progress.now.get(MeasureNumber::new(once).expect("a bar"));
        assert_eq!(judged.map(Bar::judged), Some(None));
        assert_eq!(progress.detail(once).as_deref(), Some(ONCE));
        drop(records);
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_piece_never_played_has_no_progress() {
        let (records, piece, minuet) = minuet("empty");
        let progress = progress_in(&sessions(&records), &piece, &minuet);

        assert_eq!(progress, Progress::default());
        assert!(progress.work_on_next(&minuet).is_empty());
    }

    /// Sessions of `piece`, a day apart, one attempt each.
    fn days(piece: &PieceId, attempts: Vec<AttemptRecord>) -> Vec<SessionRecord> {
        attempts
            .into_iter()
            .enumerate()
            .map(|(day, attempt)| SessionRecord {
                id: format!("day{day}"),
                piece: piece.clone(),
                started: (1_761_300_000_000 + 86_400_000 * day as u64).to_string(),
                closed: true,
                attempts: vec![attempt],
            })
            .collect()
    }

    /// The first step of bar `measure`, or its second for bar 1 (the
    /// first note of an attempt is never timed).
    fn opening(reference: &ReferencePerformance, measure: u32) -> usize {
        (1..reference.len())
            .find(|&step| reference.at(step).map(|notes| notes.measure().get()) == Some(measure))
            .expect("the bar is in the piece")
    }

    // Worst first, neighbours merged, at most three, each looped from the
    // bar before; bar 2, the fourth, is left for later.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn work_on_next_merges_neighbours_and_lists_three() {
        let (records, piece, minuet) = minuet("next");
        let at = |measure| opening(&minuet, measure);
        let stalls = [
            (at(14), 19_000_000), // a gap over 20 s is a break, not a pause
            (at(10), 16_000_000),
            (at(6), 12_000_000),
            (at(7), 12_000_000),
            (at(2), 6_000_000),
        ];
        let progress = progress_in(
            &days(&piece, vec![run_late(&minuet, &stalls)]),
            &piece,
            &minuet,
        );

        let next: Vec<(u32, u32, u32, u32)> = progress
            .work_on_next(&minuet)
            .iter()
            .map(|passage| (passage.first, passage.last, passage.from, passage.to))
            .collect();

        assert_eq!(next, vec![(14, 14, 13, 14), (10, 10, 9, 10), (6, 7, 5, 7)]);
        let facts = &progress.work_on_next(&minuet)[0].facts;
        assert!(
            facts.starts_with("about ") && facts.ends_with(" s of hesitation each time through"),
            "{facts}"
        );
        drop(records);
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn trouble_in_bar_1_loops_from_bar_1_and_clean_playing_lists_nothing() {
        let (records, piece, minuet) = minuet("first");
        let stall = opening(&minuet, 1);

        let troubled = progress_in(
            &days(&piece, vec![run(&minuet, stall, 12_000_000)]),
            &piece,
            &minuet,
        );
        let clean = progress_in(&days(&piece, vec![run(&minuet, 0, 0)]), &piece, &minuet);

        let first = &troubled.work_on_next(&minuet)[0];
        assert_eq!(
            (first.first, first.last, first.from, first.to),
            (1, 1, 1, 1)
        );
        assert!(clean.work_on_next(&minuet).is_empty());
        assert_eq!(clean.detail(1).as_deref(), Some("Now: clean."));
        drop(records);
    }

    // Three bad sessions, then three clean: the earliest three against now.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_bar_s_trend_is_its_earliest_sessions_against_now() {
        let (records, piece, minuet) = minuet("trend");
        let stall = opening(&minuet, 6);
        let bad = || run(&minuet, stall, 12_000_000);
        let good = || run(&minuet, 0, 0);
        let progress = progress_in(
            &days(&piece, vec![bad(), bad(), bad(), good(), good(), good()]),
            &piece,
            &minuet,
        );

        let line = progress.detail(6).expect("bar 6 was played");

        assert!(
            line.starts_with("Now: clean. In your first sessions: about "),
            "{line}"
        );
        assert!(
            line.ends_with(" s of hesitation each time through."),
            "{line}"
        );
        assert!(progress.work_on_next(&minuet).is_empty());
        drop(records);
    }

    #[test]
    fn facts_read_as_plain_sentences() {
        assert_eq!(facts_per_pass(0, 0, 0), "clean");
        assert_eq!(facts_per_pass(40_000, 0, 0), "clean");
        assert_eq!(facts_per_pass(0, 333, 0), "a wrong note 1 time in 3");
        assert_eq!(
            facts_per_pass(3_100_000, 1000, 2),
            "about 3.1 s of hesitation each time through, \
             1 wrong note each time through, you stopped here 2 times"
        );
        assert_eq!(
            facts_per_pass(0, 2500, 1),
            "2.5 wrong notes each time through, you stopped here 1 time"
        );
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
        let history = progress_in(&sessions(&records), &piece, &prelude);

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
