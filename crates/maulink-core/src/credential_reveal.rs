use std::{future::Future, pin::Pin, sync::Arc};

use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use crate::{
    AppError, AuthType, CredentialKind, CredentialManager, Database, ErrorCode, Secret,
    profiles::ProfileStore, storage,
};

const PASSWORD_MIN_CHARACTERS: usize = 12;
const PASSWORD_MAX_CHARACTERS: usize = 128;
const PASSWORD_MAX_BYTES: usize = 1024;
const MAX_FAILED_ATTEMPTS: i64 = 5;
const LOCK_DURATION_MS: i64 = 30 * 1000;
const ARGON2_MEMORY_KIB: u32 = 64 * 1024;
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_LANES: u32 = 1;
const DIRECT_CONFIRMATION: &str = "允许直接查看";
const DIRECT_CONFIRMATION_EN: &str = "ALLOW DIRECT VIEW";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RevealMode {
    Deny,
    Protected,
    Direct,
}

impl RevealMode {
    fn as_database_value(self) -> &'static str {
        match self {
            Self::Deny => "deny",
            Self::Protected => "protected",
            Self::Direct => "direct",
        }
    }

    fn from_database_value(value: &str) -> Result<Self, AppError> {
        match value {
            "deny" => Ok(Self::Deny),
            "protected" => Ok(Self::Protected),
            "direct" => Ok(Self::Direct),
            _ => Err(policy_unavailable()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsIdentityPurpose {
    EnableProtected,
    EnableDirect,
    ChangePassword,
    DisableProtection,
    RecoverToDeny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsIdentityError {
    Unavailable,
    Cancelled,
    Failed,
}

pub type IdentityFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), OsIdentityError>> + Send + 'a>>;
pub type AvailabilityFuture<'a> = Pin<Box<dyn Future<Output = bool> + Send + 'a>>;

/// The only route used by security policy changes to request trusted OS user presence.
pub trait OsIdentityGate: Send + Sync + 'static {
    fn is_available(&self) -> AvailabilityFuture<'_>;
    fn verify(&self, purpose: OsIdentityPurpose) -> IdentityFuture<'_>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CredentialRevealPolicy {
    pub mode: RevealMode,
    pub revision: u32,
    pub has_secondary_password: bool,
    pub native_auth_available: bool,
    pub native_auth_reason: Option<String>,
    #[ts(type = "number | null")]
    pub locked_until_ms: Option<i64>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CredentialRevealResult {
    pub kind: CredentialKind,
    pub value: String,
}

impl std::fmt::Debug for CredentialRevealResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CredentialRevealResult")
            .field("kind", &self.kind)
            .field("value", &"[REDACTED]")
            .finish()
    }
}

impl Drop for CredentialRevealResult {
    fn drop(&mut self) {
        self.value.zeroize();
    }
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EnableProtectedPayload {
    pub expected_revision: u32,
    pub current_password: Option<Secret>,
    pub password: Secret,
    pub confirm_password: Secret,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EnableDirectPayload {
    pub expected_revision: u32,
    pub current_password: Option<Secret>,
    pub confirm_first_risk: bool,
    pub confirm_second_risk: bool,
    pub confirmation_text: String,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SetDenyPayload {
    pub expected_revision: u32,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSecondaryPasswordPayload {
    pub expected_revision: u32,
    pub current_password: Secret,
    pub password: Secret,
    pub confirm_password: Secret,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RecoverRevealPolicyPayload {
    pub expected_revision: u32,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CredentialRevealTarget {
    pub server_id: String,
    pub secondary_password: Option<Secret>,
}

#[derive(Debug)]
struct PolicyRow {
    mode: RevealMode,
    password_hash: Option<String>,
    locked_until_ms: Option<i64>,
    revision: u32,
}

impl PolicyRow {
    fn from_database(
        mode: String,
        password_hash: Option<String>,
        failed_attempts: i64,
        locked_until_ms: Option<i64>,
        revision: i64,
    ) -> Result<Self, AppError> {
        let mode = RevealMode::from_database_value(&mode)?;
        let hash_state_valid = match mode {
            RevealMode::Protected => password_hash.is_some(),
            RevealMode::Deny | RevealMode::Direct => password_hash.is_none(),
        };
        if !hash_state_valid
            || !(0..=MAX_FAILED_ATTEMPTS).contains(&failed_attempts)
            || locked_until_ms.is_some_and(|value| value < 0)
        {
            return Err(policy_unavailable());
        }
        let revision = u32::try_from(revision).map_err(|_| policy_unavailable())?;
        if revision == 0 {
            return Err(policy_unavailable());
        }
        Ok(Self {
            mode,
            password_hash,
            locked_until_ms,
            revision,
        })
    }

    fn into_public(self, native_auth_available: bool) -> CredentialRevealPolicy {
        CredentialRevealPolicy {
            mode: self.mode,
            revision: self.revision,
            has_secondary_password: self.password_hash.is_some(),
            native_auth_available,
            native_auth_reason: (!native_auth_available)
                .then(|| "errors.nativeAuthenticationUnavailable".to_owned()),
            locked_until_ms: self.locked_until_ms,
        }
    }
}

type ReadCredentialFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(CredentialKind, Secret), AppError>> + Send + 'a>>;

trait CredentialRevealReader: Send + Sync + 'static {
    fn read(&self, server_id: &str) -> ReadCredentialFuture<'_>;
}

struct ManagerCredentialReader {
    profiles: ProfileStore,
    credentials: CredentialManager,
}

impl CredentialRevealReader for ManagerCredentialReader {
    fn read(&self, server_id: &str) -> ReadCredentialFuture<'_> {
        let server_id = server_id.to_owned();
        Box::pin(async move {
            let profile = self.profiles.get_server(server_id.clone()).await?;
            if !profile.has_saved_credential {
                return Err(AppError::new(
                    ErrorCode::CredentialNotFound,
                    "errors.credentialNotFound",
                ));
            }
            let kind = match profile.auth_type {
                AuthType::Password => CredentialKind::Password,
                AuthType::PrivateKey => CredentialKind::Passphrase,
            };
            let secret = self.credentials.load_for_authentication(server_id).await?;
            Ok((kind, secret))
        })
    }
}

pub struct CredentialRevealService {
    database: Database,
    reader: Arc<dyn CredentialRevealReader>,
    identity_gate: Arc<dyn OsIdentityGate>,
    operation_lock: tokio::sync::Mutex<()>,
}

impl CredentialRevealService {
    pub fn new(
        database: Database,
        credentials: CredentialManager,
        identity_gate: Arc<dyn OsIdentityGate>,
    ) -> Self {
        Self::with_reader(
            database.clone(),
            Arc::new(ManagerCredentialReader {
                profiles: ProfileStore::new(database),
                credentials,
            }),
            identity_gate,
        )
    }

    fn with_reader(
        database: Database,
        reader: Arc<dyn CredentialRevealReader>,
        identity_gate: Arc<dyn OsIdentityGate>,
    ) -> Self {
        Self {
            database,
            reader,
            identity_gate,
            operation_lock: tokio::sync::Mutex::new(()),
        }
    }

    pub async fn policy_get(&self) -> Result<CredentialRevealPolicy, AppError> {
        let row = read_policy(&self.database).await?;
        let native_auth_available = self.identity_gate.is_available().await;
        Ok(row.into_public(native_auth_available))
    }

    pub async fn enable_protected(
        &self,
        expected_revision: u32,
        current_password: Option<Secret>,
        password: Secret,
        confirm_password: Secret,
    ) -> Result<CredentialRevealPolicy, AppError> {
        validate_password_pair(&password, &confirm_password)?;
        let _guard = self.operation_lock.lock().await;
        let current = read_policy(&self.database).await?;
        check_revision(&current, expected_revision)?;
        if current.mode == RevealMode::Protected {
            let current_password = current_password.as_ref().ok_or_else(|| {
                AppError::new(
                    ErrorCode::CredentialRevealPasswordRequired,
                    "errors.credentialRevealPasswordRequired",
                )
            })?;
            verify_secondary_password(&self.database, &current, current_password).await?;
        }
        self.verify_identity(OsIdentityPurpose::EnableProtected)
            .await?;
        let hash = hash_password(password).await?;
        let row = update_policy(
            &self.database,
            expected_revision,
            RevealMode::Protected,
            Some(hash),
        )
        .await?;
        let native_auth_available = self.identity_gate.is_available().await;
        Ok(row.into_public(native_auth_available))
    }

    pub async fn enable_direct(
        &self,
        expected_revision: u32,
        current_password: Option<Secret>,
        confirm_first_risk: bool,
        confirm_second_risk: bool,
        confirmation_text: String,
    ) -> Result<CredentialRevealPolicy, AppError> {
        if !confirm_first_risk
            || !confirm_second_risk
            || ![DIRECT_CONFIRMATION, DIRECT_CONFIRMATION_EN].contains(&confirmation_text.as_str())
        {
            return Err(validation(
                "confirmation",
                "errors.directRevealConfirmationRequired",
            ));
        }
        let _guard = self.operation_lock.lock().await;
        let current = read_policy(&self.database).await?;
        check_revision(&current, expected_revision)?;
        if current.mode == RevealMode::Direct {
            return Ok(current.into_public(self.identity_gate.is_available().await));
        }
        if current.mode == RevealMode::Protected {
            let current_password = current_password.as_ref().ok_or_else(|| {
                AppError::new(
                    ErrorCode::CredentialRevealPasswordRequired,
                    "errors.credentialRevealPasswordRequired",
                )
            })?;
            verify_secondary_password(&self.database, &current, current_password).await?;
        }
        self.verify_identity(OsIdentityPurpose::EnableDirect)
            .await?;
        let row =
            update_policy(&self.database, expected_revision, RevealMode::Direct, None).await?;
        let native_auth_available = self.identity_gate.is_available().await;
        Ok(row.into_public(native_auth_available))
    }

    pub async fn set_deny(
        &self,
        expected_revision: u32,
    ) -> Result<CredentialRevealPolicy, AppError> {
        let _guard = self.operation_lock.lock().await;
        let current = read_policy(&self.database).await?;
        check_revision(&current, expected_revision)?;
        if current.mode == RevealMode::Deny {
            return Ok(current.into_public(self.identity_gate.is_available().await));
        }
        let row = update_policy(&self.database, expected_revision, RevealMode::Deny, None).await?;
        let native_auth_available = self.identity_gate.is_available().await;
        Ok(row.into_public(native_auth_available))
    }

    pub async fn change_secondary_password(
        &self,
        expected_revision: u32,
        current_password: Secret,
        password: Secret,
        confirm_password: Secret,
    ) -> Result<CredentialRevealPolicy, AppError> {
        validate_password_pair(&password, &confirm_password)?;
        let _guard = self.operation_lock.lock().await;
        let current = read_policy(&self.database).await?;
        check_revision(&current, expected_revision)?;
        if current.mode != RevealMode::Protected {
            return Err(policy_unavailable());
        }
        verify_secondary_password(&self.database, &current, &current_password).await?;
        self.verify_identity(OsIdentityPurpose::ChangePassword)
            .await?;
        let hash = hash_password(password).await?;
        let row = update_policy(
            &self.database,
            expected_revision,
            RevealMode::Protected,
            Some(hash),
        )
        .await?;
        let native_auth_available = self.identity_gate.is_available().await;
        Ok(row.into_public(native_auth_available))
    }

    pub async fn recover_to_deny(
        &self,
        expected_revision: u32,
    ) -> Result<CredentialRevealPolicy, AppError> {
        let _guard = self.operation_lock.lock().await;
        let current = read_policy(&self.database).await?;
        check_revision(&current, expected_revision)?;
        if current.mode == RevealMode::Deny {
            return Ok(current.into_public(self.identity_gate.is_available().await));
        }
        self.verify_identity(OsIdentityPurpose::RecoverToDeny)
            .await?;
        let row = update_policy(&self.database, expected_revision, RevealMode::Deny, None).await?;
        let native_auth_available = self.identity_gate.is_available().await;
        Ok(row.into_public(native_auth_available))
    }

    pub async fn reveal(
        &self,
        target: CredentialRevealTarget,
    ) -> Result<CredentialRevealResult, AppError> {
        let _guard = self.operation_lock.lock().await;
        let current = read_policy(&self.database).await?;
        match current.mode {
            RevealMode::Deny => return Err(reveal_denied()),
            RevealMode::Protected => {
                let password = target.secondary_password.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::CredentialRevealPasswordRequired,
                        "errors.credentialRevealPasswordRequired",
                    )
                })?;
                verify_secondary_password(&self.database, &current, password).await?;
            }
            RevealMode::Direct => {}
        }

        let (kind, secret) = self.reader.read(&target.server_id).await?;
        let latest = read_policy(&self.database).await?;
        if latest.revision != current.revision || latest.mode != current.mode {
            return Err(policy_revision_conflict());
        }
        let mut value = secret.into_utf8_string()?;
        let value = std::mem::take(&mut *value);
        Ok(CredentialRevealResult { kind, value })
    }

    async fn verify_identity(&self, purpose: OsIdentityPurpose) -> Result<(), AppError> {
        if !self.identity_gate.is_available().await {
            return Err(AppError::new(
                ErrorCode::NativeAuthenticationUnavailable,
                "errors.nativeAuthenticationUnavailable",
            ));
        }
        self.identity_gate
            .verify(purpose)
            .await
            .map_err(|error| match error {
                OsIdentityError::Unavailable => AppError::new(
                    ErrorCode::NativeAuthenticationUnavailable,
                    "errors.nativeAuthenticationUnavailable",
                ),
                OsIdentityError::Cancelled => AppError::new(
                    ErrorCode::NativeAuthenticationCancelled,
                    "errors.nativeAuthenticationCancelled",
                ),
                OsIdentityError::Failed => AppError::new(
                    ErrorCode::NativeAuthenticationFailed,
                    "errors.nativeAuthenticationFailed",
                ),
            })
    }
}

async fn read_policy(database: &Database) -> Result<PolicyRow, AppError> {
    database
        .execute(|connection| {
            let result = connection
                .query_row(
                    "SELECT mode, password_hash, failed_attempts, locked_until_ms, revision
                     FROM credential_reveal_policy WHERE singleton = 1",
                    [],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, Option<i64>>(3)?,
                            row.get::<_, i64>(4)?,
                        ))
                    },
                )
                .optional()
                .map_err(|_| policy_unavailable())?
                .ok_or_else(policy_unavailable)?;
            PolicyRow::from_database(result.0, result.1, result.2, result.3, result.4)
        })
        .await
}

