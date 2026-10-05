use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    AppError, Database, ErrorCode,
    settings::{
        TerminalBackgroundImageSettings, TerminalCustomColors, TerminalThemeMode,
        is_valid_accent_hex,
    },
    storage,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ServerEnvironment {
    Production,
    Staging,
    Development,
    Custom,
}

impl ServerEnvironment {
    fn as_database_value(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::Staging => "staging",
            Self::Development => "development",
            Self::Custom => "custom",
        }
    }

    fn from_database_value(value: &str) -> Result<Self, AppError> {
        match value {
            "production" => Ok(Self::Production),
            "staging" => Ok(Self::Staging),
            "development" => Ok(Self::Development),
            "custom" => Ok(Self::Custom),
            _ => Err(storage_corrupt()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalAppearanceSettings {
    pub theme_mode: TerminalThemeMode,
    pub custom_colors: TerminalCustomColors,
    pub background_image: TerminalBackgroundImageSettings,
}

impl Default for TerminalAppearanceSettings {
    fn default() -> Self {
        Self {
            theme_mode: TerminalThemeMode::FollowApp,
            custom_colors: TerminalCustomColors::default(),
            background_image: TerminalBackgroundImageSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerAppearance {
    pub server_id: String,
    pub label_color: Option<String>,
    pub environment: Option<ServerEnvironment>,
    pub terminal_override_enabled: bool,
    pub terminal_appearance: TerminalAppearanceSettings,
    pub revision: u32,
    #[ts(type = "number")]
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerAppearanceUpdate {
    pub server_id: String,
    pub expected_revision: u32,
    pub label_color: Option<String>,
    pub environment: Option<ServerEnvironment>,
    pub terminal_override_enabled: bool,
    pub terminal_appearance: TerminalAppearanceSettings,
}

#[derive(Clone)]
pub struct ServerAppearanceStore {
    database: Database,
}

impl ServerAppearanceStore {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn list(&self) -> Result<Vec<ServerAppearance>, AppError> {
        self.database
            .execute(|connection| {
                let mut statement = connection
                    .prepare(
                        "SELECT server_id, label_color, environment, terminal_override_enabled,
                        terminal_appearance_json, revision, updated_at_ms
                 FROM server_appearance ORDER BY server_id",
                    )
                    .map_err(storage::map_sqlite_error)?;
                let rows = statement
                    .query_map([], map_appearance)
                    .map_err(storage::map_sqlite_error)?;
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(storage::map_sqlite_error)
            })
            .await
    }

    pub async fn get(&self, server_id: String) -> Result<ServerAppearance, AppError> {
        validate_server_id(&server_id)?;
        self.database
            .execute(move |connection| {
                if let Some(appearance) = connection
                    .query_row(
                        "SELECT server_id, label_color, environment, terminal_override_enabled,
                        terminal_appearance_json, revision, updated_at_ms
                 FROM server_appearance WHERE server_id = ?1",
                        [&server_id],
                        map_appearance,
                    )
                    .optional()
                    .map_err(storage::map_sqlite_error)?
                {
                    return Ok(appearance);
                }
                let exists = connection
                    .query_row("SELECT 1 FROM servers WHERE id = ?1", [&server_id], |_| {
                        Ok(())
                    })
                    .optional()
                    .map_err(storage::map_sqlite_error)?
                    .is_some();
                if !exists {
                    return Err(not_found());
                }
                Ok(ServerAppearance {
                    server_id,
                    label_color: None,
                    environment: None,
                    terminal_override_enabled: false,
                    terminal_appearance: TerminalAppearanceSettings::default(),
                    revision: 0,
                    updated_at_ms: 0,
                })
            })
            .await
    }

    pub async fn update(
        &self,
        update: ServerAppearanceUpdate,
    ) -> Result<ServerAppearance, AppError> {
        validate_server_id(&update.server_id)?;
        let label_color = update.label_color.as_deref().map(str::to_owned);
        if label_color
            .as_deref()
            .is_some_and(|value| !is_valid_accent_hex(value))
        {
            return Err(validation("labelColor", "errors.serverLabelColorInvalid"));
        }
        crate::settings::validate_terminal_appearance(
            update.terminal_appearance.theme_mode,
            &update.terminal_appearance.custom_colors,
            &update.terminal_appearance.background_image,
        )?;
        let appearance_json =
            serde_json::to_string(&update.terminal_appearance).map_err(|_| storage_error())?;
        let now = storage::now_ms()?;
        self.database
            .execute(move |connection| {
                let transaction = connection
                    .transaction()
                    .map_err(storage::map_sqlite_error)?;
                let exists = transaction
                    .query_row(
                        "SELECT 1 FROM servers WHERE id = ?1",
                        [&update.server_id],
                        |_| Ok(()),
                    )
                    .optional()
                    .map_err(storage::map_sqlite_error)?
                    .is_some();
                if !exists {
                    return Err(not_found());
                }
                let environment = update.environment.map(ServerEnvironment::as_database_value);
                let changed = if update.expected_revision == 0 {
                    transaction
                        .execute(
                            "INSERT INTO server_appearance
                     (server_id, label_color, environment, terminal_override_enabled,
                      terminal_appearance_json, revision, updated_at_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)
                     ON CONFLICT(server_id) DO NOTHING",
                            params![
                                update.server_id,
                                label_color,
                                environment,
                                update.terminal_override_enabled,
                                appearance_json,
                                now
                            ],
                        )
                        .map_err(storage::map_sqlite_error)?
                } else {
                    transaction
                        .execute(
                            "UPDATE server_appearance SET label_color = ?1, environment = ?2,
                     terminal_override_enabled = ?3, terminal_appearance_json = ?4,
                     revision = revision + 1, updated_at_ms = ?5
                     WHERE server_id = ?6 AND revision = ?7",
                            params![
                                label_color,
                                environment,
                                update.terminal_override_enabled,
                                appearance_json,
                                now,
                                update.server_id,
                                update.expected_revision
                            ],
                        )
                        .map_err(storage::map_sqlite_error)?
                };
                if changed == 0 {
                    return Err(revision_conflict());
                }
                transaction.commit().map_err(storage::map_sqlite_error)?;
                get_appearance(connection, &update.server_id)
            })
            .await
    }

    pub async fn references_background_image(&self, image_id: String) -> Result<bool, AppError> {
        self.database
            .execute(move |connection| {
                let mut statement = connection
                    .prepare("SELECT terminal_appearance_json FROM server_appearance")
                    .map_err(storage::map_sqlite_error)?;
                let rows = statement
                    .query_map([], |row| row.get::<_, String>(0))
                    .map_err(storage::map_sqlite_error)?;
                for row in rows {
                    let json = row.map_err(storage::map_sqlite_error)?;
                    let appearance: TerminalAppearanceSettings =
                        serde_json::from_str(&json).map_err(|_| storage_corrupt())?;
                    if appearance.background_image.image_id.as_deref() == Some(image_id.as_str()) {
                        return Ok(true);
                    }
                }
                Ok(false)
            })
            .await
    }
}

fn get_appearance(
    connection: &rusqlite::Connection,
    server_id: &str,
) -> Result<ServerAppearance, AppError> {
    connection
        .query_row(
            "SELECT server_id, label_color, environment, terminal_override_enabled,
                terminal_appearance_json, revision, updated_at_ms
         FROM server_appearance WHERE server_id = ?1",
            [server_id],
            map_appearance,
        )
        .map_err(storage::map_sqlite_error)
}

fn map_appearance(row: &rusqlite::Row<'_>) -> rusqlite::Result<ServerAppearance> {
    let environment: Option<String> = row.get(2)?;
    let json: String = row.get(4)?;
    let appearance = serde_json::from_str(&json).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::other("invalid server appearance")),
        )
    })?;
    Ok(ServerAppearance {
        server_id: row.get(0)?,
        label_color: row.get(1)?,
        environment: environment
            .as_deref()
            .map(ServerEnvironment::from_database_value)
            .transpose()
            .map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::other("invalid server environment")),
                )
            })?,
        terminal_override_enabled: row.get(3)?,
        terminal_appearance: appearance,
        revision: row.get(5)?,
        updated_at_ms: row.get(6)?,
    })
}

