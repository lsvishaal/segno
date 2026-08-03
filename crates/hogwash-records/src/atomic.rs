//! Writing a file so that being interrupted cannot damage an older one.
//!
//! `004 §8.9` sets the boundary: *"an interruption mid-write must not corrupt
//! previously recorded Sessions or Attempts; at minimum, prior records remain
//! intact even if the most recent one does not."* NFR-100 and FR-704 say the
//! same from the requirements side.
//!
//! The obvious way to write a file is the one that cannot satisfy that.
//! Opening a record and writing over it leaves a window — from the moment the
//! file is truncated to the moment the last byte lands — in which the machine
//! losing power leaves a file that is neither the old record nor the new one.
//! It is a small window and it is exactly the one that matters, because the
//! thing being overwritten is the history the performer cannot reconstruct.
//!
//! So: write a new file beside it, flush it to the disk, and then **rename**.
//! Rename within a directory is atomic on every platform this targets — a
//! reader sees either the old file or the new one, never a partial one — and
//! the flush before it is what stops the rename landing ahead of the data it
//! is meant to publish.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Writes `contents` to `path`, atomically.
///
/// # Errors
///
/// Whatever the filesystem said. A failure here leaves `path` exactly as it
/// was — which is the property this function exists for.
pub(crate) fn write(path: &Path, contents: &str) -> std::io::Result<()> {
    let Some(directory) = path.parent() else {
        return Err(std::io::Error::other(format!(
            "{} is not somewhere a record can be written",
            path.display()
        )));
    };

    fs::create_dir_all(directory)?;

    // Beside the target, not in a temporary directory: rename is only
    // guaranteed atomic within one filesystem, and `/tmp` is frequently a
    // different one. A cross-device rename fails — or, worse, is emulated by
    // a copy, which reopens the window this whole function closes.
    let beside = path.with_extension("writing");

    {
        let mut file = File::create(&beside)?;
        file.write_all(contents.as_bytes())?;

        // Before the rename, and this ordering is the whole thing. Without
        // it the rename can reach the disk before the bytes do, and a power
        // loss in between leaves a record that exists, is named correctly,
        // and is empty.
        file.sync_all()?;
    }

    fs::rename(&beside, path)?;

    // The rename itself is a directory change, and a directory change is
    // buffered like any other. Without this a crash can lose the rename
    // while keeping both files — which is recoverable, but leaves the old
    // record in place while the caller has been told the new one was saved.
    //
    // Failing to open a directory for syncing is not fatal: some platforms
    // do not permit it, and the rename has already happened. Losing the
    // durability guarantee is worth reporting nowhere, because there is
    // nothing the caller could usefully do differently.
    if let Ok(handle) = File::open(directory) {
        drop(handle.sync_all());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::atomic::write;

    #[test]
    fn what_was_written_can_be_read_back() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("record.json");

        write(&path, "{\"schema\":1}").expect("writes");

        assert_eq!(
            std::fs::read_to_string(&path).expect("reads"),
            "{\"schema\":1}"
        );
    }

    #[test]
    fn a_record_can_be_replaced() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("record.json");

        write(&path, "first").expect("writes");
        write(&path, "second").expect("replaces");

        assert_eq!(std::fs::read_to_string(&path).expect("reads"), "second");
    }

    #[test]
    fn a_missing_directory_is_created() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("sessions").join("record.json");

        write(&path, "kept").expect("writes");

        assert!(path.exists());
    }

    // 004 §8.9 — prior records remain intact even if the most recent one does
    // not. The interruption cannot be staged directly here, so what is
    // asserted is the property that makes it true: nothing is ever written
    // through the record's own name, so there is no moment at which that name
    // refers to a partial file.
    #[test]
    fn nothing_is_left_beside_a_finished_record() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("record.json");

        write(&path, "kept").expect("writes");

        let left: Vec<String> = std::fs::read_dir(directory.path())
            .expect("reads the directory")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();

        assert_eq!(left, ["record.json"]);
    }

    // A failed write must leave the previous record readable. Writing to a
    // path whose parent cannot be created fails, and the earlier record --
    // written elsewhere -- is untouched.
    #[test]
    fn a_record_that_could_not_be_written_leaves_the_earlier_one_alone() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let kept = directory.path().join("record.json");

        write(&kept, "first").expect("writes");

        // A file cannot be a directory, so this cannot succeed.
        let impossible = kept.join("deeper").join("record.json");
        assert!(write(&impossible, "second").is_err());

        assert_eq!(std::fs::read_to_string(&kept).expect("reads"), "first");
    }
}
