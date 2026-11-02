//! Assembling written music and its engraving from what the engine returned.
//!
//! Three readings meet here, and each answers a different question:
//!
//! | Source | Question |
//! |---|---|
//! | the MEI ([`crate::mei`]) | what is each note, and which bar is it in? |
//! | the timemap ([`crate::timemap`]) | which notes are played together, in what order, on which beat? |
//! | the rendered pages | which page was each note drawn on? |
//!
//! Pure. Everything here takes strings and returns values, so the whole of
//! the adapter's judgement is testable with no library present.

use std::collections::{BTreeMap, BTreeSet};

use roxmltree::Document;
use segno_domain::{Location, MeasureNumber, Pitch};
use segno_engraving::{Details, ElementId, PageIndex, Placement};
use segno_notation::{Measure, Note, Position, Score, TICKS_PER_QUARTER};

use crate::failure::TranslationFailure;
use crate::mei::ReadMei;
use crate::timemap::Beats;

/// A Score, what the source said about it, and where its notes were drawn.
pub(crate) type Assembled = (Score, Details, Vec<(Location, Placement)>);

/// Builds a Score, its details, and where each Position was drawn.
///
/// `played` is the bars in the order they are played (the timemap's
/// `measureOn`), and `originals` traces a repeat's copied identifiers back to
/// the written ones (the expansion map). Both may be empty, for music with no
/// repeats.
pub(crate) fn assemble(
    read: &ReadMei,
    onsets: &[Vec<String>],
    played: &[String],
    beats: &Beats,
    originals: &BTreeMap<String, String>,
    pages: &[String],
) -> Result<Assembled, TranslationFailure> {
    let page_of = elements_by_page(pages)?;
    let original = |id: &str| originals.get(id).map_or(id, String::as_str).to_owned();
    let (bar_starts, lengths) = bar_times(read, played, beats, &original);

    // Each onset becomes at most one Position, gathered under the bar its
    // notes are in.
    let mut positions_by_measure: BTreeMap<MeasureNumber, Vec<(Position, Vec<ElementId>)>> =
        BTreeMap::new();

    // Grace notes seen but not yet given to a Position. They go to the first
    // Position at or after their onset (the notation crate then tolerates
    // them at the step after that too, where their main note usually is).
    let mut graces: Vec<Pitch> = Vec::new();

    // Notes tied over a barline into a bar's opening, by bar: held through
    // when the bar before is played, but struck when starting there. A
    // bar's opening is the first onset met in it.
    let mut held_into: BTreeMap<MeasureNumber, Vec<Note>> = BTreeMap::new();
    let mut opened: BTreeSet<MeasureNumber> = BTreeSet::new();

    for (index, sounding) in onsets.iter().enumerate() {
        let mut measure = None;
        let mut held = Vec::new();
        let mut bars_here = BTreeSet::new();
        let mut notes = Vec::new();
        let mut elements = Vec::new();
        let mut ornaments: Vec<Pitch> = Vec::new();

        for id in sounding {
            // A copy is a repeat's second pass over notes already read from
            // the first. The play order below is what plays them again.
            if original(id) != *id {
                continue;
            }

            // An identifier the timemap knows and the document does not.
            // Nothing can be said about such a note, and inventing one would
            // put a note in the Score that the performer cannot see.
            let Some(written) = read.note(id) else {
                continue;
            };
            bars_here.insert(written.measure);

            // Held from an earlier note, not struck again. An onset holding
            // nothing else then forms no Position at all.
            if written.continues_a_tie {
                if !written.grace
                    && let Some(note) = Note::new(written.spelling)
                {
                    held.push((written.measure, note));
                }
                continue;
            }

            let note = Note::new(written.spelling).ok_or_else(|| {
                TranslationFailure::UnreadableNote(
                    "a note lies outside the range of any key on the instrument".to_owned(),
                )
            })?;

            // Optional: tolerated if played, never waited for.
            if written.grace {
                graces.push(note.sounding());
                continue;
            }

            // A trill, mordent or turn is played as the main note plus its
            // neighbours; within two semitones of it is not a mistake.
            if written.ornamented {
                ornaments.extend(neighbours(note.sounding()));
            }

            measure.get_or_insert(written.measure);
            notes.push(note);
            elements.push(ElementId::new(id.clone()));
        }

        for (bar, note) in held {
            if !opened.contains(&bar) {
                held_into.entry(bar).or_default().push(note);
            }
        }
        opened.extend(bars_here);

        let (Some(measure), Some(position)) = (measure, Position::new(notes)) else {
            continue;
        };
        let position = position
            .tolerating(graces.drain(..).chain(ornaments))
            .at_beat(beat_of(beats, index, bar_starts.get(&measure)));

        positions_by_measure
            .entry(measure)
            .or_default()
            .push((position, elements));
    }

    let mut measures = Vec::new();
    let mut drawn: BTreeMap<MeasureNumber, Vec<Vec<ElementId>>> = BTreeMap::new();

    for &number in read.measures() {
        let (positions, elements): (Vec<_>, Vec<_>) = positions_by_measure
            .remove(&number)
            .unwrap_or_default()
            .into_iter()
            .unzip();

        drawn.insert(number, elements);
        measures.push(
            Measure::new(number, positions)
                .holding(held_into.remove(&number).unwrap_or_default())
                .lasting(lengths.get(&number).copied()),
        );
    }

    let order = play_order(read, played, &original, &drawn);
    let score = Score::new(measures)
        .and_then(|score| score.played_as(order))
        .ok_or(TranslationFailure::NotAScore)?;

    // One placement per step of the performance, so a repeated bar's steps
    // point at the same drawn notes again. Counts across the whole
    // performance, not per bar, because that is what
    // `ReferencePerformance::derived_from` produces and what `Location`
    // means. Getting this wrong would put every finding in the wrong place.
    let mut placements = Vec::new();
    let mut index = 0;

    for measure in score.performance() {
        let number = measure.number();

        for elements in drawn.get(&number).into_iter().flatten() {
            let at = Location::new(number, index);
            index += 1;

            // A page for the notes, if they were drawn. `elements` is
            // non-empty here, because a Position is.
            if let Some(page) = elements
                .first()
                .and_then(|first| page_of.get(first.as_str()))
                && let Some(placement) = Placement::new(*page, elements.clone())
            {
                placements.push((at, placement));
            }
        }
    }

    Ok((score, read.details().clone(), placements))
}

