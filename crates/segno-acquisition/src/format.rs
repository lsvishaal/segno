//! Recognising what a performer offered.
//!
//! This comes **before** anything tries to read the material as music, and
//! that ordering is the point. The requirement requires unsupported content to be
//! rejected without producing a partial piece, and the design requires the
//! rejection to name the format supplied. An engine's "could not parse this"
//! cannot do that: it says the reading failed, not what the thing was.
//!
//! # Recognition is by content, not by extension
//!
//! A file called `sonata.musicxml` that is really a PDF is a mistake a
//! performer makes constantly — a browser saved it, or an export went wrong —
//! and telling them "your `MusicXML` is invalid" sends them to check the wrong
//! thing. So nothing here looks at a filename. The requirement's second acceptance
//! criterion says as much.

/// What a piece of offered material appears to be.
///
/// Deliberately a short list. This is not a format detector in general; it
/// names the things a performer plausibly offers a piano practice
/// application, so that each can be answered usefully. Anything else is
/// [`Unrecognised`](OfferedFormat::Unrecognised), which is an honest answer
/// rather than a failure to try harder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OfferedFormat {
    /// `MusicXML`, uncompressed. The format the system exists to read.
    MusicXml,

    /// `MusicXML` in its compressed `.mxl` container.
    CompressedMusicXml,

    /// MEI. Not a stated input format, but the engraver reads it and a score
    /// in it is genuinely practisable, so refusing would be pedantry.
    Mei,

    /// A MIDI file. **Permanently excluded as a notation source**
    /// (the design's `Deprecated`) — it records a performance, not
    /// written music.
    Midi,

    /// A PDF. An image of a page, not notation a machine can read.
    Pdf,

    /// A `MuseScore` file. Notation, in a format only `MuseScore` reads.
    MuseScore,

    /// Something else.
    Unrecognised,
}

impl OfferedFormat {
    /// Whether material in this format can become a Piece.
    #[must_use]
    pub const fn is_supported(self) -> bool {
        matches!(self, Self::MusicXml | Self::CompressedMusicXml | Self::Mei)
    }

    /// What to call it when telling the performer.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MusicXml => "MusicXML",
            Self::CompressedMusicXml => "compressed MusicXML",
            Self::Mei => "MEI",
            Self::Midi => "MIDI",
            Self::Pdf => "PDF",
            Self::MuseScore => "MuseScore",
            Self::Unrecognised => "an unrecognised format",
        }
    }
}

/// How much of the material is examined.
///
/// Formats announce themselves at the start, and reading further would mean
/// holding a large file in order to answer a question its first paragraph
/// already settles. A score whose root element is four kilobytes in is
/// pathological, and `Unrecognised` is the right answer for it anyway.
const ENOUGH: usize = 4096;

/// Recognises `material` by what it contains.
#[must_use]
pub fn recognise(material: &[u8]) -> OfferedFormat {
    let opening = material
        .get(..ENOUGH.min(material.len()))
        .unwrap_or(material);

    // Magic numbers first: these are definitive, and cheap.
    if opening.starts_with(b"MThd") {
        return OfferedFormat::Midi;
    }

    if opening.starts_with(b"%PDF") {
        return OfferedFormat::Pdf;
    }

    if opening.starts_with(b"PK") {
        return compressed(material);
    }

    // Then the XML formats, by their root element. `contains` rather than a
    // parse, because this runs on material that may not be XML at all, and a
    // parser's answer to that is an error rather than a name.
    if contains(opening, b"score-partwise") || contains(opening, b"score-timewise") {
        return OfferedFormat::MusicXml;
    }

    if contains(opening, b"music-encoding.org") || contains(opening, b"<mei") {
        return OfferedFormat::Mei;
    }

    OfferedFormat::Unrecognised
}