async fn update_policy(
    database: &Database,
    expected_revision: u32,
    mode: RevealMode,
    password_hash: Option<String>,
) -> Result<PolicyRow, AppError> {
    let mode = mode.as_database_value().to_owned();
    database
        .execute(move |connection| {
            let changed = connection
                .execute(
                    "UPDATE credential_reveal_policy
                     SET mode = ?1, password_hash = ?2, failed_attempts = 0,
                         locked_until_ms = NULL, revision = revision + 1
                     WHERE singleton = 1 AND revision = ?3",
                    params![mode, password_hash, i64::from(expected_revision)],
                )
                .map_err(|_| policy_unavailable())?;
            if changed != 1 {
                return Err(policy_revision_conflict());
            }
            read_policy_row(connection)
        })
        .await
}

fn read_policy_row(connection: &rusqlite::Connection) -> Result<PolicyRow, AppError> {
    let result = connection
        .query_row(
            "SELECT mode, password_hash, failed_attempts, locked_until_ms, revision
             FROM credential_reveal_policy WHERE singleton = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .map_err(|_| policy_unavailable())?;
    PolicyRow::from_database(result.0, result.1, result.2, result.3, result.4)
}

async fn verify_secondary_password(
    database: &Database,
    policy: &PolicyRow,
    password: &Secret,
) -> Result<(), AppError> {
    let now = storage::now_ms()?;
    if let Some(locked_until_ms) = policy.locked_until_ms {
        if locked_until_ms > now {
            return Err(reveal_locked());
        }
        clear_expired_lock(database).await?;
    }
    let hash = policy
        .password_hash
        .clone()
        .ok_or_else(policy_unavailable)?;
    let password_bytes = Zeroizing::new(password.expose().to_vec());
    let verified = tokio::task::spawn_blocking(move || verify_password(&password_bytes, &hash))
        .await
        .map_err(|_| policy_unavailable())??;
    if !verified {
        record_failed_attempt(database).await?;
        let latest = read_policy(database).await?;
        if latest.locked_until_ms.is_some_and(|locked| locked > now) {
            return Err(reveal_locked());
        }
        return Err(AppError::new(
            ErrorCode::CredentialRevealPasswordInvalid,
            "errors.credentialRevealPasswordInvalid",
        ));
    }
    clear_failed_attempts(database).await
}

async fn clear_expired_lock(database: &Database) -> Result<(), AppError> {
    database
        .execute(|connection| {
            connection
                .execute(
                    "UPDATE credential_reveal_policy
                     SET failed_attempts = 0, locked_until_ms = NULL
                     WHERE singleton = 1 AND locked_until_ms IS NOT NULL
                       AND locked_until_ms <= ?1",
                    [storage::now_ms()?],
                )
                .map_err(storage::map_sqlite_error)?;
            Ok(())
        })
        .await
}

async fn clear_failed_attempts(database: &Database) -> Result<(), AppError> {
    database
        .execute(|connection| {
            connection
                .execute(
                    "UPDATE credential_reveal_policy
                     SET failed_attempts = 0, locked_until_ms = NULL WHERE singleton = 1",
                    [],
                )
                .map_err(storage::map_sqlite_error)?;
            Ok(())
        })
        .await
}

async fn record_failed_attempt(database: &Database) -> Result<(), AppError> {
    database
        .execute(|connection| {
            let now = storage::now_ms()?;
            connection
                .execute(
                    "UPDATE credential_reveal_policy
                     SET failed_attempts = MIN(failed_attempts + 1, ?1),
                         locked_until_ms = CASE
                           WHEN failed_attempts + 1 >= ?1 THEN ?2 + ?3
                           ELSE locked_until_ms
                         END
                     WHERE singleton = 1",
                    params![MAX_FAILED_ATTEMPTS, now, LOCK_DURATION_MS],
                )
                .map_err(storage::map_sqlite_error)?;
            Ok(())
        })
        .await
}

async fn hash_password(password: Secret) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || hash_password_bytes(password.expose()))
        .await
        .map_err(|_| policy_unavailable())?
}

