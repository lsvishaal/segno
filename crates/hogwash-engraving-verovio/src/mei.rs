//! Reading Verovio's MEI. Pure: a string in, written music out.
//!
//! This is where the real work of the adapter is, so it is deliberately
//! separated from the C boundary and tested against a checked-in MEI file
//! that Verovio actually produced. No library is needed to run these tests.

use std::collections::{BTreeMap, BTreeSet};

use hogwash_domain::{MeasureNumber, PitchSpelling, Step};
use hogwash_engraving::Details;
use roxmltree::{Document, Node};

use crate::failure::TranslationFailure;

/// One written note, as the document has it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WrittenNote {
    /// Which bar it is in.
    pub(crate) measure: MeasureNumber,
    /// How it is written — letter, alteration, octave.
    pub(crate) spelling: PitchSpelling,
    /// Whether a tie reaches this note from an earlier one. A tied-to note is
    /// drawn but not struck again, so it is never waited for.
    pub(crate) continues_a_tie: bool,
    /// A grace note: optional, never waited for.
    pub(crate) grace: bool,
    /// Carries a trill, mordent or turn, whose extra notes are tolerated.
    pub(crate) ornamented: bool,
}

/// Everything read out of one MEI document.
#[derive(Debug)]
pub(crate) struct ReadMei {
    details: Details,
    measures: Vec<MeasureNumber>,
    measure_ids: BTreeMap<String, MeasureNumber>,
    // A BTreeMap so the reading is reproducible whatever order the document
    // is walked in (FR-406, NFR-301).
    notes: BTreeMap<String, WrittenNote>,
}

impl ReadMei {
    /// What the document says about the work (FR-201).
    pub(crate) const fn details(&self) -> &Details {
        &self.details
    }

    /// Every bar in the document, in document order.
    ///
    /// Bars with no notes are included. A Score must account for its music
    /// completely (`003 §7.10`), and dropping a bar of rests would renumber
    /// everything after it.
    pub(crate) fn measures(&self) -> &[MeasureNumber] {
        &self.measures
    }

    /// The bar the document calls `id`.
    pub(crate) fn measure(&self, id: &str) -> Option<MeasureNumber> {
        self.measure_ids.get(id).copied()
    }

    /// The note the document calls `id`.
    pub(crate) fn note(&self, id: &str) -> Option<WrittenNote> {
        self.notes.get(id).copied()
    }
}

/// Reads written music out of Verovio's MEI.
pub(crate) fn read(mei: &str) -> Result<ReadMei, TranslationFailure> {
    let document = Document::parse(mei)
        .map_err(|failure| TranslationFailure::NotADocument(failure.to_string()))?;

    let mut measures = Vec::new();
    let mut measure_ids = BTreeMap::new();
    let mut notes = BTreeMap::new();
    let tied_to = tie_continuations(&document);
    let ornamented = ornament_targets(&document);

    // Every `measure` element anywhere in the document, in document order.
    // Verovio nests sections — the fixture has a `section` inside a `section`
    // — so walking a fixed path would find nothing. Depth does not carry
    // meaning here; order does.
    for measure in document
        .descendants()
        .filter(|node| node.has_tag_name("measure"))
    {
        let number = measure_number(measure)?;
        measures.push(number);
        if let Some(id) = identifier(measure) {
            measure_ids.insert(id.to_owned(), number);
        }

        for note in measure
            .descendants()
            .filter(|node| node.has_tag_name("note"))
        {
            let Some(id) = identifier(note) else {
                // A note the engine did not name cannot be pointed at later,
                // so it cannot be placed. Skipping is safer than inventing an
                // identifier that matches nothing in the SVG.
                continue;
            };

            notes.insert(
                id.to_owned(),
                WrittenNote {
                    measure: number,
                    spelling: spelling_of(note)?,
                    continues_a_tie: tied_to.contains(id),
                    grace: is_grace(note),
                    ornamented: ornamented.contains(id)
                        || note
                            .ancestors()
                            .filter(|node| node.has_tag_name("chord"))
                            .filter_map(identifier)
                            .any(|chord| ornamented.contains(chord)),
                },
            );
        }
    }

    if measures.is_empty() {
        return Err(TranslationFailure::NoMusic);
    }

    Ok(ReadMei {
        details: details_of(&document),
        measures,
        measure_ids,
        notes,
    })
}

