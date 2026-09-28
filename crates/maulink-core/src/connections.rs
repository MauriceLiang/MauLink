use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;
use uuid::Uuid;

use crate::{AppError, CredentialKind, ErrorAction, ErrorCode, HostKeyRecord, Secret, storage};

const DEFAULT_CONNECTION_CAPACITY: usize = 100;
const DEFAULT_CHALLENGE_TTL: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionMode {
    Workspace,
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionState {
    Created,
    Resolving,
    Connecting,
    VerifyingHostKey,
    AwaitingHostTrust,
    AwaitingCredentials,
    Authenticating,
    Ready,
    Disconnecting,
    Closed,
    Failed,
    Cancelled,
}

impl ConnectionState {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Closed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum HostKeyDecision {
    Reject,
    TrustOnce,
    TrustAndSave,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HostKeyChallenge {
    pub challenge_id: String,
    pub connection_id: String,
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint_sha256: String,
    pub previous_fingerprint_sha256: Option<String>,
    pub previous_revision: Option<u32>,
    #[ts(type = "number")]
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationChallenge {
    pub challenge_id: String,
    pub connection_id: String,
    pub credential_kind: CredentialKind,
    #[ts(type = "number")]
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NegotiatedAlgorithms {
    pub key_exchange: String,
    pub host_key: String,
    pub cipher: String,
    pub client_mac: String,
    pub server_mac: String,
    pub client_compression: String,
    pub server_compression: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSnapshot {
    pub connection_id: String,
    pub server_id: Option<String>,
    pub mode: ConnectionMode,
    pub state: ConnectionState,
    pub host_key_challenge: Option<HostKeyChallenge>,
    pub authentication_challenge: Option<AuthenticationChallenge>,
    pub negotiated_algorithms: Option<NegotiatedAlgorithms>,
    pub error: Option<AppError>,
    #[ts(type = "number")]
    pub created_at_ms: i64,
    #[ts(type = "number")]
    pub updated_at_ms: i64,
}

struct ConnectionEntry {
    snapshot: ConnectionSnapshot,
    cancellation: CancellationToken,
    host_key_responder: Option<oneshot::Sender<HostKeyDecision>>,
    authentication_responder: Option<oneshot::Sender<Secret>>,
}

struct RegistryState {
    entries: HashMap<String, ConnectionEntry>,
    insertion_order: VecDeque<String>,
}

#[derive(Clone)]
pub struct ConnectionRegistry {
    state: Arc<Mutex<RegistryState>>,
    capacity: usize,
    challenge_ttl: Duration,
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new(DEFAULT_CONNECTION_CAPACITY, DEFAULT_CHALLENGE_TTL)
    }
}

impl ConnectionRegistry {
    pub fn new(capacity: usize, challenge_ttl: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(RegistryState {
                entries: HashMap::new(),
                insertion_order: VecDeque::new(),
            })),
            capacity,
            challenge_ttl,
        }
    }

    pub fn create(
        &self,
        server_id: String,
        mode: ConnectionMode,
    ) -> Result<ConnectionSnapshot, AppError> {
        validate_uuid(&server_id, "serverId")?;
        self.create_entry(Some(server_id), mode)
    }

    pub fn create_draft(&self, mode: ConnectionMode) -> Result<ConnectionSnapshot, AppError> {
        self.create_entry(None, mode)
    }

    fn create_entry(
        &self,
        server_id: Option<String>,
        mode: ConnectionMode,
    ) -> Result<ConnectionSnapshot, AppError> {
        let now = storage::now_ms()?;
        let mut state = self.lock()?;
        prune_terminal_entries(&mut state, self.capacity);
        if self.capacity == 0 || state.entries.len() >= self.capacity {
            return Err(AppError::new(
                ErrorCode::ResourceLimit,
                "errors.connectionLimitReached",
            ));
        }
        let connection_id = Uuid::new_v4().to_string();
        let snapshot = ConnectionSnapshot {
            connection_id: connection_id.clone(),
            server_id,
            mode,
            state: ConnectionState::Created,
            host_key_challenge: None,
            authentication_challenge: None,
            negotiated_algorithms: None,
            error: None,
            created_at_ms: now,
            updated_at_ms: now,
        };
        state.insertion_order.push_back(connection_id.clone());
        state.entries.insert(
            connection_id,
            ConnectionEntry {
                snapshot: snapshot.clone(),
                cancellation: CancellationToken::new(),
                host_key_responder: None,
                authentication_responder: None,
            },
        );
        Ok(snapshot)
    }

    pub fn get(&self, connection_id: &str) -> Result<ConnectionSnapshot, AppError> {
        validate_uuid(connection_id, "connectionId")?;
        self.lock()?
            .entries
            .get(connection_id)
            .map(|entry| entry.snapshot.clone())
            .ok_or_else(connection_not_found)
    }

    pub fn snapshots(&self) -> Result<Vec<ConnectionSnapshot>, AppError> {
        let state = self.lock()?;
        Ok(state
            .insertion_order
            .iter()
            .filter_map(|id| state.entries.get(id).map(|entry| entry.snapshot.clone()))
            .collect())
    }

    pub fn set_negotiated_algorithms(
        &self,
        connection_id: &str,
        algorithms: NegotiatedAlgorithms,
    ) -> Result<(), AppError> {
        validate_uuid(connection_id, "connectionId")?;
        let now = storage::now_ms()?;
        let mut state = self.lock()?;
        let entry = state
            .entries
            .get_mut(connection_id)
            .ok_or_else(connection_not_found)?;
        if entry.snapshot.state.is_terminal() {
            return Ok(());
        }
        entry.snapshot.negotiated_algorithms = Some(algorithms);
        entry.snapshot.updated_at_ms = now;
        Ok(())
    }

    pub fn cancellation_token(&self, connection_id: &str) -> Result<CancellationToken, AppError> {
        validate_uuid(connection_id, "connectionId")?;
        self.lock()?
            .entries
            .get(connection_id)
            .map(|entry| entry.cancellation.clone())
            .ok_or_else(connection_not_found)
    }

    pub fn transition(
        &self,
        connection_id: &str,
        next: ConnectionState,
    ) -> Result<ConnectionSnapshot, AppError> {
        validate_uuid(connection_id, "connectionId")?;
        let now = storage::now_ms()?;
        let mut state = self.lock()?;
        let entry = state
            .entries
            .get_mut(connection_id)
            .ok_or_else(connection_not_found)?;
        if entry.snapshot.state == next {
            return Ok(entry.snapshot.clone());
        }
        if !transition_allowed(entry.snapshot.state, next) {
            return Err(AppError::new(
                ErrorCode::ValidationFailed,
                "errors.connectionTransitionInvalid",
            )
            .with_param("from", format!("{:?}", entry.snapshot.state))
            .with_param("to", format!("{next:?}")));
        }
        entry.snapshot.state = next;
        entry.snapshot.updated_at_ms = now;
        if next.is_terminal() {
            entry.cancellation.cancel();
            entry.snapshot.host_key_challenge = None;
            entry.snapshot.authentication_challenge = None;
            entry.host_key_responder = None;
            entry.authentication_responder = None;
        }
        Ok(entry.snapshot.clone())
    }

    pub fn fail(
        &self,
        connection_id: &str,
        error: AppError,
    ) -> Result<ConnectionSnapshot, AppError> {
        validate_uuid(connection_id, "connectionId")?;
        let now = storage::now_ms()?;
        let mut state = self.lock()?;
        let entry = state
            .entries
            .get_mut(connection_id)
            .ok_or_else(connection_not_found)?;
        if entry.snapshot.state.is_terminal() {
            return Ok(entry.snapshot.clone());
        }
        entry.snapshot.state = ConnectionState::Failed;
        entry.snapshot.error = Some(error);
        entry.snapshot.host_key_challenge = None;
        entry.snapshot.authentication_challenge = None;
        entry.snapshot.updated_at_ms = now;
        entry.host_key_responder = None;
        entry.authentication_responder = None;
        entry.cancellation.cancel();
        Ok(entry.snapshot.clone())
    }

    pub fn cancel(&self, connection_id: &str) -> Result<ConnectionSnapshot, AppError> {
        validate_uuid(connection_id, "connectionId")?;
        let now = storage::now_ms()?;
        let mut state = self.lock()?;
        let entry = state
            .entries
            .get_mut(connection_id)
            .ok_or_else(connection_not_found)?;
        if entry.snapshot.state.is_terminal() {
            return Ok(entry.snapshot.clone());
        }
        entry.cancellation.cancel();
        entry.snapshot.state = ConnectionState::Cancelled;
        entry.snapshot.host_key_challenge = None;
        entry.snapshot.authentication_challenge = None;
        entry.snapshot.updated_at_ms = now;
        entry.host_key_responder = None;
        entry.authentication_responder = None;
        Ok(entry.snapshot.clone())
    }

    pub async fn request_host_key_decision(
        &self,
        connection_id: &str,
        host: String,
        port: u16,
        algorithm: String,
        fingerprint_sha256: String,
        previous: Option<HostKeyRecord>,
    ) -> Result<HostKeyDecision, AppError> {
        validate_uuid(connection_id, "connectionId")?;
        let now = storage::now_ms()?;
        let ttl_ms = i64::try_from(self.challenge_ttl.as_millis()).unwrap_or(i64::MAX);
        let expires_at_ms = now.checked_add(ttl_ms).unwrap_or(i64::MAX);
        let challenge = HostKeyChallenge {
            challenge_id: Uuid::new_v4().to_string(),
            connection_id: connection_id.to_owned(),
            host,
            port,
            algorithm,
            fingerprint_sha256,
            previous_fingerprint_sha256: previous
                .as_ref()
                .map(|record| record.fingerprint_sha256.clone()),
            previous_revision: previous.as_ref().map(|record| record.revision),
            expires_at_ms,
        };
        let (sender, receiver) = oneshot::channel();
        let cancellation = {
            let mut state = self.lock()?;
            let entry = state
                .entries
                .get_mut(connection_id)
                .ok_or_else(connection_not_found)?;
            if entry.snapshot.state != ConnectionState::VerifyingHostKey {
                return Err(AppError::new(
                    ErrorCode::ValidationFailed,
                    "errors.hostKeyChallengeStateInvalid",
                ));
            }
            entry.snapshot.state = ConnectionState::AwaitingHostTrust;
            entry.snapshot.host_key_challenge = Some(challenge.clone());
            entry.snapshot.updated_at_ms = now;
            entry.host_key_responder = Some(sender);
            entry.cancellation.clone()
        };

        let outcome = tokio::select! {
            _ = cancellation.cancelled() => Err(AppError::new(ErrorCode::Cancelled, "errors.connectionCancelled")),
            _ = tokio::time::sleep(self.challenge_ttl) => Err(challenge_expired()),
            result = receiver => match result {
                Ok(decision) => Ok(decision),
                Err(_) if cancellation.is_cancelled() => {
                    Err(AppError::new(ErrorCode::Cancelled, "errors.connectionCancelled"))
                }
                Err(_) => Err(challenge_expired()),
            },
        };
        self.finish_host_key_challenge(connection_id, &challenge.challenge_id, &outcome)?;
        outcome
    }

    pub fn respond_host_key(
        &self,
        connection_id: &str,
        challenge_id: &str,
        decision: HostKeyDecision,
    ) -> Result<(), AppError> {
        validate_uuid(connection_id, "connectionId")?;
        validate_uuid(challenge_id, "challengeId")?;
        let now = storage::now_ms()?;
        let mut state = self.lock()?;
        let entry = state
            .entries
            .get_mut(connection_id)
            .ok_or_else(connection_not_found)?;
        let challenge = entry
            .snapshot
            .host_key_challenge
            .as_ref()
            .filter(|challenge| challenge.challenge_id == challenge_id)
            .ok_or_else(challenge_expired)?;
        if now >= challenge.expires_at_ms {
            entry.host_key_responder = None;
            entry.snapshot.host_key_challenge = None;
            entry.snapshot.state = ConnectionState::Failed;
            entry.snapshot.error = Some(challenge_expired());
            entry.snapshot.updated_at_ms = now;
            entry.cancellation.cancel();
            return Err(challenge_expired());
        }
        entry
            .host_key_responder
            .take()
            .ok_or_else(challenge_expired)?
            .send(decision)
            .map_err(|_| challenge_expired())
    }

    pub async fn request_authentication_secret(
        &self,
        connection_id: &str,
        credential_kind: CredentialKind,
    ) -> Result<Secret, AppError> {
        validate_uuid(connection_id, "connectionId")?;
        let now = storage::now_ms()?;
        let ttl_ms = i64::try_from(self.challenge_ttl.as_millis()).unwrap_or(i64::MAX);
        let challenge = AuthenticationChallenge {
            challenge_id: Uuid::new_v4().to_string(),
            connection_id: connection_id.to_owned(),
            credential_kind,
            expires_at_ms: now.checked_add(ttl_ms).unwrap_or(i64::MAX),
        };
        let (sender, receiver) = oneshot::channel();
        let cancellation = {
            let mut state = self.lock()?;
            let entry = state
                .entries
                .get_mut(connection_id)
                .ok_or_else(connection_not_found)?;
            if entry.snapshot.state != ConnectionState::VerifyingHostKey {
                return Err(AppError::new(
                    ErrorCode::ValidationFailed,
                    "errors.authChallengeStateInvalid",
                ));
            }
            entry.snapshot.state = ConnectionState::AwaitingCredentials;
            entry.snapshot.authentication_challenge = Some(challenge.clone());
            entry.snapshot.updated_at_ms = now;
            entry.authentication_responder = Some(sender);
            entry.cancellation.clone()
        };

        let outcome = tokio::select! {
            _ = cancellation.cancelled() => Err(AppError::new(ErrorCode::Cancelled, "errors.connectionCancelled")),
            _ = tokio::time::sleep(self.challenge_ttl) => Err(auth_challenge_expired()),
            result = receiver => match result {
                Ok(secret) => Ok(secret),
                Err(_) if cancellation.is_cancelled() => {
                    Err(AppError::new(ErrorCode::Cancelled, "errors.connectionCancelled"))
                }
                Err(_) => Err(auth_challenge_expired()),
            },
        };
        self.finish_authentication_challenge(connection_id, &challenge.challenge_id, &outcome)?;
        outcome
    }

    pub fn respond_authentication(
        &self,
        connection_id: &str,
        challenge_id: &str,
        secret: Secret,
    ) -> Result<(), AppError> {
        validate_uuid(connection_id, "connectionId")?;
        validate_uuid(challenge_id, "challengeId")?;
        let now = storage::now_ms()?;
        let mut state = self.lock()?;
        let entry = state
            .entries
            .get_mut(connection_id)
            .ok_or_else(connection_not_found)?;
        let challenge = entry
            .snapshot
            .authentication_challenge
            .as_ref()
            .filter(|challenge| challenge.challenge_id == challenge_id)
            .ok_or_else(auth_challenge_expired)?;
        if now >= challenge.expires_at_ms {
            entry.authentication_responder = None;
            entry.snapshot.authentication_challenge = None;
            entry.snapshot.state = ConnectionState::Failed;
            entry.snapshot.error = Some(auth_challenge_expired());
            entry.snapshot.updated_at_ms = now;
            entry.cancellation.cancel();
            return Err(auth_challenge_expired());
        }
        entry
            .authentication_responder
            .take()
            .ok_or_else(auth_challenge_expired)?
            .send(secret)
            .map_err(|_| auth_challenge_expired())
    }

    fn finish_authentication_challenge(
        &self,
        connection_id: &str,
        challenge_id: &str,
        outcome: &Result<Secret, AppError>,
    ) -> Result<(), AppError> {
        let now = storage::now_ms()?;
        let mut state = self.lock()?;
        let Some(entry) = state.entries.get_mut(connection_id) else {
            return Ok(());
        };
        if entry
            .snapshot
            .authentication_challenge
            .as_ref()
            .is_none_or(|challenge| challenge.challenge_id != challenge_id)
        {
            return Ok(());
        }
        entry.snapshot.authentication_challenge = None;
        entry.authentication_responder = None;
        entry.snapshot.updated_at_ms = now;
        match outcome {
            Ok(_) => entry.snapshot.state = ConnectionState::Authenticating,
            Err(error) if error.code == ErrorCode::Cancelled => {
                entry.snapshot.state = ConnectionState::Cancelled;
                entry.snapshot.error = None;
            }
            Err(error) => {
                entry.snapshot.state = ConnectionState::Failed;
                entry.snapshot.error = Some(error.clone());
                entry.cancellation.cancel();
            }
        }
        Ok(())
    }

    fn finish_host_key_challenge(
        &self,
        connection_id: &str,
        challenge_id: &str,
        outcome: &Result<HostKeyDecision, AppError>,
    ) -> Result<(), AppError> {
        let now = storage::now_ms()?;
        let mut state = self.lock()?;
        let Some(entry) = state.entries.get_mut(connection_id) else {
            return Ok(());
        };
        if entry
            .snapshot
            .host_key_challenge
            .as_ref()
            .is_none_or(|challenge| challenge.challenge_id != challenge_id)
        {
            return Ok(());
        }
        let host_key_changed = entry
            .snapshot
            .host_key_challenge
            .as_ref()
            .is_some_and(|challenge| challenge.previous_fingerprint_sha256.is_some());
        entry.snapshot.host_key_challenge = None;
        entry.snapshot.authentication_challenge = None;
        entry.host_key_responder = None;
        entry.authentication_responder = None;
        entry.snapshot.updated_at_ms = now;
        match outcome {
            Ok(HostKeyDecision::Reject) => {
                let mut error = if host_key_changed {
                    AppError::new(ErrorCode::HostKeyChanged, "errors.hostKeyChanged")
                } else {
                    AppError::new(ErrorCode::HostKeyRejected, "errors.hostKeyRejected")
                }
                .with_stage("verifyingHostKey");
                if error.code == ErrorCode::HostKeyChanged {
                    error.action = ErrorAction::ReviewHostKey;
                }
                entry.snapshot.state = ConnectionState::Failed;
                entry.snapshot.error = Some(error);
                entry.cancellation.cancel();
            }
            Ok(HostKeyDecision::TrustOnce | HostKeyDecision::TrustAndSave) => {
                entry.snapshot.state = ConnectionState::VerifyingHostKey;
            }
            Err(error) if error.code == ErrorCode::Cancelled => {
                entry.snapshot.state = ConnectionState::Cancelled;
                entry.snapshot.error = None;
            }
            Err(error) => {
                entry.snapshot.state = ConnectionState::Failed;
                entry.snapshot.error = Some(error.clone());
                entry.cancellation.cancel();
            }
        }
        Ok(())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, RegistryState>, AppError> {
        self.state.lock().map_err(|_| {
            AppError::new(ErrorCode::Internal, "errors.connectionRegistryUnavailable")
                .with_stage("connection")
        })
    }
}

fn prune_terminal_entries(state: &mut RegistryState, capacity: usize) {
    while state.entries.len() >= capacity {
        let Some(position) = state.insertion_order.iter().position(|id| {
            state
                .entries
                .get(id)
                .is_some_and(|entry| entry.snapshot.state.is_terminal())
        }) else {
            break;
        };
        if let Some(id) = state.insertion_order.remove(position) {
            state.entries.remove(&id);
        }
    }
}

fn transition_allowed(current: ConnectionState, next: ConnectionState) -> bool {
    if matches!(next, ConnectionState::Failed | ConnectionState::Cancelled) {
        return !current.is_terminal();
    }
    matches!(
        (current, next),
        (ConnectionState::Created, ConnectionState::Resolving)
            | (ConnectionState::Resolving, ConnectionState::Connecting)
            | (
                ConnectionState::Connecting,
                ConnectionState::VerifyingHostKey
            )
            | (ConnectionState::Ready, ConnectionState::VerifyingHostKey)
            | (
                ConnectionState::VerifyingHostKey,
                ConnectionState::AwaitingCredentials
            )
            | (
                ConnectionState::VerifyingHostKey,
                ConnectionState::Authenticating
            )
            | (
                ConnectionState::VerifyingHostKey,
                ConnectionState::Disconnecting
            )
            | (
                ConnectionState::AwaitingHostTrust,
                ConnectionState::Disconnecting
            )
            | (
                ConnectionState::AwaitingCredentials,
                ConnectionState::Authenticating
            )
            | (
                ConnectionState::AwaitingCredentials,
                ConnectionState::Disconnecting
            )
            | (ConnectionState::Authenticating, ConnectionState::Ready)
            | (
                ConnectionState::Authenticating,
                ConnectionState::Disconnecting
            )
            | (ConnectionState::Ready, ConnectionState::Disconnecting)
            | (ConnectionState::Disconnecting, ConnectionState::Closed)
            | (ConnectionState::Authenticating, ConnectionState::Closed)
    )
}

fn validate_uuid(value: &str, field: &'static str) -> Result<(), AppError> {
    Uuid::parse_str(value).map(|_| ()).map_err(|_| {
        AppError::new(ErrorCode::ValidationFailed, "errors.resourceIdInvalid")
            .with_param("field", field)
    })
}

fn connection_not_found() -> AppError {
    AppError::new(ErrorCode::ResourceNotFound, "errors.connectionNotFound")
}

fn challenge_expired() -> AppError {
    AppError::new(ErrorCode::ChallengeExpired, "errors.challengeExpired")
        .with_stage("verifyingHostKey")
}

fn auth_challenge_expired() -> AppError {
    AppError::new(ErrorCode::ChallengeExpired, "errors.challengeExpired")
        .with_stage("authenticating")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> ConnectionRegistry {
        ConnectionRegistry::new(3, Duration::from_secs(1))
    }

    fn reach_host_key_verification(
        registry: &ConnectionRegistry,
        connection_id: &str,
    ) -> Result<(), AppError> {
        registry.transition(connection_id, ConnectionState::Resolving)?;
        registry.transition(connection_id, ConnectionState::Connecting)?;
        registry.transition(connection_id, ConnectionState::VerifyingHostKey)?;
        Ok(())
    }

    async fn wait_for_challenge(
        registry: &ConnectionRegistry,
        connection_id: &str,
    ) -> HostKeyChallenge {
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(challenge) = registry
                    .get(connection_id)
                    .expect("connection snapshot")
                    .host_key_challenge
                {
                    return challenge;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("challenge published")
    }

    #[test]
    fn enforces_state_transitions_and_terminal_idempotency() {
        let registry = registry();
        let server_id = Uuid::new_v4().to_string();
        let connection = registry
            .create(server_id, ConnectionMode::Workspace)
            .expect("create connection");
        assert!(
            registry
                .transition(&connection.connection_id, ConnectionState::Ready)
                .is_err()
        );
        reach_host_key_verification(&registry, &connection.connection_id)
            .expect("reach host verification");
        registry
            .transition(&connection.connection_id, ConnectionState::Authenticating)
            .expect("authenticate");
        registry
            .transition(&connection.connection_id, ConnectionState::Ready)
            .expect("ready");
        registry
            .transition(&connection.connection_id, ConnectionState::Disconnecting)
            .expect("disconnect");
        registry
            .transition(&connection.connection_id, ConnectionState::Closed)
            .expect("close");
        let closed = registry
            .cancel(&connection.connection_id)
            .expect("terminal cancel is idempotent");
        assert_eq!(closed.state, ConnectionState::Closed);
    }

    #[test]
    fn capacity_never_evicts_active_connections() {
        let registry = ConnectionRegistry::new(1, Duration::from_secs(1));
        let first = registry
            .create(Uuid::new_v4().to_string(), ConnectionMode::Workspace)
            .expect("create first");
        let full = registry
            .create(Uuid::new_v4().to_string(), ConnectionMode::Workspace)
            .expect_err("active connection must not be evicted");
        assert_eq!(full.code, ErrorCode::ResourceLimit);

        registry.cancel(&first.connection_id).expect("cancel first");
        let second = registry
            .create(Uuid::new_v4().to_string(), ConnectionMode::Workspace)
            .expect("terminal connection can be pruned");
        assert_ne!(first.connection_id, second.connection_id);
        assert!(registry.get(&first.connection_id).is_err());
    }

    #[tokio::test]
    async fn host_key_challenge_accepts_once_and_rejects_late_response() {
        let registry = registry();
        let connection = registry
            .create(Uuid::new_v4().to_string(), ConnectionMode::Test)
            .expect("create connection");
        reach_host_key_verification(&registry, &connection.connection_id)
            .expect("reach host verification");
        let waiting_registry = registry.clone();
        let connection_id = connection.connection_id.clone();
        let waiter = tokio::spawn(async move {
            waiting_registry
                .request_host_key_decision(
                    &connection_id,
                    "example.test".to_owned(),
                    22,
                    "ssh-ed25519".to_owned(),
                    "SHA256:candidate".to_owned(),
                    None,
                )
                .await
        });
        let challenge = wait_for_challenge(&registry, &connection.connection_id).await;
        registry
            .respond_host_key(
                &connection.connection_id,
                &challenge.challenge_id,
                HostKeyDecision::TrustOnce,
            )
            .expect("respond");
        assert_eq!(
            waiter.await.expect("join waiter").expect("decision"),
            HostKeyDecision::TrustOnce
        );
        assert_eq!(
            registry
                .get(&connection.connection_id)
                .expect("snapshot after decision")
                .state,
            ConnectionState::VerifyingHostKey
        );
        let late = registry
            .respond_host_key(
                &connection.connection_id,
                &challenge.challenge_id,
                HostKeyDecision::TrustAndSave,
            )
            .expect_err("late response must fail");
        assert_eq!(late.code, ErrorCode::ChallengeExpired);
    }

    #[tokio::test]
    async fn cancellation_ends_waiting_challenge() {
        let registry = registry();
        let connection = registry
            .create(Uuid::new_v4().to_string(), ConnectionMode::Workspace)
            .expect("create connection");
        reach_host_key_verification(&registry, &connection.connection_id)
            .expect("reach host verification");
        let waiting_registry = registry.clone();
        let connection_id = connection.connection_id.clone();
        let waiter = tokio::spawn(async move {
            waiting_registry
                .request_host_key_decision(
                    &connection_id,
                    "example.test".to_owned(),
                    22,
                    "ssh-ed25519".to_owned(),
                    "SHA256:candidate".to_owned(),
                    None,
                )
                .await
        });
        let _ = wait_for_challenge(&registry, &connection.connection_id).await;
        registry
            .cancel(&connection.connection_id)
            .expect("cancel connection");
        let error = waiter
            .await
            .expect("join waiter")
            .expect_err("challenge cancelled");
        assert_eq!(error.code, ErrorCode::Cancelled);
        assert_eq!(
            registry
                .get(&connection.connection_id)
                .expect("cancelled snapshot")
                .state,
            ConnectionState::Cancelled
        );
    }

    #[tokio::test]
    async fn host_key_challenge_expires_and_cannot_be_reused() {
        let registry = ConnectionRegistry::new(2, Duration::from_millis(1));
        let connection = registry
            .create(Uuid::new_v4().to_string(), ConnectionMode::Workspace)
            .expect("create connection");
        reach_host_key_verification(&registry, &connection.connection_id)
            .expect("reach host verification");
        let error = registry
            .request_host_key_decision(
                &connection.connection_id,
                "example.test".to_owned(),
                22,
                "ssh-ed25519".to_owned(),
                "SHA256:candidate".to_owned(),
                None,
            )
            .await
            .expect_err("challenge must expire");
        assert_eq!(error.code, ErrorCode::ChallengeExpired);
        let snapshot = registry.get(&connection.connection_id).expect("snapshot");
        assert_eq!(snapshot.state, ConnectionState::Failed);
        assert!(snapshot.host_key_challenge.is_none());
    }
}
