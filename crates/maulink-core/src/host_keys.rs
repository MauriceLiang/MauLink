use std::net::IpAddr;

use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;

use crate::{
    AppError, ConnectionRegistry, Database, ErrorAction, ErrorCode, HostKeyDecision, storage,
};

const MAX_HOST_KEY_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostKeyCandidate {
    pub normalized_host: String,
    pub port: u16,
    pub algorithm: String,
    pub public_key_blob: Vec<u8>,
    pub fingerprint_sha256: String,
}

impl HostKeyCandidate {
    pub fn new(
        host: &str,
        port: u16,
        algorithm: String,
        public_key_blob: Vec<u8>,
    ) -> Result<Self, AppError> {
        let normalized_host = normalize_host(host)?;
        if port == 0 {
            return Err(validation("port", "errors.portOutOfRange"));
        }
        let algorithm = algorithm.trim().to_owned();
        if algorithm.is_empty()
            || algorithm.len() > 128
            || algorithm.chars().any(char::is_whitespace)
            || algorithm.chars().any(char::is_control)
        {
            return Err(validation("algorithm", "errors.hostKeyAlgorithmInvalid"));
        }
        if public_key_blob.is_empty() || public_key_blob.len() > MAX_HOST_KEY_BYTES {
            return Err(validation("publicKey", "errors.hostKeyInvalid"));
        }
        let fingerprint_sha256 = fingerprint(&public_key_blob);
        Ok(Self {
            normalized_host,
            port,
            algorithm,
            public_key_blob,
            fingerprint_sha256,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HostKeyRecord {
    pub normalized_host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint_sha256: String,
    pub revision: u32,
    #[ts(type = "number")]
    pub trusted_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKeyCheck {
    Unknown,
    Trusted(HostKeyRecord),
    Changed(HostKeyRecord),
}

#[derive(Clone)]
pub struct HostKeyStore {
    database: Database,
}

#[derive(Clone)]
pub struct HostKeyVerifier {
    store: HostKeyStore,
    connections: ConnectionRegistry,
}

impl HostKeyVerifier {
    pub fn new(store: HostKeyStore, connections: ConnectionRegistry) -> Self {
        Self { store, connections }
    }

    pub async fn verify(
        &self,
        connection_id: &str,
        candidate: HostKeyCandidate,
    ) -> Result<(), AppError> {
        let previous = match self.store.check(candidate.clone()).await? {
            HostKeyCheck::Trusted(_) => return Ok(()),
            HostKeyCheck::Unknown => None,
            HostKeyCheck::Changed(record) => Some(record),
        };
        let decision = self
            .connections
            .request_host_key_decision(
                connection_id,
                candidate.normalized_host.clone(),
                candidate.port,
                candidate.algorithm.clone(),
                candidate.fingerprint_sha256.clone(),
                previous.clone(),
            )
            .await?;
        match decision {
            HostKeyDecision::Reject => {
                let changed = previous.is_some();
                let mut error = AppError::new(
                    if changed {
                        ErrorCode::HostKeyChanged
                    } else {
                        ErrorCode::HostKeyRejected
                    },
                    if changed {
                        "errors.hostKeyChanged"
                    } else {
                        "errors.hostKeyRejected"
                    },
                )
                .with_stage("verifyingHostKey");
                if changed {
                    error.action = ErrorAction::ReviewHostKey;
                }
                Err(error)
            }
            HostKeyDecision::TrustOnce => Ok(()),
            HostKeyDecision::TrustAndSave => self
                .store
                .trust(candidate, previous.map(|record| record.revision))
                .await
                .map(|_| ()),
        }
    }
}

impl HostKeyStore {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn get(&self, host: &str, port: u16) -> Result<Option<HostKeyRecord>, AppError> {
        let normalized_host = normalize_host(host)?;
        if port == 0 {
            return Err(validation("port", "errors.portOutOfRange"));
        }
        self.database
            .execute(move |connection| {
                let stored = connection
                    .query_row(
                        "SELECT key_algorithm, public_key_blob, fingerprint_sha256, revision,
                                trusted_at_ms
                         FROM known_hosts WHERE normalized_host = ?1 AND port = ?2",
                        params![normalized_host, port],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, Vec<u8>>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, u32>(3)?,
                                row.get::<_, i64>(4)?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(storage::map_sqlite_error)?;
                let Some(stored) = stored else {
                    return Ok(None);
                };
                if stored.2 != fingerprint(&stored.1) {
                    return Err(storage_corrupt());
                }
                Ok(Some(HostKeyRecord {
                    normalized_host,
                    port,
                    algorithm: stored.0,
                    fingerprint_sha256: stored.2,
                    revision: stored.3,
                    trusted_at_ms: stored.4,
                }))
            })
            .await
    }

    pub async fn check(&self, candidate: HostKeyCandidate) -> Result<HostKeyCheck, AppError> {
        self.database
            .execute(move |connection| {
                let stored = connection
                    .query_row(
                        "SELECT key_algorithm, public_key_blob, fingerprint_sha256, revision,
                                trusted_at_ms
                         FROM known_hosts WHERE normalized_host = ?1 AND port = ?2",
                        params![candidate.normalized_host, candidate.port],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, Vec<u8>>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, u32>(3)?,
                                row.get::<_, i64>(4)?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(storage::map_sqlite_error)?;
                let Some(stored) = stored else {
                    return Ok(HostKeyCheck::Unknown);
                };
                if stored.2 != fingerprint(&stored.1) {
                    return Err(storage_corrupt());
                }
                let record = HostKeyRecord {
                    normalized_host: candidate.normalized_host,
                    port: candidate.port,
                    algorithm: stored.0.clone(),
                    fingerprint_sha256: stored.2,
                    revision: stored.3,
                    trusted_at_ms: stored.4,
                };
                if stored.0 == candidate.algorithm && stored.1 == candidate.public_key_blob {
                    Ok(HostKeyCheck::Trusted(record))
                } else {
                    Ok(HostKeyCheck::Changed(record))
                }
            })
            .await
    }

    pub async fn trust(
        &self,
        candidate: HostKeyCandidate,
        expected_revision: Option<u32>,
    ) -> Result<HostKeyRecord, AppError> {
        let now = storage::now_ms()?;
        self.database
            .execute(move |connection| {
                let transaction = connection
                    .transaction()
                    .map_err(storage::map_sqlite_error)?;
                let current_revision = transaction
                    .query_row(
                        "SELECT revision FROM known_hosts
                         WHERE normalized_host = ?1 AND port = ?2",
                        params![candidate.normalized_host, candidate.port],
                        |row| row.get::<_, u32>(0),
                    )
                    .optional()
                    .map_err(storage::map_sqlite_error)?;
                match (current_revision, expected_revision) {
                    (None, None) => {
                        transaction
                            .execute(
                                "INSERT INTO known_hosts
                                 (normalized_host, port, key_algorithm, public_key_blob,
                                  fingerprint_sha256, revision, trusted_at_ms)
                                 VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
                                params![
                                    candidate.normalized_host,
                                    candidate.port,
                                    candidate.algorithm,
                                    candidate.public_key_blob,
                                    candidate.fingerprint_sha256,
                                    now,
                                ],
                            )
                            .map_err(storage::map_sqlite_error)?;
                    }
                    (Some(actual), Some(expected)) if actual == expected => {
                        let changed = transaction
                            .execute(
                                "UPDATE known_hosts
                                 SET key_algorithm = ?3, public_key_blob = ?4,
                                     fingerprint_sha256 = ?5, revision = revision + 1,
                                     trusted_at_ms = ?6
                                 WHERE normalized_host = ?1 AND port = ?2 AND revision = ?7",
                                params![
                                    candidate.normalized_host,
                                    candidate.port,
                                    candidate.algorithm,
                                    candidate.public_key_blob,
                                    candidate.fingerprint_sha256,
                                    now,
                                    expected,
                                ],
                            )
                            .map_err(storage::map_sqlite_error)?;
                        if changed != 1 {
                            return Err(revision_conflict(expected, None));
                        }
                    }
                    (actual, expected) => return Err(revision_conflict_option(expected, actual)),
                }
                transaction.commit().map_err(storage::map_sqlite_error)?;
                Ok(HostKeyRecord {
                    normalized_host: candidate.normalized_host,
                    port: candidate.port,
                    algorithm: candidate.algorithm,
                    fingerprint_sha256: candidate.fingerprint_sha256,
                    revision: expected_revision.map_or(1, |revision| revision + 1),
                    trusted_at_ms: now,
                })
            })
            .await
    }
}

pub fn normalize_host(host: &str) -> Result<String, AppError> {
    let host = host.trim();
    if host.is_empty()
        || host.len() > 253
        || host.chars().any(char::is_whitespace)
        || host.chars().any(char::is_control)
        || host.contains(['/', '\\', '@'])
        || host.contains("://")
    {
        return Err(validation("host", "errors.hostInvalid"));
    }
    let unwrapped = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);
    if let Ok(address) = unwrapped.parse::<IpAddr>() {
        return Ok(address.to_string());
    }
    let normalized = host.trim_end_matches('.').to_ascii_lowercase();
    if normalized.is_empty() || normalized.contains(':') {
        return Err(validation("host", "errors.hostInvalid"));
    }
    Ok(normalized)
}

fn fingerprint(public_key_blob: &[u8]) -> String {
    format!(
        "SHA256:{}",
        STANDARD_NO_PAD.encode(Sha256::digest(public_key_blob))
    )
}

fn revision_conflict_option(expected: Option<u32>, actual: Option<u32>) -> AppError {
    let mut error = AppError::new(
        ErrorCode::RevisionConflict,
        "errors.hostKeyRevisionConflict",
    );
    error.action = ErrorAction::Reload;
    if let Some(expected) = expected {
        error = error.with_param("expectedRevision", expected.to_string());
    }
    if let Some(actual) = actual {
        error = error.with_param("actualRevision", actual.to_string());
    }
    error
}

fn revision_conflict(expected: u32, actual: Option<u32>) -> AppError {
    revision_conflict_option(Some(expected), actual)
}

fn validation(field: &'static str, message_key: &'static str) -> AppError {
    AppError::new(ErrorCode::ValidationFailed, message_key).with_param("field", field)
}

fn storage_corrupt() -> AppError {
    AppError::new(ErrorCode::Internal, "errors.storageDataInvalid").with_stage("hostKey")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConnectionMode, ConnectionState};
    use std::time::Duration;
    use uuid::Uuid;

    fn candidate(host: &str, key: &[u8]) -> HostKeyCandidate {
        HostKeyCandidate::new(host, 22, "ssh-ed25519".to_owned(), key.to_vec())
            .expect("host key candidate")
    }

    #[test]
    fn normalizes_dns_and_ip_endpoints() {
        assert_eq!(normalize_host("Example.COM.").expect("dns"), "example.com");
        assert_eq!(
            normalize_host("[2001:0db8::1]").expect("ipv6"),
            "2001:db8::1"
        );
        assert!(normalize_host("user@example.com").is_err());
    }

    #[tokio::test]
    async fn trust_check_change_and_cas_update() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("test.sqlite3")).expect("database");
        let store = HostKeyStore::new(database);
        let first = candidate("Example.COM", b"first-key");
        assert_eq!(
            store.check(first.clone()).await.expect("check"),
            HostKeyCheck::Unknown
        );

        let saved = store.trust(first.clone(), None).await.expect("trust first");
        assert_eq!(saved.normalized_host, "example.com");
        assert_eq!(saved.revision, 1);
        assert!(matches!(
            store.check(first).await.expect("check trusted"),
            HostKeyCheck::Trusted(record) if record.revision == 1
        ));

        let second = candidate("example.com.", b"second-key");
        assert!(matches!(
            store.check(second.clone()).await.expect("check changed"),
            HostKeyCheck::Changed(record) if record.fingerprint_sha256 == saved.fingerprint_sha256
        ));
        let stale = store
            .trust(second.clone(), Some(2))
            .await
            .expect_err("stale trust update");
        assert_eq!(stale.code, ErrorCode::RevisionConflict);

        let updated = store
            .trust(second.clone(), Some(1))
            .await
            .expect("update key");
        assert_eq!(updated.revision, 2);
        assert!(matches!(
            store.check(second).await.expect("check updated"),
            HostKeyCheck::Trusted(record) if record.revision == 2
        ));
    }

    #[tokio::test]
    async fn get_returns_safe_record_for_normalized_host_and_exact_port() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("test.sqlite3")).expect("database");
        let store = HostKeyStore::new(database);
        let saved = store
            .trust(candidate("Example.COM", b"saved-key"), None)
            .await
            .expect("trust key");

        assert_eq!(
            store.get("EXAMPLE.com.", 22).await.expect("lookup"),
            Some(saved)
        );
        assert_eq!(
            store.get("example.com", 2222).await.expect("other port"),
            None
        );
        assert_eq!(
            store
                .get("missing.example", 22)
                .await
                .expect("unknown host"),
            None
        );
        assert_eq!(
            store.get("bad/host", 22).await.unwrap_err().code,
            ErrorCode::ValidationFailed
        );
        assert_eq!(
            store.get("example.com", 0).await.unwrap_err().code,
            ErrorCode::ValidationFailed
        );
    }

    #[tokio::test]
    async fn verifier_requires_challenge_and_saves_only_after_response() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Database::open(directory.path().join("test.sqlite3")).expect("database");
        let store = HostKeyStore::new(database);
        let connections = ConnectionRegistry::new(4, Duration::from_secs(1));
        let verifier = HostKeyVerifier::new(store.clone(), connections.clone());
        let connection = connections
            .create(Uuid::new_v4().to_string(), ConnectionMode::Test)
            .expect("create connection");
        connections
            .transition(&connection.connection_id, ConnectionState::Resolving)
            .expect("resolving");
        connections
            .transition(&connection.connection_id, ConnectionState::Connecting)
            .expect("connecting");
        connections
            .transition(&connection.connection_id, ConnectionState::VerifyingHostKey)
            .expect("verifying");

        let candidate = candidate("example.test", b"verified-key");
        let verifying = verifier.clone();
        let candidate_for_task = candidate.clone();
        let connection_id = connection.connection_id.clone();
        let task =
            tokio::spawn(async move { verifying.verify(&connection_id, candidate_for_task).await });
        let challenge = tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(challenge) = connections
                    .get(&connection.connection_id)
                    .expect("snapshot")
                    .host_key_challenge
                {
                    return challenge;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("challenge published");
        assert_eq!(
            store.check(candidate.clone()).await.expect("not saved yet"),
            HostKeyCheck::Unknown
        );
        connections
            .respond_host_key(
                &connection.connection_id,
                &challenge.challenge_id,
                HostKeyDecision::TrustAndSave,
            )
            .expect("accept and save");
        task.await.expect("join verifier").expect("verify key");
        assert!(matches!(
            store.check(candidate).await.expect("saved key"),
            HostKeyCheck::Trusted(_)
        ));
    }
}