/// Tells apart the two zip containers a performer might offer.
///
/// Both `.mxl` and `.mscz` are zip archives, so the magic number cannot
/// separate them. Zip stores entry names **uncompressed**, in the local
/// header before each entry and again in the central directory, so the names
/// are readable in the raw bytes without unpacking anything.
///
/// A `MuseScore` archive contains a `.mscx` document; a compressed `MusicXML` one
/// does not. That is the whole distinction, and it is a heuristic rather than
/// a guarantee — but the cost of being wrong is a less specific message, not
/// a wrong piece, because the engraver still has the final say on whether the
/// contents are music.
fn compressed(material: &[u8]) -> OfferedFormat {
    if contains(material, b".mscx") || contains(material, b".mscz") {
        OfferedFormat::MuseScore
    } else {
        OfferedFormat::CompressedMusicXml
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use crate::format::{OfferedFormat, recognise};

    const MUSICXML: &[u8] = include_bytes!("../tests/fixtures/two-bars.musicxml");

    #[test]
    fn a_musicxml_file_is_recognised() {
        assert_eq!(recognise(MUSICXML), OfferedFormat::MusicXml);
    }

    #[test]
    fn a_midi_file_is_recognised() {
        let midi = b"MThd\x00\x00\x00\x06\x00\x01\x00\x01\x01\xe0MTrk";

        assert_eq!(recognise(midi), OfferedFormat::Midi);
    }

    #[test]
    fn a_pdf_is_recognised() {
        assert_eq!(
            recognise(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n"),
            OfferedFormat::Pdf
        );
    }

    #[test]
    fn a_mei_document_is_recognised() {
        let mei = br#"<?xml version="1.0"?><mei xmlns="http://www.music-encoding.org/ns/mei"/>"#;

        assert_eq!(recognise(mei), OfferedFormat::Mei);
    }

    // Both are zip archives, so the magic number cannot tell them apart. The
    // entry names can, and the design needs them told apart: "this is a
    // MuseScore file" is the sentence that helps.
    #[test]
    fn a_zip_containing_a_musescore_document_is_musescore() {
        let mut archive = b"PK\x03\x04".to_vec();
        archive.extend_from_slice(b"META-INF/container.xml");
        archive.extend_from_slice(b"score.mscx");

        assert_eq!(recognise(&archive), OfferedFormat::MuseScore);
    }

    #[test]
    fn any_other_zip_archive_is_taken_for_compressed_musicxml() {
        let mut archive = b"PK\x03\x04".to_vec();
        archive.extend_from_slice(b"META-INF/container.xmlscore.xml");

        assert_eq!(recognise(&archive), OfferedFormat::CompressedMusicXml);
    }

    #[test]
    fn something_that_is_not_a_score_is_unrecognised() {
        assert_eq!(
            recognise(b"dear diary, today I "),
            OfferedFormat::Unrecognised
        );
    }

    #[test]
    fn nothing_at_all_is_unrecognised() {
        assert_eq!(recognise(b""), OfferedFormat::Unrecognised);
    }

    // The requirement's second criterion. The extension is not consulted anywhere, so
    // this is really a test that content wins by construction; it is here
    // because the day someone adds a path parameter, it should fail.
    #[test]
    fn a_pdf_named_like_a_score_is_still_a_pdf() {
        assert_eq!(recognise(b"%PDF-1.4 sonata.musicxml"), OfferedFormat::Pdf);
    }

    // The design requires the performer to be told what they supplied. A format
    // that cannot be named cannot be explained.
    #[test]
    fn every_format_can_be_named() {
        for format in [
            OfferedFormat::MusicXml,
            OfferedFormat::CompressedMusicXml,
            OfferedFormat::Mei,
            OfferedFormat::Midi,
            OfferedFormat::Pdf,
            OfferedFormat::MuseScore,
            OfferedFormat::Unrecognised,
        ] {
            assert!(!format.name().is_empty());
        }
    }

    // The design excludes MIDI as a notation source permanently. If
    // this ever passes, a rejected product decision has come back.
    #[test]
    fn midi_and_pdf_are_not_supported() {
        assert!(!OfferedFormat::Midi.is_supported());
        assert!(!OfferedFormat::Pdf.is_supported());
        assert!(!OfferedFormat::MuseScore.is_supported());
        assert!(!OfferedFormat::Unrecognised.is_supported());
    }

    #[test]
    fn musicxml_compressed_or_not_is_supported() {
        assert!(OfferedFormat::MusicXml.is_supported());
        assert!(OfferedFormat::CompressedMusicXml.is_supported());
    }
}