fn hash_password_bytes(password: &[u8]) -> Result<String, AppError> {
    let salt = Uuid::new_v4();
    let params = Params::new(ARGON2_MEMORY_KIB, ARGON2_ITERATIONS, ARGON2_LANES, Some(32))
        .map_err(|_| policy_unavailable())?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    argon2
        .hash_password_with_salt(password, salt.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|_| policy_unavailable())
}

fn verify_password(password: &[u8], hash: &str) -> Result<bool, AppError> {
    if hash.len() > 512 {
        return Err(policy_unavailable());
    }
    let parsed = PasswordHash::new(hash).map_err(|_| policy_unavailable())?;
    let expected_params = format!("m={ARGON2_MEMORY_KIB},t={ARGON2_ITERATIONS},p={ARGON2_LANES}");
    if parsed.algorithm.as_str() != "argon2id"
        || parsed.version != Some(19)
        || parsed.params.as_str() != expected_params
        || parsed.salt.is_none()
        || parsed.hash.as_ref().is_none_or(|output| output.len() != 32)
    {
        return Err(policy_unavailable());
    }
    let params = Params::new(ARGON2_MEMORY_KIB, ARGON2_ITERATIONS, ARGON2_LANES, Some(32))
        .map_err(|_| policy_unavailable())?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    Ok(argon2.verify_password(password, &parsed).is_ok())
}

