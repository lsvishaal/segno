//! Assembling written music and its engraving from what the engine returned.
//!
//! Three readings meet here, and each answers a different question:
//!
//! | Source | Question |
//! |---|---|
//! | the MEI ([`crate::mei`]) | what is each note, and which bar is it in? |
//! | the timemap ([`crate::timemap`]) | which notes are played together, in what order? |
//! | the rendered pages | which page was each note drawn on? |
//!
//! Pure. Everything here takes strings and returns values, so the whole of
//! the adapter's judgement is testable with no library present.

use std::collections::BTreeMap;

use hogwash_domain::{Location, MeasureNumber};
use hogwash_engraving::{ElementId, PageIndex, Placement};
use hogwash_notation::{Measure, Note, Position, Score};
use roxmltree::Document;

use crate::failure::TranslationFailure;
use crate::mei::ReadMei;

/// Builds a Score and its placements.
pub(crate) fn assemble(
    read: &ReadMei,
    onsets: &[Vec<String>],
    pages: &[String],
) -> Result<(Score, Vec<(Location, Placement)>), TranslationFailure> {
    let page_of = elements_by_page(pages)?;

    // Each onset becomes at most one Position, gathered under the bar its
    // notes are in.
    let mut positions_by_measure: BTreeMap<MeasureNumber, Vec<(Position, Vec<ElementId>)>> =
        BTreeMap::new();

    for sounding in onsets {
        let mut measure = None;
        let mut notes = Vec::new();
        let mut elements = Vec::new();

        for id in sounding {
            // An identifier the timemap knows and the document does not.
            // Nothing can be said about such a note, and inventing one would
            // put a note in the Score that the performer cannot see.
            let Some(written) = read.note(id) else {
                continue;
            };

            let note = Note::new(written.spelling).ok_or_else(|| {
                TranslationFailure::UnreadableNote(
                    "a note lies outside the range of any key on the instrument".to_owned(),
                )
            })?;

            measure.get_or_insert(written.measure);
            notes.push(note);
            elements.push(ElementId::new(id.clone()));
        }

        let (Some(measure), Some(position)) = (measure, Position::new(notes)) else {
            continue;
        };

        positions_by_measure
            .entry(measure)
            .or_default()
            .push((position, elements));
    }

    let mut measures = Vec::new();
    let mut placements = Vec::new();
    // Counts across the whole piece, not per bar, because that is what
    // `ReferencePerformance::derived_from` produces and what `Location`
    // means. Getting this wrong would put every finding in the wrong place.
    let mut index = 0;

    for &number in read.measures() {
        let sounding = positions_by_measure.remove(&number).unwrap_or_default();
        let mut positions = Vec::new();

        for (position, elements) in sounding {
            let at = Location::new(number, index);
            index += 1;

            // A page for the notes, if they were drawn. `elements` is
            // non-empty here, because a Position is.
            if let Some(page) = elements
                .first()
                .and_then(|first| page_of.get(first.as_str()))
                && let Some(placement) = Placement::new(*page, elements)
            {
                placements.push((at, placement));
            }

            positions.push(position);
        }

        measures.push(Measure::new(number, positions));
    }

    let score = Score::new(measures).ok_or(TranslationFailure::NotAScore)?;

    Ok((score, placements))
}

/// Which page each drawn element is on.
///
/// Every identified element is recorded, not only notes: the caller asks
/// about the ones it has, and filtering here would mean knowing what the
/// caller will want.
fn elements_by_page(pages: &[String]) -> Result<BTreeMap<String, PageIndex>, TranslationFailure> {
    let mut located = BTreeMap::new();

    for (number, page) in pages.iter().enumerate() {
        let drawn = Document::parse(page)
            .map_err(|failure| TranslationFailure::NotADocument(failure.to_string()))?;

        for element in drawn.descendants() {
            if let Some(id) = element.attribute("id") {
                // First page wins. An identifier drawn on two pages would be
                // an engine defect; picking the earlier one at least keeps
                // the choice reproducible (FR-406).
                located
                    .entry(id.to_owned())
                    .or_insert(PageIndex::new(number));
            }
        }
    }

    Ok(located)
}

#[cfg(test)]
mod tests {
    use hogwash_domain::{Location, MeasureNumber, Step};

    use crate::mei::read;
    use crate::timemap::onsets;
    use crate::translate::assemble;

