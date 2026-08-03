//! Reading Verovio's MEI. Pure: a string in, written music out.
//!
//! This is where the real work of the adapter is, so it is deliberately
//! separated from the C boundary and tested against a checked-in MEI file
//! that Verovio actually produced. No library is needed to run these tests.

use std::collections::BTreeMap;

use hogwash_domain::{MeasureNumber, PitchSpelling, Step};
use roxmltree::{Document, Node};

use crate::failure::TranslationFailure;

/// One written note, as the document has it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WrittenNote {
    /// Which bar it is in.
    pub(crate) measure: MeasureNumber,
    /// How it is written — letter, alteration, octave.
    pub(crate) spelling: PitchSpelling,
}

/// Everything read out of one MEI document.
#[derive(Debug)]
pub(crate) struct ReadMei {
    measures: Vec<MeasureNumber>,
    // A BTreeMap so the reading is reproducible whatever order the document
    // is walked in (FR-406, NFR-301).
    notes: BTreeMap<String, WrittenNote>,
}

impl ReadMei {
    /// Every bar in the document, in document order.
    ///
    /// Bars with no notes are included. A Score must account for its music
    /// completely (`003 §7.10`), and dropping a bar of rests would renumber
    /// everything after it.
    pub(crate) fn measures(&self) -> &[MeasureNumber] {
        &self.measures
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
    let mut notes = BTreeMap::new();

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
                },
            );
        }
    }

    if measures.is_empty() {
        return Err(TranslationFailure::NoMusic);
    }

    Ok(ReadMei { measures, notes })
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

    let octave: i8 = note
        .attribute("oct")
        .and_then(|written| written.parse().ok())
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

    #[test]
    fn a_document_that_is_not_xml_is_refused() {
        assert!(read("this is not a score").is_err());
    }
}
