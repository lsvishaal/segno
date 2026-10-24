//! Reading which notes sound together, from Verovio's timemap.
//!
//! # Why the timemap rather than the document's structure
//!
//! A Position is *the notes required together* (`003 §7.4`), and in written
//! music "together" is not a structural relationship. A chord is one element,
//! but the right hand and left hand of a piano score are two staves, and a
//! note in each sounding on the same beat is still one Position — the
//! performer plays them at once. Two voices in one staff are the same story.
//!
//! Walking the document's nesting would put the right hand's notes and the
//! left hand's in separate Positions, and the cursor would then ask for the
//! whole treble line before accepting a single bass note. That is not a rare
//! failure: it is every piano piece.
//!
//! The timemap groups by onset, which is exactly the question being asked, so
//! it is what this reads. Verovio lists individual note identifiers under
//! each onset — a chord appears as its three notes, not as the chord element
//! — which is also what is wanted.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::failure::TranslationFailure;

/// One moment in Verovio's timemap.
///
/// `tempo` and `tstamp` are not read: they are wall-clock quantities, and
/// with the cursor waiting for the performer, the performer's speed is the
/// tempo. `qstamp` is the written rhythm, which is read.
#[derive(Debug, Deserialize)]
struct Moment {
    /// When, in quarter notes from the start.
    qstamp: f64,
    /// The notes that begin sounding here.
    #[serde(default)]
    on: Vec<String>,
    /// The bar that begins here, if one does.
    #[serde(default, rename = "measureOn")]
    measure_on: Option<String>,
}

/// The notes that sound together, in the order they are played.
///
/// Each entry is one Position's worth of note identifiers.
pub(crate) fn onsets(timemap: &str) -> Result<Vec<Vec<String>>, TranslationFailure> {
    Ok(moments(timemap)?
        .into_iter()
        // A moment where notes only stop is not a Position. The timemap's
        // last entry is always one of these.
        .filter(|moment| !moment.on.is_empty())
        .map(|moment| moment.on)
        .collect())
}

/// When things happen in the written rhythm, in quarter notes from the start.
#[derive(Debug, Default)]
pub(crate) struct Beats {
    /// When each of [`onsets`] sounds, in the same order.
    pub(crate) onsets: Vec<f64>,
    /// When each of [`measures`] begins, in the same order.
    pub(crate) bars: Vec<f64>,
    /// When the last note stops.
    pub(crate) end: f64,
}

/// The written rhythm of what [`onsets`] and [`measures`] list.
pub(crate) fn beats(timemap: &str) -> Result<Beats, TranslationFailure> {
    let moments = moments(timemap)?;
    Ok(Beats {
        onsets: moments
            .iter()
            .filter(|moment| !moment.on.is_empty())
            .map(|moment| moment.qstamp)
            .collect(),
        bars: moments
            .iter()
            .filter(|moment| moment.measure_on.is_some())
            .map(|moment| moment.qstamp)
            .collect(),
        end: moments.last().map_or(0.0, |moment| moment.qstamp),
    })
}

/// The bars, by identifier, in the order they are played.
///
/// The timemap is built from a copy of the document with repeats written
/// out, so a repeated bar appears again here under a copy's identifier —
/// see [`originals`].
pub(crate) fn measures(timemap: &str) -> Result<Vec<String>, TranslationFailure> {
    Ok(moments(timemap)?
        .into_iter()
        .filter_map(|moment| moment.measure_on)
        .collect())
}

fn moments(timemap: &str) -> Result<Vec<Moment>, TranslationFailure> {
    let mut moments: Vec<Moment> = serde_json::from_str(timemap)
        .map_err(|failure| TranslationFailure::UnreadableTiming(failure.to_string()))?;

    // Verovio emits these in order already. Sorting anyway costs nothing at
    // this size and means the reading does not depend on that staying true.
    //
    // `total_cmp` rather than `partial_cmp`: a NaN qstamp would otherwise
    // make the comparison inconsistent and the sort order arbitrary.
    moments.sort_by(|earlier, later| earlier.qstamp.total_cmp(&later.qstamp));

    Ok(moments)
}

/// Which element each identifier is a copy of, from Verovio's expansion map.
///
/// The map lists, for each repeated element, every identifier it has —
/// `{"a": ["a", "a-rend2"], "a-rend2": ["a", "a-rend2"]}` — the original
/// first. An identifier that is not in the map is not a copy of anything.
pub(crate) fn originals(
    expansion_map: &str,
) -> Result<BTreeMap<String, String>, TranslationFailure> {
    let map: BTreeMap<String, Vec<String>> = serde_json::from_str(expansion_map)
        .map_err(|failure| TranslationFailure::UnreadableTiming(failure.to_string()))?;

    let mut originals = BTreeMap::new();
    for copies in map.into_values() {
        if let Some((original, _)) = copies.split_first() {
            for copy in &copies {
                originals.insert(copy.clone(), original.clone());
            }
        }
    }

    Ok(originals)
}

