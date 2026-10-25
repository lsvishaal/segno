//! A piece's history: every attempt ever kept for it, added up bar by bar,
//! to say which bars are hard for this performer over time.

use hogwash_analysis::Bars;
use hogwash_domain::PieceId;
use hogwash_notation::ReferencePerformance;
use hogwash_records::{Ended, Records};
use serde::Serialize;

/// Which bars of a piece have been hard, over every attempt kept.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeakBars {
    /// How many attempts (runs and loop passes) this is from.
    pub runs: usize,
    /// Each bar played at least twice, and how hard it is next to the
    /// others: 0 went fine, 1–4 lighter to deeper.
    pub bars: Vec<WeakBar>,
}

/// One bar's place on the scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeakBar {
    pub measure: u32,
    pub level: u8,
}

/// Replays every attempt kept for `piece` against `reference`. Sessions
/// that cannot be read are skipped, as the library does.
#[must_use]
pub fn weak_bars(records: &Records, piece: &PieceId, reference: &ReferencePerformance) -> WeakBars {
    let Ok((sessions, _unreadable)) = records.sessions() else {
        return WeakBars::default();
    };
    let mut total = Bars::default();
    let mut runs = 0;
    for attempt in sessions
        .iter()
        .filter(|session| &session.piece == piece)
        .flat_map(|session| &session.attempts)
    {
        // A score re-imported shorter leaves old attempts past its end.
        let to = attempt.to.unwrap_or(reference.len()).min(reference.len());
        let gave_up = matches!(attempt.ended, Some(Ended::Restart | Ended::Stop));
        total.add(&Bars::of(
            &attempt.played,
            reference,
            attempt.from.min(to)..to,
            gave_up,
        ));
        runs += 1;
    }
    WeakBars {
        runs,
        bars: total
            .levels()
            .into_iter()
            .map(|(measure, level)| WeakBar {
                measure: measure.get(),
                level,
            })
            .collect(),
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
            .find(|piece| piece.title() == "Minuet in G Major")
            .expect("the Minuet is on the shelf");
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

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_bar_stalled_in_across_sessions_is_the_weakest() {
        let (records, piece, minuet) = minuet("stalled");
        let stall = minuet.len() / 2;
        keep(
            &records,
            &piece,
            "one",
            vec![run(&minuet, stall, 3_000_000)],
        );
        keep(
            &records,
            &piece,
            "two",
            vec![run(&minuet, stall, 3_000_000)],
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
        assert_eq!(level(weak), Some(4));
        assert!(
            history
                .bars
                .iter()
                .all(|bar| bar.measure == weak || bar.level == 0)
        );
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_piece_never_played_has_no_weak_bars() {
        let (records, piece, minuet) = minuet("empty");

        assert_eq!(weak_bars(&records, &piece, &minuet), WeakBars::default());
    }

    // Budget: opening a piece stays quick with a long history.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn fifty_sessions_add_up_in_well_under_a_tenth_of_a_second() {
        let (records, piece, minuet) = minuet("fifty");
        for session in 0..50 {
            keep(
                &records,
                &piece,
                &format!("s{session}"),
                vec![run(&minuet, session, 1_000_000)],
            );
        }

        let began = Instant::now();
        let history = weak_bars(&records, &piece, &minuet);

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
