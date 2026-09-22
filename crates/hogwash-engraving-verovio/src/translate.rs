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
use hogwash_engraving::{Details, ElementId, PageIndex, Placement};
use hogwash_notation::{Measure, Note, Position, Score};
use roxmltree::Document;

use crate::failure::TranslationFailure;
use crate::mei::ReadMei;

/// A Score, what the source said about it, and where its notes were drawn.
pub(crate) type Assembled = (Score, Details, Vec<(Location, Placement)>);

/// Builds a Score, its details, and where each Position was drawn.
pub(crate) fn assemble(
    read: &ReadMei,
    onsets: &[Vec<String>],
    pages: &[String],
) -> Result<Assembled, TranslationFailure> {
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

            // Held from an earlier note, not struck again. An onset holding
            // nothing else then forms no Position at all.
            if written.continues_a_tie {
                continue;
            }

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

    Ok((score, read.details().clone(), placements))
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
    fn engraved() -> super::Assembled {
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
        let (score, _, _) = engraved();

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
        let (score, _, _) = engraved();
        let bar_two = &score.measures()[1];

        assert_eq!(bar_two.positions()[1].notes().len(), 3);
    }

    // The whole point of the adapter: a place in the music resolves to the
    // drawn glyphs, which is what MVP §6.6 highlights.
    #[test]
    fn every_position_is_placed_on_the_page_it_was_drawn_on() {
        let (score, _, placements) = engraved();

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

    // The timemap lists a tied-to note as sounding again; the Score must not
    // ask for it. Here the tie runs from the first C into the second, which
    // shares its onset with a bass E: only the E is required there.
    #[test]
    fn a_note_a_tie_arrives_at_is_not_required_again() {
        let mei = r##"<mei xmlns="http://www.music-encoding.org/ns/mei"><music><body><mdiv><score>
            <section><measure xml:id="m" n="1">
              <staff n="1"><layer><note xml:id="c1" oct="5" pname="c"/><note xml:id="c2" oct="5" pname="c"/><note xml:id="d" oct="5" pname="d"/></layer></staff>
              <staff n="2"><layer><note xml:id="lc" oct="3" pname="c"/><note xml:id="le" oct="3" pname="e"/></layer></staff>
              <tie startid="#c1" endid="#c2"/>
            </measure></section></score></mdiv></body></music></mei>"##;
        let timemap = r#"[{"on":["c1","lc"],"qstamp":0},{"on":["c2","le"],"qstamp":2},{"on":["d"],"qstamp":3}]"#;
        let page = r#"<svg xmlns="http://www.w3.org/2000/svg"><g id="c1"/><g id="c2"/><g id="d"/><g id="lc"/><g id="le"/></svg>"#;

        let (score, _, _) = assemble(
            &read(mei).expect("readable"),
            &onsets(timemap).expect("readable"),
            &[page.to_owned()],
        )
        .expect("assembles");

        let required: Vec<Vec<u8>> = score.measures()[0]
            .positions()
            .iter()
            .map(|position| {
                position
                    .notes()
                    .iter()
                    .map(|note| u8::from(note.sounding()))
                    .collect()
            })
            .collect();
        assert_eq!(required, [vec![72, 48], vec![52], vec![74]]);
    }

    // An onset where the only thing that happens is a held note continuing
    // is not a place the performer has to play anything.
    #[test]
    fn an_onset_holding_only_a_tie_continuation_is_not_a_position() {
        let mei = r##"<mei xmlns="http://www.music-encoding.org/ns/mei"><music><body><mdiv><score>
            <section><measure xml:id="m" n="1"><staff><layer>
              <note xml:id="a" oct="4" pname="c"/><note xml:id="b" oct="4" pname="c"/>
            </layer></staff><tie startid="#a" endid="#b"/></measure></section>
            </score></mdiv></body></music></mei>"##;
        let timemap = r#"[{"on":["a"],"qstamp":0},{"on":["b"],"qstamp":2}]"#;
        let page = r#"<svg xmlns="http://www.w3.org/2000/svg"><g id="a"/><g id="b"/></svg>"#;

        let (score, _, placements) = assemble(
            &read(mei).expect("readable"),
            &onsets(timemap).expect("readable"),
            &[page.to_owned()],
        )
        .expect("assembles");

        assert_eq!(score.measures()[0].positions().len(), 1);
        assert_eq!(placements.len(), 1);
    }

    // A chord's placement covers all three glyphs, so a half-played chord can
    // be shown half played.
    #[test]
    fn a_chords_placement_covers_every_one_of_its_notes() {
        let (_, _, placements) = engraved();
        let (_, chord) = placements.last().expect("six placements");

        assert_eq!(chord.elements().len(), 3);
    }

    // Location's position counts across the whole piece, matching the index
    // ReferencePerformance::derived_from produces. If these disagreed, every
    // finding after bar one would point at the wrong note.
    #[test]
    fn positions_are_numbered_across_the_piece_and_not_restarted_each_bar() {
        let (_, _, placements) = engraved();

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
        let (score, _, _) = engraved();

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
        let (score, _, _) = engraved();

        let chord = &score.measures()[1].positions()[1];
        let top = chord.notes()[2];

        assert_eq!(top.spelling().step(), Step::G);
        assert_eq!(u8::from(top.sounding()), 66);
    }
}
