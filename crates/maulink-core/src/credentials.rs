use std::{
    fmt,
    sync::{Arc, Mutex, mpsc},
    thread,
};

use keyring_core::Entry;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use ts_rs::TS;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use crate::{
    AppError, AuthType, Database, ErrorCode, ProfileStore, ServerProfile, ServerProfileInput,
    profiles, storage,
};

const CREDENTIAL_QUEUE_CAPACITY: usize = 16;
const DEFAULT_SERVICE: &str = "io.maulink.desktop.credentials";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CredentialKind {
    Password,
    Passphrase,
}

impl CredentialKind {
    fn as_database_value(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Passphrase => "passphrase",
        }
    }

    fn from_database_value(value: &str) -> Result<Self, AppError> {
        match value {
            "password" => Ok(Self::Password),
            "passphrase" => Ok(Self::Passphrase),
            _ => Err(storage_corrupt()),
        }
    }
}

#[derive(TS)]
#[ts(type = "string")]
pub struct Secret(Vec<u8>);

impl Secret {
    pub fn new(bytes: Vec<u8>) -> Result<Self, AppError> {
        if bytes.is_empty() || bytes.len() > 64 * 1024 {
            return Err(validation("secret", "errors.credentialSecretInvalid"));
        }
        Ok(Self(bytes))
    }

    fn expose(&self) -> &[u8] {
        &self.0
    }

    pub(crate) fn into_utf8_string(mut self) -> Result<Zeroizing<String>, AppError> {
        let value = std::str::from_utf8(&self.0)
            .map(str::to_owned)
            .map_err(|_| validation("secret", "errors.credentialSecretInvalid"));
        self.0.zeroize();
        value.map(Zeroizing::new)
    }

    #[cfg(test)]
    fn to_vec(&self) -> Vec<u8> {
        self.0.clone()
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value.into_bytes()).map_err(serde::de::Error::custom)
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret([REDACTED])")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureStoreError {
    AccessDenied,
    NotFound,
    Invalid,
    Platform,
}

pub trait SecureStore: Send + Sync + 'static {
    fn save(&self, credential_id: &str, secret: &Secret) -> Result<(), SecureStoreError>;
    fn load(&self, credential_id: &str) -> Result<Secret, SecureStoreError>;
    fn delete(&self, credential_id: &str) -> Result<(), SecureStoreError>;
}

pub struct NativeSecureStore {
    store: Arc<keyring_core::CredentialStore>,
    service: String,
}

impl NativeSecureStore {
    pub fn new() -> Result<Self, AppError> {
        #[cfg(target_os = "macos")]
        let store = apple_native_keyring_store::keychain::Store::new()
            .map_err(map_keyring_initialization_error)?;

        #[cfg(windows)]
        let store = windows_native_keyring_store::store::Store::new()
            .map_err(map_keyring_initialization_error)?;

        #[cfg(not(any(target_os = "macos", windows)))]
        return Err(AppError::new(
            ErrorCode::CredentialAccessDenied,
            "errors.credentialPlatformUnsupported",
        ));

        Ok(Self {
            store,
            service: DEFAULT_SERVICE.to_owned(),
        })
    }

    fn entry(&self, credential_id: &str) -> Result<Entry, SecureStoreError> {
        self.store
            .build(&self.service, credential_id, None)
            .map_err(map_keyring_error)
    }
}

impl SecureStore for NativeSecureStore {
    fn save(&self, credential_id: &str, secret: &Secret) -> Result<(), SecureStoreError> {
        self.entry(credential_id)?
            .set_secret(secret.expose())
            .map_err(map_keyring_error)
    }

    fn load(&self, credential_id: &str) -> Result<Secret, SecureStoreError> {
        self.entry(credential_id)?
            .get_secret()
            .map(Secret)
            .map_err(map_keyring_error)
    }

    fn delete(&self, credential_id: &str) -> Result<(), SecureStoreError> {
        self.entry(credential_id)?
            .delete_credential()
            .map_err(map_keyring_error)
    }
}

fn map_keyring_initialization_error(error: keyring_core::Error) -> AppError {
    map_secure_store_error(map_keyring_error(error))
}

fn map_keyring_error(error: keyring_core::Error) -> SecureStoreError {
    match error {
        keyring_core::Error::NoStorageAccess(_) => SecureStoreError::AccessDenied,
        keyring_core::Error::PlatformFailure(error)
            if is_platform_access_denied(error.as_ref()) =>
        {
            SecureStoreError::AccessDenied
        }
        keyring_core::Error::NoEntry => SecureStoreError::NotFound,
        keyring_core::Error::Invalid(_, _)
        | keyring_core::Error::TooLong(_, _)
        | keyring_core::Error::BadEncoding(_)
        | keyring_core::Error::BadDataFormat(_, _)
        | keyring_core::Error::BadStoreFormat(_)
        | keyring_core::Error::Ambiguous(_)
        | keyring_core::Error::NotSupportedByStore(_) => SecureStoreError::Invalid,
        _ => SecureStoreError::Platform,
    }
}

