//! Resolving offered material into a Piece or an explained refusal
//! (`004 §8.2`).

use hogwash_domain::PieceId;
use hogwash_engraving::{Engraver, EngravingFailure};
use hogwash_library::Piece;

use crate::format::recognise;
use crate::report::{UnsupportedInput, ValidationIssue, ValidationReport};

/// What became of material a performer offered.
///
/// Two outcomes and no third. `004 §8.2` requires an assessment failure to
/// resolve into [`NotAccepted`](Outcome::NotAccepted) *with an explanation* —
/// it never crashes, and it never discards the material without a result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The material became a Piece.
    Accepted {
        /// The Piece, ready to join the Library.
        piece: Box<Piece>,
        /// What was noticed while reading it (FR-105, FR-108).
        ///
        /// Present even when nothing was wrong: an empty report is the
        /// statement that nothing was lost, which `MVP §6.10` needs said.
        report: ValidationReport,
    },

    /// The material could not become a Piece, and here is why (`003 §7.15`).
    NotAccepted(UnsupportedInput),
}

/// Assesses `material`, using `engraver` to read it if it is worth reading.
///
/// # The order of operations is the requirement
///
/// The format is recognised **first**, before anything tries to read the
/// material as music. FR-104 requires unsupported content to be rejected
/// without producing a partial piece, and `MVP §6.9` requires the rejection
/// to name what was supplied — which an engine's parse failure cannot do,
/// because it reports that reading failed rather than what the thing was.
///
/// Handing a PDF to the engraver and relaying its complaint would satisfy
/// neither: the performer would be told their `MusicXML` is malformed, and
/// would go and check the wrong thing.
///
/// # Why the name is given rather than derived
///
/// `id` is supplied by the caller. It could have been derived here — from the
/// title, or from the material — but what a Piece's durable name is made of
/// is a persistence question (`ADR-0010`), and deriving one here would answer
/// it in a pure crate that has no business knowing. The caller is storage, or
/// something holding storage's answer.
///
/// `file_name` is what the performer's file was called, if known: the
/// title of last resort for a score that names itself nowhere.
///
/// # This never repairs anything
///
/// `003 §7.15` and `Product_Discovery.md §13.4`: material is never converted
/// or repaired. `material` is taken by shared reference, so that is carried
/// by the signature rather than by a promise.
pub fn assess(
    material: &[u8],
    id: PieceId,
    file_name: Option<&str>,
    engraver: &mut impl Engraver,
) -> Outcome {
    let format = recognise(material);

    if !format.is_supported() {
        return Outcome::NotAccepted(UnsupportedInput::in_format(format));
    }

    match engraver.engrave(material) {
        Ok(drawn) => {
            let details = drawn.details();
            let piece = Piece::new(
                id,
                &hogwash_library::title_from(details.title.as_deref(), file_name),
                details.composer.clone(),
                drawn.score().clone(),
            );

            Outcome::Accepted {
                piece: Box::new(piece),
                report: ValidationReport::of(unplaced(&drawn)),
            }
        }

        // The material looked like a supported format and turned out not to
        // be readable as music. That is still a refusal the performer can act
        // on — they know what they offered — so the format they were told is
        // the one recognised above, and the engine's complaint becomes the
        // reason.
        Err(EngravingFailure::Unreadable { detail }) => {
            Outcome::NotAccepted(UnsupportedInput::unreadable(format, &detail))
        }

        // Not the material's fault, and saying so matters: telling a
        // performer their score is broken when the engraving library is
        // missing sends them to fix a file that is fine.
        Err(EngravingFailure::EngineUnavailable { detail }) => {
            Outcome::NotAccepted(UnsupportedInput::engine_unavailable(format, &detail))
        }

        Err(EngravingFailure::Malformed(malformed)) => Outcome::NotAccepted(
            UnsupportedInput::engine_unavailable(format, &format!("{malformed:?}")),
        ),
    }
}

