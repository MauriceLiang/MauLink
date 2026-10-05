use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, ErrorCode as SqliteErrorCode, backup::Backup};
use tokio::sync::oneshot;

use crate::{AppError, ErrorCode};

const CURRENT_SCHEMA_VERSION: i64 = 4;
const REQUEST_QUEUE_CAPACITY: usize = 32;
const INITIAL_MIGRATION: &str = include_str!("../migrations/0001_initial.sql");
const PROFILE_LIST_REVISION_MIGRATION: &str =
    include_str!("../migrations/0002_profile_list_revision.sql");
const ADVANCED_SSH_MIGRATION: &str = include_str!("../migrations/0003_advanced_ssh.sql");
const SERVER_APPEARANCE_MIGRATION: &str = include_str!("../migrations/0004_server_appearance.sql");

type Job = Box<dyn FnOnce(&mut Connection) + Send + 'static>;

enum Message {
    Execute(Job),
    Shutdown,
}

struct Inner {
    sender: mpsc::SyncSender<Message>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Shutdown);
        if let Some(worker) = self.worker.lock().expect("DB worker lock poisoned").take() {
            let _ = worker.join();
        }
    }
}

/// A bounded, single-owner SQLite worker.
#[derive(Clone)]
pub struct Database {
    inner: Arc<Inner>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let path = path.as_ref().to_owned();
        prepare_parent_directory(&path)?;

        let (sender, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
        let (started_sender, started_receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("maulink-sqlite".to_owned())
            .spawn(move || match open_and_migrate(&path) {
                Ok(mut connection) => {
                    let _ = started_sender.send(Ok(()));
                    while let Ok(message) = receiver.recv() {
                        match message {
                            Message::Execute(job) => job(&mut connection),
                            Message::Shutdown => break,
                        }
                    }
                }
                Err(error) => {
                    let _ = started_sender.send(Err(error));
                }
            })
            .map_err(|_| storage_error("errors.storageWorkerStartFailed"))?;

        match started_receiver.recv() {
            Ok(Ok(())) => Ok(Self {
                inner: Arc::new(Inner {
                    sender,
                    worker: Mutex::new(Some(worker)),
                }),
            }),
            Ok(Err(error)) => {
                let _ = worker.join();
                Err(error)
            }
            Err(_) => {
                let _ = worker.join();
                Err(storage_error("errors.storageWorkerStartFailed"))
            }
        }
    }

    pub(crate) async fn execute<T, F>(&self, operation: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T, AppError> + Send + 'static,
    {
        let (result_sender, result_receiver) = oneshot::channel();
        let job = Box::new(move |connection: &mut Connection| {
            let _ = result_sender.send(operation(connection));
        });

        self.inner
            .sender
            .try_send(Message::Execute(job))
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => {
                    AppError::new(ErrorCode::StorageBusy, "errors.storageQueueFull").with_retry()
                }
                mpsc::TrySendError::Disconnected(_) => storage_error("errors.storageWorkerStopped"),
            })?;

        result_receiver
            .await
            .map_err(|_| storage_error("errors.storageWorkerStopped"))?
    }
}

fn prepare_parent_directory(path: &Path) -> Result<(), AppError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|_| storage_error("errors.storageDirectoryCreateFailed"))?;
    Ok(())
}

fn open_and_migrate(path: &Path) -> Result<Connection, AppError> {
    let existed_with_data = path.metadata().map(|meta| meta.len() > 0).unwrap_or(false);
    let mut connection = Connection::open(path).map_err(map_sqlite_open_error)?;
    connection
        .busy_timeout(Duration::from_secs(3))
        .map_err(map_sqlite_error)?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(map_sqlite_error)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|_| storage_error("errors.storageFilePermissionsFailed"))?;
    }

    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(map_sqlite_error)?;
    if version > CURRENT_SCHEMA_VERSION {
        return Err(
            AppError::new(ErrorCode::SchemaTooNew, "errors.schemaTooNew")
                .with_param("schemaVersion", version.to_string()),
        );
    }

    if version < CURRENT_SCHEMA_VERSION {
        if existed_with_data {
            create_backup(path, &connection, version)?;
        }
        migrate(&mut connection, version)?;
    }
    Ok(connection)
}

fn create_backup(path: &Path, source: &Connection, version: i64) -> Result<(), AppError> {
    let timestamp = now_ms()?;
    let backup_path = backup_path(path, version, timestamp);
    let mut destination = Connection::open(&backup_path)
        .map_err(|_| migration_error("errors.migrationBackupFailed"))?;
    Backup::new(source, &mut destination)
        .and_then(|backup| backup.run_to_completion(64, Duration::from_millis(5), None))
        .map_err(|_| migration_error("errors.migrationBackupFailed"))?;
    drop(destination);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&backup_path, fs::Permissions::from_mode(0o600))
            .map_err(|_| migration_error("errors.migrationBackupPermissionsFailed"))?;
    }
    prune_backups(path)?;
    Ok(())
}

fn migrate(connection: &mut Connection, from_version: i64) -> Result<(), AppError> {
    let transaction = connection
        .transaction()
        .map_err(|_| migration_error("errors.migrationFailed"))?;
    if from_version < 1 {
        transaction
            .execute_batch(INITIAL_MIGRATION)
            .map_err(|_| migration_error("errors.migrationFailed"))?;
    }
    if from_version < 2 {
        transaction
            .execute_batch(PROFILE_LIST_REVISION_MIGRATION)
            .map_err(|_| migration_error("errors.migrationFailed"))?;
    }
    if from_version < 3 {
        transaction
            .execute_batch(ADVANCED_SSH_MIGRATION)
            .map_err(|_| migration_error("errors.migrationFailed"))?;
    }
    if from_version < 4 {
        transaction
            .execute_batch(SERVER_APPEARANCE_MIGRATION)
            .map_err(|_| migration_error("errors.migrationFailed"))?;
    }
    transaction
        .pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)
        .map_err(|_| migration_error("errors.migrationFailed"))?;
    transaction
        .commit()
        .map_err(|_| migration_error("errors.migrationFailed"))
}