#[cfg(target_os = "macos")]
fn is_platform_access_denied(error: &(dyn std::error::Error + Send + Sync + 'static)) -> bool {
    error
        .downcast_ref::<security_framework::base::Error>()
        .is_some_and(|error| matches!(error.code(), -60_008 | -25_308 | -25_293))
}

#[cfg(not(target_os = "macos"))]
fn is_platform_access_denied(_: &(dyn std::error::Error + Send + Sync + 'static)) -> bool {
    false
}

type StoreJob = Box<dyn FnOnce(&dyn SecureStore) + Send + 'static>;

enum StoreMessage {
    Execute(StoreJob),
    Shutdown,
}

struct WorkerInner {
    sender: mpsc::SyncSender<StoreMessage>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}

impl Drop for WorkerInner {
    fn drop(&mut self) {
        let _ = self.sender.send(StoreMessage::Shutdown);
        if let Some(worker) = self
            .worker
            .lock()
            .expect("credential worker lock poisoned")
            .take()
        {
            let _ = worker.join();
        }
    }
}

#[derive(Clone)]
pub struct CredentialWorker {
    inner: Arc<WorkerInner>,
}

impl CredentialWorker {
    pub fn new(store: Arc<dyn SecureStore>) -> Result<Self, AppError> {
        let (sender, receiver) = mpsc::sync_channel(CREDENTIAL_QUEUE_CAPACITY);
        let worker = thread::Builder::new()
            .name("maulink-credentials".to_owned())
            .spawn(move || {
                while let Ok(message) = receiver.recv() {
                    match message {
                        StoreMessage::Execute(job) => job(store.as_ref()),
                        StoreMessage::Shutdown => break,
                    }
                }
            })
            .map_err(|_| internal("errors.credentialWorkerStartFailed"))?;
        Ok(Self {
            inner: Arc::new(WorkerInner {
                sender,
                worker: Mutex::new(Some(worker)),
            }),
        })
    }

    pub fn native() -> Result<Self, AppError> {
        Self::new(Arc::new(NativeSecureStore::new()?))
    }

    async fn execute<T, F>(&self, operation: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce(&dyn SecureStore) -> Result<T, SecureStoreError> + Send + 'static,
    {
        let (result_sender, result_receiver) = oneshot::channel();
        let job = Box::new(move |store: &dyn SecureStore| {
            let _ = result_sender.send(operation(store));
        });
        self.inner
            .sender
            .try_send(StoreMessage::Execute(job))
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => {
                    AppError::new(ErrorCode::ResourceLimit, "errors.credentialQueueFull")
                        .with_retry()
                }
                mpsc::TrySendError::Disconnected(_) => internal("errors.credentialWorkerStopped"),
            })?;
        result_receiver
            .await
            .map_err(|_| internal("errors.credentialWorkerStopped"))?
            .map_err(map_secure_store_error)
    }

    async fn save(&self, credential_id: String, secret: Secret) -> Result<(), AppError> {
        self.execute(move |store| store.save(&credential_id, &secret))
            .await
    }

    async fn load(&self, credential_id: String) -> Result<Secret, AppError> {
        self.execute(move |store| store.load(&credential_id)).await
    }

    async fn delete(&self, credential_id: String) -> Result<(), AppError> {
        self.execute(move |store| match store.delete(&credential_id) {
            Err(SecureStoreError::NotFound) => Ok(()),
            result => result,
        })
        .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CredentialReplaceResult {
    pub credential_ref_id: String,
    pub cleanup_pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CredentialDeleteResult {
    pub credential_cleanup_pending: bool,
}

#[derive(Debug, Deserialize, TS)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum CredentialUpdate {
    Keep,
    Clear,
    Replace {
        #[ts(type = "string")]
        secret: Secret,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerMutationResult {
    pub server: ServerProfile,
    pub credential_cleanup_pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RetainedCredential {
    pub credential_ref_id: String,
    pub owner_server_id: String,
    pub kind: CredentialKind,
    #[ts(type = "number")]
    pub created_at_ms: i64,
    #[ts(type = "number")]
    pub updated_at_ms: i64,
}

#[derive(Clone)]
pub struct CredentialManager {
    database: Database,
    worker: CredentialWorker,
}

impl CredentialManager {
    pub fn new(database: Database, worker: CredentialWorker) -> Self {
        Self { database, worker }
    }

    pub async fn replace_for_server(
        &self,
        server_id: String,
        kind: CredentialKind,
        secret: Secret,
    ) -> Result<CredentialReplaceResult, AppError> {
        validate_uuid(&server_id, "serverId")?;
        let credential_id = Uuid::new_v4().to_string();
        let now = storage::now_ms()?;
        register_pending_write(
            &self.database,
            credential_id.clone(),
            server_id.clone(),
            kind,
            now,
        )
        .await?;

        if let Err(error) = self.worker.save(credential_id.clone(), secret).await {
            let _ = delete_credential_metadata(&self.database, credential_id).await;
            return Err(error);
        }

        let old_credential =
            match activate_credential(&self.database, credential_id.clone(), server_id, kind, now)
                .await
            {
                Ok(old) => old,
                Err(error) => {
                    if self.worker.delete(credential_id.clone()).await.is_ok() {
                        let _ = delete_credential_metadata(&self.database, credential_id).await;
                    }
                    return Err(error);
                }
            };

        let cleanup_pending = if let Some(old_id) = old_credential {
            if self.worker.delete(old_id.clone()).await.is_ok() {
                delete_credential_metadata(&self.database, old_id).await?;
                false
            } else {
                true
            }
        } else {
            false
        };
        Ok(CredentialReplaceResult {
            credential_ref_id: credential_id,
            cleanup_pending,
        })
    }

    pub async fn create_server(
        &self,
        input: ServerProfileInput,
        credential: CredentialUpdate,
    ) -> Result<ServerMutationResult, AppError> {
        if matches!(&credential, CredentialUpdate::Keep) {
            return Err(validation(
                "credential",
                "errors.credentialKeepInvalidForCreate",
            ));
        }
        let profile_store = ProfileStore::new(self.database.clone());
        let server = profile_store.create_server(input).await?;
        match credential {
            CredentialUpdate::Keep => unreachable!("handled above"),
            CredentialUpdate::Clear => Ok(ServerMutationResult {
                server,
                credential_cleanup_pending: false,
            }),
            CredentialUpdate::Replace { secret } => {
                let kind = credential_kind_for_auth(server.auth_type);
                match self
                    .replace_for_server(server.id.clone(), kind, secret)
                    .await
                {
                    Ok(result) => Ok(ServerMutationResult {
                        server: profile_store.get_server(server.id).await?,
                        credential_cleanup_pending: result.cleanup_pending,
                    }),
                    Err(error) => {
                        let _ = profile_store
                            .delete_server(server.id, server.revision)
                            .await;
                        Err(error)
                    }
                }
            }
        }
    }

    pub async fn update_server(
        &self,
        server_id: String,
        expected_revision: u32,
        input: ServerProfileInput,
        credential: CredentialUpdate,
    ) -> Result<ServerMutationResult, AppError> {
        validate_uuid(&server_id, "serverId")?;
        let input = profiles::validate_server_input(input)?;
        let now = storage::now_ms()?;
        let mode = match &credential {
            CredentialUpdate::Keep => UpdateCredentialMode::Keep,
            CredentialUpdate::Clear => UpdateCredentialMode::Clear,
            CredentialUpdate::Replace { .. } => UpdateCredentialMode::Replace,
        };
        let pending = match credential {
            CredentialUpdate::Replace { secret } => {
                let credential_id = Uuid::new_v4().to_string();
                let kind = credential_kind_for_auth(input.auth_type);
                register_pending_write(
                    &self.database,
                    credential_id.clone(),
                    server_id.clone(),
                    kind,
                    now,
                )
                .await?;
                if let Err(error) = self.worker.save(credential_id.clone(), secret).await {
                    let _ = delete_credential_metadata(&self.database, credential_id).await;
                    return Err(error);
                }
                Some((credential_id, kind))
            }
            CredentialUpdate::Keep | CredentialUpdate::Clear => None,
        };
        let outcome = update_server_transaction(
            &self.database,
            server_id,
            expected_revision,
            input,
            mode,
            pending.clone(),
            now,
        )
        .await;
        let (server, old_credential) = match outcome {
            Ok(value) => value,
            Err(error) => {
                if let Some((credential_id, _)) = pending
                    && self.worker.delete(credential_id.clone()).await.is_ok()
                {
                    let _ = delete_credential_metadata(&self.database, credential_id).await;
                }
                return Err(error);
            }
        };
        let credential_cleanup_pending = self.try_delete_registered(old_credential).await?;
        Ok(ServerMutationResult {
            server,
            credential_cleanup_pending,
        })
    }

    pub async fn load_for_authentication(&self, server_id: String) -> Result<Secret, AppError> {
        validate_uuid(&server_id, "serverId")?;
        let credential_id = active_credential_for_server(&self.database, server_id).await?;
        self.worker.load(credential_id).await
    }

    pub async fn delete_server(
        &self,
        server_id: String,
        expected_revision: u32,
        remove_credentials: bool,
    ) -> Result<CredentialDeleteResult, AppError> {
        validate_uuid(&server_id, "serverId")?;
        let now = storage::now_ms()?;
        let credential_id = prepare_server_delete(
            &self.database,
            server_id,
            expected_revision,
            remove_credentials,
            now,
        )
        .await?;
        let cleanup_pending = if remove_credentials {
            self.try_delete_registered(credential_id).await?
        } else {
            false
        };
        Ok(CredentialDeleteResult {
            credential_cleanup_pending: cleanup_pending,
        })
    }

    pub async fn list_retained(&self) -> Result<Vec<RetainedCredential>, AppError> {
        self.database
            .execute(|connection| {
                let mut statement = connection
                    .prepare(
                        "SELECT id, owner_server_id, kind, created_at_ms, updated_at_ms
                         FROM credential_refs WHERE state = 'retained'
                         ORDER BY updated_at_ms DESC, id",
                    )
                    .map_err(storage::map_sqlite_error)?;
                let rows = statement
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, i64>(4)?,
                        ))
                    })
                    .map_err(storage::map_sqlite_error)?;
                rows.map(|row| {
                    let (credential_ref_id, owner_server_id, kind, created_at_ms, updated_at_ms) =
                        row.map_err(storage::map_sqlite_error)?;
                    Ok(RetainedCredential {
                        credential_ref_id,
                        owner_server_id,
                        kind: CredentialKind::from_database_value(&kind)?,
                        created_at_ms,
                        updated_at_ms,
                    })
                })
                .collect()
            })
            .await
    }

    pub async fn delete_retained(
        &self,
        credential_id: String,
    ) -> Result<CredentialDeleteResult, AppError> {
        validate_uuid(&credential_id, "credentialRefId")?;
        mark_retained_for_delete(&self.database, credential_id.clone(), storage::now_ms()?).await?;
        let cleanup_pending = self.try_delete_registered(Some(credential_id)).await?;
        Ok(CredentialDeleteResult {
            credential_cleanup_pending: cleanup_pending,
        })
    }

    pub async fn retry_cleanup(
        &self,
        credential_id: String,
    ) -> Result<CredentialDeleteResult, AppError> {
        validate_uuid(&credential_id, "credentialRefId")?;
        ensure_pending_cleanup(&self.database, credential_id.clone()).await?;
        self.worker.delete(credential_id.clone()).await?;
        delete_credential_metadata(&self.database, credential_id).await?;
        Ok(CredentialDeleteResult {
            credential_cleanup_pending: false,
        })
    }

    pub async fn recover_pending(&self) -> Result<usize, AppError> {
        let pending = pending_cleanup_ids(&self.database).await?;
        let mut cleaned = 0;
        for credential_id in pending {
            self.worker.delete(credential_id.clone()).await?;
            delete_credential_metadata(&self.database, credential_id).await?;
            cleaned += 1;
        }
        Ok(cleaned)
    }

    async fn try_delete_registered(&self, credential_id: Option<String>) -> Result<bool, AppError> {
        let Some(credential_id) = credential_id else {
            return Ok(false);
        };
        if self.worker.delete(credential_id.clone()).await.is_err() {
            return Ok(true);
        }
        delete_credential_metadata(&self.database, credential_id).await?;
        Ok(false)
    }
}

#[derive(Clone, Copy)]
enum UpdateCredentialMode {
    Keep,
    Clear,
    Replace,
}

fn credential_kind_for_auth(auth_type: AuthType) -> CredentialKind {
    match auth_type {
        AuthType::Password => CredentialKind::Password,
        AuthType::PrivateKey => CredentialKind::Passphrase,
    }
}

async fn update_server_transaction(
    database: &Database,
    server_id: String,
    expected_revision: u32,
    input: ServerProfileInput,
    mode: UpdateCredentialMode,
    pending: Option<(String, CredentialKind)>,
    now: i64,
) -> Result<(ServerProfile, Option<String>), AppError> {
    database
        .execute(move |connection| {
            let transaction = connection
                .transaction()
                .map_err(storage::map_sqlite_error)?;
            profiles::ensure_group_exists(&transaction, input.group_id.as_deref())?;
            let current = transaction
                .query_row(
                    "SELECT revision, host, port, username, auth_type, private_key_path,
                            private_key_path_encoding, credential_ref_id
                     FROM servers WHERE id = ?1",
                    [&server_id],
                    |row| {
                        Ok((
                            row.get::<_, u32>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, u16>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, Option<Vec<u8>>>(5)?,
                            row.get::<_, Option<String>>(6)?,
                            row.get::<_, Option<String>>(7)?,
                        ))
                    },
                )
                .optional()
                .map_err(storage::map_sqlite_error)?
                .ok_or_else(|| {
                    AppError::new(ErrorCode::ResourceNotFound, "errors.serverNotFound")
                })?;
            if current.0 != expected_revision {
                return Err(revision_conflict(expected_revision, current.0));
            }

            let input_path = input.private_key_path.as_ref();
            let identity_changed = current.1 != input.host
                || current.2 != input.port
                || current.3 != input.username
                || current.4 != input.auth_type.as_database_value()
                || current.5.as_deref() != input_path.map(|path| path.bytes.as_slice())
                || current.6.as_deref() != input_path.map(|path| path.encoding.as_database_value());
            if matches!(mode, UpdateCredentialMode::Keep) && current.7.is_some() && identity_changed
            {
                return Err(validation(
                    "credential",
                    "errors.credentialExplicitUpdateRequired",
                ));
            }

            let old_credential = current.7;
            let new_credential = match mode {
                UpdateCredentialMode::Keep => old_credential.clone(),
                UpdateCredentialMode::Clear => None,
                UpdateCredentialMode::Replace => {
                    let (credential_id, kind) = pending
                        .as_ref()
                        .ok_or_else(|| internal("errors.credentialStateInvalid"))?;
                    if *kind != credential_kind_for_auth(input.auth_type) {
                        return Err(validation("credential", "errors.credentialKindMismatch"));
                    }
                    let changed = transaction
                        .execute(
                            "UPDATE credential_refs SET state = 'active', updated_at_ms = ?2
                             WHERE id = ?1 AND state = 'pending_write'",
                            params![credential_id, now],
                        )
                        .map_err(storage::map_sqlite_error)?;
                    if changed != 1 {
                        return Err(internal("errors.credentialStateInvalid"));
                    }
                    Some(credential_id.clone())
                }
            };
            let credential_to_delete = match mode {
                UpdateCredentialMode::Keep => None,
                UpdateCredentialMode::Clear | UpdateCredentialMode::Replace => {
                    if let Some(old_id) = &old_credential {
                        transaction
                            .execute(
                                "UPDATE credential_refs
                                 SET state = 'pending_delete', updated_at_ms = ?2 WHERE id = ?1",
                                params![old_id, now],
                            )
                            .map_err(storage::map_sqlite_error)?;
                    }
                    old_credential
                }
            };
            transaction
                .execute(
                    "UPDATE servers
                     SET name = ?1, host = ?2, port = ?3, username = ?4, auth_type = ?5,
                         private_key_path = ?6, private_key_path_encoding = ?7, group_id = ?8,
                         connect_timeout_ms = ?9, keepalive_interval_s = ?10,
                         credential_ref_id = ?11, revision = revision + 1, updated_at_ms = ?12
                     WHERE id = ?13 AND revision = ?14",
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
                        new_credential,
                        now,
                        server_id,
                        expected_revision,
                    ],
                )
                .map_err(storage::map_sqlite_error)?;
            bump_profile_list_revision(&transaction)?;
            transaction.commit().map_err(storage::map_sqlite_error)?;
            Ok((
                profiles::get_server(connection, &server_id)?,
                credential_to_delete,
            ))
        })
        .await
}

async fn prepare_server_delete(
    database: &Database,
    server_id: String,
    expected_revision: u32,
    remove_credentials: bool,
    now: i64,
) -> Result<Option<String>, AppError> {
    database
        .execute(move |connection| {
            let transaction = connection
                .transaction()
                .map_err(storage::map_sqlite_error)?;
            let server = transaction
                .query_row(
                    "SELECT revision, credential_ref_id FROM servers WHERE id = ?1",
                    [&server_id],
                    |row| Ok((row.get::<_, u32>(0)?, row.get::<_, Option<String>>(1)?)),
                )
                .optional()
                .map_err(storage::map_sqlite_error)?
                .ok_or_else(|| {
                    AppError::new(ErrorCode::ResourceNotFound, "errors.serverNotFound")
                })?;
            if server.0 != expected_revision {
                return Err(revision_conflict(expected_revision, server.0));
            }
            if let Some(credential_id) = &server.1 {
                let state = if remove_credentials {
                    "pending_delete"
                } else {
                    "retained"
                };
                let changed = transaction
                    .execute(
                        "UPDATE credential_refs SET state = ?2, updated_at_ms = ?3
                         WHERE id = ?1 AND state = 'active'",
                        params![credential_id, state, now],
                    )
                    .map_err(storage::map_sqlite_error)?;
                if changed != 1 {
                    return Err(storage_corrupt());
                }
            }
            transaction
                .execute(
                    "DELETE FROM servers WHERE id = ?1 AND revision = ?2",
                    params![server_id, expected_revision],
                )
                .map_err(storage::map_sqlite_error)?;
            bump_profile_list_revision(&transaction)?;
            transaction.commit().map_err(storage::map_sqlite_error)?;
            Ok(server.1)
        })
        .await
}

async fn mark_retained_for_delete(
    database: &Database,
    credential_id: String,
    now: i64,
) -> Result<(), AppError> {
    database
        .execute(move |connection| {
            let changed = connection
                .execute(
                    "UPDATE credential_refs SET state = 'pending_delete', updated_at_ms = ?2
                     WHERE id = ?1 AND state = 'retained'",
                    params![credential_id, now],
                )
                .map_err(storage::map_sqlite_error)?;
            if changed == 1 {
                Ok(())
            } else {
                Err(AppError::new(
                    ErrorCode::CredentialNotFound,
                    "errors.retainedCredentialNotFound",
                ))
            }
        })
        .await
}

async fn ensure_pending_cleanup(
    database: &Database,
    credential_id: String,
) -> Result<(), AppError> {
    database
        .execute(move |connection| {
            let exists = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM credential_refs
                     WHERE id = ?1 AND state IN ('pending_write', 'pending_delete'))",
                    [credential_id],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(storage::map_sqlite_error)?;
            if exists {
                Ok(())
            } else {
                Err(AppError::new(
                    ErrorCode::CredentialNotFound,
                    "errors.pendingCredentialNotFound",
                ))
            }
        })
        .await
}

