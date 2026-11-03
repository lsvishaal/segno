//! What was noticed, and what to say when nothing can be done.

use segno_domain::Location;

use crate::format::OfferedFormat;

/// Something noticed while reading material that was otherwise accepted.
///
/// An issue is **not** a refusal. The Piece exists and can be practised;
/// this records what will be a little worse than it should be, which
/// The design requires to be visible rather than silent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValidationIssue {
    /// A note is in the music but nothing on the page corresponds to it.
    ///
    /// It will still be expected, and playing it will still advance the
    /// cursor; what will not happen is the design's highlight, because there
    /// is nothing to highlight. Silently, that reads as a bug.
    NotDrawn {
        /// Where in the music.
        at: Location,
    },
}

impl ValidationIssue {
    /// The issue in words, for the performer.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::NotDrawn { at } => format!(
                "Bar {}: a note could not be shown on the score. \
                 It will still be expected when you play.",
                at.measure().get()
            ),
        }
    }
}

/// What was noticed about one piece of material.
///
/// Produced whether or not anything was wrong. An empty report is a
/// statement — *nothing was lost* — and the design needs that said as much
/// as it needs the other case.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationReport {
    issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// A report of `issues`, in the order they occur in the music.
    #[must_use]
    pub fn of(mut issues: Vec<ValidationIssue>) -> Self {
        // Sorted so the report reads in the order a performer would meet the
        // problems, and so two runs agree.
        issues.sort();
        issues.dedup();

        Self { issues }
    }

    /// Everything noticed, in the order it occurs.
    #[must_use]
    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }

    /// Whether the material was represented completely.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    /// The report in words, one line per issue.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        if self.issues.is_empty() {
            return vec!["The whole score was read.".to_owned()];
        }

        self.issues.iter().map(ValidationIssue::describe).collect()
    }
}

/// Material that cannot become a Piece, and why.
///
/// The design requires that *"what it is, and why it cannot be used, are
/// always known"*, and the design requires three things to reach the
/// performer: the format supplied, why it will not work, and the route to one
/// that will. So all three are fields, and none of them is optional — a
/// refusal with no way forward is the thing the design exists
/// to prevent.
///
/// It carries **no implication that the performer did anything wrong**
///, and the wording throughout is chosen accordingly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedInput {
    format: OfferedFormat,
    reason: String,
    route: String,
    blames_the_material: bool,
}

impl UnsupportedInput {
    /// Material in a format the system does not read.
    #[must_use]
    pub fn in_format(format: OfferedFormat) -> Self {
        Self {
            format,
            reason: why(format),
            route: route_from(format),
            blames_the_material: true,
        }
    }

    /// Material in a supported format that turned out not to be readable.
    #[must_use]
    pub fn unreadable(format: OfferedFormat, detail: &str) -> Self {
        Self {
            format,
            reason: format!(
                "This looks like {}, but the music in it could not be read: {detail}",
                format.name()
            ),
            route: "Try exporting it from your notation program again, as MusicXML.".to_owned(),
            blames_the_material: true,
        }
    }

    /// A bar is numbered with letters ("12a"), which Segno can't carry yet
    /// (DECISIONS, 3 Nov, E3). The whole message is the owner's sentence, so
    /// there is no separate route.
    #[must_use]
    pub fn lettered_bar(format: OfferedFormat, written: &str) -> Self {
        Self {
            format,
            reason: format!(
                "This score numbers a bar {written}; Segno can't open scores with lettered bar numbers yet."
            ),
            route: String::new(),
            blames_the_material: true,
        }
    }

    /// The engraver could not be used, which is nothing to do with the file.
    ///
    /// Kept distinct because the useful sentence is entirely different.
    /// Telling a performer their score is broken when the engraving library
    /// is missing sends them to fix a file that is fine.
    #[must_use]
    pub fn engine_unavailable(format: OfferedFormat, detail: &str) -> Self {
        Self {
            format,
            reason: format!(
                "The part of Segno that draws music could not be started, \
                 so nothing could be read: {detail}"
            ),
            route: "This is not a problem with your file. Reinstalling Segno \
                    should restore it; MusicXML files will work again once it is."
                .to_owned(),
            blames_the_material: false,
        }
    }

    /// What the material was recognised as.
    #[must_use]
    pub const fn format(&self) -> OfferedFormat {
        self.format
    }

    /// Why it cannot be used. Never empty.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// A way to get to something that will work. Never empty.
    #[must_use]
    pub fn route(&self) -> &str {
        &self.route
    }

    /// Whether the material itself is the obstacle.
    ///
    /// `false` when Segno is the problem. The distinction exists so that a
    /// performer is not sent to fix a file that is fine.
    #[must_use]
    pub const fn blames_the_material(&self) -> bool {
        self.blames_the_material
    }
}

