use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use ts_rs::TS;

use crate::{AppError, Database, ErrorCode, storage};

const SETTINGS_KEY: &str = "app_settings";

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AppIconStyle {
    #[default]
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum Language {
    #[serde(rename = "zh-CN")]
    #[ts(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en")]
    #[ts(rename = "en")]
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: Theme,
    #[serde(default)]
    pub app_icon_style: AppIconStyle,
    pub language: Language,
    pub terminal_font_family: String,
    pub terminal_font_size: f32,
    pub terminal_cursor_style: CursorStyle,
    pub terminal_scrollback_lines: u32,
    pub download_directory_token: Option<String>,
    pub confirm_before_disconnect: bool,
    #[serde(default = "default_true")]
    pub show_size_column: bool,
    #[serde(default = "default_true")]
    pub show_file_sizes: bool,
    #[serde(default)]
    pub show_folder_sizes: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            app_icon_style: AppIconStyle::Light,
            language: Language::ZhCn,
            terminal_font_family: "monospace".to_owned(),
            terminal_font_size: 14.0,
            terminal_cursor_style: CursorStyle::Block,
            terminal_scrollback_lines: 10_000,
            download_directory_token: None,
            confirm_before_disconnect: true,
            show_size_column: true,
            show_file_sizes: true,
            show_folder_sizes: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SettingsRecord {
    pub value: AppSettings,
    pub revision: u32,
    #[ts(type = "number | null")]
    pub updated_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdate {
    pub expected_revision: u32,
    pub value: AppSettings,
}

#[derive(Clone)]
pub struct SettingsService {
    database: Database,
    changes: watch::Sender<SettingsRecord>,
}

impl SettingsService {
    pub async fn load(database: Database) -> Result<Self, AppError> {
        let current = load_settings(&database).await?;
        let (changes, _) = watch::channel(current);
        Ok(Self { database, changes })
    }

    pub fn current(&self) -> SettingsRecord {
        self.changes.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<SettingsRecord> {
        self.changes.subscribe()
    }

    pub async fn update(&self, update: SettingsUpdate) -> Result<SettingsRecord, AppError> {
        validate_settings(&update.value)?;
        let now = storage::now_ms()?;
        let stored = self
            .database
            .execute(move |connection| {
                let json = serde_json::to_string(&update.value)
                    .map_err(|_| storage_error("errors.settingsSerializeFailed"))?;
                let transaction = connection
                    .transaction()
                    .map_err(storage::map_sqlite_error)?;
                let current_revision = transaction
                    .query_row(
                        "SELECT revision FROM settings WHERE key = ?1",
                        [SETTINGS_KEY],
                        |row| row.get::<_, u32>(0),
                    )
                    .optional()
                    .map_err(storage::map_sqlite_error)?
                    .unwrap_or(0);
                if current_revision != update.expected_revision {
                    return Err(revision_conflict(
                        update.expected_revision,
                        current_revision,
                    ));
                }
                let next_revision = current_revision + 1;
                transaction
                    .execute(
                        "INSERT INTO settings (key, value_json, revision, updated_at_ms)
                         VALUES (?1, ?2, ?3, ?4)
                         ON CONFLICT(key) DO UPDATE SET
                           value_json = excluded.value_json,
                           revision = excluded.revision,
                           updated_at_ms = excluded.updated_at_ms",
                        params![SETTINGS_KEY, json, next_revision, now],
                    )
                    .map_err(storage::map_sqlite_error)?;
                transaction.commit().map_err(storage::map_sqlite_error)?;
                Ok(SettingsRecord {
                    value: update.value,
                    revision: next_revision,
                    updated_at_ms: Some(now),
                })
            })
            .await?;
        self.changes.send_replace(stored.clone());
        Ok(stored)
    }
}

async fn load_settings(database: &Database) -> Result<SettingsRecord, AppError> {
    database
        .execute(|connection| {
            let stored = connection
                .query_row(
                    "SELECT value_json, revision, updated_at_ms FROM settings WHERE key = ?1",
                    [SETTINGS_KEY],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, u32>(1)?,
                            row.get::<_, i64>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(storage::map_sqlite_error)?;
            let Some((json, revision, updated_at_ms)) = stored else {
                return Ok(SettingsRecord::default());
            };
            let value = serde_json::from_str(&json)
                .map_err(|_| storage_error("errors.settingsDataInvalid"))?;
            validate_settings(&value)?;
            Ok(SettingsRecord {
                value,
                revision,
                updated_at_ms: Some(updated_at_ms),
            })
        })
        .await
}

fn validate_settings(settings: &AppSettings) -> Result<(), AppError> {
    let font_length = settings.terminal_font_family.trim().chars().count();
    if !(1..=128).contains(&font_length)
        || settings.terminal_font_family.chars().any(char::is_control)
    {
        return Err(validation(
            "terminalFontFamily",
            "errors.terminalFontFamilyInvalid",
        ));
    }
    if !settings.terminal_font_size.is_finite()
        || !(8.0..=72.0).contains(&settings.terminal_font_size)
    {
        return Err(validation(
            "terminalFontSize",
            "errors.terminalFontSizeOutOfRange",
        ));
    }
    if !(1_000..=100_000).contains(&settings.terminal_scrollback_lines) {
        return Err(validation(
            "terminalScrollbackLines",
            "errors.terminalScrollbackOutOfRange",
        ));
    }
    if settings
        .download_directory_token
        .as_ref()
        .is_some_and(|token| {
            token.is_empty() || token.len() > 512 || token.chars().any(char::is_control)
        })
    {
        return Err(validation(
            "downloadDirectoryToken",
            "errors.downloadDirectoryTokenInvalid",
        ));
    }
    Ok(())
}

fn revision_conflict(expected: u32, actual: u32) -> AppError {
    let mut error = AppError::new(ErrorCode::RevisionConflict, "errors.revisionConflict");
    error
        .params
        .insert("expectedRevision".to_owned(), expected.to_string());
    error
        .params
        .insert("actualRevision".to_owned(), actual.to_string());
    error
}

fn validation(field: &'static str, message_key: &'static str) -> AppError {
    let mut error = AppError::new(ErrorCode::ValidationFailed, message_key);
    error.params.insert("field".to_owned(), field.to_owned());
    error
}

fn storage_error(message_key: &'static str) -> AppError {
    let mut error = AppError::new(ErrorCode::Internal, message_key);
    error.stage = Some("storage".to_owned());
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn persists_settings_and_publishes_only_committed_changes() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("test.sqlite3");
        let database = Database::open(&path).expect("database");
        let service = SettingsService::load(database)
            .await
            .expect("settings service");
        let mut changes = service.subscribe();
        assert_eq!(service.current(), SettingsRecord::default());

        let value = AppSettings {
            theme: Theme::Dark,
            app_icon_style: AppIconStyle::Dark,
            ..AppSettings::default()
        };
        let stored = service
            .update(SettingsUpdate {
                expected_revision: 0,
                value: value.clone(),
            })
            .await
            .expect("update settings");
        changes.changed().await.expect("settings notification");
        assert_eq!(changes.borrow().clone(), stored);
        assert_eq!(stored.revision, 1);

        let conflict = service
            .update(SettingsUpdate {
                expected_revision: 0,
                value: AppSettings::default(),
            })
            .await
            .expect_err("stale revision must fail");
        assert_eq!(conflict.code, ErrorCode::RevisionConflict);
        assert_eq!(service.current(), stored);

        drop(service);
        let database = Database::open(&path).expect("reopen database");
        let reloaded = SettingsService::load(database)
            .await
            .expect("reload settings");
        assert_eq!(reloaded.current().value, value);
    }

    #[test]
    fn old_settings_default_to_light_icon_and_invalid_styles_fail() {
        let mut value = serde_json::to_value(AppSettings::default()).expect("settings JSON");
        value
            .as_object_mut()
            .expect("object")
            .remove("appIconStyle");
        let old: AppSettings = serde_json::from_value(value.clone()).expect("old settings");
        assert_eq!(old.app_icon_style, AppIconStyle::Light);
        value["appIconStyle"] = serde_json::json!("system");
        assert!(serde_json::from_value::<AppSettings>(value).is_err());
    }

    #[test]
    fn rejects_non_finite_font_size() {
        let settings = AppSettings {
            terminal_font_size: f32::NAN,
            ..AppSettings::default()
        };
        assert_eq!(
            validate_settings(&settings)
                .expect_err("NaN font size must fail")
                .code,
            ErrorCode::ValidationFailed
        );
    }
}
