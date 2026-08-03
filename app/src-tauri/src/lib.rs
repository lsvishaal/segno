//! The application shell (`004 §8.8`).
//!
//! Presentation is split across a boundary now: the *state* is here, in Rust,
//! and the *drawing* is in a webview. `004 §8.8` says Presentation "owns
//! nothing — it observes Score, Cursor and Feedback, and renders them", and
//! that description survives the split intact. What matters is which side of
//! the boundary each responsibility sits on:
//!
//! - **Rust decides everything.** Which notes were expected, which were
//!   played, what the feedback says. Every sentence the performer reads about
//!   their own playing is produced by `hogwash-feedback` and crosses as a
//!   finished string.
//! - **The frontend draws.** It receives locations and words, and turns them
//!   into elements and classes. It judges nothing.
//!
//! That division is not stylistic. `003 §7.7` forbids Feedback from adding
//! anything the Analysis did not establish, and the guards that hold it — no
//! advisory wording, never naming the note that was played — live in the Rust
//! crates. A frontend that composed its own sentences would route around all
//! of them, and nothing would notice.

use tauri::Manager as _;

// Public so integration tests can drive them. The window is the only part of
// this crate that cannot be exercised headlessly, and everything else being
// reachable is what keeps that part small.
pub mod commands;
pub mod engraving;
pub mod provided;
pub mod startup;
pub mod state;

pub use startup::{Startup, Unavailable};

/// Starts the application, and runs until the window closes.
///
/// # Errors
///
/// If the window cannot be created at all. There is nothing to show a
/// performer in that case — there is no window to show it in — so this is
/// returned rather than panicked, and `main` prints it where a terminal can
/// see it. Everything the performer might plausibly have got wrong (a missing
/// engraving library, an unreadable score, no keyboard) is a value carried to
/// the interface instead (`004 §8.2`).
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Startup can fail in ways a performer can act on — a missing
            // engraving library, above all — and `004 §8.2` wants those
            // explained rather than fatal. So the failure becomes state the
            // interface can render as a screen, and the window opens either
            // way. A program that exited here would have nowhere to say why.
            let started = engraving::Engraving::start().and_then(|engraving| {
                let records = hogwash_records::Records::under(&Startup::data_directory(app.path()));

                state::Hogwash::start(engraving, records)
            });

            match started {
                Ok(hogwash) => {
                    app.manage(state::Shared(std::sync::Mutex::new(hogwash)));
                }
                Err(unavailable) => {
                    app.manage(state::Failed(unavailable));
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::library,
            commands::open_piece,
            commands::startup_failure,
        ])
        .run(tauri::generate_context!())
}
