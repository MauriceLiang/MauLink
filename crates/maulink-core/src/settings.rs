use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use ts_rs::TS;

use crate::{AppError, Database, ErrorCode, storage};

const SETTINGS_KEY: &str = "app_settings";

fn default_true() -> bool {
    true
}

fn default_terminal_line_height() -> f32 {
    1.35
}

fn default_image_opacity() -> u8 {
    100
}

fn default_overlay_opacity() -> u8 {
    45
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AccentColor {
    #[default]
    Blue,
    Indigo,
    Purple,
    Green,
    Orange,
    Red,
    Custom,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TerminalThemeMode {
    #[default]
    FollowApp,
    Light,
    Dark,
    CustomColor,
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalCustomColors {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    pub selection: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TerminalBackgroundFit {
    #[default]
    Cover,
    Contain,
    Stretch,
    Original,
    Tile,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TerminalBackgroundPosition {
    #[default]
    Center,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TerminalBackgroundOverlayKind {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalBackgroundImageSettings {
    #[serde(default)]
    pub image_id: Option<String>,
    #[serde(default)]
    pub fit: TerminalBackgroundFit,
    #[serde(default)]
    pub position: TerminalBackgroundPosition,
    #[serde(default = "default_image_opacity")]
    pub image_opacity: u8,
    #[serde(default)]
    pub overlay_kind: TerminalBackgroundOverlayKind,
    #[serde(default = "default_overlay_opacity")]
    pub overlay_opacity: u8,
    #[serde(default)]
    pub blur_px: u8,
}

impl Default for TerminalBackgroundImageSettings {
    fn default() -> Self {
        Self {
            image_id: None,
            fit: TerminalBackgroundFit::Cover,
            position: TerminalBackgroundPosition::Center,
            image_opacity: default_image_opacity(),
            overlay_kind: TerminalBackgroundOverlayKind::Dark,
            overlay_opacity: default_overlay_opacity(),
            blur_px: 0,
        }
    }
}

impl Default for TerminalCustomColors {
    fn default() -> Self {
        Self {
            background: "#111318".to_owned(),
            foreground: "#EAECF0".to_owned(),
            cursor: "#3B82F6".to_owned(),
            selection: "#3B82F6".to_owned(),
        }
    }
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
    #[serde(default)]
    pub accent_color: AccentColor,
    #[serde(default)]
    pub custom_accent_color: Option<String>,
    pub language: Language,
    pub terminal_font_family: String,
    pub terminal_font_size: f32,
    pub terminal_cursor_style: CursorStyle,
    pub terminal_scrollback_lines: u32,
    #[serde(default)]
    pub terminal_theme_mode: TerminalThemeMode,
    #[serde(default)]
    pub terminal_custom_colors: TerminalCustomColors,
    #[serde(default)]
    pub terminal_background_image: TerminalBackgroundImageSettings,
    #[serde(default = "default_terminal_line_height")]
    pub terminal_line_height: f32,
    #[serde(default = "default_true")]
    pub terminal_cursor_blink: bool,
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
            accent_color: AccentColor::Blue,
            custom_accent_color: None,
            language: Language::ZhCn,
            terminal_font_family: "monospace".to_owned(),
            terminal_font_size: 14.0,
            terminal_cursor_style: CursorStyle::Block,
            terminal_scrollback_lines: 10_000,
            terminal_theme_mode: TerminalThemeMode::FollowApp,
            terminal_custom_colors: TerminalCustomColors::default(),
            terminal_background_image: TerminalBackgroundImageSettings::default(),
            terminal_line_height: default_terminal_line_height(),
            terminal_cursor_blink: true,
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
            let value: AppSettings = serde_json::from_str(&json)
                .map_err(|_| storage_error("errors.settingsDataInvalid"))?;
            let value = migrate_settings(value);
            validate_settings(&value)?;
            Ok(SettingsRecord {
                value,
                revision,
                updated_at_ms: Some(updated_at_ms),
            })
        })
        .await
}

fn migrate_settings(mut settings: AppSettings) -> AppSettings {
    // Phase 6 exposed Image mode before background assets existed. Reset that
    // incomplete preference instead of making the saved settings unloadable.
    if settings.terminal_theme_mode == TerminalThemeMode::Image
        && settings.terminal_background_image.image_id.is_none()
    {
        settings.terminal_theme_mode = TerminalThemeMode::FollowApp;
    }
    settings
}

fn validate_settings(settings: &AppSettings) -> Result<(), AppError> {
    if settings
        .custom_accent_color
        .as_deref()
        .is_some_and(|color| !is_valid_accent_hex(color))
        || (settings.accent_color == AccentColor::Custom && settings.custom_accent_color.is_none())
    {
        return Err(validation(
            "customAccentColor",
            "errors.customAccentColorInvalid",
        ));
    }
    if [
        &settings.terminal_custom_colors.background,
        &settings.terminal_custom_colors.foreground,
        &settings.terminal_custom_colors.cursor,
        &settings.terminal_custom_colors.selection,
    ]
    .into_iter()
    .any(|color| !is_valid_accent_hex(color))
    {
        return Err(validation(
            "terminalCustomColors",
            "errors.terminalCustomColorInvalid",
        ));
    }
    if settings
        .terminal_background_image
        .image_id
        .as_ref()
        .is_some_and(|id| {
            uuid::Uuid::parse_str(id).map_or(true, |parsed| parsed.to_string() != *id)
        })
    {
        return Err(validation(
            "terminalBackgroundImage",
            "errors.terminalBackgroundImageIdInvalid",
        ));
    }
    if settings.terminal_background_image.image_opacity > 100
        || settings.terminal_background_image.image_opacity < 10
        || settings.terminal_background_image.overlay_opacity > 90
        || settings.terminal_background_image.blur_px > 16
    {
        return Err(validation(
            "terminalBackgroundImage",
            "errors.terminalBackgroundImageSettingsOutOfRange",
        ));
    }
    if settings.terminal_theme_mode == TerminalThemeMode::Image
        && settings.terminal_background_image.image_id.is_none()
    {
        return Err(validation(
            "terminalBackgroundImage",
            "errors.terminalBackgroundImageRequired",
        ));
    }
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
    if !settings.terminal_line_height.is_finite()
        || !(1.0..=2.0).contains(&settings.terminal_line_height)
    {
        return Err(validation(
            "terminalLineHeight",
            "errors.terminalLineHeightOutOfRange",
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

fn is_valid_accent_hex(value: &str) -> bool {
    value
        .strip_prefix('#')
        .is_some_and(|hex| hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
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
            accent_color: AccentColor::Custom,
            custom_accent_color: Some("#12AbEf".to_owned()),
            terminal_theme_mode: TerminalThemeMode::CustomColor,
            terminal_custom_colors: TerminalCustomColors {
                background: "#102030".to_owned(),
                foreground: "#E0E0E0".to_owned(),
                cursor: "#33AAFF".to_owned(),
                selection: "#7755CC".to_owned(),
            },
            terminal_background_image: TerminalBackgroundImageSettings {
                image_id: Some(uuid::Uuid::new_v4().to_string()),
                fit: TerminalBackgroundFit::Contain,
                position: TerminalBackgroundPosition::TopRight,
                image_opacity: 82,
                overlay_kind: TerminalBackgroundOverlayKind::Light,
                overlay_opacity: 28,
                blur_px: 3,
            },
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
    fn terminal_background_settings_validate_ranges_and_require_an_image_in_image_mode() {
        let settings = AppSettings {
            terminal_theme_mode: TerminalThemeMode::Image,
            ..AppSettings::default()
        };
        assert_eq!(
            validate_settings(&settings)
                .expect_err("image mode requires an asset id")
                .message_key,
            "errors.terminalBackgroundImageRequired"
        );

        let valid = AppSettings {
            terminal_theme_mode: TerminalThemeMode::Image,
            terminal_background_image: TerminalBackgroundImageSettings {
                image_id: Some(uuid::Uuid::new_v4().to_string()),
                ..TerminalBackgroundImageSettings::default()
            },
            ..AppSettings::default()
        };
        assert!(validate_settings(&valid).is_ok());

        for (image_opacity, overlay_opacity, blur_px) in
            [(9, 45, 0), (101, 45, 0), (100, 91, 0), (100, 45, 17)]
        {
            let settings = AppSettings {
                terminal_background_image: TerminalBackgroundImageSettings {
                    image_id: Some(uuid::Uuid::new_v4().to_string()),
                    image_opacity,
                    overlay_opacity,
                    blur_px,
                    ..TerminalBackgroundImageSettings::default()
                },
                ..AppSettings::default()
            };
            assert_eq!(
                validate_settings(&settings)
                    .expect_err("image settings must stay within UI limits")
                    .message_key,
                "errors.terminalBackgroundImageSettingsOutOfRange"
            );
        }
    }

    #[test]
    fn old_settings_default_to_light_icon_and_invalid_styles_fail() {
        let mut value = serde_json::to_value(AppSettings::default()).expect("settings JSON");
        let object = value.as_object_mut().expect("object");
        object.remove("appIconStyle");
        object.remove("accentColor");
        object.remove("customAccentColor");
        object.remove("terminalThemeMode");
        object.remove("terminalCustomColors");
        object.remove("terminalBackgroundImage");
        object.remove("terminalLineHeight");
        object.remove("terminalCursorBlink");
        let old: AppSettings = serde_json::from_value(value.clone()).expect("old settings");
        assert_eq!(old.app_icon_style, AppIconStyle::Light);
        assert_eq!(old.accent_color, AccentColor::Blue);
        assert_eq!(old.custom_accent_color, None);
        assert_eq!(old.terminal_theme_mode, TerminalThemeMode::FollowApp);
        assert_eq!(old.terminal_custom_colors, TerminalCustomColors::default());
        assert_eq!(
            old.terminal_background_image,
            TerminalBackgroundImageSettings::default()
        );
        assert_eq!(old.terminal_line_height, 1.35);
        assert!(old.terminal_cursor_blink);
        value["appIconStyle"] = serde_json::json!("system");
        assert!(serde_json::from_value::<AppSettings>(value).is_err());
    }

    #[tokio::test]
    async fn migrates_pre_asset_image_mode_when_loading_saved_settings() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("settings.sqlite3")).expect("database");
        let mut legacy = serde_json::to_value(AppSettings {
            terminal_theme_mode: TerminalThemeMode::Image,
            ..AppSettings::default()
        })
        .expect("legacy settings JSON");
        legacy
            .as_object_mut()
            .expect("settings object")
            .remove("terminalBackgroundImage");
        let legacy_json = serde_json::to_string(&legacy).expect("legacy JSON string");
        database
            .execute(move |connection| {
                connection
                    .execute(
                        "INSERT INTO settings (key, value_json, revision, updated_at_ms) VALUES (?1, ?2, ?3, ?4)",
                        params![SETTINGS_KEY, legacy_json, 1u32, 1i64],
                    )
                    .map_err(storage::map_sqlite_error)?;
                Ok(())
            })
            .await
            .expect("write old setting");

        let settings = SettingsService::load(database)
            .await
            .expect("load migrated settings");
        assert_eq!(
            settings.current().value.terminal_theme_mode,
            TerminalThemeMode::FollowApp
        );
        assert_eq!(
            settings.current().value.terminal_background_image,
            TerminalBackgroundImageSettings::default()
        );
    }

    #[test]
    fn custom_accent_accepts_only_six_digit_hex_colors() {
        let valid = AppSettings {
            accent_color: AccentColor::Custom,
            custom_accent_color: Some("#aBcD09".to_owned()),
            ..AppSettings::default()
        };
        assert!(validate_settings(&valid).is_ok());

        for color in [
            "#123",
            "123456",
            "#12345678",
            "#12GG56",
            "var(--color-primary)",
        ] {
            let invalid = AppSettings {
                accent_color: AccentColor::Custom,
                custom_accent_color: Some(color.to_owned()),
                ..AppSettings::default()
            };
            assert_eq!(
                validate_settings(&invalid)
                    .expect_err("unsupported CSS color must be rejected")
                    .message_key,
                "errors.customAccentColorInvalid"
            );
        }

        let missing = AppSettings {
            accent_color: AccentColor::Custom,
            ..AppSettings::default()
        };
        assert_eq!(
            validate_settings(&missing)
                .expect_err("custom accent requires a color")
                .code,
            ErrorCode::ValidationFailed
        );
    }

    #[test]
    fn terminal_custom_colors_and_line_height_are_validated() {
        for color in ["#123", "rgb(1,2,3)", "#12345678"] {
            let settings = AppSettings {
                terminal_custom_colors: TerminalCustomColors {
                    selection: color.to_owned(),
                    ..TerminalCustomColors::default()
                },
                ..AppSettings::default()
            };
            assert_eq!(
                validate_settings(&settings)
                    .expect_err("custom terminal colors must be hex")
                    .message_key,
                "errors.terminalCustomColorInvalid"
            );
        }

        for line_height in [f32::NAN, 0.99, 2.01] {
            let settings = AppSettings {
                terminal_line_height: line_height,
                ..AppSettings::default()
            };
            assert_eq!(
                validate_settings(&settings)
                    .expect_err("line height outside the range must fail")
                    .message_key,
                "errors.terminalLineHeightOutOfRange"
            );
        }
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
