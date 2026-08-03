//! Finding the engraving engine, and saying so when it is not there.
//!
//! `ADR-0005` loads Verovio at run time rather than linking it, which is what
//! keeps `cargo build` free of a C++ toolchain and what makes LGPL-3.0 §4's
//! replaceable-library obligation true by construction (`ADR-0007`). The cost
//! of that choice is paid here: the library has to be *found*, and it can be
//! absent.
//!
//! `004 §8.2` requires a failure to resolve into an explanation. So a missing
//! engine is a value describing what was looked for and where — never a
//! panic, and never an empty window.

use std::path::PathBuf;

use hogwash_engraving_verovio::VerovioEngraver;
use serde::Serialize;

/// Why the application cannot do its job, in terms a performer can act on.
///
/// Serialised straight to the interface, which renders it as a screen rather
/// than a dialog: this is not a transient error, it is the application being
/// unable to start, and it deserves the room to say what to do.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Unavailable {
    /// One line, for a heading.
    pub summary: String,
    /// What went wrong, in full.
    pub detail: String,
    /// Every place that was looked in, so the performer can put it in one.
    pub looked_in: Vec<String>,
}

/// Where the engine and the performer's data live.
#[derive(Debug)]
pub struct Startup;

impl Startup {
    /// Finds and starts the engraving engine.
    ///
    /// # Where it looks, and why in that order
    ///
    /// 1. **Beside the executable.** This is how a packaged application finds
    ///    the library it shipped with, and it must win, so that an installed
    ///    copy is not silently affected by a developer's environment.
    /// 2. **`HOGWASH_VEROVIO_LIB` / `HOGWASH_VEROVIO_DATA`.** For development,
    ///    and for a performer who has deliberately substituted their own build
    ///    — which `ADR-0007` exists to permit.
    ///
    /// # Errors
    ///
    /// [`Unavailable`], naming every path that was tried. "Could not find the
    /// engraving library" is not actionable; "could not find it in these two
    /// places" is.
    pub fn engine() -> Result<VerovioEngraver, Unavailable> {
        let mut looked_in = Vec::new();

        for (library, resources) in Self::candidates() {
            looked_in.push(library.display().to_string());

            if !library.exists() {
                continue;
            }

            return VerovioEngraver::open(&library, &resources).map_err(|failure| Unavailable {
                summary: "The music engraving engine could not be started.".to_owned(),
                detail: format!("{failure:?}"),
                looked_in,
            });
        }

        Err(Unavailable {
            summary: "The music engraving engine is missing.".to_owned(),
            detail: "Hogwash draws sheet music with Verovio, which ships beside the \
                     application as a separate file. It was not found, so no score can \
                     be displayed."
                .to_owned(),
            looked_in,
        })
    }

    /// The places an engine might be, in the order they are tried.
    fn candidates() -> Vec<(PathBuf, PathBuf)> {
        let mut candidates = Vec::new();

        if let Ok(executable) = std::env::current_exe()
            && let Some(beside) = executable.parent()
        {
            candidates.push((beside.join(Self::LIBRARY), beside.join("verovio-data")));
        }

        if let (Ok(library), Ok(resources)) = (
            std::env::var("HOGWASH_VEROVIO_LIB"),
            std::env::var("HOGWASH_VEROVIO_DATA"),
        ) {
            candidates.push((PathBuf::from(library), PathBuf::from(resources)));
        }

        candidates
    }

    /// What the shared library is called on this platform.
    #[cfg(target_os = "windows")]
    const LIBRARY: &'static str = "verovio.dll";
    #[cfg(target_os = "macos")]
    const LIBRARY: &'static str = "libverovio.dylib";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    const LIBRARY: &'static str = "libverovio.so";

    /// Where the performer's library and practice history are kept.
    ///
    /// Given by the platform rather than chosen: `004 §8.9` keeps history
    /// local, and a program that invented its own location would put a
    /// performer's records somewhere their backups do not look.
    #[must_use]
    pub fn data_directory(resolver: &tauri::path::PathResolver<tauri::Wry>) -> PathBuf {
        resolver
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
    }
}
