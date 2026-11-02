//! The performer's settings, kept beside their history.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::atomic;
use crate::store::{Records, RecordsFailure};

/// The version of the settings shape below. A newer one is refused, and the
/// file is left alone so the newer Hogwash that wrote it keeps it.
const SETTINGS_SCHEMA: u32 = 1;

/// What the performer chose, remembered across restarts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Whether marks stay on the score while playing.
    pub marks: bool,
    /// The keyboard last started on, by name.
    pub keyboard: Option<String>,
    /// The score's zoom, in percent.
    pub zoom: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            marks: true,
            keyboard: None,
            zoom: 100,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct StoredSettings {
    schema: u32,
    #[serde(flatten)]
    settings: Settings,
}

impl Records {
    fn settings_path(&self) -> PathBuf {
        self.root().join("settings.json")
    }

    /// The settings kept, or `None` when none have been yet.
    ///
    /// # Errors
    ///
    /// [`RecordsFailure::Unreadable`] for a file that can't be understood,
    /// [`RecordsFailure::FromTheFuture`] for one a newer Hogwash wrote.
    pub fn settings(&self) -> Result<Option<Settings>, RecordsFailure> {
        let path = self.settings_path();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(failure) => return Err(failure.into()),
        };
        let stored: StoredSettings =
            serde_json::from_str(&text).map_err(|failure| RecordsFailure::Unreadable {
                path: path.clone(),
                detail: failure.to_string(),
            })?;
        if stored.schema > SETTINGS_SCHEMA {
            return Err(RecordsFailure::FromTheFuture {
                path,
                schema: stored.schema,
            });
        }
        Ok(Some(stored.settings))
    }

    /// Keeps `settings`, unless the file there is from a newer Hogwash.
    ///
    /// # Errors
    ///
    /// [`RecordsFailure::FromTheFuture`] (nothing written), or
    /// [`RecordsFailure::Storage`] if it could not be written.
    pub fn keep_settings(&self, settings: &Settings) -> Result<(), RecordsFailure> {
        if let Err(newer @ RecordsFailure::FromTheFuture { .. }) = self.settings() {
            return Err(newer);
        }
        let stored = StoredSettings {
            schema: SETTINGS_SCHEMA,
            settings: settings.clone(),
        };
        let text = serde_json::to_string_pretty(&stored).map_err(|failure| {
            RecordsFailure::Unreadable {
                path: self.settings_path(),
                detail: failure.to_string(),
            }
        })?;
        atomic::write(&self.settings_path(), &text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{Records, RecordsFailure, Settings};

    #[test]
    fn settings_round_trip_and_start_absent() {
        let folder = tempfile::tempdir().expect("a folder");
        let records = Records::under(folder.path());
        assert_eq!(records.settings().ok(), Some(None));

        let chosen = Settings {
            marks: false,
            keyboard: Some("Digital Piano".to_owned()),
            zoom: 130,
        };
        records.keep_settings(&chosen).expect("kept");

        assert_eq!(records.settings().ok(), Some(Some(chosen)));
        let text = fs::read_to_string(folder.path().join("settings.json")).expect("written");
        assert!(text.contains("\"schema\": 1"), "{text}");
    }

    #[test]
    fn a_newer_file_is_refused_and_left_alone() {
        let folder = tempfile::tempdir().expect("a folder");
        let path = folder.path().join("settings.json");
        let newer = r#"{"schema": 9, "marks": false, "zoom": 150, "colour": "blue"}"#;
        fs::write(&path, newer).expect("written");
        let records = Records::under(folder.path());

        assert!(matches!(
            records.settings(),
            Err(RecordsFailure::FromTheFuture { schema: 9, .. })
        ));
        assert!(matches!(
            records.keep_settings(&Settings::default()),
            Err(RecordsFailure::FromTheFuture { .. })
        ));
        assert_eq!(fs::read_to_string(&path).expect("there"), newer);
    }

    #[test]
    fn an_unreadable_file_is_reported_and_can_be_replaced() {
        let folder = tempfile::tempdir().expect("a folder");
        fs::write(folder.path().join("settings.json"), "{not json").expect("written");
        let records = Records::under(folder.path());

        assert!(matches!(
            records.settings(),
            Err(RecordsFailure::Unreadable { .. })
        ));
        records
            .keep_settings(&Settings::default())
            .expect("replaced");
        assert_eq!(records.settings().ok(), Some(Some(Settings::default())));
    }
}
