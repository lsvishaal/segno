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

mod startup;

pub use startup::{Engine, Startup};

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
        .run(tauri::generate_context!())
}
