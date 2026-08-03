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

use serde::Deserialize;

use crate::failure::TranslationFailure;

/// One moment in Verovio's timemap.
///
/// Only two fields are read of the several Verovio writes. `tempo` and
/// `tstamp` are wall-clock quantities, and `003 §15` does not model time —
/// `MVP §5`'s wait-for-me cursor makes the performer's speed the tempo, so
/// there is nothing here to compare them against. Reading them would be
/// storing up a temptation.
#[derive(Debug, Deserialize)]
struct Moment {
    /// When, in quarter notes from the start.
    qstamp: f64,
    /// The notes that begin sounding here.
    #[serde(default)]
    on: Vec<String>,
}

/// The notes that sound together, in the order they are played.
///
/// Each entry is one Position's worth of note identifiers.
pub(crate) fn onsets(timemap: &str) -> Result<Vec<Vec<String>>, TranslationFailure> {
    let mut moments: Vec<Moment> = serde_json::from_str(timemap)
        .map_err(|failure| TranslationFailure::UnreadableTiming(failure.to_string()))?;

    // Verovio emits these in order already. Sorting anyway costs nothing at
    // this size and means the reading does not depend on that staying true —
    // FR-406 is release-blocking (`MVP §9.2`), and "it happened to be sorted"
    // is not a property to rest it on.
    //
    // `total_cmp` rather than `partial_cmp`: a NaN qstamp would otherwise
    // make the comparison inconsistent and the sort order arbitrary.
    moments.sort_by(|earlier, later| earlier.qstamp.total_cmp(&later.qstamp));

    Ok(moments
        .into_iter()
        // A moment where notes only stop is not a Position. The timemap's
        // last entry is always one of these.
        .filter(|moment| !moment.on.is_empty())
        .map(|moment| moment.on)
        .collect())
}

#[cfg(test)]
mod tests {
    use crate::timemap::onsets;

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

    #[test]
    fn a_timemap_that_is_not_json_is_refused() {
        assert!(onsets("{{").is_err());
    }

    #[test]
    fn an_entry_with_no_time_is_refused() {
        assert!(onsets(r#"[{"on":["a"]}]"#).is_err());
    }
}