fn validate_server_id(value: &str) -> Result<(), AppError> {
    let parsed = uuid::Uuid::parse_str(value).ok();
    if parsed.as_ref().map_or(true, |id| id.to_string() != value) {
        return Err(validation("serverId", "errors.resourceIdInvalid"));
    }
    Ok(())
}

fn validation(field: &'static str, message: &'static str) -> AppError {
    AppError::new(ErrorCode::ValidationFailed, message).with_param("field", field)
}
fn not_found() -> AppError {
    AppError::new(ErrorCode::ResourceNotFound, "errors.resourceNotFound")
}
fn revision_conflict() -> AppError {
    AppError::new(ErrorCode::RevisionConflict, "errors.revisionConflict")
}
fn storage_error() -> AppError {
    AppError::new(ErrorCode::Internal, "errors.storageOperationFailed").with_stage("storage")
}
fn storage_corrupt() -> AppError {
    AppError::new(ErrorCode::Internal, "errors.storageCorrupt").with_stage("storage")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    async fn fixture() -> (tempfile::TempDir, Database, String, ServerAppearanceStore) {
        let directory = tempfile::tempdir().expect("temp dir");
        let database = Database::open(directory.path().join("test.sqlite3")).expect("database");
        let server_id = uuid::Uuid::new_v4().to_string();
        let id_for_insert = server_id.clone();
        database.execute(move |connection| {
            connection.execute(
                "INSERT INTO servers (id, name, host, port, username, auth_type, revision, created_at_ms, updated_at_ms)
                 VALUES (?1, 'Production', 'prod.example.com', 22, 'root', 'password', 1, 1, 1)",
                params![id_for_insert],
            ).map_err(storage::map_sqlite_error)?;
            Ok(())
        }).await.expect("insert fixture server");
        let store = ServerAppearanceStore::new(database.clone());
        (directory, database, server_id, store)
    }

    #[tokio::test]
    async fn appearance_uses_optimistic_revision_and_cascades_with_server() {
        let (_directory, database, server_id, store) = fixture().await;
        let empty = store
            .get(server_id.clone())
            .await
            .expect("default appearance");
        assert_eq!(empty.revision, 0);
        assert_eq!(empty.environment, None);

        let first = store
            .update(ServerAppearanceUpdate {
                server_id: server_id.clone(),
                expected_revision: 0,
                label_color: Some("#D92D20".to_owned()),
                environment: Some(ServerEnvironment::Production),
                terminal_override_enabled: true,
                terminal_appearance: TerminalAppearanceSettings::default(),
            })
            .await
            .expect("insert appearance");
        assert_eq!(first.revision, 1);
        assert_eq!(first.environment, Some(ServerEnvironment::Production));

        let conflict = store
            .update(ServerAppearanceUpdate {
                server_id: server_id.clone(),
                expected_revision: 0,
                label_color: None,
                environment: None,
                terminal_override_enabled: false,
                terminal_appearance: TerminalAppearanceSettings::default(),
            })
            .await
            .expect_err("stale revision");
        assert_eq!(conflict.code, ErrorCode::RevisionConflict);

        let image_id = uuid::Uuid::new_v4().to_string();
        let second = store
            .update(ServerAppearanceUpdate {
                server_id: server_id.clone(),
                expected_revision: 1,
                label_color: first.label_color.clone(),
                environment: first.environment,
                terminal_override_enabled: true,
                terminal_appearance: TerminalAppearanceSettings {
                    theme_mode: TerminalThemeMode::Image,
                    background_image: TerminalBackgroundImageSettings {
                        image_id: Some(image_id.clone()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            })
            .await
            .expect("set image appearance");
        assert_eq!(second.revision, 2);
        assert!(
            store
                .references_background_image(image_id.clone())
                .await
                .expect("image reference")
        );

        database
            .execute(move |connection| {
                connection
                    .execute("DELETE FROM servers WHERE id = ?1", [&server_id])
                    .map_err(storage::map_sqlite_error)?;
                Ok(())
            })
            .await
            .expect("delete server");
        assert!(
            !store
                .references_background_image(image_id)
                .await
                .expect("removed reference")
        );
    }

    #[tokio::test]
    async fn invalid_color_and_image_mode_are_rejected() {
        let (_directory, _database, server_id, store) = fixture().await;
        let invalid_color = store
            .update(ServerAppearanceUpdate {
                server_id: server_id.clone(),
                expected_revision: 0,
                label_color: Some("red".to_owned()),
                environment: None,
                terminal_override_enabled: false,
                terminal_appearance: TerminalAppearanceSettings::default(),
            })
            .await
            .expect_err("reject invalid label color");
        assert_eq!(invalid_color.code, ErrorCode::ValidationFailed);

        let image_without_asset = store
            .update(ServerAppearanceUpdate {
                server_id,
                expected_revision: 0,
                label_color: None,
                environment: None,
                terminal_override_enabled: true,
                terminal_appearance: TerminalAppearanceSettings {
                    theme_mode: TerminalThemeMode::Image,
                    ..Default::default()
                },
            })
            .await
            .expect_err("require image asset");
        assert_eq!(image_without_asset.code, ErrorCode::ValidationFailed);
    }
}
