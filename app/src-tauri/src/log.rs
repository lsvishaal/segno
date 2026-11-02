//! `segno.log` in the app data folder: warnings, errors and panics, so a
//! performer can send something when Segno goes wrong. At most [`CAP`]
//! bytes, plus one older file (`segno.log.1`). Every line also goes to
//! stderr.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// How big the log grows before it becomes `segno.log.1`.
const CAP: u64 = 1_000_000;

static FILE: OnceLock<Mutex<PathBuf>> = OnceLock::new();

/// Starts logging to `segno.log` in `folder`, and logs panics there too.
pub fn start(folder: &Path) {
    if FILE.set(Mutex::new(folder.join("segno.log"))).is_err() {
        return;
    }
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let place = panic
            .location()
            .map(|place| format!(" at {}:{}", place.file(), place.line()))
            .unwrap_or_default();
        let message = panic
            .payload()
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned())
            .or_else(|| panic.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        error(&format!("panic{place}: {message}"));
        default(panic);
    }));
}

/// Something went wrong that Segno got past.
pub fn warn(line: &str) {
    write("WARN", line);
}

/// Something went wrong that the performer may have noticed.
pub fn error(line: &str) {
    write("ERROR", line);
}

fn write(level: &str, line: &str) {
    eprintln!("segno: {level} {line}");
    let Some(file) = FILE.get() else { return };
    let Ok(path) = file.lock() else { return };
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    // The log is a last resort: if it can't be written, stderr has it.
    drop(append(
        &path,
        &format!("{} {level} {line}\n", timestamp(seconds)),
    ));
}

/// Appends `text` to `path`, first moving a full log to `<path>.1`.
fn append(path: &Path, text: &str) -> std::io::Result<()> {
    if fs::metadata(path).is_ok_and(|meta| meta.len() + text.len() as u64 > CAP) {
        fs::rename(path, path.with_extension("log.1"))?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(text.as_bytes())
}

/// Seconds since 1970 as "2026-11-02T10:00:00Z" (UTC).
fn timestamp(seconds: u64) -> String {
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    // Days to a civil date: Howard Hinnant's `civil_from_days`.
    let z = days.cast_signed() + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{CAP, append, timestamp};

    #[test]
    fn a_full_log_moves_aside_and_one_older_file_is_kept() {
        let folder = std::env::temp_dir().join(format!("segno-log-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("a folder");
        let path = folder.join("segno.log");
        let line = "x".repeat(100_000);

        for _ in 0..25 {
            append(&path, &line).expect("written");
        }

        let now = fs::metadata(&path).expect("the log").len();
        let older = fs::metadata(path.with_extension("log.1"))
            .expect("the older")
            .len();
        assert!(now <= CAP && older <= CAP, "{now} {older}");
        assert_eq!(now + older, 1_500_000);
        drop(fs::remove_dir_all(&folder));
    }

    #[test]
    fn times_read_as_utc_dates() {
        assert_eq!(timestamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(timestamp(1_793_613_600), "2026-11-02T10:00:00Z");
        assert_eq!(timestamp(951_782_400), "2000-02-29T00:00:00Z");
    }
}