/// What the header says about the work.
///
/// Absent rather than guessed. Verovio writes an empty `<title/>` for a
/// source that gave none, and an empty title is not a title — the Piece
/// supplies a placeholder, which is a decision about presentation and not the
/// engraver's to make.
///
/// The composer is a `<persName>` with `@role="composer"`, or failing that a
/// `<composer>` element; `MusicXML`'s creator types survive transcoding as
/// either, depending on the exporter.
///
/// Many real files (`MuseScore` 1 and 2 exports) carry neither, only the text
/// printed at the top of the first page, which Verovio keeps as
/// `<pgHead func="first"><rend halign=…>`. Where the header is silent, that
/// page is read the way a musician reads it: the centred line is the title,
/// the right-aligned one the composer.
fn details_of(document: &Document<'_>) -> Details {
    let text_of = |node: Node<'_, '_>| {
        // Text nodes only. `descendants` yields the element itself as well,
        // and an element's `text()` is its first text child — so counting
        // both reads every title twice.
        let text: String = node
            .descendants()
            .filter(roxmltree::Node::is_text)
            .filter_map(|node| node.text())
            .collect();
        let text = text.trim().to_owned();

        (!text.is_empty()).then_some(text)
    };

    let first = |tag: &str| {
        document
            .descendants()
            .find(|node| node.has_tag_name(tag))
            .and_then(text_of)
    };

    let composer = document
        .descendants()
        .find(|node| node.has_tag_name("persName") && node.attribute("role") == Some("composer"))
        .and_then(text_of)
        .or_else(|| first("composer"));

    // The first page's printed header, in order.
    let printed = |halign: &str| {
        document
            .descendants()
            .find(|node| node.has_tag_name("pgHead") && node.attribute("func") == Some("first"))
            .and_then(|head| {
                head.children()
                    .filter(|node| {
                        node.has_tag_name("rend") && node.attribute("halign") == Some(halign)
                    })
                    .find_map(text_of)
                    // A title set in several styled pieces reaches here cut
                    // short ("Sonate No. 14, " before an italic "Moonlight").
                    .map(|text| {
                        text.trim_end_matches([',', ';', ':', '-', '–', '—', ' '])
                            .to_owned()
                    })
                    .filter(|text| !text.is_empty())
            })
    };

    Details {
        title: first("title").or_else(|| printed("center")),
        composer: composer.or_else(|| {
            printed("right").map(|name| {
                name.strip_prefix("By ")
                    .or_else(|| name.strip_prefix("by "))
                    .unwrap_or(&name)
                    .trim()
                    .to_owned()
            })
        }),
    }
}

/// Every note a tie arrives at.
///
/// Verovio writes ties as `<tie startid="#a" endid="#b"/>` control events;
/// MEI also allows `@tie="m"` / `@tie="t"` (medial, terminal) on the note
/// itself. Both are read. Verovio's timemap lists a tied-to note as sounding
/// again, which would make the cursor wait for a key that is only held.
fn tie_continuations(document: &Document<'_>) -> BTreeSet<String> {
    let by_element = document
        .descendants()
        .filter(|node| node.has_tag_name("tie"))
        .filter_map(|tie| tie.attribute("endid"))
        .map(|end| end.trim_start_matches('#').to_owned());

    let by_attribute = document
        .descendants()
        .filter(|node| node.has_tag_name("note"))
        .filter(|note| {
            note.attribute("tie")
                .is_some_and(|tie| tie.contains('m') || tie.contains('t'))
        })
        .filter_map(identifier)
        .map(str::to_owned);

    by_element.chain(by_attribute).collect()
}