fn validate_password_pair(password: &Secret, confirmation: &Secret) -> Result<(), AppError> {
    let text = std::str::from_utf8(password.expose())
        .map_err(|_| validation("password", "errors.secondaryPasswordInvalid"))?;
    let confirmation = std::str::from_utf8(confirmation.expose())
        .map_err(|_| validation("confirmPassword", "errors.secondaryPasswordInvalid"))?;
    let characters = text.chars().count();
    if !(PASSWORD_MIN_CHARACTERS..=PASSWORD_MAX_CHARACTERS).contains(&characters)
        || text.len() > PASSWORD_MAX_BYTES
        || text != confirmation
    {
        return Err(validation("password", "errors.secondaryPasswordInvalid"));
    }
    Ok(())
}

fn check_revision(policy: &PolicyRow, expected_revision: u32) -> Result<(), AppError> {
    if policy.revision != expected_revision {
        return Err(policy_revision_conflict());
    }
    Ok(())
}

fn reveal_denied() -> AppError {
    AppError::new(
        ErrorCode::CredentialRevealDenied,
        "errors.credentialRevealDenied",
    )
}

fn reveal_locked() -> AppError {
    AppError::new(
        ErrorCode::CredentialRevealLocked,
        "errors.credentialRevealLocked",
    )
}

