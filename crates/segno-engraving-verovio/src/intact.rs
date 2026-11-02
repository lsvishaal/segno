//! Whether a file is whole, checked before Verovio sees it.
//!
//! Verovio throws a C++ exception on a zip container it cannot read ("bad
//! zip"), and an exception cannot cross into Rust: the whole application
//! would abort. A plain `MusicXML` file cut short is worse in a quieter way:
//! Verovio reads what is there, and half a piece looks like a piece. A file
//! that stopped downloading part way is the ordinary way to get either, so
//! both are refused here, with a reason, instead.

/// Why `source` is not a whole file, or `None` if it is.
pub(crate) fn damage(source: &[u8]) -> Option<&'static str> {
    if source.starts_with(b"PK") {
        return zip_damage(source);
    }

    let text = std::str::from_utf8(source).ok()?;
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..roxmltree::ParsingOptions::default()
    };
    roxmltree::Document::parse_with_options(text, options)
        .err()
        .map(|_| "the file is incomplete or damaged: its MusicXML is cut short or malformed")
}

const END_OF_DIRECTORY: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
const DIRECTORY_ENTRY: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];
const LOCAL_HEADER: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];

/// A zip is whole when its end record is there, and its directory and every
/// entry the directory lists lie inside the file.
///
/// ponytail: structure only, not the compressed data itself; a zip damaged
/// inside an entry could still reach Verovio. Decompressing here (a zip
/// dependency) is the upgrade if that is ever seen.
fn zip_damage(source: &[u8]) -> Option<&'static str> {
    const CUT_SHORT: &str = "the file is incomplete or damaged: its zip container is cut short";

    let u16_at = |at: usize| -> Option<usize> {
        Some(usize::from(u16::from_le_bytes(
            source.get(at..at + 2)?.try_into().ok()?,
        )))
    };
    let u32_at = |at: usize| -> Option<usize> {
        usize::try_from(u32::from_le_bytes(source.get(at..at + 4)?.try_into().ok()?)).ok()
    };

    // The end record is the last thing in a zip, up to a 64 KiB comment from
    // the end.
    let Some(end) = source
        .windows(4)
        .rposition(|window| window == END_OF_DIRECTORY)
        .filter(|&end| source.len() - end <= 22 + 0xFFFF)
    else {
        return Some(CUT_SHORT);
    };

    let (Some(entries), Some(size), Some(start), Some(comment)) = (
        u16_at(end + 10),
        u32_at(end + 12),
        u32_at(end + 16),
        u16_at(end + 20),
    ) else {
        return Some(CUT_SHORT);
    };
    // The end record, and the comment it announces, must be all there.
    if end + 22 + comment > source.len() {
        return Some(CUT_SHORT);
    }
    if start.checked_add(size).is_none_or(|finish| finish > end) {
        return Some(CUT_SHORT);
    }

    let mut at = start;
    for _ in 0..entries {
        if source.get(at..at + 4) != Some(&DIRECTORY_ENTRY[..]) {
            return Some(CUT_SHORT);
        }
        let (Some(compressed), Some(name), Some(extra), Some(comment), Some(local)) = (
            u32_at(at + 20),
            u16_at(at + 28),
            u16_at(at + 30),
            u16_at(at + 32),
            u32_at(at + 42),
        ) else {
            return Some(CUT_SHORT);
        };

        if source.get(local..local + 4) != Some(&LOCAL_HEADER[..]) {
            return Some(CUT_SHORT);
        }
        let (Some(local_name), Some(local_extra)) = (u16_at(local + 26), u16_at(local + 28)) else {
            return Some(CUT_SHORT);
        };
        if local + 30 + local_name + local_extra + compressed > start {
            return Some(CUT_SHORT);
        }

        at += 46 + name + extra + comment;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::damage;

    // Real files: two shipped pieces, and the Minuet zipped the way MuseScore
    // exports `.mxl` (`META-INF/container.xml` plus the score).
    const MINUET: &[u8] = include_bytes!("../../../app/src-tauri/library/minuet-in-g.musicxml");
    const PRELUDE: &[u8] = include_bytes!("../../../app/src-tauri/library/prelude-in-c.musicxml");
    const MINUET_MXL: &[u8] = include_bytes!("../tests/fixtures/minuet-in-g.mxl");

    /// The owner's own `.mxl` exports, from the ignored `test-scores/` (or
    /// `HOGWASH_TEST_SCORES`). They aren't redistributable, so they may be
    /// absent; whatever is there is checked.
    fn personal_scores() -> Vec<Vec<u8>> {
        let folder = std::env::var_os("HOGWASH_TEST_SCORES").map_or_else(
            || std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-scores"),
            std::path::PathBuf::from,
        );
        let found: Vec<Vec<u8>> = std::fs::read_dir(&folder)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|kind| kind == "mxl"))
            .filter_map(|path| std::fs::read(path).ok())
            .collect();
        if found.is_empty() {
            eprintln!("skipped: no personal test scores in {}", folder.display());
        }
        found
    }

    #[test]
    fn every_real_file_is_whole() {
        for file in [MINUET, PRELUDE, MINUET_MXL] {
            assert_eq!(damage(file), None);
        }
        for file in personal_scores() {
            assert_eq!(damage(&file), None);
        }
    }

    // A download that stopped anywhere, down to the last byte.
    #[test]
    fn a_zip_cut_short_anywhere_is_damaged() {
        for keep in (1..20)
            .map(|twentieths| MINUET_MXL.len() * twentieths / 20)
            .chain([MINUET_MXL.len() - 1])
        {
            assert!(
                damage(&MINUET_MXL[..keep]).is_some(),
                "{keep} of {} bytes",
                MINUET_MXL.len()
            );
        }
    }

    #[test]
    fn musicxml_cut_short_anywhere_is_damaged() {
        for keep in (1..20).map(|twentieths| MINUET.len() * twentieths / 20) {
            assert!(
                damage(&MINUET[..keep]).is_some(),
                "{keep} of {} bytes",
                MINUET.len()
            );
        }
    }
}