fn backup_path(database_path: &Path, version: i64, timestamp: i64) -> PathBuf {
    let file_name = database_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("maulink.sqlite3");
    database_path.with_file_name(format!("{file_name}.backup-v{version}-{timestamp}"))
}

fn prune_backups(database_path: &Path) -> Result<(), AppError> {
    let Some(parent) = database_path.parent() else {
        return Ok(());
    };
    let file_name = database_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("maulink.sqlite3");
    let prefix = format!("{file_name}.backup-v");
    let mut backups = fs::read_dir(parent)
        .map_err(|_| migration_error("errors.migrationBackupCleanupFailed"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        })
        .collect::<Vec<_>>();
    backups.sort();
    let remove_count = backups.len().saturating_sub(3);
    for backup in backups.into_iter().take(remove_count) {
        fs::remove_file(backup)
            .map_err(|_| migration_error("errors.migrationBackupCleanupFailed"))?;
    }
    Ok(())
}

pub(crate) fn now_ms() -> Result<i64, AppError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| storage_error("errors.systemClockInvalid"))?
        .as_millis();
    i64::try_from(millis).map_err(|_| storage_error("errors.systemClockInvalid"))
}

pub(crate) fn map_sqlite_error(error: rusqlite::Error) -> AppError {
    if matches!(
        error,
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: SqliteErrorCode::DatabaseBusy | SqliteErrorCode::DatabaseLocked,
                ..
            },
            _
        )
    ) {
        return AppError::new(ErrorCode::StorageBusy, "errors.storageBusy")
            .with_stage("storage")
            .with_retry();
    }
    storage_error("errors.storageOperationFailed")
}

fn map_sqlite_open_error(_: rusqlite::Error) -> AppError {
    storage_error("errors.storageOpenFailed")
}

fn storage_error(message_key: &'static str) -> AppError {
    AppError::new(ErrorCode::Internal, message_key).with_stage("storage")
}

fn migration_error(message_key: &'static str) -> AppError {
    AppError::new(ErrorCode::MigrationFailed, message_key).with_stage("migration")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_schema_and_rejects_newer_versions() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("maulink.sqlite3");
        drop(Database::open(&path).expect("create database"));

        let connection = Connection::open(&path).expect("open database");
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
        connection
            .pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION + 1)
            .expect("set newer version");
        drop(connection);

        let error = match Database::open(&path) {
            Ok(_) => panic!("newer schema must fail"),
            Err(error) => error,
        };
        assert_eq!(error.code, ErrorCode::SchemaTooNew);
    }

    #[test]
    fn backs_up_existing_version_zero_database() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("maulink.sqlite3");
        let connection = Connection::open(&path).expect("open old database");
        connection
            .execute_batch("CREATE TABLE legacy_marker (value TEXT NOT NULL);")
            .expect("create old schema marker");
        drop(connection);

        drop(Database::open(&path).expect("migrate database"));
        let backup_count = fs::read_dir(directory.path())
            .expect("list backup directory")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("maulink.sqlite3.backup-v0-")
            })
            .count();
        assert_eq!(backup_count, 1);
    }

    #[test]
    fn migrates_version_two_profiles_with_safe_advanced_defaults() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("maulink.sqlite3");
        let connection = Connection::open(&path).expect("open old database");
        connection
            .execute_batch(INITIAL_MIGRATION)
            .expect("create initial schema");
        connection
            .execute_batch(PROFILE_LIST_REVISION_MIGRATION)
            .expect("create version two schema");
        connection
            .execute(
                "INSERT INTO servers
                 (id, name, host, port, username, auth_type, created_at_ms, updated_at_ms)
                 VALUES ('server-id', 'legacy', 'legacy.example.test', 22, 'user', 'password', 1, 1)",
                [],
            )
            .expect("insert legacy server");
        connection
            .pragma_update(None, "user_version", 2_i64)
            .expect("mark version two");
        drop(connection);

        drop(Database::open(&path).expect("migrate version two database"));
        let connection = Connection::open(&path).expect("open migrated database");
        let (jump_host, jump_port, proxy_type, proxy_host, proxy_port): (
            Option<String>,
            u16,
            Option<String>,
            Option<String>,
            Option<u16>,
        ) = connection
            .query_row(
                "SELECT jump_host, jump_port, proxy_type, proxy_host, proxy_port
                 FROM servers WHERE id = 'server-id'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("read migrated server");
        assert_eq!(jump_host, None);
        assert_eq!(jump_port, 22);
        assert_eq!(proxy_type, None);
        assert_eq!(proxy_host, None);
        assert_eq!(proxy_port, None);
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read migrated version");
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn failed_migration_keeps_original_database_intact() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("maulink.sqlite3");
        let connection = Connection::open(&path).expect("open old database");
        connection
            .execute_batch("CREATE TABLE server_groups (legacy_value TEXT NOT NULL);")
            .expect("create conflicting old schema");
        drop(connection);

        let error = match Database::open(&path) {
            Ok(_) => panic!("conflicting migration must fail"),
            Err(error) => error,
        };
        assert_eq!(error.code, ErrorCode::MigrationFailed);

        let connection = Connection::open(&path).expect("reopen original database");
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        assert_eq!(version, 0);
        let legacy_column_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('server_groups')
                 WHERE name = 'legacy_value'",
                [],
                |row| row.get(0),
            )
            .expect("check original schema");
        assert_eq!(legacy_column_count, 1);
    }
}