/// Positions the engraver could not attribute to anything drawn.
///
/// FR-105 and `MVP §6.10` require partial fidelity loss to be **visible
/// rather than silent**. A position with no placement is exactly that: the
/// music is there and will be practised, but nothing can be highlighted on
/// the page for it, so `MVP §6.6` degrades quietly at that note. Saying so is
/// the difference between a known limitation and a mystery.
fn unplaced(engraved: &hogwash_engraving::EngravedScore) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut index = 0;

    for measure in engraved.score().measures() {
        for _ in measure.positions() {
            let at = hogwash_domain::Location::new(measure.number(), index);
            index += 1;

            if engraved.placement_of(at).is_none() {
                issues.push(ValidationIssue::NotDrawn { at });
            }
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use hogwash_domain::{Location, MeasureNumber, PieceId, PitchSpelling, Step};
    use hogwash_engraving::{
        Details, ElementId, EngravedScore, Engraver, EngravingFailure, PageIndex, Placement,
    };
    use hogwash_notation::{Measure, Note, Position, Score};

    use crate::assessment::{Outcome, assess};
    use crate::format::OfferedFormat;
    use crate::report::ValidationIssue;

    const MUSICXML: &[u8] = include_bytes!("../tests/fixtures/two-bars.musicxml");

    /// An engraver that returns what the test tells it to.
    ///
    /// The real one needs a shared library, and none of what is being tested
    /// here is about Verovio: this is the decision about *what to do* with an
    /// engraver's answer, which is exactly what a port exists to let us test
    /// without one.
    struct Fake(Result<EngravedScore, EngravingFailure>);

    impl std::fmt::Debug for Fake {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.debug_struct("Fake").finish_non_exhaustive()
        }
    }

    impl Engraver for Fake {
        fn engrave(&mut self, _source: &[u8]) -> Result<EngravedScore, EngravingFailure> {
            self.0.clone()
        }
    }

    fn named() -> PieceId {
        PieceId::new("fixture").expect("a plain name is usable")
    }

    fn one_note_score() -> Score {
        let note = Note::new(PitchSpelling::new(Step::C, 0, 4)).expect("middle C sounds");
        let position = Position::new(vec![note]).expect("one note is a position");
        let measure = Measure::new(
            MeasureNumber::new(1).expect("bars are numbered from one"),
            vec![position],
        );

        Score::new(vec![measure]).expect("one measure is a score")
    }

    fn engraving(details: Details, placed: bool) -> EngravedScore {
        let placements = if placed {
            vec![(
                Location::new(MeasureNumber::FIRST, 0),
                Placement::new(PageIndex::FIRST, vec![ElementId::new("n1".to_owned())])
                    .expect("one element is not none"),
            )]
        } else {
            vec![]
        };

        EngravedScore::new(
            one_note_score(),
            details,
            vec!["<svg/>".to_owned()],
            placements,
        )
        .expect("well formed")
    }

    fn accepting(details: Details) -> Fake {
        Fake(Ok(engraving(details, true)))
    }

    // FR-100 — "given a well-formed, supported MusicXML file, when it is
    // imported, then the system produces a corresponding piece".
    #[test]
    fn a_supported_file_becomes_a_piece() {
        let details = Details {
            title: Some("Minuet in G".to_owned()),
            composer: Some("Petzold".to_owned()),
        };

        match assess(MUSICXML, named(), None, &mut accepting(details)) {
            Outcome::Accepted { piece, report } => {
                assert_eq!(piece.title(), "Minuet in G");
                assert_eq!(piece.composer(), Some("Petzold"));
                assert!(report.is_clean());
            }
            Outcome::NotAccepted(refused) => panic!("expected acceptance, got {refused:?}"),
        }
    }

    // FR-201 — a title is always available, even when the file gave none.
    #[test]
    fn a_file_with_no_title_still_becomes_a_named_piece() {
        match assess(MUSICXML, named(), None, &mut accepting(Details::default())) {
            Outcome::Accepted { piece, .. } => assert!(!piece.title().is_empty()),
            Outcome::NotAccepted(refused) => panic!("expected acceptance, got {refused:?}"),
        }
    }

    // FR-104 and MVP §6.9. The engraver is never asked: a PDF is refused on
    // what it is, so the performer is told they offered a PDF rather than
    // that their MusicXML is malformed.
    #[test]
    fn an_unsupported_format_is_refused_without_reading_it() {
        let mut never_called = Fake(Err(EngravingFailure::Unreadable {
            detail: "the engraver should not have been asked".to_owned(),
        }));

        match assess(b"%PDF-1.7 ...", named(), None, &mut never_called) {
            Outcome::NotAccepted(refused) => {
                assert_eq!(refused.format(), OfferedFormat::Pdf);
                assert!(refused.reason().contains("PDF"));
            }
            Outcome::Accepted { piece, .. } => panic!("expected refusal, got {piece:?}"),
        }
    }

    // FR-101 Deprecated and 001 §11 — MIDI is excluded permanently because it
    // records a performance rather than written music, and that reason is the
    // one thing a performer offering a MIDI file most needs told.
    #[test]
    fn a_midi_file_is_refused_with_the_reason_it_is_excluded() {
        let mut never_called = Fake(Err(EngravingFailure::Unreadable {
            detail: String::new(),
        }));

        match assess(b"MThd\x00\x00\x00\x06", named(), None, &mut never_called) {
            Outcome::NotAccepted(refused) => {
                let reason = refused.reason().to_lowercase();
                assert!(
                    reason.contains("performance") || reason.contains("played"),
                    "the reason should say a MIDI file records playing, and says: {reason}"
                );
            }
            Outcome::Accepted { piece, .. } => panic!("expected refusal, got {piece:?}"),
        }
    }

    // MVP §6.9 — "names the format supplied, explains why it will not work,
    // and states the route to a supported format". All three, for every
    // format that can be refused. An empty route is a rejection without a way
    // forward, which is the thing Product_Discovery.md §5 forbids.
    #[test]
    fn every_refusal_names_a_route_to_a_supported_format() {
        for material in [
            b"%PDF-1.7".as_slice(),
            b"MThd\x00\x00\x00\x06".as_slice(),
            b"PK\x03\x04score.mscx".as_slice(),
            b"dear diary".as_slice(),
        ] {
            let mut engraver = Fake(Err(EngravingFailure::Unreadable {
                detail: String::new(),
            }));

            match assess(material, named(), None, &mut engraver) {
                Outcome::NotAccepted(refused) => {
                    assert!(!refused.reason().is_empty());
                    assert!(
                        !refused.route().is_empty(),
                        "{:?} was refused with no way forward",
                        refused.format()
                    );
                    assert!(refused.route().contains("MusicXML"));
                }
                Outcome::Accepted { piece, .. } => panic!("expected refusal, got {piece:?}"),
            }
        }
    }

    // FR-102 — a supported format whose contents are not valid produces no
    // piece and reports that the content is invalid.
    #[test]
    fn a_supported_format_that_is_not_readable_produces_no_piece() {
        let mut engraver = Fake(Err(EngravingFailure::Unreadable {
            detail: "no music was found".to_owned(),
        }));

        match assess(MUSICXML, named(), None, &mut engraver) {
            Outcome::NotAccepted(refused) => {
                assert_eq!(refused.format(), OfferedFormat::MusicXml);
                assert!(refused.reason().contains("no music was found"));
            }
            Outcome::Accepted { piece, .. } => panic!("expected refusal, got {piece:?}"),
        }
    }

    // A missing engraver is not a broken score, and a performer sent to fix
    // their file when the library is missing will not find anything wrong
    // with it.
    #[test]
    fn a_missing_engraver_is_not_reported_as_a_bad_file() {
        let mut engraver = Fake(Err(EngravingFailure::EngineUnavailable {
            detail: "libverovio.so is missing".to_owned(),
        }));

        match assess(MUSICXML, named(), None, &mut engraver) {
            Outcome::NotAccepted(refused) => {
                assert!(!refused.blames_the_material());
                assert!(refused.reason().contains("libverovio"));
            }
            Outcome::Accepted { piece, .. } => panic!("expected refusal, got {piece:?}"),
        }
    }

    // FR-105 / MVP §6.10 — partial fidelity loss is visible rather than
    // silent. A note that could not be attributed to anything drawn will not
    // highlight, and saying so turns a mystery into a known limitation.
    #[test]
    fn a_note_that_could_not_be_drawn_is_reported() {
        let mut engraver = Fake(Ok(engraving(Details::default(), false)));

        match assess(MUSICXML, named(), None, &mut engraver) {
            Outcome::Accepted { report, .. } => {
                assert!(!report.is_clean());
                assert_eq!(
                    report.issues(),
                    [ValidationIssue::NotDrawn {
                        at: Location::new(MeasureNumber::FIRST, 0)
                    }]
                );
            }
            Outcome::NotAccepted(refused) => panic!("expected acceptance, got {refused:?}"),
        }
    }

    // 003 §7.15 and Product_Discovery.md §13.4 — never converted, never
    // repaired. Carried by the signature, and asserted so that a future
    // `&mut [u8]` has to break a test to arrive.
    #[test]
    fn assessment_leaves_the_material_untouched() {
        let material = MUSICXML.to_vec();
        let before = material.clone();

        let _ = assess(&material, named(), None, &mut accepting(Details::default()));

        assert_eq!(material, before);
    }
}
