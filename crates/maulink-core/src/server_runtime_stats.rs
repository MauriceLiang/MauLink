use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{AppError, ConnectionPreflightResult, Database, ErrorCode, storage};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerRuntimeStats {
    pub server_id: String,
    #[ts(type = "number | null")]
    pub last_success_at_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub last_failure_at_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub last_preflight_at_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub last_preflight_latency_ms: Option<i64>,
    pub last_failure_code: Option<String>,
    #[ts(type = "number")]
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerRuntimeStatsPayload {
    pub server_id: String,
}

#[derive(Clone)]
pub struct ServerRuntimeStatsStore {
    database: Database,
}

impl ServerRuntimeStatsStore {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn get(&self, server_id: String) -> Result<ServerRuntimeStats, AppError> {
        validate_server_id(&server_id)?;
        self.database
            .execute(move |connection| {
                if let Some(stats) = connection
                    .query_row(
                        "SELECT server_id, last_success_at_ms, last_failure_at_ms,
                         last_preflight_at_ms, last_preflight_latency_ms, last_failure_code,
                         updated_at_ms
                         FROM server_runtime_stats WHERE server_id = ?1",
                        [&server_id],
                        map_stats,
                    )
                    .optional()
                    .map_err(storage::map_sqlite_error)?
                {
                    return Ok(stats);
                }
                server_exists(connection, &server_id)?;
                Ok(empty_stats(server_id))
            })
            .await
    }

    pub async fn record_preflight(
        &self,
        server_id: String,
        result: ConnectionPreflightResult,
    ) -> Result<(), AppError> {
        validate_server_id(&server_id)?;
        let latency_ms = result
            .tcp_connect_duration_ms
            .map(i64::try_from)
            .transpose()
            .map_err(|_| invalid_timestamp())?;
        self.database
            .execute(move |connection| {
                connection
                    .execute(
                        "INSERT INTO server_runtime_stats
                         (server_id, last_preflight_at_ms, last_preflight_latency_ms, updated_at_ms)
                         VALUES (?1, ?2, ?3, ?2)
                         ON CONFLICT(server_id) DO UPDATE SET
                           last_preflight_at_ms = excluded.last_preflight_at_ms,
                           last_preflight_latency_ms = excluded.last_preflight_latency_ms,
                           updated_at_ms = excluded.updated_at_ms",
                        params![server_id, result.checked_at_ms, latency_ms],
                    )
                    .map_err(storage::map_sqlite_error)?;
                Ok(())
            })
            .await
    }

    pub(crate) async fn record_success(
        &self,
        server_id: String,
        at_ms: i64,
    ) -> Result<(), AppError> {
        self.record_connection_outcome(server_id, at_ms, None).await
    }

    pub(crate) async fn record_failure(
        &self,
        server_id: String,
        at_ms: i64,
        failure_code: String,
    ) -> Result<(), AppError> {
        self.record_connection_outcome(server_id, at_ms, Some(failure_code))
            .await
    }

    async fn record_connection_outcome(
        &self,
        server_id: String,
        at_ms: i64,
        failure_code: Option<String>,
    ) -> Result<(), AppError> {
        validate_server_id(&server_id)?;
        self.database
            .execute(move |connection| {
                match failure_code {
                    None => {
                        connection
                            .execute(
                                "INSERT INTO server_runtime_stats
                                 (server_id, last_success_at_ms, updated_at_ms)
                                 VALUES (?1, ?2, ?2)
                                 ON CONFLICT(server_id) DO UPDATE SET
                                   last_success_at_ms = excluded.last_success_at_ms,
                                   updated_at_ms = excluded.updated_at_ms",
                                params![server_id, at_ms],
                            )
                            .map_err(storage::map_sqlite_error)?;
                    }
                    Some(code) => {
                        connection
                            .execute(
                                "INSERT INTO server_runtime_stats
                                 (server_id, last_failure_at_ms, last_failure_code, updated_at_ms)
                                 VALUES (?1, ?2, ?3, ?2)
                                 ON CONFLICT(server_id) DO UPDATE SET
                                   last_failure_at_ms = excluded.last_failure_at_ms,
                                   last_failure_code = excluded.last_failure_code,
                                   updated_at_ms = excluded.updated_at_ms",
                                params![server_id, at_ms, code],
                            )
                            .map_err(storage::map_sqlite_error)?;
                    }
                }
                Ok(())
            })
            .await
    }
}

fn map_stats(row: &rusqlite::Row<'_>) -> rusqlite::Result<ServerRuntimeStats> {
    Ok(ServerRuntimeStats {
        server_id: row.get(0)?,
        last_success_at_ms: row.get(1)?,
        last_failure_at_ms: row.get(2)?,
        last_preflight_at_ms: row.get(3)?,
        last_preflight_latency_ms: row.get(4)?,
        last_failure_code: row.get(5)?,
        updated_at_ms: row.get(6)?,
    })
}

fn empty_stats(server_id: String) -> ServerRuntimeStats {
    ServerRuntimeStats {
        server_id,
        last_success_at_ms: None,
        last_failure_at_ms: None,
        last_preflight_at_ms: None,
        last_preflight_latency_ms: None,
        last_failure_code: None,
        updated_at_ms: 0,
    }
}

fn server_exists(connection: &rusqlite::Connection, server_id: &str) -> Result<(), AppError> {
    let exists = connection
        .query_row("SELECT 1 FROM servers WHERE id = ?1", [server_id], |_| {
            Ok(())
        })
        .optional()
        .map_err(storage::map_sqlite_error)?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(AppError::new(
            ErrorCode::ResourceNotFound,
            "errors.serverNotFound",
        ))
    }
}

