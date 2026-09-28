use std::path::PathBuf;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;
use uuid::Uuid;

use crate::{AppError, Database, ErrorCode, storage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AuthType {
    Password,
    PrivateKey,
}

impl AuthType {
    pub(crate) fn as_database_value(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::PrivateKey => "private_key",
        }
    }

    fn from_database_value(value: &str) -> Result<Self, AppError> {
        match value {
            "password" => Ok(Self::Password),
            "private_key" => Ok(Self::PrivateKey),
            _ => Err(storage_corrupt()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathEncoding {
    UnixBytes,
    WindowsUtf16le,
}

impl PathEncoding {
    pub(crate) fn as_database_value(self) -> &'static str {
        match self {
            Self::UnixBytes => "unix_bytes",
            Self::WindowsUtf16le => "windows_utf16le",
        }
    }

    fn from_database_value(value: &str) -> Result<Self, AppError> {
        match value {
            "unix_bytes" => Ok(Self::UnixBytes),
            "windows_utf16le" => Ok(Self::WindowsUtf16le),
            _ => Err(storage_corrupt()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredPath {
    pub bytes: Vec<u8>,
    pub encoding: PathEncoding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: String,
    pub sort_order: i32,
    pub revision: u32,
    #[ts(type = "number")]
    pub created_at_ms: i64,
    #[ts(type = "number")]
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GroupCreate {
    pub name: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GroupUpdate {
    pub name: String,
    pub sort_order: i32,
    pub expected_revision: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfileInput {
    pub name: Option<String>,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: AuthType,
    pub private_key_path: Option<StoredPath>,
    pub group_id: Option<String>,
    pub connect_timeout_ms: u32,
    pub keepalive_interval_seconds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: AuthType,
    pub has_private_key: bool,
    pub group_id: Option<String>,
    pub has_saved_credential: bool,
    pub connect_timeout_ms: u32,
    pub keepalive_interval_seconds: u32,
    pub revision: u32,
    #[ts(type = "number")]
    pub created_at_ms: i64,
    #[ts(type = "number")]
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerListQuery {
    pub query: Option<String>,
    pub group_id: Option<String>,
    pub limit: u16,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerListPage {
    pub items: Vec<ServerProfile>,
    pub next_cursor: Option<String>,
}

impl Default for ServerListQuery {
    fn default() -> Self {
        Self {
            query: None,
            group_id: None,
            limit: 100,
            cursor: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerListCursor {
    list_revision: u64,
    offset: u32,
    query_fingerprint: String,
}

#[derive(Clone)]
pub struct ProfileStore {
    database: Database,
}

impl ProfileStore {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn list_groups(&self) -> Result<Vec<Group>, AppError> {
        self.database
            .execute(|connection| {
                let mut statement = connection
                    .prepare(
                        "SELECT id, name, sort_order, revision, created_at_ms, updated_at_ms
                         FROM server_groups ORDER BY sort_order, name COLLATE NOCASE, id",
                    )
                    .map_err(storage::map_sqlite_error)?;
                let rows = statement
                    .query_map([], map_group_row)
                    .map_err(storage::map_sqlite_error)?;
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(storage::map_sqlite_error)
            })
            .await
    }

    pub async fn create_group(&self, input: GroupCreate) -> Result<Group, AppError> {
        let name = validate_name("name", input.name, 64)?;
        let now = storage::now_ms()?;
        let group = Group {
            id: Uuid::new_v4().to_string(),
            name,
            sort_order: input.sort_order,
            revision: 1,
            created_at_ms: now,
            updated_at_ms: now,
        };
        let stored = group.clone();
        self.database
            .execute(move |connection| {
                let transaction = connection
                    .transaction()
                    .map_err(storage::map_sqlite_error)?;
                transaction
                    .execute(
                        "INSERT INTO server_groups
                         (id, name, sort_order, revision, created_at_ms, updated_at_ms)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            stored.id,
                            stored.name,
                            stored.sort_order,
                            stored.revision,
                            stored.created_at_ms,
                            stored.updated_at_ms
                        ],
                    )
                    .map_err(storage::map_sqlite_error)?;
                bump_profile_list_revision(&transaction)?;
                transaction.commit().map_err(storage::map_sqlite_error)?;
                Ok(stored)
            })
            .await
    }

    pub async fn update_group(
        &self,
        group_id: String,
        input: GroupUpdate,
    ) -> Result<Group, AppError> {
        validate_id(&group_id)?;
        let name = validate_name("name", input.name, 64)?;
        let now = storage::now_ms()?;
        self.database
            .execute(move |connection| {
                let transaction = connection
                    .transaction()
                    .map_err(storage::map_sqlite_error)?;
                let changed = transaction
                    .execute(
                        "UPDATE server_groups
                         SET name = ?1, sort_order = ?2, revision = revision + 1, updated_at_ms = ?3
                         WHERE id = ?4 AND revision = ?5",
                        params![
                            name,
                            input.sort_order,
                            now,
                            group_id,
                            input.expected_revision
                        ],
                    )
                    .map_err(storage::map_sqlite_error)?;
                if changed == 0 {
                    return Err(missing_or_conflict(
                        &transaction,
                        "server_groups",
                        &group_id,
                        input.expected_revision,
                    ));
                }
                bump_profile_list_revision(&transaction)?;
                transaction.commit().map_err(storage::map_sqlite_error)?;
                get_group(connection, &group_id)
            })
            .await
    }

    pub async fn delete_group(
        &self,
        group_id: String,
        expected_revision: u32,
    ) -> Result<(), AppError> {
        validate_id(&group_id)?;
        self.database
            .execute(move |connection| {
                let transaction = connection
                    .transaction()
                    .map_err(storage::map_sqlite_error)?;
                let changed = transaction
                    .execute(
                        "DELETE FROM server_groups WHERE id = ?1 AND revision = ?2",
                        params![group_id, expected_revision],
                    )
                    .map_err(storage::map_sqlite_error)?;
                if changed == 0 {
                    return Err(missing_or_conflict(
                        &transaction,
                        "server_groups",
                        &group_id,
                        expected_revision,
                    ));
                }
                bump_profile_list_revision(&transaction)?;
                transaction.commit().map_err(storage::map_sqlite_error)
            })
            .await
    }

    pub async fn create_server(
        &self,
        input: ServerProfileInput,
    ) -> Result<ServerProfile, AppError> {
        let input = validate_server_input(input)?;
        let now = storage::now_ms()?;
        let server_id = Uuid::new_v4().to_string();
        self.database
            .execute(move |connection| {
                let transaction = connection
                    .transaction()
                    .map_err(storage::map_sqlite_error)?;
                ensure_group_exists(&transaction, input.group_id.as_deref())?;
                transaction
                    .execute(
                        "INSERT INTO servers
                         (id, name, host, port, username, auth_type, private_key_path,
                          private_key_path_encoding, group_id, connect_timeout_ms,
                          keepalive_interval_s, revision, created_at_ms, updated_at_ms)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1, ?12, ?12)",
                        params![
                            server_id,
                            input.name.as_deref().expect("validated name"),
                            input.host,
                            input.port,
                            input.username,
                            input.auth_type.as_database_value(),
                            input
                                .private_key_path
                                .as_ref()
                                .map(|path| path.bytes.as_slice()),
                            input
                                .private_key_path
                                .as_ref()
                                .map(|path| path.encoding.as_database_value()),
                            input.group_id,
                            input.connect_timeout_ms,
                            input.keepalive_interval_seconds,
                            now,
                        ],
                    )
                    .map_err(storage::map_sqlite_error)?;
                bump_profile_list_revision(&transaction)?;
                transaction.commit().map_err(storage::map_sqlite_error)?;
                get_server(connection, &server_id)
            })
            .await
    }

    pub async fn get_server(&self, server_id: String) -> Result<ServerProfile, AppError> {
        validate_id(&server_id)?;
        self.database
            .execute(move |connection| get_server(connection, &server_id))
            .await
    }

    /// Returns the stored private-key path only for rebuilding a full update
    /// input inside the trusted native adapter. This value must not cross IPC.
    pub async fn private_key_path_for_update(
        &self,
        server_id: String,
        expected_revision: u32,
    ) -> Result<StoredPath, AppError> {
        validate_id(&server_id)?;
        self.database
            .execute(move |connection| {
                let row = connection
                    .query_row(
                        "SELECT revision, auth_type, private_key_path, private_key_path_encoding
                         FROM servers WHERE id = ?1",
                        [&server_id],
                        |row| {
                            Ok((
                                row.get::<_, u32>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, Option<Vec<u8>>>(2)?,
                                row.get::<_, Option<String>>(3)?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(storage::map_sqlite_error)?
                    .ok_or_else(|| not_found("serverId"))?;
                if row.0 != expected_revision {
                    return Err(missing_or_conflict(
                        connection,
                        "servers",
                        &server_id,
                        expected_revision,
                    ));
                }
                match (row.1.as_str(), row.2, row.3) {
                    ("private_key", Some(bytes), Some(encoding)) => Ok(StoredPath {
                        bytes,
                        encoding: PathEncoding::from_database_value(&encoding)?,
                    }),
                    ("private_key", _, _) => Err(storage_corrupt()),
                    _ => Err(validation(
                        "privateKeyToken",
                        "errors.privateKeyTokenRequired",
                    )),
                }
            })
            .await
    }

    pub async fn private_key_path_for_authentication(
        &self,
        server_id: String,
        expected_revision: u32,
    ) -> Result<PathBuf, AppError> {
        let stored = self
            .private_key_path_for_update(server_id, expected_revision)
            .await?;
        native_path_from_stored(stored)
    }

    pub async fn list_servers(&self, query: ServerListQuery) -> Result<ServerListPage, AppError> {
        if !(1..=200).contains(&query.limit) {
            return Err(validation("limit", "errors.limitOutOfRange"));
        }
        if let Some(group_id) = &query.group_id {
            validate_id(group_id)?;
        }
        let search = query.query.unwrap_or_default().trim().to_owned();
        if search.chars().count() > 256 {
            return Err(validation("query", "errors.queryTooLong"));
        }
        let search_pattern = if search.is_empty() {
            None
        } else {
            Some(format!("%{}%", escape_like_pattern(&search)))
        };
        let fingerprint = query_fingerprint(&search, query.group_id.as_deref());
        let decoded_cursor = query.cursor.as_deref().map(decode_cursor).transpose()?;
        if decoded_cursor
            .as_ref()
            .is_some_and(|cursor| cursor.query_fingerprint != fingerprint)
        {
            return Err(validation("cursor", "errors.cursorQueryMismatch"));
        }
        self.database
            .execute(move |connection| {
                let list_revision = profile_list_revision(connection)?;
                let offset = match decoded_cursor {
                    Some(cursor) if cursor.list_revision == list_revision => cursor.offset,
                    Some(_) => {
                        let mut error =
                            AppError::new(ErrorCode::RevisionConflict, "errors.listCursorExpired");
                        error.action = crate::ErrorAction::Reload;
                        return Err(error);
                    }
                    None => 0,
                };
                let row_limit = u32::from(query.limit) + 1;
                let mut statement = connection
                    .prepare(
                        "SELECT s.id, s.name, s.host, s.port, s.username, s.auth_type,
                                s.private_key_path, s.private_key_path_encoding, s.group_id,
                                s.credential_ref_id IS NOT NULL, s.connect_timeout_ms,
                                s.keepalive_interval_s, s.revision, s.created_at_ms, s.updated_at_ms
                         FROM servers s
                         LEFT JOIN server_groups g ON g.id = s.group_id
                         WHERE (?1 IS NULL OR s.name COLLATE NOCASE LIKE ?1 ESCAPE '\\'
                                OR s.host COLLATE NOCASE LIKE ?1 ESCAPE '\\'
                                OR g.name COLLATE NOCASE LIKE ?1 ESCAPE '\\')
                           AND (?2 IS NULL OR s.group_id = ?2)
                         ORDER BY s.updated_at_ms DESC, s.id
                         LIMIT ?3 OFFSET ?4",
                    )
                    .map_err(storage::map_sqlite_error)?;
                let rows = statement
                    .query_map(
                        params![search_pattern, query.group_id, row_limit, offset],
                        map_server_row,
                    )
                    .map_err(storage::map_sqlite_error)?;
                let mut items = rows
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(storage::map_sqlite_error)?
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, AppError>>()?;
                let has_more = items.len() > usize::from(query.limit);
                items.truncate(usize::from(query.limit));
                let next_cursor = has_more
                    .then(|| {
                        let item_count =
                            u32::try_from(items.len()).map_err(|_| storage_corrupt())?;
                        encode_cursor(&ServerListCursor {
                            list_revision,
                            offset: offset.checked_add(item_count).ok_or_else(storage_corrupt)?,
                            query_fingerprint: fingerprint,
                        })
                    })
                    .transpose()?;
                Ok(ServerListPage { items, next_cursor })
            })
            .await
    }

    pub async fn update_server(
        &self,
        server_id: String,
        expected_revision: u32,
        input: ServerProfileInput,
    ) -> Result<ServerProfile, AppError> {
        validate_id(&server_id)?;
        let input = validate_server_input(input)?;
        let now = storage::now_ms()?;
        self.database
            .execute(move |connection| {
                let transaction = connection
                    .transaction()
                    .map_err(storage::map_sqlite_error)?;
                ensure_group_exists(&transaction, input.group_id.as_deref())?;
                let changed = transaction
                    .execute(
                        "UPDATE servers
                         SET name = ?1, host = ?2, port = ?3, username = ?4, auth_type = ?5,
                             private_key_path = ?6, private_key_path_encoding = ?7, group_id = ?8,
                             connect_timeout_ms = ?9, keepalive_interval_s = ?10,
                             revision = revision + 1, updated_at_ms = ?11
                         WHERE id = ?12 AND revision = ?13",
                        params![
                            input.name.as_deref().expect("validated name"),
                            input.host,
                            input.port,
                            input.username,
                            input.auth_type.as_database_value(),
                            input
                                .private_key_path
                                .as_ref()
                                .map(|path| path.bytes.as_slice()),
                            input
                                .private_key_path
                                .as_ref()
                                .map(|path| path.encoding.as_database_value()),
                            input.group_id,
                            input.connect_timeout_ms,
                            input.keepalive_interval_seconds,
                            now,
                            server_id,
                            expected_revision,
                        ],
                    )
                    .map_err(storage::map_sqlite_error)?;
                if changed == 0 {
                    return Err(missing_or_conflict(
                        &transaction,
                        "servers",
                        &server_id,
                        expected_revision,
                    ));
                }
                bump_profile_list_revision(&transaction)?;
                transaction.commit().map_err(storage::map_sqlite_error)?;
                get_server(connection, &server_id)
            })
            .await
    }

    pub async fn delete_server(
        &self,
        server_id: String,
        expected_revision: u32,
    ) -> Result<(), AppError> {
        validate_id(&server_id)?;
        self.database
            .execute(move |connection| {
                let transaction = connection
                    .transaction()
                    .map_err(storage::map_sqlite_error)?;
                let server = transaction
                    .query_row(
                        "SELECT revision, credential_ref_id IS NOT NULL FROM servers WHERE id = ?1",
                        [&server_id],
                        |row| Ok((row.get::<_, u32>(0)?, row.get::<_, bool>(1)?)),
                    )
                    .optional()
                    .map_err(storage::map_sqlite_error)?
                    .ok_or_else(|| not_found("serverId"))?;
                if server.0 != expected_revision {
                    return Err(missing_or_conflict(
                        &transaction,
                        "servers",
                        &server_id,
                        expected_revision,
                    ));
                }
                if server.1 {
                    return Err(validation(
                        "removeCredentials",
                        "errors.credentialDispositionRequired",
                    ));
                }
                let changed = transaction
                    .execute(
                        "DELETE FROM servers WHERE id = ?1 AND revision = ?2",
                        params![server_id, expected_revision],
                    )
                    .map_err(storage::map_sqlite_error)?;
                if changed == 0 {
                    return Err(missing_or_conflict(
                        &transaction,
                        "servers",
                        &server_id,
                        expected_revision,
                    ));
                }
                bump_profile_list_revision(&transaction)?;
                transaction.commit().map_err(storage::map_sqlite_error)
            })
            .await
    }
}

pub(crate) fn validate_server_input(
    mut input: ServerProfileInput,
) -> Result<ServerProfileInput, AppError> {
    let host = input.host.trim().to_owned();
    if host.is_empty()
        || host.chars().count() > 253
        || host.chars().any(char::is_whitespace)
        || host.chars().any(char::is_control)
        || host.contains("@")
        || host.contains("/")
        || host.contains("\\")
        || host.contains("://")
    {
        return Err(validation("host", "errors.hostInvalid"));
    }
    input.host = host;

    if input.port == 0 {
        return Err(validation("port", "errors.portOutOfRange"));
    }

    if input.username.is_empty()
        || input.username.chars().count() > 256
        || input.username.contains(['\0', '\r', '\n'])
    {
        return Err(validation("username", "errors.usernameInvalid"));
    }
    if !(1_000..=120_000).contains(&input.connect_timeout_ms) {
        return Err(validation(
            "connectTimeoutMs",
            "errors.connectTimeoutOutOfRange",
        ));
    }
    if !(5..=300).contains(&input.keepalive_interval_seconds) {
        return Err(validation(
            "keepaliveIntervalSeconds",
            "errors.keepaliveOutOfRange",
        ));
    }
    match (input.auth_type, &input.private_key_path) {
        (AuthType::Password, Some(_)) | (AuthType::PrivateKey, None) => {
            return Err(validation("privateKeyPath", "errors.privateKeyPathInvalid"));
        }
        (_, Some(path)) if path.bytes.is_empty() => {
            return Err(validation("privateKeyPath", "errors.privateKeyPathInvalid"));
        }
        _ => {}
    }
    if let Some(group_id) = &input.group_id {
        validate_id(group_id)?;
    }
    let name = match input.name {
        Some(name) if !name.trim().is_empty() => validate_name("name", name, 128)?,
        Some(_) | None => validate_name("name", format!("{}@{}", input.username, input.host), 128)?,
    };
    input.name = Some(name);
    Ok(input)
}

fn escape_like_pattern(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn query_fingerprint(query: &str, group_id: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    hasher.update([0]);
    hasher.update(group_id.unwrap_or_default().as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn encode_cursor(cursor: &ServerListCursor) -> Result<String, AppError> {
    serde_json::to_vec(cursor)
        .map(|value| URL_SAFE_NO_PAD.encode(value))
        .map_err(|_| storage_corrupt())
}

fn decode_cursor(value: &str) -> Result<ServerListCursor, AppError> {
    if value.len() > 512 {
        return Err(validation("cursor", "errors.cursorInvalid"));
    }
    let decoded = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| validation("cursor", "errors.cursorInvalid"))?;
    serde_json::from_slice(&decoded).map_err(|_| validation("cursor", "errors.cursorInvalid"))
}

fn profile_list_revision(connection: &Connection) -> Result<u64, AppError> {
    let revision = connection
        .query_row(
            "SELECT integer_value FROM app_metadata WHERE key = 'profile_list_revision'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage::map_sqlite_error)?;
    u64::try_from(revision).map_err(|_| storage_corrupt())
}

fn bump_profile_list_revision(connection: &Connection) -> Result<(), AppError> {
    let changed = connection
        .execute(
            "UPDATE app_metadata SET integer_value = integer_value + 1
             WHERE key = 'profile_list_revision'",
            [],
        )
        .map_err(storage::map_sqlite_error)?;
    if changed == 1 {
        Ok(())
    } else {
        Err(storage_corrupt())
    }
}

fn validate_name(field: &'static str, name: String, max: usize) -> Result<String, AppError> {
    let name = name.trim().to_owned();
    let length = name.chars().count();
    if length == 0 || length > max || name.chars().any(char::is_control) {
        return Err(validation(field, "errors.nameInvalid"));
    }
    Ok(name)
}

pub(crate) fn native_path_from_stored(path: StoredPath) -> Result<PathBuf, AppError> {
    #[cfg(unix)]
    {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};

        if path.encoding != PathEncoding::UnixBytes {
            return Err(validation("privateKeyPath", "errors.privateKeyPathInvalid"));
        }
        Ok(PathBuf::from(OsString::from_vec(path.bytes)))
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;

        if path.encoding != PathEncoding::WindowsUtf16le || path.bytes.len() % 2 != 0 {
            return Err(validation("privateKeyPath", "errors.privateKeyPathInvalid"));
        }
        let wide = path
            .bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        Ok(PathBuf::from(std::ffi::OsString::from_wide(&wide)))
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(validation("privateKeyPath", "errors.privateKeyPathInvalid"))
    }
}

fn validate_id(id: &str) -> Result<(), AppError> {
    Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| validation("id", "errors.resourceIdInvalid"))
}

pub(crate) fn ensure_group_exists(
    connection: &Connection,
    group_id: Option<&str>,
) -> Result<(), AppError> {
    let Some(group_id) = group_id else {
        return Ok(());
    };
    let exists = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM server_groups WHERE id = ?1)",
            [group_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(storage::map_sqlite_error)?;
    if exists {
        Ok(())
    } else {
        Err(not_found("groupId"))
    }
}

fn get_group(connection: &Connection, group_id: &str) -> Result<Group, AppError> {
    connection
        .query_row(
            "SELECT id, name, sort_order, revision, created_at_ms, updated_at_ms
             FROM server_groups WHERE id = ?1",
            [group_id],
            map_group_row,
        )
        .optional()
        .map_err(storage::map_sqlite_error)?
        .ok_or_else(|| not_found("groupId"))
}

fn map_group_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Group> {
    Ok(Group {
        id: row.get(0)?,
        name: row.get(1)?,
        sort_order: row.get(2)?,
        revision: row.get(3)?,
        created_at_ms: row.get(4)?,
        updated_at_ms: row.get(5)?,
    })
}

pub(crate) fn get_server(
    connection: &Connection,
    server_id: &str,
) -> Result<ServerProfile, AppError> {
    let raw = connection
        .query_row(
            "SELECT id, name, host, port, username, auth_type, private_key_path,
                    private_key_path_encoding, group_id, credential_ref_id IS NOT NULL,
                    connect_timeout_ms, keepalive_interval_s, revision, created_at_ms, updated_at_ms
             FROM servers WHERE id = ?1",
            [server_id],
            map_server_row,
        )
        .optional()
        .map_err(storage::map_sqlite_error)?
        .ok_or_else(|| not_found("serverId"))?;
    raw.try_into()
}

type RawServerRow = (
    String,
    String,
    String,
    u16,
    String,
    String,
    Option<Vec<u8>>,
    Option<String>,
    Option<String>,
    bool,
    u32,
    u32,
    u32,
    i64,
    i64,
);

fn map_server_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawServerRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
        row.get(13)?,
        row.get(14)?,
    ))
}

impl TryFrom<RawServerRow> for ServerProfile {
    type Error = AppError;

    fn try_from(raw: RawServerRow) -> Result<Self, Self::Error> {
        let has_private_key = match (raw.6, raw.7) {
            (Some(_), Some(encoding)) => {
                PathEncoding::from_database_value(&encoding)?;
                Some(())
            }
            (None, None) => None,
            _ => return Err(storage_corrupt()),
        }
        .is_some();
        Ok(Self {
            id: raw.0,
            name: raw.1,
            host: raw.2,
            port: raw.3,
            username: raw.4,
            auth_type: AuthType::from_database_value(&raw.5)?,
            has_private_key,
            group_id: raw.8,
            has_saved_credential: raw.9,
            connect_timeout_ms: raw.10,
            keepalive_interval_seconds: raw.11,
            revision: raw.12,
            created_at_ms: raw.13,
            updated_at_ms: raw.14,
        })
    }
}

fn missing_or_conflict(
    connection: &Connection,
    table: &str,
    id: &str,
    expected_revision: u32,
) -> AppError {
    let sql = format!("SELECT revision FROM {table} WHERE id = ?1");
    match connection
        .query_row(&sql, [id], |row| row.get::<_, u32>(0))
        .optional()
    {
        Ok(Some(actual_revision)) => {
            let mut error = AppError::new(ErrorCode::RevisionConflict, "errors.revisionConflict");
            error
                .params
                .insert("expectedRevision".to_owned(), expected_revision.to_string());
            error
                .params
                .insert("actualRevision".to_owned(), actual_revision.to_string());
            error
        }
        Ok(None) => not_found("id"),
        Err(_) => storage_corrupt(),
    }
}

fn validation(field: &'static str, message_key: &'static str) -> AppError {
    let mut error = AppError::new(ErrorCode::ValidationFailed, message_key);
    error.params.insert("field".to_owned(), field.to_owned());
    error
}

fn not_found(field: &'static str) -> AppError {
    let mut error = AppError::new(ErrorCode::ResourceNotFound, "errors.resourceNotFound");
    error.params.insert("field".to_owned(), field.to_owned());
    error
}

fn storage_corrupt() -> AppError {
    let mut error = AppError::new(ErrorCode::Internal, "errors.storageDataInvalid");
    error.stage = Some("storage".to_owned());
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    fn password_server(group_id: Option<String>) -> ServerProfileInput {
        ServerProfileInput {
            name: None,
            host: "server.example.test".to_owned(),
            port: 22,
            username: "deploy".to_owned(),
            auth_type: AuthType::Password,
            private_key_path: None,
            group_id,
            connect_timeout_ms: 15_000,
            keepalive_interval_seconds: 30,
        }
    }

    #[tokio::test]
    async fn group_delete_keeps_server_and_revision_conflicts_are_visible() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("test.sqlite3")).expect("database");
        let store = ProfileStore::new(database);
        let group = store
            .create_group(GroupCreate {
                name: "Production".to_owned(),
                sort_order: 0,
            })
            .await
            .expect("create group");
        let server = store
            .create_server(password_server(Some(group.id.clone())))
            .await
            .expect("create server");

        let conflict = store
            .update_group(
                group.id.clone(),
                GroupUpdate {
                    name: "Prod".to_owned(),
                    sort_order: 1,
                    expected_revision: group.revision + 1,
                },
            )
            .await
            .expect_err("stale revision must fail");
        assert_eq!(conflict.code, ErrorCode::RevisionConflict);

        store
            .delete_group(group.id, group.revision)
            .await
            .expect("delete group");
        let stored = store.get_server(server.id).await.expect("server remains");
        assert_eq!(stored.group_id, None);
    }

    #[tokio::test]
    async fn persists_and_searches_servers_after_reopen() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("test.sqlite3");
        let server_id = {
            let database = Database::open(&path).expect("database");
            let store = ProfileStore::new(database);
            store
                .create_server(password_server(None))
                .await
                .expect("create server")
                .id
        };

        let database = Database::open(&path).expect("reopen database");
        let store = ProfileStore::new(database);
        let results = store
            .list_servers(ServerListQuery {
                query: Some("EXAMPLE".to_owned()),
                ..ServerListQuery::default()
            })
            .await
            .expect("search servers");
        assert_eq!(results.items.len(), 1);
        assert_eq!(results.items[0].id, server_id);

        let wildcard_results = store
            .list_servers(ServerListQuery {
                query: Some("%".to_owned()),
                ..ServerListQuery::default()
            })
            .await
            .expect("search literal wildcard");
        assert!(wildcard_results.items.is_empty());
    }

    #[tokio::test]
    async fn list_cursor_is_bound_to_query_and_list_revision() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("test.sqlite3")).expect("database");
        let store = ProfileStore::new(database);
        store
            .create_server(password_server(None))
            .await
            .expect("create first server");
        let mut second = password_server(None);
        second.host = "second.example.test".to_owned();
        store
            .create_server(second)
            .await
            .expect("create second server");

        let first_page = store
            .list_servers(ServerListQuery {
                limit: 1,
                ..ServerListQuery::default()
            })
            .await
            .expect("first page");
        let cursor = first_page.next_cursor.expect("next cursor");
        let second_page = store
            .list_servers(ServerListQuery {
                limit: 1,
                cursor: Some(cursor.clone()),
                ..ServerListQuery::default()
            })
            .await
            .expect("second page");
        assert_eq!(second_page.items.len(), 1);
        assert_ne!(first_page.items[0].id, second_page.items[0].id);

        let mismatch = store
            .list_servers(ServerListQuery {
                query: Some("example".to_owned()),
                limit: 1,
                cursor: Some(cursor.clone()),
                ..ServerListQuery::default()
            })
            .await
            .expect_err("cursor query mismatch must fail");
        assert_eq!(mismatch.code, ErrorCode::ValidationFailed);

        let mut third = password_server(None);
        third.host = "third.example.test".to_owned();
        store
            .create_server(third)
            .await
            .expect("mutate profile list");
        let expired = store
            .list_servers(ServerListQuery {
                limit: 1,
                cursor: Some(cursor),
                ..ServerListQuery::default()
            })
            .await
            .expect_err("stale cursor must fail");
        assert_eq!(expired.code, ErrorCode::RevisionConflict);
        assert_eq!(expired.action, crate::ErrorAction::Reload);
    }

    #[test]
    fn validates_server_fields_and_private_key_consistency() {
        let mut input = password_server(None);
        input.host = "ssh://example.test".to_owned();
        assert_eq!(
            validate_server_input(input)
                .expect_err("URL host must fail")
                .code,
            ErrorCode::ValidationFailed
        );

        let mut input = password_server(None);
        input.auth_type = AuthType::PrivateKey;
        assert_eq!(
            validate_server_input(input)
                .expect_err("private key auth requires a path")
                .code,
            ErrorCode::ValidationFailed
        );

        let mut input = password_server(None);
        input.port = 0;
        assert_eq!(
            validate_server_input(input)
                .expect_err("zero port must fail")
                .code,
            ErrorCode::ValidationFailed
        );
    }
}