/// When each bar first begins, in quarter notes, and how long it lasts, in
/// ticks. A repeat plays a bar again later; its first time through is the
/// one written down. Empty when the timemap names no bars.
fn bar_times(
    read: &ReadMei,
    played: &[String],
    beats: &Beats,
    original: &impl Fn(&str) -> String,
) -> (BTreeMap<MeasureNumber, f64>, BTreeMap<MeasureNumber, u32>) {
    let mut starts = BTreeMap::new();
    let mut lengths = BTreeMap::new();
    if beats.bars.len() != played.len() {
        return (starts, lengths);
    }

    let ends = beats.bars.iter().skip(1).chain([&beats.end]);
    for ((id, &start), &end) in played.iter().zip(&beats.bars).zip(ends) {
        let Some(number) = read.measure(&original(id)) else {
            continue;
        };
        if starts.contains_key(&number) {
            continue;
        }
        starts.insert(number, start);
        if let Some(length) = ticks(end - start) {
            lengths.insert(number, length);
        }
    }
    (starts, lengths)
}

/// How far onset `index` falls into the bar that starts at `bar_start`.
fn beat_of(beats: &Beats, index: usize, bar_start: Option<&f64>) -> Option<u32> {
    ticks(beats.onsets.get(index)? - bar_start?)
}

/// A span of quarter notes in ticks, or `None` if it is negative or too long
/// to count.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "rounded and checked to be in range first"
)]
fn ticks(quarters: f64) -> Option<u32> {
    let ticks = (quarters * f64::from(TICKS_PER_QUARTER)).round();
    (0.0..=f64::from(u32::MAX))
        .contains(&ticks)
        .then_some(ticks as u32)
}