fn bump_profile_list_revision(connection: &rusqlite::Connection) -> Result<(), AppError> {
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

fn revision_conflict(expected_revision: u32, actual_revision: u32) -> AppError {
    AppError::new(ErrorCode::RevisionConflict, "errors.revisionConflict")
        .with_param("expectedRevision", expected_revision.to_string())
        .with_param("actualRevision", actual_revision.to_string())
}

async fn register_pending_write(
    database: &Database,
    credential_id: String,
    server_id: String,
    kind: CredentialKind,
    now: i64,
) -> Result<(), AppError> {
    database
        .execute(move |connection| {
            let server_exists = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM servers WHERE id = ?1)",
                    [&server_id],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(storage::map_sqlite_error)?;
            if !server_exists {
                return Err(AppError::new(
                    ErrorCode::ResourceNotFound,
                    "errors.serverNotFound",
                ));
            }
            connection
                .execute(
                    "INSERT INTO credential_refs
                     (id, owner_server_id, kind, state, created_at_ms, updated_at_ms)
                     VALUES (?1, ?2, ?3, 'pending_write', ?4, ?4)",
                    params![credential_id, server_id, kind.as_database_value(), now],
                )
                .map_err(storage::map_sqlite_error)?;
            Ok(())
        })
        .await
}