fn policy_unavailable() -> AppError {
    AppError::new(
        ErrorCode::SecurityPolicyUnavailable,
        "errors.securityPolicyUnavailable",
    )
}

fn policy_revision_conflict() -> AppError {
    AppError::new(
        ErrorCode::SecurityPolicyRevisionConflict,
        "errors.securityPolicyRevisionConflict",
    )
}

fn validation(field: &'static str, message_key: &'static str) -> AppError {
    AppError::new(ErrorCode::ValidationFailed, message_key).with_param("field", field)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use super::*;

    struct FakeGate {
        available: AtomicBool,
        cancelled: AtomicBool,
        calls: AtomicUsize,
    }

    impl FakeGate {
        fn available() -> Self {
            Self {
                available: AtomicBool::new(true),
                cancelled: AtomicBool::new(false),
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl OsIdentityGate for FakeGate {
        fn is_available(&self) -> AvailabilityFuture<'_> {
            Box::pin(std::future::ready(self.available.load(Ordering::SeqCst)))
        }

        fn verify(&self, _purpose: OsIdentityPurpose) -> IdentityFuture<'_> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let result = if !self.available.load(Ordering::SeqCst) {
                Err(OsIdentityError::Unavailable)
            } else if self.cancelled.load(Ordering::SeqCst) {
                Err(OsIdentityError::Cancelled)
            } else {
                Ok(())
            };
            Box::pin(std::future::ready(result))
        }
    }

    struct FakeReader {
        calls: AtomicUsize,
        kind: CredentialKind,
        value: &'static [u8],
    }

    impl CredentialRevealReader for FakeReader {
        fn read(&self, _server_id: &str) -> ReadCredentialFuture<'_> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(std::future::ready(Ok((
                self.kind,
                Secret::new(self.value.to_vec()).expect("fake secret"),
            ))))
        }
    }

    fn setup() -> (
        tempfile::TempDir,
        Database,
        Arc<FakeReader>,
        Arc<FakeGate>,
        CredentialRevealService,
    ) {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("test.sqlite3")).expect("database");
        let reader = Arc::new(FakeReader {
            calls: AtomicUsize::new(0),
            kind: CredentialKind::Password,
            value: b"fixture-secret-never-use",
        });
        let gate = Arc::new(FakeGate::available());
        let service =
            CredentialRevealService::with_reader(database.clone(), reader.clone(), gate.clone());
        (directory, database, reader, gate, service)
    }

    fn secret(value: &str) -> Secret {
        Secret::new(value.as_bytes().to_vec()).expect("secret")
    }

    #[tokio::test]
    async fn new_database_starts_deny_and_denied_reveal_never_reads_credential() {
        let (_directory, _database, reader, gate, service) = setup();
        let policy = service.policy_get().await.expect("policy");
        assert_eq!(policy.mode, RevealMode::Deny);
        assert_eq!(policy.revision, 1);
        assert!(!policy.has_secondary_password);
        assert!(policy.native_auth_available);

        let error = service
            .reveal(CredentialRevealTarget {
                server_id: "server-id".to_owned(),
                secondary_password: None,
            })
            .await
            .expect_err("deny mode rejects reveal");
        assert_eq!(error.code, ErrorCode::CredentialRevealDenied);
        assert_eq!(reader.calls.load(Ordering::SeqCst), 0);
        assert_eq!(gate.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn protected_mode_requires_os_auth_and_correct_secondary_password() {
        let (_directory, _database, reader, gate, service) = setup();
        let policy = service
            .enable_protected(
                1,
                None,
                secret("twelve-characters"),
                secret("twelve-characters"),
            )
            .await
            .expect("enable protected");
        assert_eq!(policy.mode, RevealMode::Protected);
        assert!(policy.has_secondary_password);
        assert_eq!(gate.calls.load(Ordering::SeqCst), 1);
        let serialized = serde_json::to_string(&policy).expect("serialize public policy");
        assert!(!serialized.contains("password_hash"));
        assert!(!serialized.contains("twelve-characters"));

        let wrong = service
            .reveal(CredentialRevealTarget {
                server_id: "server-id".to_owned(),
                secondary_password: Some(secret("wrong-secondary")),
            })
            .await
            .expect_err("wrong password denied");
        assert_eq!(wrong.code, ErrorCode::CredentialRevealPasswordInvalid);
        assert_eq!(reader.calls.load(Ordering::SeqCst), 0);

        let result = service
            .reveal(CredentialRevealTarget {
                server_id: "server-id".to_owned(),
                secondary_password: Some(secret("twelve-characters")),
            })
            .await
            .expect("correct password reveals one value");
        assert_eq!(result.kind, CredentialKind::Password);
        assert_eq!(result.value, "fixture-secret-never-use");
        assert_eq!(reader.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn cancelled_or_unavailable_identity_authentication_keeps_deny() {
        let (_directory, _database, _reader, gate, service) = setup();
        gate.cancelled.store(true, Ordering::SeqCst);
        let error = service
            .enable_protected(
                1,
                None,
                secret("twelve-characters"),
                secret("twelve-characters"),
            )
            .await
            .expect_err("cancelled native authentication denies setup");
        assert_eq!(error.code, ErrorCode::NativeAuthenticationCancelled);
        assert_eq!(
            service.policy_get().await.expect("policy").mode,
            RevealMode::Deny
        );

        gate.cancelled.store(false, Ordering::SeqCst);
        gate.available.store(false, Ordering::SeqCst);
        let error = service
            .enable_direct(1, None, true, true, DIRECT_CONFIRMATION.to_owned())
            .await
            .expect_err("unavailable native authentication denies direct mode");
        assert_eq!(error.code, ErrorCode::NativeAuthenticationUnavailable);
        assert_eq!(
            service.policy_get().await.expect("policy").mode,
            RevealMode::Deny
        );
    }

    #[tokio::test]
    async fn direct_mode_requires_two_acknowledgements_and_confirmation_phrase() {
        let (_directory, _database, _reader, gate, service) = setup();
        let error = service
            .enable_direct(1, None, true, false, DIRECT_CONFIRMATION.to_owned())
            .await
            .expect_err("both risk acknowledgements required");
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        assert_eq!(gate.calls.load(Ordering::SeqCst), 0);

        let policy = service
            .enable_direct(1, None, true, true, DIRECT_CONFIRMATION.to_owned())
            .await
            .expect("direct mode with OS auth");
        assert_eq!(policy.mode, RevealMode::Direct);
        assert_eq!(gate.calls.load(Ordering::SeqCst), 1);
        let result = service
            .reveal(CredentialRevealTarget {
                server_id: "server-id".to_owned(),
                secondary_password: None,
            })
            .await
            .expect("direct reveals only on explicit request");
        assert_eq!(result.value, "fixture-secret-never-use");
    }

    #[tokio::test]
    async fn five_failures_persist_a_lock_until_expiry() {
        let (_directory, database, _reader, _gate, service) = setup();
        service
            .enable_protected(
                1,
                None,
                secret("twelve-characters"),
                secret("twelve-characters"),
            )
            .await
            .expect("enable protected");
        for attempt in 0..MAX_FAILED_ATTEMPTS {
            let error = service
                .reveal(CredentialRevealTarget {
                    server_id: "server-id".to_owned(),
                    secondary_password: Some(secret("incorrect-password")),
                })
                .await
                .expect_err("wrong password denied");
            assert_eq!(
                error.code,
                if attempt + 1 == MAX_FAILED_ATTEMPTS {
                    ErrorCode::CredentialRevealLocked
                } else {
                    ErrorCode::CredentialRevealPasswordInvalid
                }
            );
        }
        let locked = service.policy_get().await.expect("locked policy");
        assert!(locked.locked_until_ms.is_some());
        let restarted = CredentialRevealService::with_reader(
            database,
            Arc::new(FakeReader {
                calls: AtomicUsize::new(0),
                kind: CredentialKind::Password,
                value: b"fixture-secret-never-use",
            }),
            Arc::new(FakeGate::available()),
        );
        let error = restarted
            .reveal(CredentialRevealTarget {
                server_id: "server-id".to_owned(),
                secondary_password: Some(secret("twelve-characters")),
            })
            .await
            .expect_err("restart does not bypass lock");
        assert_eq!(error.code, ErrorCode::CredentialRevealLocked);
    }

    #[tokio::test]
    async fn protected_to_direct_requires_current_password_and_native_authentication() {
        let (_directory, _database, _reader, gate, service) = setup();
        let protected = service
            .enable_protected(
                1,
                None,
                secret("twelve-characters"),
                secret("twelve-characters"),
            )
            .await
            .expect("enable protected");

        let error = service
            .enable_direct(
                protected.revision,
                Some(secret("wrong-secondary")),
                true,
                true,
                DIRECT_CONFIRMATION.to_owned(),
            )
            .await
            .expect_err("direct mode requires the old password");
        assert_eq!(error.code, ErrorCode::CredentialRevealPasswordInvalid);
        assert_eq!(gate.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            service.policy_get().await.expect("policy").mode,
            RevealMode::Protected
        );

        let direct = service
            .enable_direct(
                protected.revision,
                Some(secret("twelve-characters")),
                true,
                true,
                DIRECT_CONFIRMATION_EN.to_owned(),
            )
            .await
            .expect("correct old password and native auth");
        assert_eq!(direct.mode, RevealMode::Direct);
        assert_eq!(gate.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn changing_protected_password_requires_old_password_and_resets_rate_limit() {
        let (_directory, _database, reader, gate, service) = setup();
        let policy = service
            .enable_protected(
                1,
                None,
                secret("twelve-characters"),
                secret("twelve-characters"),
            )
            .await
            .expect("enable protected");
        let error = service
            .change_secondary_password(
                policy.revision,
                secret("wrong-secondary"),
                secret("replacement-password"),
                secret("replacement-password"),
            )
            .await
            .expect_err("old password is checked");
        assert_eq!(error.code, ErrorCode::CredentialRevealPasswordInvalid);

        let changed = service
            .change_secondary_password(
                policy.revision,
                secret("twelve-characters"),
                secret("replacement-password"),
                secret("replacement-password"),
            )
            .await
            .expect("replace password");
        let old = service
            .reveal(CredentialRevealTarget {
                server_id: "server-id".to_owned(),
                secondary_password: Some(secret("twelve-characters")),
            })
            .await
            .expect_err("old password no longer works");
        assert_eq!(old.code, ErrorCode::CredentialRevealPasswordInvalid);
        let new = service
            .reveal(CredentialRevealTarget {
                server_id: "server-id".to_owned(),
                secondary_password: Some(secret("replacement-password")),
            })
            .await
            .expect("new password works");
        assert_eq!(new.value, "fixture-secret-never-use");
        assert_eq!(gate.calls.load(Ordering::SeqCst), 2);
        assert_eq!(reader.calls.load(Ordering::SeqCst), 1);
        assert_eq!(changed.mode, RevealMode::Protected);
    }

    #[test]
    fn malformed_or_unbounded_argon_parameters_fail_closed_before_verification() {
        let hash = hash_password_bytes(b"password").expect("hash");
        let unbounded = hash.replace("m=65536", "m=999999999");
        let error = verify_password(b"password", &unbounded).expect_err("unbounded KDF rejected");
        assert_eq!(error.code, ErrorCode::SecurityPolicyUnavailable);
    }

    #[tokio::test]
    async fn switching_to_deny_invalidates_old_revision_and_reveal_access() {
        let (_directory, _database, reader, _gate, service) = setup();
        let protected = service
            .enable_protected(
                1,
                None,
                secret("twelve-characters"),
                secret("twelve-characters"),
            )
            .await
            .expect("enable protected");
        let denied = service
            .set_deny(protected.revision)
            .await
            .expect("disable reveal protection");
        assert_eq!(denied.mode, RevealMode::Deny);
        let error = service
            .reveal(CredentialRevealTarget {
                server_id: "server-id".to_owned(),
                secondary_password: Some(secret("twelve-characters")),
            })
            .await
            .expect_err("deny mode takes effect immediately");
        assert_eq!(error.code, ErrorCode::CredentialRevealDenied);
        assert_eq!(reader.calls.load(Ordering::SeqCst), 0);
        let conflict = service
            .set_deny(protected.revision)
            .await
            .expect_err("stale policy revision rejected");
        assert_eq!(conflict.code, ErrorCode::SecurityPolicyRevisionConflict);
    }

    #[tokio::test]
    async fn missing_policy_row_fails_closed() {
        let (_directory, database, reader, gate, service) = setup();
        database
            .execute(|connection| {
                connection
                    .execute("DELETE FROM credential_reveal_policy", [])
                    .map_err(storage::map_sqlite_error)?;
                Ok(())
            })
            .await
            .expect("delete policy row");
        let error = service
            .reveal(CredentialRevealTarget {
                server_id: "server-id".to_owned(),
                secondary_password: None,
            })
            .await
            .expect_err("missing policy does not allow reveal");
        assert_eq!(error.code, ErrorCode::SecurityPolicyUnavailable);
        assert_eq!(reader.calls.load(Ordering::SeqCst), 0);
        assert_eq!(gate.calls.load(Ordering::SeqCst), 0);
    }
}