/// Every pitch within two semitones of `main`, `main` included.
fn neighbours(main: Pitch) -> impl Iterator<Item = Pitch> {
    let main = u8::from(main);
    (main.saturating_sub(2)..=main.saturating_add(2)).filter_map(Pitch::new)
}

/// The bars in the order they are played, as written bar numbers.
///
/// Empty — written order — when the timemap names no bars, or when it misses
/// a bar that has notes in it: trusting an order that would silently drop
/// music is worse than ignoring the repeats.
fn play_order(
    read: &ReadMei,
    played: &[String],
    original: &impl Fn(&str) -> String,
    drawn: &BTreeMap<MeasureNumber, Vec<Vec<ElementId>>>,
) -> Vec<MeasureNumber> {
    let order: Vec<MeasureNumber> = played
        .iter()
        .filter_map(|id| read.measure(&original(id)))
        .collect();

    let complete = drawn
        .iter()
        .filter(|(_, positions)| !positions.is_empty())
        .all(|(number, _)| order.contains(number));

    if complete { order } else { Vec::new() }
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
                // the choice reproducible.
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
    use std::collections::BTreeMap;

    use segno_domain::{Location, MeasureNumber, Step};

    use crate::mei::read;
    use crate::timemap::{Beats, onsets};
    use crate::translate::assemble;

    const MEI: &str = include_str!("../tests/fixtures/two-bars.mei");
    const TIMEMAP: &str = include_str!("../tests/fixtures/two-bars.timemap.json");
    const PAGE: &str = include_str!("../tests/fixtures/two-bars.page1.svg");

    /// The fixture, read end to end from real engine output.
    fn engraved() -> super::Assembled {
        assemble(
            &read(MEI).expect("readable MEI"),
            &onsets(TIMEMAP).expect("readable timemap"),
            &[],
            &Beats::default(),
            &BTreeMap::new(),
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

    // a Position is the notes required together. The closing chord
    // is one Position of three notes, not three Positions.
    #[test]
    fn a_chord_is_one_position_holding_all_its_notes() {
        let (score, _, _) = engraved();
        let bar_two = &score.measures()[1];

        assert_eq!(bar_two.positions()[1].notes().len(), 3);
    }

    // The whole point of the adapter: a place in the music resolves to the
    // drawn glyphs, which is what the design highlights.
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
            &[],
            &Beats::default(),
            &BTreeMap::new(),
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
            &[],
            &Beats::default(),
            &BTreeMap::new(),
            &[page.to_owned()],
        )
        .expect("assembles");

        assert_eq!(score.measures()[0].positions().len(), 1);
        assert_eq!(placements.len(), 1);
    }

    /// Two bars, one note each, inside a repeat: `|: a | b :|`. Verovio's
    /// timemap plays them twice, the second time under copied identifiers.
    fn repeated(played: &[&str]) -> super::Assembled {
        let mei = r#"<mei xmlns="http://www.music-encoding.org/ns/mei"><music><body><mdiv><score><section>
            <measure xml:id="m1" n="1"><staff><layer><note xml:id="a" oct="4" pname="c"/></layer></staff></measure>
            <measure xml:id="m2" n="2"><staff><layer><note xml:id="b" oct="4" pname="d"/></layer></staff></measure>
            </section></score></mdiv></body></music></mei>"#;
        let timemap = r#"[{"on":["a"],"qstamp":0},{"on":["b"],"qstamp":4},
            {"on":["a-rend2"],"qstamp":8},{"on":["b-rend2"],"qstamp":12}]"#;
        let expansion = r#"{"a":["a","a-rend2"],"b":["b","b-rend2"],
            "m1":["m1","m1-rend2"],"m2":["m2","m2-rend2"]}"#;
        let page = r#"<svg xmlns="http://www.w3.org/2000/svg"><g id="a"/><g id="b"/></svg>"#;

        assemble(
            &read(mei).expect("readable"),
            &onsets(timemap).expect("readable"),
            &played.iter().map(|&id| id.to_owned()).collect::<Vec<_>>(),
            &crate::timemap::beats(timemap).expect("readable"),
            &crate::timemap::originals(expansion).expect("readable"),
            &[page.to_owned()],
        )
        .expect("assembles")
    }

    // The repeat is played twice, in playing order, and each pass points at
    // the same drawn notes.
    #[test]
    fn a_repeat_is_expected_twice_in_the_order_it_is_played() {
        let (score, _, placements) = repeated(&["m1", "m2", "m1-rend2", "m2-rend2"]);

        let reference = segno_notation::ReferencePerformance::derived_from(&score);
        let required: Vec<u8> = reference
            .iter()
            .flat_map(|step| step.sounding().map(u8::from))
            .collect();
        assert_eq!(required, [60, 62, 60, 62]);

        assert_eq!(score.measures().len(), 2, "written once");
        assert_eq!(
            placements
                .iter()
                .map(|(at, placement)| (
                    at.measure().get(),
                    at.position(),
                    placement.elements()[0].as_str()
                ))
                .collect::<Vec<_>>(),
            [(1, 0, "a"), (2, 1, "b"), (1, 2, "a"), (2, 3, "b")]
        );
    }

    // An order that leaves out a bar with notes in it would silently drop
    // music. Written order is the safer answer.
    #[test]
    fn an_order_missing_a_bar_with_notes_falls_back_to_written_order() {
        let (score, _, placements) = repeated(&["m1", "m1-rend2"]);

        let played: Vec<u32> = score
            .performance()
            .iter()
            .map(|measure| measure.number().get())
            .collect();
        assert_eq!(played, [1, 2]);
        assert_eq!(placements.len(), 2);
    }

    // As Verovio lays out a grace note: it shares its onset with the other
    // hand's note, and its main note sounds a moment later. It is required
    // nowhere, and tolerated at both of those steps. The trilled F tolerates
    // its neighbours.
    #[test]
    fn grace_notes_are_tolerated_and_ornaments_tolerate_their_neighbours() {
        let mei = r##"<mei xmlns="http://www.music-encoding.org/ns/mei"><music><body><mdiv><score>
            <section><measure xml:id="m" n="1">
              <staff n="1"><layer><note xml:id="g" oct="5" pname="d" grace="acc"/><note xml:id="e" oct="5" pname="e"/><note xml:id="f" oct="5" pname="f"/></layer></staff>
              <staff n="2"><layer><note xml:id="b" oct="2" pname="g"/></layer></staff>
              <trill startid="#f"/>
            </measure></section></score></mdiv></body></music></mei>"##;
        let timemap =
            r#"[{"on":["g","b"],"qstamp":0},{"on":["e"],"qstamp":0.5},{"on":["f"],"qstamp":1}]"#;
        let page = r#"<svg xmlns="http://www.w3.org/2000/svg"><g id="g"/><g id="e"/><g id="f"/><g id="b"/></svg>"#;

        let (score, _, _) = assemble(
            &read(mei).expect("readable"),
            &onsets(timemap).expect("readable"),
            &[],
            &Beats::default(),
            &BTreeMap::new(),
            &[page.to_owned()],
        )
        .expect("assembles");
        let reference = segno_notation::ReferencePerformance::derived_from(&score);
        let pitch = |number| segno_domain::Pitch::new(number).expect("in range");

        let required: Vec<Vec<u8>> = reference
            .iter()
            .map(|step| step.sounding().map(u8::from).collect())
            .collect();
        assert_eq!(required, [vec![43], vec![76], vec![77]], "no grace note");

        let d = pitch(74);
        assert!(reference.at(0).expect("G2").tolerates(d));
        assert!(reference.at(1).expect("E5").tolerates(d));
        assert!(!reference.at(2).expect("F5").tolerates(d));

        let trilled = reference.at(2).expect("F5");
        assert!(trilled.tolerates(pitch(78)) && trilled.tolerates(pitch(79)));
        assert!(!trilled.tolerates(pitch(80)));
        assert!(reference.tolerates_after_the_end(pitch(77)));
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

    // The design end to end, against the engine rather than against a fixture
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