async fn activate_credential(
    database: &Database,
    credential_id: String,
    server_id: String,
    kind: CredentialKind,
    now: i64,
) -> Result<Option<String>, AppError> {
    database
        .execute(move |connection| {
            let transaction = connection
                .transaction()
                .map_err(storage::map_sqlite_error)?;
            let old_id = transaction
                .query_row(
                    "SELECT credential_ref_id FROM servers WHERE id = ?1",
                    [&server_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .optional()
                .map_err(storage::map_sqlite_error)?
                .ok_or_else(|| {
                    AppError::new(ErrorCode::ResourceNotFound, "errors.serverNotFound")
                })?;
            let expected_kind = match kind {
                CredentialKind::Password => "password",
                CredentialKind::Passphrase => "private_key",
            };
            let auth_matches = transaction
                .query_row(
                    "SELECT auth_type = ?2 FROM servers WHERE id = ?1",
                    params![server_id, expected_kind],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(storage::map_sqlite_error)?;
            if !auth_matches {
                return Err(validation("credential", "errors.credentialKindMismatch"));
            }
            let changed = transaction
                .execute(
                    "UPDATE credential_refs SET state = 'active', updated_at_ms = ?2
                     WHERE id = ?1 AND state = 'pending_write'",
                    params![credential_id, now],
                )
                .map_err(storage::map_sqlite_error)?;
            if changed != 1 {
                return Err(internal("errors.credentialStateInvalid"));
            }
            transaction
                .execute(
                    "UPDATE servers
                     SET credential_ref_id = ?2, revision = revision + 1, updated_at_ms = ?3
                     WHERE id = ?1",
                    params![server_id, credential_id, now],
                )
                .map_err(storage::map_sqlite_error)?;
            if let Some(old_id) = &old_id {
                transaction
                    .execute(
                        "UPDATE credential_refs SET state = 'pending_delete', updated_at_ms = ?2
                         WHERE id = ?1",
                        params![old_id, now],
                    )
                    .map_err(storage::map_sqlite_error)?;
            }
            bump_profile_list_revision(&transaction)?;
            transaction.commit().map_err(storage::map_sqlite_error)?;
            Ok(old_id)
        })
        .await
}

async fn active_credential_for_server(
    database: &Database,
    server_id: String,
) -> Result<String, AppError> {
    database
        .execute(move |connection| {
            connection
                .query_row(
                    "SELECT c.id FROM servers s
                     JOIN credential_refs c ON c.id = s.credential_ref_id
                     WHERE s.id = ?1 AND c.state = 'active'",
                    [server_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage::map_sqlite_error)?
                .ok_or_else(|| {
                    AppError::new(ErrorCode::CredentialNotFound, "errors.credentialNotFound")
                })
        })
        .await
}

async fn pending_cleanup_ids(database: &Database) -> Result<Vec<String>, AppError> {
    database
        .execute(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id FROM credential_refs
                     WHERE state IN ('pending_write', 'pending_delete')
                     ORDER BY created_at_ms LIMIT 16",
                )
                .map_err(storage::map_sqlite_error)?;
            statement
                .query_map([], |row| row.get(0))
                .map_err(storage::map_sqlite_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage::map_sqlite_error)
        })
        .await
}

async fn delete_credential_metadata(
    database: &Database,
    credential_id: String,
) -> Result<(), AppError> {
    database
        .execute(move |connection| {
            connection
                .execute(
                    "DELETE FROM credential_refs WHERE id = ?1
                     AND state IN ('pending_write', 'pending_delete')",
                    [credential_id],
                )
                .map_err(storage::map_sqlite_error)?;
            Ok(())
        })
        .await
}

fn validate_uuid(value: &str, field: &'static str) -> Result<(), AppError> {
    Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| validation(field, "errors.resourceIdInvalid"))
}

fn map_secure_store_error(error: SecureStoreError) -> AppError {
    match error {
        SecureStoreError::AccessDenied => AppError::new(
            ErrorCode::CredentialAccessDenied,
            "errors.credentialAccessDenied",
        ),
        SecureStoreError::NotFound => {
            AppError::new(ErrorCode::CredentialNotFound, "errors.credentialNotFound")
        }
        SecureStoreError::Invalid => AppError::new(
            ErrorCode::ValidationFailed,
            "errors.credentialStoreRejected",
        ),
        SecureStoreError::Platform => internal("errors.credentialStoreFailed"),
    }
    .with_stage("credential")
}

fn validation(field: &'static str, message_key: &'static str) -> AppError {
    AppError::new(ErrorCode::ValidationFailed, message_key).with_param("field", field)
}

fn internal(message_key: &'static str) -> AppError {
    AppError::new(ErrorCode::Internal, message_key).with_stage("credential")
}

fn storage_corrupt() -> AppError {
    AppError::new(ErrorCode::Internal, "errors.storageDataInvalid").with_stage("storage")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthType, ProfileStore, ServerProfileInput};
    use std::collections::HashMap;

    #[derive(Default)]
    struct MockSecureStore {
        values: Mutex<HashMap<String, Vec<u8>>>,
        fail_save: Mutex<bool>,
        fail_delete: Mutex<bool>,
    }

    impl SecureStore for MockSecureStore {
        fn save(&self, credential_id: &str, secret: &Secret) -> Result<(), SecureStoreError> {
            if *self.fail_save.lock().expect("mock save failure lock") {
                return Err(SecureStoreError::AccessDenied);
            }
            self.values
                .lock()
                .expect("mock values lock")
                .insert(credential_id.to_owned(), secret.to_vec());
            Ok(())
        }

        fn load(&self, credential_id: &str) -> Result<Secret, SecureStoreError> {
            self.values
                .lock()
                .expect("mock values lock")
                .get(credential_id)
                .cloned()
                .map(Secret)
                .ok_or(SecureStoreError::NotFound)
        }

        fn delete(&self, credential_id: &str) -> Result<(), SecureStoreError> {
            if *self.fail_delete.lock().expect("mock failure lock") {
                return Err(SecureStoreError::AccessDenied);
            }
            self.values
                .lock()
                .expect("mock values lock")
                .remove(credential_id)
                .map(|_| ())
                .ok_or(SecureStoreError::NotFound)
        }
    }

    fn password_input() -> ServerProfileInput {
        ServerProfileInput {
            name: None,
            host: "server.example.test".to_owned(),
            port: 22,
            username: "deploy".to_owned(),
            auth_type: AuthType::Password,
            private_key_path: None,
            group_id: None,
            connect_timeout_ms: 15_000,
            keepalive_interval_seconds: 30,
        }
    }

    async fn password_server(database: &Database) -> String {
        ProfileStore::new(database.clone())
            .create_server(password_input())
            .await
            .expect("create server")
            .id
    }

    #[tokio::test]
    async fn replacement_preserves_new_credential_and_tracks_failed_old_cleanup() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("test.sqlite3")).expect("database");
        let server_id = password_server(&database).await;
        let store = Arc::new(MockSecureStore::default());
        let worker = CredentialWorker::new(store.clone()).expect("credential worker");
        let manager = CredentialManager::new(database, worker);

        let first = manager
            .replace_for_server(
                server_id.clone(),
                CredentialKind::Password,
                Secret::new(b"first".to_vec()).expect("secret"),
            )
            .await
            .expect("save first credential");
        assert!(!first.cleanup_pending);
        *store.fail_delete.lock().expect("mock failure lock") = true;
        let second = manager
            .replace_for_server(
                server_id.clone(),
                CredentialKind::Password,
                Secret::new(b"second".to_vec()).expect("secret"),
            )
            .await
            .expect("replace credential");
        assert!(second.cleanup_pending);
        assert_eq!(
            manager
                .load_for_authentication(server_id)
                .await
                .expect("load active credential")
                .to_vec(),
            b"second"
        );
        assert_ne!(first.credential_ref_id, second.credential_ref_id);
    }

    #[tokio::test]
    async fn server_delete_can_retain_then_explicitly_remove_credential() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("test.sqlite3")).expect("database");
        let server_id = password_server(&database).await;
        let store = Arc::new(MockSecureStore::default());
        let worker = CredentialWorker::new(store.clone()).expect("credential worker");
        let manager = CredentialManager::new(database.clone(), worker);
        let saved = manager
            .replace_for_server(
                server_id.clone(),
                CredentialKind::Password,
                Secret::new(b"retained".to_vec()).expect("secret"),
            )
            .await
            .expect("save credential");

        let deleted = manager
            .delete_server(server_id.clone(), 2, false)
            .await
            .expect("delete server and retain credential");
        assert!(!deleted.credential_cleanup_pending);
        assert!(
            ProfileStore::new(database)
                .get_server(server_id)
                .await
                .is_err()
        );
        let retained = manager.list_retained().await.expect("list retained");
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].credential_ref_id, saved.credential_ref_id);

        let cleanup = manager
            .delete_retained(saved.credential_ref_id.clone())
            .await
            .expect("delete retained credential");
        assert!(!cleanup.credential_cleanup_pending);
        assert!(
            manager
                .list_retained()
                .await
                .expect("list retained")
                .is_empty()
        );
        assert!(
            !store
                .values
                .lock()
                .expect("mock values lock")
                .contains_key(&saved.credential_ref_id)
        );
    }

    #[tokio::test]
    async fn server_delete_tracks_failed_credential_cleanup() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("test.sqlite3")).expect("database");
        let server_id = password_server(&database).await;
        let store = Arc::new(MockSecureStore::default());
        let worker = CredentialWorker::new(store.clone()).expect("credential worker");
        let manager = CredentialManager::new(database, worker);
        let saved = manager
            .replace_for_server(
                server_id.clone(),
                CredentialKind::Password,
                Secret::new(b"pending-delete".to_vec()).expect("secret"),
            )
            .await
            .expect("save credential");
        *store.fail_delete.lock().expect("mock failure lock") = true;

        let deleted = manager
            .delete_server(server_id, 2, true)
            .await
            .expect("profile deletion succeeds with cleanup warning");
        assert!(deleted.credential_cleanup_pending);
        *store.fail_delete.lock().expect("mock failure lock") = false;
        let retried = manager
            .retry_cleanup(saved.credential_ref_id)
            .await
            .expect("retry cleanup");
        assert!(!retried.credential_cleanup_pending);
    }

    #[tokio::test]
    async fn create_server_requires_explicit_credential_action() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("test.sqlite3")).expect("database");
        let store = Arc::new(MockSecureStore::default());
        let worker = CredentialWorker::new(store.clone()).expect("credential worker");
        let manager = CredentialManager::new(database.clone(), worker);

        let error = manager
            .create_server(password_input(), CredentialUpdate::Keep)
            .await
            .expect_err("create cannot keep a credential that does not exist");
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        assert!(
            ProfileStore::new(database.clone())
                .list_servers(crate::ServerListQuery::default())
                .await
                .expect("list servers")
                .items
                .is_empty()
        );

        *store.fail_save.lock().expect("mock save failure lock") = true;
        let error = manager
            .create_server(
                password_input(),
                CredentialUpdate::Replace {
                    secret: Secret::new(b"rejected".to_vec()).expect("secret"),
                },
            )
            .await
            .expect_err("failed credential save must fail create");
        assert_eq!(error.code, ErrorCode::CredentialAccessDenied);
        assert!(
            ProfileStore::new(database.clone())
                .list_servers(crate::ServerListQuery::default())
                .await
                .expect("list servers after failed create")
                .items
                .is_empty()
        );
        *store.fail_save.lock().expect("mock save failure lock") = false;

        let created = manager
            .create_server(
                password_input(),
                CredentialUpdate::Replace {
                    secret: Secret::new(b"created".to_vec()).expect("secret"),
                },
            )
            .await
            .expect("create with credential");
        assert!(created.server.has_saved_credential);
        assert_eq!(created.server.revision, 2);
        assert_eq!(
            manager
                .load_for_authentication(created.server.id)
                .await
                .expect("load created credential")
                .to_vec(),
            b"created"
        );
    }

    #[tokio::test]
    async fn update_requires_explicit_credential_action_for_identity_change() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("test.sqlite3")).expect("database");
        let server_id = password_server(&database).await;
        let store = Arc::new(MockSecureStore::default());
        let worker = CredentialWorker::new(store.clone()).expect("credential worker");
        let manager = CredentialManager::new(database.clone(), worker);
        let saved = manager
            .replace_for_server(
                server_id.clone(),
                CredentialKind::Password,
                Secret::new(b"old-identity".to_vec()).expect("secret"),
            )
            .await
            .expect("save credential");
        let mut changed = password_input();
        changed.host = "changed.example.test".to_owned();

        let error = manager
            .update_server(
                server_id.clone(),
                2,
                changed.clone(),
                CredentialUpdate::Keep,
            )
            .await
            .expect_err("identity change cannot keep old credential");
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        let unchanged = ProfileStore::new(database.clone())
            .get_server(server_id.clone())
            .await
            .expect("unchanged server");
        assert_eq!(unchanged.host, "server.example.test");
        assert_eq!(unchanged.revision, 2);

        *store.fail_save.lock().expect("mock save failure lock") = true;
        let save_error = manager
            .update_server(
                server_id.clone(),
                2,
                changed.clone(),
                CredentialUpdate::Replace {
                    secret: Secret::new(b"rejected".to_vec()).expect("secret"),
                },
            )
            .await
            .expect_err("failed replacement save must keep old identity");
        assert_eq!(save_error.code, ErrorCode::CredentialAccessDenied);
        let after_failed_save = ProfileStore::new(database.clone())
            .get_server(server_id.clone())
            .await
            .expect("server after failed replacement");
        assert_eq!(after_failed_save.host, "server.example.test");
        assert_eq!(after_failed_save.revision, 2);
        assert_eq!(
            manager
                .load_for_authentication(server_id.clone())
                .await
                .expect("old credential remains active")
                .to_vec(),
            b"old-identity"
        );
        *store.fail_save.lock().expect("mock save failure lock") = false;

        *store.fail_delete.lock().expect("mock failure lock") = true;
        let updated = manager
            .update_server(
                server_id.clone(),
                2,
                changed.clone(),
                CredentialUpdate::Clear,
            )
            .await
            .expect("clear old credential while changing identity");
        assert_eq!(updated.server.host, "changed.example.test");
        assert_eq!(updated.server.revision, 3);
        assert!(!updated.server.has_saved_credential);
        assert!(updated.credential_cleanup_pending);

        *store.fail_delete.lock().expect("mock failure lock") = false;
        manager
            .retry_cleanup(saved.credential_ref_id)
            .await
            .expect("retry old credential cleanup");

        let replaced = manager
            .update_server(
                server_id.clone(),
                3,
                changed,
                CredentialUpdate::Replace {
                    secret: Secret::new(b"new-identity".to_vec()).expect("secret"),
                },
            )
            .await
            .expect("replace credential for new identity");
        assert_eq!(replaced.server.revision, 4);
        assert!(replaced.server.has_saved_credential);
        assert_eq!(
            manager
                .load_for_authentication(server_id)
                .await
                .expect("load replacement")
                .to_vec(),
            b"new-identity"
        );
    }

    #[test]
    fn secret_debug_is_redacted() {
        let marker = "MAULINK_SECRET_MARKER";
        let secret = Secret::new(marker.as_bytes().to_vec()).expect("secret");
        let debug = format!("{secret:?}");
        assert!(!debug.contains(marker));
        assert!(debug.contains("REDACTED"));
    }

    #[cfg(any(target_os = "macos", windows))]
    #[tokio::test]
    #[ignore = "writes a short-lived item to the native credential store"]
    async fn native_store_round_trip() {
        let worker = CredentialWorker::native().expect("native credential worker");
        let credential_id = format!("integration-test-{}", Uuid::new_v4());
        let marker = format!("maulink-test-secret-{}", Uuid::new_v4());
        worker
            .save(
                credential_id.clone(),
                Secret::new(marker.as_bytes().to_vec()).expect("test secret"),
            )
            .await
            .expect("save native credential");
        let loaded = worker
            .load(credential_id.clone())
            .await
            .expect("load native credential");
        assert_eq!(loaded.to_vec(), marker.as_bytes());
        worker
            .delete(credential_id)
            .await
            .expect("delete native credential");
    }
}