/// Whether a note is a grace note: `@grace` on it or on its chord, or
/// inside a `<graceGrp>`.
fn is_grace(note: Node<'_, '_>) -> bool {
    note.attribute("grace").is_some()
        || note.ancestors().any(|node| {
            node.has_tag_name("graceGrp")
                || (node.has_tag_name("chord") && node.attribute("grace").is_some())
        })
}

/// Every note or chord a trill, mordent or turn is attached to (`@startid`).
fn ornament_targets(document: &Document<'_>) -> BTreeSet<String> {
    document
        .descendants()
        .filter(|node| {
            node.has_tag_name("trill") || node.has_tag_name("mordent") || node.has_tag_name("turn")
        })
        .filter_map(|ornament| ornament.attribute("startid"))
        .map(|start| start.trim_start_matches('#').to_owned())
        .collect()
}

fn measure_number(measure: Node<'_, '_>) -> Result<MeasureNumber, TranslationFailure> {
    let written = measure.attribute("n").unwrap_or("");

    written
        .parse::<u32>()
        .ok()
        .and_then(MeasureNumber::new)
        .ok_or_else(|| TranslationFailure::UnnumberedMeasure(written.to_owned()))
}

/// The `xml:id` an element carries, which is also its identifier in the SVG.
///
/// Verovio writes the same identifier in both, which is the whole mechanism
/// behind `MVP §6.6`.
fn identifier<'document>(node: Node<'document, '_>) -> Option<&'document str> {
    node.attribute(("http://www.w3.org/XML/1998/namespace", "id"))
        .or_else(|| node.attribute("id"))
}

fn spelling_of(note: Node<'_, '_>) -> Result<PitchSpelling, TranslationFailure> {
    let name = note
        .attribute("pname")
        .ok_or_else(|| TranslationFailure::UnreadableNote("a note with no letter".to_owned()))?;

    let step = step_of(name)?;

    // Under an octave line (8va, 8vb) `@oct` is where the note is drawn and
    // `@oct.ges` where it sounds; the sounding octave is what is played.
    let octave: i8 = note
        .attribute("oct.ges")
        .or_else(|| note.attribute("oct"))
        .and_then(|octave| octave.parse().ok())
        .ok_or_else(|| {
            TranslationFailure::UnreadableNote(format!("{name} is in no readable octave"))
        })?;

    Ok(PitchSpelling::new(step, alteration_of(note)?, octave))
}

fn step_of(name: &str) -> Result<Step, TranslationFailure> {
    match name {
        "c" => Ok(Step::C),
        "d" => Ok(Step::D),
        "e" => Ok(Step::E),
        "f" => Ok(Step::F),
        "g" => Ok(Step::G),
        "a" => Ok(Step::A),
        "b" => Ok(Step::B),
        other => Err(TranslationFailure::UnreadableNote(format!(
            "{other} is not a note letter"
        ))),
    }
}

/// How far the note is altered, in semitones.
///
/// MEI keeps two accidentals and they are not the same question:
///
/// - `@accid` is the accidental **drawn on the page**.
/// - `@accid.ges` is the accidental that **applies** — from the key
///   signature, or from an earlier accidental in the same bar — whether or
///   not anything is drawn.
///
/// What matters here is the second. `ADR-0001` compares sounding pitch, and
/// the sounding pitch of a note in D major written as a plain F is F sharp,
/// with no accidental drawn anywhere near it. Reading only the drawn one
/// would make the system call correctly-played notes wrong throughout every
/// piece that has a key signature.
///
/// This does not undermine ADR-0004's wording either: the note is then named
/// "F sharp", which is what a musician calls it, drawn accidental or not.
///
/// Either may sit on the note itself or on a child `<accid>` element;
/// Verovio writes the child form, and both are accepted.
fn alteration_of(note: Node<'_, '_>) -> Result<i8, TranslationFailure> {
    let children = || note.children().filter(|node| node.has_tag_name("accid"));

    let applying = note
        .attribute("accid.ges")
        .or_else(|| children().find_map(|accid| accid.attribute("accid.ges")));

    let drawn = note
        .attribute("accid")
        .or_else(|| children().find_map(|accid| accid.attribute("accid")));

    match applying.or(drawn) {
        None => Ok(0),
        Some(accidental) => semitones(accidental),
    }
}