    const MEI: &str = include_str!("../tests/fixtures/two-bars.mei");
    const TIMEMAP: &str = include_str!("../tests/fixtures/two-bars.timemap.json");
    const PAGE: &str = include_str!("../tests/fixtures/two-bars.page1.svg");

    /// The fixture, read end to end from real engine output.
    fn engraved() -> (
        hogwash_notation::Score,
        Vec<(Location, hogwash_engraving::Placement)>,
    ) {
        assemble(
            &read(MEI).expect("readable MEI"),
            &onsets(TIMEMAP).expect("readable timemap"),
            &[PAGE.to_owned()],
        )
        .expect("the fixture assembles")
    }

    fn bar(number: u32) -> MeasureNumber {
        MeasureNumber::new(number).expect("fixture measures are numbered from one")
    }

    // The fixture is C D E F# | Gb + a C-E-G chord: two bars, four positions
    // then two.
    #[test]
    fn the_score_has_the_bars_and_positions_the_music_has() {
        let (score, _) = engraved();

        assert_eq!(
            score
                .measures()
                .iter()
                .map(|measure| (measure.number().get(), measure.positions().len()))
                .collect::<Vec<_>>(),
            [(1, 4), (2, 2)]
        );
    }

    // 003 §7.4 — a Position is the notes required together. The closing chord
    // is one Position of three notes, not three Positions.
    #[test]
    fn a_chord_is_one_position_holding_all_its_notes() {
        let (score, _) = engraved();
        let bar_two = &score.measures()[1];

        assert_eq!(bar_two.positions()[1].notes().len(), 3);
    }

    // The whole point of the adapter: a place in the music resolves to the
    // drawn glyphs, which is what MVP §6.6 highlights.
    #[test]
    fn every_position_is_placed_on_the_page_it_was_drawn_on() {
        let (score, placements) = engraved();

        let positions: usize = score
            .measures()
            .iter()
            .map(|measure| measure.positions().len())
            .sum();

        assert_eq!(placements.len(), positions);
        assert!(
            placements
                .iter()
                .all(|(_, placement)| placement.page().get() == 0)
        );
    }

    // A chord's placement covers all three glyphs, so a half-played chord can
    // be shown half played.
    #[test]
    fn a_chords_placement_covers_every_one_of_its_notes() {
        let (_, placements) = engraved();
        let (_, chord) = placements.last().expect("six placements");

        assert_eq!(chord.elements().len(), 3);
    }

    // Location's position counts across the whole piece, matching the index
    // ReferencePerformance::derived_from produces. If these disagreed, every
    // finding after bar one would point at the wrong note.
    #[test]
    fn positions_are_numbered_across_the_piece_and_not_restarted_each_bar() {
        let (_, placements) = engraved();

        assert_eq!(
            placements.iter().map(|(at, _)| *at).collect::<Vec<_>>(),
            [
                Location::new(bar(1), 0),
                Location::new(bar(1), 1),
                Location::new(bar(1), 2),
                Location::new(bar(1), 3),
                Location::new(bar(2), 4),
                Location::new(bar(2), 5),
            ]
        );
    }

    // ADR-0001 end to end, against the engine rather than against a fixture
    // written to agree with us: the F sharp closing bar 1 and the G flat
    // opening bar 2 are written differently and sound the same.
    #[test]
    fn the_score_keeps_both_spellings_of_one_key() {
        let (score, _) = engraved();

        let f_sharp = score.measures()[0].positions()[3].notes()[0];
        let g_flat = score.measures()[1].positions()[0].notes()[0];

        assert_eq!(f_sharp.spelling().step(), Step::F);
        assert_eq!(g_flat.spelling().step(), Step::G);
        assert_eq!(f_sharp.sounding(), g_flat.sounding());
    }

    // The finding that would otherwise be found in the field: an accidental
    // earlier in the bar still applies to a later note with nothing drawn on
    // it. The chord's G sounds flat, one semitone below a natural G.
    #[test]
    fn an_accidental_still_in_force_reaches_the_sounding_pitch() {
        let (score, _) = engraved();

        let chord = &score.measures()[1].positions()[1];
        let top = chord.notes()[2];

        assert_eq!(top.spelling().step(), Step::G);
        assert_eq!(u8::from(top.sounding()), 66);
    }
}