#[cfg(test)]
mod tests {
    use crate::timemap::{beats, measures, onsets, originals};

    /// Real Verovio output for `tests/fixtures/two-bars.musicxml`.
    const TWO_BARS: &str = include_str!("../tests/fixtures/two-bars.timemap.json");

    // The fixture is C D E F# | Gb, then a C-E-G chord: five single notes
    // and one chord of three.
    #[test]
    fn each_onset_gathers_the_notes_that_start_together() {
        let onsets = onsets(TWO_BARS).expect("real Verovio output is readable");

        assert_eq!(
            onsets.iter().map(Vec::len).collect::<Vec<_>>(),
            [1, 1, 1, 1, 1, 3]
        );
    }

    #[test]
    fn a_chords_notes_arrive_together_and_in_order() {
        let onsets = onsets(TWO_BARS).expect("readable");

        assert_eq!(
            onsets.last().expect("six onsets"),
            &["qhghzyb", "s82fpt", "t3elz0m"]
        );
    }

    // The timemap has an entry for every moment something changes, including
    // the moment the last chord stops. Those carry only `off`, and a Position
    // where nothing starts is not a Position.
    #[test]
    fn a_moment_where_only_notes_stop_is_not_a_position() {
        let onsets =
            onsets(r#"[{"on":["a"],"qstamp":0},{"off":["a"],"qstamp":1},{"on":["b"],"qstamp":2}]"#)
                .expect("readable");

        assert_eq!(onsets, vec![vec!["a".to_owned()], vec!["b".to_owned()]]);
    }

    // FR-406 / NFR-301. The order the Cursor walks is the order the music is
    // played in, and it must not depend on the order the engine happened to
    // emit its entries.
    #[test]
    fn onsets_are_ordered_by_when_they_sound() {
        let onsets = onsets(r#"[{"on":["late"],"qstamp":9},{"on":["early"],"qstamp":1}]"#)
            .expect("readable");

        assert_eq!(
            onsets,
            vec![vec!["early".to_owned()], vec!["late".to_owned()]]
        );
    }

    // With includeMeasures, each bar's start is marked. A repeat plays bars
    // again under copied identifiers, in the order they are played.
    #[test]
    fn bars_are_listed_in_the_order_they_are_played() {
        let played = measures(
            r#"[{"on":["a"],"qstamp":0,"measureOn":"m1"},{"on":["b"],"qstamp":4,"measureOn":"m1-rend2"},
                {"on":["c"],"qstamp":2},{"qstamp":8,"measureOn":"m2"}]"#,
        )
        .expect("readable");

        assert_eq!(played, ["m1", "m1-rend2", "m2"]);
    }

    #[test]
    fn the_rhythm_is_read_alongside_the_onsets_and_bars() {
        let beats = beats(
            r#"[{"on":["a"],"qstamp":0,"measureOn":"m1"},{"on":["b"],"qstamp":1.5},
                {"off":["b"],"qstamp":3,"measureOn":"m2"},{"off":["c"],"qstamp":6}]"#,
        )
        .expect("readable");

        assert_eq!(beats.onsets, [0.0, 1.5]);
        assert_eq!(beats.bars, [0.0, 3.0]);
        assert!((beats.end - 6.0).abs() < f64::EPSILON);
    }

    // Verovio's own shape: every identifier of a repeated element maps to all
    // of them, the original first.
    #[test]
    fn a_copy_is_traced_back_to_its_original() {
        let originals = originals(
            r#"{"n": ["n", "n-rend2", "n-rend3"], "n-rend2": ["n", "n-rend2", "n-rend3"]}"#,
        )
        .expect("readable");

        assert_eq!(originals.get("n-rend2").map(String::as_str), Some("n"));
        assert_eq!(originals.get("n-rend3").map(String::as_str), Some("n"));
        assert_eq!(originals.get("n").map(String::as_str), Some("n"));
        assert_eq!(originals.get("other"), None);
    }

    #[test]
    fn an_expansion_map_that_is_not_json_is_refused() {
        assert!(originals("[").is_err());
    }

    #[test]
    fn a_timemap_that_is_not_json_is_refused() {
        assert!(onsets("{{").is_err());
    }

    #[test]
    fn an_entry_with_no_time_is_refused() {
        assert!(onsets(r#"[{"on":["a"]}]"#).is_err());
    }
}
