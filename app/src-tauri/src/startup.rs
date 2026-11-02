//! Finding the engraving engine, and saying so when it is not there.
//!
//! The design loads Verovio at run time rather than linking it, which is what
//! keeps `cargo build` free of a C++ toolchain and what makes section 4 of LGPL-3.0's
//! replaceable-library obligation true by construction. The cost
//! of that choice is paid here: the library has to be *found*, and it can be
//! absent.
//!
//! The design requires a failure to resolve into an explanation. So a missing
//! engine is a value describing what was looked for and where — never a
//! panic, and never an empty window.

use std::path::{Path, PathBuf};

use segno_engraving_verovio::VerovioEngraver;
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
    /// 1. **The package's resource folder** (`resources`, from Tauri). A
    ///    packaged application keeps its files there, not beside the binary
    ///    (an `AppImage` has `usr/bin` and `usr/lib/<name>`), and it must win so
    ///    that an installed copy is not silently affected by a developer's
    ///    environment.
    /// 2. **Beside the executable.** A plain build with the files copied next
    ///    to it.
    /// 3. **`SEGNO_VEROVIO_LIB` / `SEGNO_VEROVIO_DATA`.** For development,
    ///    and for a performer who has deliberately substituted their own build
    ///    — which the design exists to permit.
    ///
    /// # Errors
    ///
    /// [`Unavailable`], naming every path that was tried. "Could not find the
    /// engraving library" is not actionable; "could not find it in these two
    /// places" is.
    pub fn engine(resources: Option<&Path>) -> Result<VerovioEngraver, Unavailable> {
        let mut looked_in = Vec::new();

        for (library, data) in Self::candidates(resources) {
            looked_in.push(library.display().to_string());

            if !library.exists() {
                continue;
            }

            return VerovioEngraver::open(&library, &data).map_err(|failure| Unavailable {
                summary: "The music engraving engine could not be started.".to_owned(),
                detail: format!("{failure:?}"),
                looked_in,
            });
        }

        Err(Unavailable {
            summary: "The music engraving engine is missing.".to_owned(),
            detail: "Segno draws sheet music with Verovio, which ships beside the \
                     application as a separate file. It was not found, so no score can \
                     be displayed."
                .to_owned(),
            looked_in,
        })
    }

    /// The places an engine might be, in the order they are tried.
    fn candidates(resources: Option<&Path>) -> Vec<(PathBuf, PathBuf)> {
        let executable = std::env::current_exe().ok();
        let variables = match (
            std::env::var("SEGNO_VEROVIO_LIB"),
            std::env::var("SEGNO_VEROVIO_DATA"),
        ) {
            (Ok(library), Ok(data)) => Some((PathBuf::from(library), PathBuf::from(data))),
            _ => None,
        };

        Self::candidates_from(resources, executable.as_deref(), variables)
    }

    /// [`Self::candidates`] with everything it reads handed in, so the order
    /// can be tested without touching the environment.
    fn candidates_from(
        resources: Option<&Path>,
        executable: Option<&Path>,
        variables: Option<(PathBuf, PathBuf)>,
    ) -> Vec<(PathBuf, PathBuf)> {
        let mut candidates = Vec::new();

        for folder in resources
            .into_iter()
            .chain(executable.and_then(Path::parent))
        {
            candidates.push((folder.join(Self::LIBRARY), folder.join("verovio-data")));
        }
        candidates.extend(variables);

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
    /// Given by the platform rather than chosen: the design keeps history
    /// local, and a program that invented its own location would put a
    /// performer's records somewhere their backups do not look.
    ///
    /// # Errors
    ///
    /// When the platform can't say: Segno then says so, rather than
    /// keeping records in whatever folder it was started from.
    pub fn data_directory(
        resolver: &tauri::path::PathResolver<tauri::Wry>,
    ) -> Result<PathBuf, Unavailable> {
        Self::data_directory_from(
            resolver
                .app_data_dir()
                .map_err(|failure| failure.to_string()),
        )
    }

    /// [`Self::data_directory`] with the platform's answer handed in.
    fn data_directory_from(found: Result<PathBuf, String>) -> Result<PathBuf, Unavailable> {
        found.map_err(|reason| Unavailable {
            summary: "Segno couldn't find a folder to keep your practice in.".to_owned(),
            detail: format!(
                "Your system didn't say where an application's data belongs ({reason}), \
                 so nothing was saved anywhere else instead."
            ),
            looked_in: vec!["the system's application data folder".to_owned()],
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::Startup;

    #[test]
    fn with_no_data_folder_it_says_so_and_never_uses_the_current_one() {
        let failed =
            Startup::data_directory_from(Err("HOME is not set".to_owned())).expect_err("no folder");

        assert_eq!(
            failed.summary,
            "Segno couldn't find a folder to keep your practice in."
        );
        assert!(
            failed.detail.contains("HOME is not set"),
            "{}",
            failed.detail
        );
        assert_eq!(
            Startup::data_directory_from(Ok(PathBuf::from("/data"))).ok(),
            Some(PathBuf::from("/data"))
        );
    }

    #[test]
    fn a_packaged_engine_is_found_before_the_environment_and_beside_the_binary() {
        let found = Startup::candidates_from(
            Some(Path::new("/app/usr/lib/Segno")),
            Some(Path::new("/app/usr/bin/segno-app")),
            Some((
                PathBuf::from("/dev/libverovio.so"),
                PathBuf::from("/dev/data"),
            )),
        );

        let libraries: Vec<_> = found.iter().map(|(library, _)| library.clone()).collect();
        assert_eq!(
            libraries,
            [
                Path::new("/app/usr/lib/Segno").join(Startup::LIBRARY),
                Path::new("/app/usr/bin").join(Startup::LIBRARY),
                PathBuf::from("/dev/libverovio.so"),
            ]
        );
        assert_eq!(found[0].1, Path::new("/app/usr/lib/Segno/verovio-data"));
        assert_eq!(found[1].1, Path::new("/app/usr/bin/verovio-data"));
    }

    #[test]
    fn a_development_run_has_only_the_variables() {
        let found = Startup::candidates_from(
            None,
            None,
            Some((
                PathBuf::from("/dev/libverovio.so"),
                PathBuf::from("/dev/data"),
            )),
        );
        assert_eq!(found.len(), 1);
    }
}