fn semitones(accidental: &str) -> Result<i8, TranslationFailure> {
    match accidental {
        "n" => Ok(0),
        "s" => Ok(1),
        "f" => Ok(-1),
        "ss" | "x" => Ok(2),
        "ff" => Ok(-2),
        "ts" | "xs" | "sx" => Ok(3),
        "tf" => Ok(-3),
        // Quarter tones and the rest of MEI's microtonal vocabulary. A whole
        // number of semitones cannot hold them, and rounding would move the
        // note — after which the system would report a performer wrong for
        // playing exactly what is on the page. Refusing says so instead.
        other => Err(TranslationFailure::UnsupportedAccidental(other.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use hogwash_domain::Step;

    use crate::mei::read;

    /// Real Verovio output for `tests/fixtures/two-bars.musicxml`, captured
    /// from the engine rather than written by hand — a format guessed at is a
    /// format the tests agree with and reality does not.
    const TWO_BARS: &str = include_str!("../tests/fixtures/two-bars.mei");

    #[test]
    fn every_measure_in_the_document_is_read_in_order() {
        let read = read(TWO_BARS).expect("real Verovio output is readable");

        assert_eq!(
            read.measures()
                .iter()
                .map(|bar| bar.get())
                .collect::<Vec<_>>(),
            [1, 2]
        );
    }

    // The fixture's bar 1 is C D E F#, and the note ids are Verovio's own.
    #[test]
    fn a_note_keeps_the_letter_and_octave_the_score_wrote() {
        let read = read(TWO_BARS).expect("readable");
        let note = read.note("yv3lqdm").expect("the first note of bar 1");

        assert_eq!(note.spelling.step(), Step::C);
        assert_eq!(note.spelling.octave(), 4);
        assert_eq!(note.spelling.alter(), 0);
        assert_eq!(note.measure.get(), 1);
    }

    // Real Verovio output for a note drawn as D4 under an 8va line.
    #[test]
    fn a_note_under_an_octave_line_is_read_at_the_octave_it_sounds() {
        let mei = TWO_BARS.replacen(r#"oct="4""#, r#"oct="4" oct.ges="5""#, 1);
        let read = read(&mei).expect("readable");

        assert_eq!(
            read.note("yv3lqdm")
                .expect("the first note")
                .spelling
                .octave(),
            5
        );
    }

    // MEI writes an accidental as a child <accid> element, not an attribute
    // of the note (S0 Q2b). A reader that only looked at attributes would
    // silently turn every F sharp into an F.
    #[test]
    fn a_written_accidental_reaches_the_spelling() {
        let read = read(TWO_BARS).expect("readable");
        let f_sharp = read.note("b8j2ung").expect("the F sharp ending bar 1");

        assert_eq!(f_sharp.spelling.step(), Step::F);
        assert_eq!(f_sharp.spelling.alter(), 1);
    }

    // ADR-0001, confirmed against real engine output rather than reasoned
    // about: F sharp and G flat are two spellings of one key.
    #[test]
    fn an_enharmonic_pair_keeps_two_spellings_and_one_sound() {
        let read = read(TWO_BARS).expect("readable");
        let f_sharp = read.note("b8j2ung").expect("F sharp").spelling;
        let g_flat = read.note("o1w2vfrn").expect("G flat").spelling;

        assert_ne!(f_sharp, g_flat);
        assert_eq!(f_sharp.sounding(), g_flat.sounding());
    }

    // The one that would otherwise be found in the field.
    //
    // MEI distinguishes the accidental that is *drawn* (@accid) from the one
    // that merely *applies* (@accid.ges) — from a key signature, or from an
    // earlier accidental in the same bar. The fixture's closing chord is
    // written C E G with no accidental on the G, but bar 2 opened with a G
    // flat, so the G sounds flat and Verovio says so with @accid.ges.
    //
    // Reading only @accid would make that note sound 67 while the performer
    // correctly plays 66, and the system would call a right note wrong. This
    // is not an edge case: in any key signature, most altered notes carry no
    // drawn accidental at all.
    #[test]
    fn an_accidental_that_applies_without_being_drawn_still_alters_the_note() {
        let read = read(TWO_BARS).expect("readable");
        let g = read.note("t3elz0m").expect("the G of the closing chord");

        assert_eq!(g.spelling.step(), Step::G);
        assert_eq!(
            g.spelling.alter(),
            -1,
            "the bar's earlier G flat still applies"
        );
    }

    // When a note carries both — a courtesy accidental is drawn while
    // something else is what actually applies — the one that applies wins,
    // because ADR-0001 compares what sounds. The fixture above cannot pin
    // this: its flat-by-context G carries no drawn accidental at all, so
    // either precedence would give the same answer there.
    #[test]
    fn where_both_accidentals_are_given_the_one_that_applies_wins() {
        let read = read(
            r#"<mei xmlns="http://www.music-encoding.org/ns/mei"><music><body><mdiv><score>
               <section><measure xml:id="m" n="1"><staff><layer>
               <note xml:id="q" oct="4" pname="c"><accid accid="n" accid.ges="s"/></note>
               </layer></staff></measure></section>
               </score></mdiv></body></music></mei>"#,
        )
        .expect("readable");

        assert_eq!(read.note("q").expect("the note").spelling.alter(), 1);
    }

    // FR-201 — a Piece carries "at minimum a title and, where available, a
    // composer". Both come from the header of the same document the notes do.
    #[test]
    fn the_title_and_composer_are_read_from_the_header() {
        let read = read(
            r#"<mei xmlns="http://www.music-encoding.org/ns/mei">
               <meiHead><fileDesc><titleStmt>
                 <title>Minuet in G</title>
                 <respStmt><persName role="composer">Petzold</persName></respStmt>
               </titleStmt></fileDesc></meiHead>
               <music><body><mdiv><score><section>
                 <measure xml:id="m" n="1"/>
               </section></score></mdiv></body></music></mei>"#,
        )
        .expect("readable");

        assert_eq!(read.details().title.as_deref(), Some("Minuet in G"));
        assert_eq!(read.details().composer.as_deref(), Some("Petzold"));
    }

    // Verovio writes an empty <title/> where the source gave none, and an
    // empty title is not a title. Reporting it as absent lets the Piece
    // supply a placeholder, which is a presentation decision and not the
    // engraver's to make. The project's own fixture is exactly this case.
    #[test]
    fn a_document_with_no_title_reports_none_rather_than_an_empty_one() {
        let read = read(TWO_BARS).expect("readable");

        assert_eq!(read.details().title, None);
        assert_eq!(read.details().composer, None);
    }

    /// The header Verovio writes for a `MusicXML` file with no `<work-title>`
    /// or creator, only printed page text: `pgHead` copied from its output
    /// for the performer's own Gymnopédie No. 1 and Ode to Joy files.
    fn printed_only(page_head: &str) -> String {
        format!(
            r#"<mei xmlns="http://www.music-encoding.org/ns/mei">
               <meiHead><fileDesc><titleStmt><title/></titleStmt></fileDesc></meiHead>
               <music><body><mdiv><score><scoreDef>{page_head}</scoreDef><section>
                 <measure xml:id="m" n="1"/>
               </section></score></mdiv></body></music></mei>"#
        )
    }

    #[test]
    fn a_title_printed_on_the_page_is_read_when_the_header_has_none() {
        let read = read(&printed_only(
            r#"<pgHead xml:id="x1qtaits" func="first">
                 <rend xml:id="v16ok2te" halign="center" valign="top">Gymnopédie No.1</rend>
                 <rend xml:id="ybfexq8" halign="right" valign="top">Erik Satie</rend>
               </pgHead>"#,
        ))
        .expect("readable");

        assert_eq!(read.details().title.as_deref(), Some("Gymnopédie No.1"));
        assert_eq!(read.details().composer.as_deref(), Some("Erik Satie"));
    }

    #[test]
    fn a_printed_by_line_names_the_composer() {
        let read = read(&printed_only(
            r#"<pgHead xml:id="upbef6r" func="first">
                 <rend xml:id="s8wdo1n" halign="center" valign="top">Ode to Joy</rend>
                 <rend xml:id="vc9ps8j" halign="right" valign="bottom">By Ludwig Van Beethoven </rend>
                 <rend xml:id="a1aoe0yg" halign="center" valign="top">(Easy variation)</rend>
                 <rend xml:id="c1achc5r" halign="left" valign="bottom">Arranged</rend>
               </pgHead>"#,
        ))
        .expect("readable");

        assert_eq!(read.details().title.as_deref(), Some("Ode to Joy"));
        assert_eq!(
            read.details().composer.as_deref(),
            Some("Ludwig Van Beethoven")
        );
    }

    // Beethoven's "Sonate No. 14, *Moonlight*" as Verovio keeps it: only the
    // first styled piece, ending in a comma.
    #[test]
    fn a_printed_title_cut_short_loses_its_dangling_punctuation() {
        let read = read(&printed_only(
            r#"<pgHead xml:id="o1fct1ci" func="first">
                 <rend xml:id="m1c31gxz" halign="center" valign="top">Sonate No. 14, </rend>
                 <rend xml:id="snkm232" halign="center" valign="top">1 </rend>
                 <rend xml:id="a10n7sni" halign="right" valign="bottom">Ludwig van Beethoven </rend>
               </pgHead>"#,
        ))
        .expect("readable");

        assert_eq!(read.details().title.as_deref(), Some("Sonate No. 14"));
        assert_eq!(
            read.details().composer.as_deref(),
            Some("Ludwig van Beethoven")
        );
    }

    // Canon in D has both; what the file states wins over what it prints.
    #[test]
    fn the_header_wins_over_the_printed_page() {
        let read = read(
            r#"<mei xmlns="http://www.music-encoding.org/ns/mei">
               <meiHead><fileDesc><titleStmt>
                 <title>Canon in D</title>
                 <respStmt><persName role="composer">Johann Pachelbel</persName></respStmt>
               </titleStmt></fileDesc></meiHead>
               <music><body><mdiv><score><scoreDef>
                 <pgHead func="first"><rend halign="center">Printed title</rend>
                   <rend halign="right">Printed composer</rend></pgHead>
               </scoreDef><section><measure xml:id="m" n="1"/></section></score></mdiv></body></music></mei>"#,
        )
        .expect("readable");

        assert_eq!(read.details().title.as_deref(), Some("Canon in D"));
        assert_eq!(read.details().composer.as_deref(), Some("Johann Pachelbel"));
    }

    // Footers (Für Elise's rights line on every page) are never a title.
    #[test]
    fn a_footer_is_never_taken_for_a_title() {
        let read = read(&printed_only(
            r#"<pgFoot func="first"><rend halign="center">pianolessenassen.nl/bladmuziek</rend></pgFoot>"#,
        ))
        .expect("readable");

        assert_eq!(read.details().title, None);
    }

    #[test]
    fn a_note_that_is_not_in_the_document_is_not_found() {
        let read = read(TWO_BARS).expect("readable");

        assert!(read.note("no-such-note").is_none());
    }

    // 003 §7.8 — measures are numbered from one. A document numbering from
    // zero is not something to quietly renumber.
    #[test]
    fn a_measure_numbered_zero_is_refused() {
        let refused = read(
            r#"<mei xmlns="http://www.music-encoding.org/ns/mei"><music><body><mdiv><score>
               <section><measure xml:id="m" n="0"/></section>
               </score></mdiv></body></music></mei>"#,
        );

        assert!(refused.is_err());
    }

    // Quarter-tone accidentals exist in MEI and cannot be represented as a
    // whole number of semitones. Refusing is honest; rounding would move the
    // note and then report the performer wrong for playing what is written.
    #[test]
    fn an_accidental_finer_than_a_semitone_is_refused() {
        let refused = read(
            r#"<mei xmlns="http://www.music-encoding.org/ns/mei"><music><body><mdiv><score>
               <section><measure xml:id="m" n="1"><staff><layer>
               <note xml:id="q" oct="4" pname="c"><accid accid="1qs"/></note>
               </layer></staff></measure></section>
               </score></mdiv></body></music></mei>"#,
        );

        assert!(refused.is_err());
    }

    // A held note is struck once. The note a tie arrives at is still read
    // (it is drawn), but it is marked so nothing waits for it.
    #[test]
    fn a_note_a_tie_arrives_at_is_marked_as_continuing_it() {
        let read = read(
            r##"<mei xmlns="http://www.music-encoding.org/ns/mei"><music><body><mdiv><score>
               <section><measure xml:id="m" n="1"><staff><layer>
               <note xml:id="first" oct="5" pname="c"/>
               <note xml:id="held" oct="5" pname="c"/>
               <note xml:id="after" oct="5" pname="d"/>
               </layer></staff>
               <tie xml:id="t1" startid="#first" endid="#held"/>
               </measure></section>
               </score></mdiv></body></music></mei>"##,
        )
        .expect("readable");

        assert!(!read.note("first").expect("first").continues_a_tie);
        assert!(read.note("held").expect("held").continues_a_tie);
        assert!(!read.note("after").expect("after").continues_a_tie);
    }

    #[test]
    fn a_tie_written_as_an_attribute_is_read_too() {
        let read = read(
            r#"<mei xmlns="http://www.music-encoding.org/ns/mei"><music><body><mdiv><score>
               <section><measure xml:id="m" n="1"><staff><layer>
               <note xml:id="i" oct="5" pname="c" tie="i"/>
               <note xml:id="m2" oct="5" pname="c" tie="m"/>
               <note xml:id="t" oct="5" pname="c" tie="t"/>
               </layer></staff></measure></section>
               </score></mdiv></body></music></mei>"#,
        )
        .expect("readable");

        assert!(!read.note("i").expect("i").continues_a_tie);
        assert!(read.note("m2").expect("m2").continues_a_tie);
        assert!(read.note("t").expect("t").continues_a_tie);
    }

    // As Verovio writes them from MusicXML: @grace on a note, and a trill as
    // a control event pointing at its note. A mordent on a chord marks every
    // note of it.
    #[test]
    fn grace_notes_and_ornamented_notes_are_marked() {
        let read = read(
            r##"<mei xmlns="http://www.music-encoding.org/ns/mei"><music><body><mdiv><score>
               <section><measure xml:id="m" n="1"><staff><layer>
               <note xml:id="g" oct="5" pname="d" grace="acc"/>
               <note xml:id="e" oct="5" pname="e"/>
               <note xml:id="f" oct="5" pname="f"/>
               <chord xml:id="ch"><note xml:id="c1" oct="4" pname="c"/><note xml:id="c2" oct="4" pname="e"/></chord>
               <graceGrp><note xml:id="gg" oct="4" pname="b"/></graceGrp>
               </layer></staff>
               <trill startid="#f"/><mordent startid="#ch"/>
               </measure></section>
               </score></mdiv></body></music></mei>"##,
        )
        .expect("readable");

        let note = |id| read.note(id).expect("present");
        assert!(note("g").grace && note("gg").grace);
        assert!(!note("e").grace && !note("f").grace);
        assert!(note("f").ornamented);
        assert!(note("c1").ornamented && note("c2").ornamented);
        assert!(!note("e").ornamented);
    }

    #[test]
    fn a_document_that_is_not_xml_is_refused() {
        assert!(read("this is not a score").is_err());
    }
}
