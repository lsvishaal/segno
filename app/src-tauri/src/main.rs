// Prevents a console window opening beside the application on Windows.
// The `windows_subsystem` attribute only applies to release builds, so a
// developer still gets stdout.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    match segno_app_lib::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            segno_app_lib::log::error(&format!(
                "the application window could not be created: {failure}"
            ));
            ExitCode::FAILURE
        }
    }
}