/// Why this format cannot be used, in the performer's terms.
fn why(format: OfferedFormat) -> String {
    match format {
        OfferedFormat::Midi => "A MIDI file records a performance — which keys were \
             pressed and when — rather than written music. It has no staves, no key \
             signature and no note spellings, so there is no score to read from."
            .to_owned(),

        OfferedFormat::Pdf => "A PDF is a picture of a page. Segno needs to know which \
             notes are written where, and a picture does not say."
            .to_owned(),

        OfferedFormat::MuseScore => {
            "This is a MuseScore file, which only MuseScore reads.".to_owned()
        }

        OfferedFormat::Unrecognised => "This file is not in a format Segno recognises as \
             written music."
            .to_owned(),

        // Not reachable through `in_format`, which is only called for
        // unsupported formats — but a supported format arriving here is a
        // defect, and a defect that says something true is better than one
        // that says something confident and wrong.
        supported => format!(
            "{} can be read, so this refusal is a fault in Segno rather than \
             a problem with your file.",
            supported.name()
        ),
    }
}

/// What to do instead.
fn route_from(format: OfferedFormat) -> String {
    match format {
        OfferedFormat::Midi => "If you have the sheet music in a notation program such as \
             MuseScore, Finale or Sibelius, export it as MusicXML and open that instead. \
             MuseScore Studio (free) can also open a MIDI file and export MusicXML, but a \
             recording has no written rhythm, so expect a rough score."
            .to_owned(),

        OfferedFormat::Pdf => "Look for the same piece as MusicXML — many public-domain \
             scores are available in it — or scan the PDF with a program such as Audiveris \
             (free) and export MusicXML from there. Results vary with the quality of the \
             print."
            .to_owned(),

        OfferedFormat::MuseScore => "Open it in MuseScore and choose File, then Export, \
             then MusicXML. Open the exported file here."
            .to_owned(),

        _ => "Segno reads MusicXML, compressed or not. Most notation programs can \
             export it."
            .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use segno_domain::{Location, MeasureNumber};

    use crate::format::OfferedFormat;
    use crate::report::{UnsupportedInput, ValidationIssue, ValidationReport};

    // The design requires all three of format, reason and route. A refusal
    // missing any one of them is a dead end.
    #[test]
    fn every_unsupported_format_has_a_reason_and_a_route() {
        for format in [
            OfferedFormat::Midi,
            OfferedFormat::Pdf,
            OfferedFormat::MuseScore,
            OfferedFormat::Unrecognised,
        ] {
            let refused = UnsupportedInput::in_format(format);

            assert_eq!(refused.format(), format);
            assert!(!refused.reason().is_empty(), "{format:?} has no reason");
            assert!(!refused.route().is_empty(), "{format:?} has no route");
        }
    }

    // The two conversions that exist are named, so the route is something to do.
    #[test]
    fn the_routes_for_midi_and_pdf_name_a_free_tool() {
        assert!(
            UnsupportedInput::in_format(OfferedFormat::Midi)
                .route()
                .contains("MuseScore Studio")
        );
        assert!(
            UnsupportedInput::in_format(OfferedFormat::Pdf)
                .route()
                .contains("Audiveris")
        );
    }

    // Unsupported Input "carries no implication that the
    // performer did anything wrong". Scanning the words is how that stays
    // true as the wording changes.
    #[test]
    fn a_refusal_never_blames_the_performer() {
        let accusing = [
            "you should",
            "your mistake",
            "invalid",
            "illegal",
            "wrong file",
            "error",
            "failed",
            "unsupported file",
        ];

        for format in [
            OfferedFormat::Midi,
            OfferedFormat::Pdf,
            OfferedFormat::MuseScore,
            OfferedFormat::Unrecognised,
        ] {
            let refused = UnsupportedInput::in_format(format);
            let words = format!("{} {}", refused.reason(), refused.route()).to_lowercase();

            for accusation in accusing {
                assert!(
                    !words.contains(accusation),
                    "{format:?} says `{accusation}`: {words}"
                );
            }
        }
    }

    #[test]
    fn a_missing_engraver_does_not_blame_the_file() {
        let refused = UnsupportedInput::engine_unavailable(OfferedFormat::MusicXml, "missing");

        assert!(!refused.blames_the_material());
        assert!(refused.route().contains("not a problem with your file"));
    }

    // the clean case has to be said, not merely implied by
    // silence.
    #[test]
    fn a_report_with_nothing_wrong_says_so() {
        let report = ValidationReport::default();

        assert!(report.is_clean());
        assert_eq!(report.lines().len(), 1);
    }

    #[test]
    fn issues_are_reported_in_the_order_they_occur() {
        let bar = |number: u32, index: usize| ValidationIssue::NotDrawn {
            at: Location::new(
                MeasureNumber::new(number).expect("bars are numbered from one"),
                index,
            ),
        };

        let report = ValidationReport::of(vec![bar(4, 9), bar(1, 0), bar(4, 9)]);

        assert_eq!(report.issues(), [bar(1, 0), bar(4, 9)]);
    }

    // An issue says what a performer will notice, not what a parser noticed.
    #[test]
    fn an_issue_says_where_and_what_will_happen() {
        let described = ValidationIssue::NotDrawn {
            at: Location::new(MeasureNumber::new(7).expect("from one"), 12),
        }
        .describe();

        assert!(described.contains('7'));
        assert!(described.contains("still be expected"));
    }
}