fn validate_server_id(server_id: &str) -> Result<(), AppError> {
    Uuid::parse_str(server_id).map(|_| ()).map_err(|_| {
        AppError::new(ErrorCode::ValidationFailed, "errors.resourceIdInvalid")
            .with_param("field", "serverId")
    })
}

fn invalid_timestamp() -> AppError {
    AppError::new(ErrorCode::ValidationFailed, "errors.runtimeActivityInvalid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthType, ProfileStore, ServerProfileInput};

    fn profile() -> ServerProfileInput {
        ServerProfileInput {
            name: Some("Activity fixture".to_owned()),
            host: "activity.example.test".to_owned(),
            port: 22,
            username: "fixture".to_owned(),
            auth_type: AuthType::Password,
            private_key_path: None,
            group_id: None,
            connect_timeout_ms: 15_000,
            keepalive_interval_seconds: 30,
            jump_host: None,
            jump_port: 22,
            proxy_type: None,
            proxy_host: None,
            proxy_port: None,
        }
    }

    #[tokio::test]
    async fn persists_summary_fields_and_cascades_with_server_deletion() {
        let directory = tempfile::tempdir().expect("temporary database directory");
        let database = Database::open(directory.path().join("activity.sqlite3")).expect("database");
        let profiles = ProfileStore::new(database.clone());
        let server = profiles
            .create_server(profile())
            .await
            .expect("create test server");
        let store = ServerRuntimeStatsStore::new(database);

        store
            .record_success(server.id.clone(), 10)
            .await
            .expect("record success");
        store
            .record_failure(server.id.clone(), 20, "AUTH_FAILED".to_owned())
            .await
            .expect("record failure");
        store
            .record_preflight(
                server.id.clone(),
                ConnectionPreflightResult {
                    resolved_addresses: vec!["192.0.2.1".to_owned()],
                    selected_address: Some("192.0.2.1".to_owned()),
                    dns_duration_ms: 4,
                    tcp_reachable: Some(true),
                    tcp_connect_duration_ms: Some(47),
                    error: None,
                    checked_at_ms: 30,
                },
            )
            .await
            .expect("record preflight");
        assert_eq!(
            store.get(server.id.clone()).await.expect("read activity"),
            ServerRuntimeStats {
                server_id: server.id.clone(),
                last_success_at_ms: Some(10),
                last_failure_at_ms: Some(20),
                last_preflight_at_ms: Some(30),
                last_preflight_latency_ms: Some(47),
                last_failure_code: Some("AUTH_FAILED".to_owned()),
                updated_at_ms: 30,
            }
        );

        profiles
            .delete_server(server.id.clone(), server.revision)
            .await
            .expect("delete server");
        assert_eq!(
            store
                .get(server.id)
                .await
                .expect_err("server was deleted")
                .code,
            ErrorCode::ResourceNotFound
        );
    }
}
