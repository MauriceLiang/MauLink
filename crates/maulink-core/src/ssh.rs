use std::{
    collections::HashMap,
    fs,
    future::Future,
    io,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use russh::{
    Disconnect,
    client::{self, Handle, Handler},
    keys::{self, PrivateKeyWithHashAlg},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpStream, lookup_host},
    sync::{Mutex, Notify},
};
use tokio_util::sync::CancellationToken;
use zeroize::{Zeroize, Zeroizing};

use crate::{
    AppError, AuthType, ConnectionMode, ConnectionRegistry, ConnectionSnapshot, ConnectionState,
    CredentialKind, CredentialManager, ErrorCode, HostKeyCandidate, HostKeyDecision,
    HostKeyVerifier, NegotiatedAlgorithms, ProfileStore, ProxyType, Secret,
    host_keys::normalize_host,
};

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_AUTH_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);
const CONNECT_BUDGET_TICK: Duration = Duration::from_millis(40);
const MAX_ACTIVE_CONNECTIONS: usize = 20;
const MAX_IN_FLIGHT_CONNECTIONS: usize = 4;
const MAX_PRIVATE_KEY_FILE_BYTES: u64 = 1024 * 1024;
const TERMINAL_CHANNEL_WINDOW_BYTES: u32 = 256 * 1024;
const TERMINAL_CHANNEL_BUFFER_MESSAGES: usize = 4;
const TERMINAL_OPEN_TIMEOUT: Duration = Duration::from_secs(10);
const SFTP_OPEN_TIMEOUT: Duration = Duration::from_secs(10);
const MONITOR_EXEC_OUTPUT_LIMIT: usize = 256 * 1024;
const SUDO_WRITE_TIMEOUT: Duration = Duration::from_secs(90);
const SUDO_EXEC_OUTPUT_LIMIT: usize = 16 * 1024;

#[derive(Clone)]
pub struct SshConnector {
    connections: ConnectionRegistry,
    host_keys: HostKeyVerifier,
}

pub struct SshSession {
    connection_id: String,
    server_host: String,
    handle: Handle<HostKeyHandler>,
    upstream_handles: Vec<Handle<HostKeyHandler>>,
    connections: ConnectionRegistry,
}

pub(crate) struct FixedExecOutput {
    pub(crate) stdout: String,
    pub(crate) exit_status: Option<u32>,
}

#[derive(Clone)]
pub struct SshConnectionManager {
    profiles: ProfileStore,
    credentials: CredentialManager,
    connections: ConnectionRegistry,
    connector: SshConnector,
    sessions: Arc<Mutex<HashMap<String, SshSession>>>,
    sessions_changed: Arc<Notify>,
    operations_lock: Arc<Mutex<()>>,
}

struct HostKeyHandler {
    connection_id: String,
    host: String,
    port: u16,
    resume_state: Option<ConnectionState>,
    connections: ConnectionRegistry,
    verifier: HostKeyVerifier,
}

struct SshConnectionProfile {
    host: String,
    port: u16,
    username: String,
    auth_type: AuthType,
    private_key_path: Option<PathBuf>,
    connect_timeout_ms: u32,
    keepalive_interval_seconds: u32,
    jump_host: Option<String>,
    jump_port: u16,
    proxy_type: Option<ProxyType>,
    proxy_host: Option<String>,
    proxy_port: Option<u16>,
}

#[derive(Clone, Copy)]
struct ProxyTransport<'a> {
    protocol: ProxyType,
    host: &'a str,
    port: u16,
}

struct StreamConnectOptions<'a> {
    connection_id: &'a str,
    host: &'a str,
    port: u16,
    keepalive_interval: Duration,
    cancellation: &'a CancellationToken,
    deadline: tokio::time::Instant,
}

impl SshConnectionProfile {
    fn connect_timeout(&self) -> Duration {
        Duration::from_millis(u64::from(self.connect_timeout_ms))
    }

    fn keepalive_interval(&self) -> Duration {
        Duration::from_secs(u64::from(self.keepalive_interval_seconds))
    }
}

struct ConnectionAttempt {
    profile: SshConnectionProfile,
    credential_server_id: Option<String>,
    credential: Option<Secret>,
}

#[derive(Debug, thiserror::Error)]
enum ClientHandlerError {
    #[error(transparent)]
    Ssh(#[from] russh::Error),
    #[error(transparent)]
    App(#[from] AppError),
}

impl SshConnector {
    pub fn new(connections: ConnectionRegistry, host_keys: HostKeyVerifier) -> Self {
        Self {
            connections,
            host_keys,
        }
    }

    pub async fn connect_default(
        &self,
        connection_id: &str,
        host: &str,
        port: u16,
    ) -> Result<SshSession, AppError> {
        self.connect(
            connection_id,
            host,
            port,
            DEFAULT_CONNECT_TIMEOUT,
            DEFAULT_KEEPALIVE_INTERVAL,
        )
        .await
    }

    pub async fn connect(
        &self,
        connection_id: &str,
        host: &str,
        port: u16,
        connect_timeout: Duration,
        keepalive_interval: Duration,
    ) -> Result<SshSession, AppError> {
        let result = self
            .connect_inner(
                connection_id,
                host,
                port,
                connect_timeout,
                keepalive_interval,
            )
            .await;
        if let Err(error) = &result
            && error.code != ErrorCode::Cancelled
        {
            let _ = self.connections.fail(connection_id, error.clone());
        }
        result
    }

    async fn connect_inner(
        &self,
        connection_id: &str,
        host: &str,
        port: u16,
        connect_timeout: Duration,
        keepalive_interval: Duration,
    ) -> Result<SshSession, AppError> {
        let normalized_host = normalize_host(host)?;
        if port == 0 {
            return Err(validation("port", "errors.portOutOfRange"));
        }
        self.connections
            .transition(connection_id, ConnectionState::Resolving)?;
        let cancellation = self.connections.cancellation_token(connection_id)?;
        let deadline = tokio::time::Instant::now() + connect_timeout;
        let addresses = resolve_addresses(&normalized_host, port, &cancellation, deadline).await?;
        if addresses.is_empty() {
            return Err(
                AppError::new(ErrorCode::DnsFailed, "errors.dnsFailed").with_stage("resolving")
            );
        }
        self.connections
            .transition(connection_id, ConnectionState::Connecting)?;

        let mut last_error = None;
        for address in addresses {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(connection_timeout("connecting"));
            }
            let handler = self.host_key_handler(connection_id, &normalized_host, port);
            let connecting = client::connect(client_config(keepalive_interval), address, handler);
            match connect_with_budget(
                connecting,
                &self.connections,
                connection_id,
                &cancellation,
                deadline,
            )
            .await
            {
                Ok(handle) => {
                    return Ok(SshSession {
                        connection_id: connection_id.to_owned(),
                        server_host: normalized_host,
                        handle,
                        upstream_handles: Vec::new(),
                        connections: self.connections.clone(),
                    });
                }
                Err(error) if error.code == ErrorCode::ConnectionRefused => {
                    last_error = Some(error);
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_error.unwrap_or_else(|| connection_timeout("connecting")))
    }

    fn host_key_handler(&self, connection_id: &str, host: &str, port: u16) -> HostKeyHandler {
        HostKeyHandler {
            connection_id: connection_id.to_owned(),
            host: host.to_owned(),
            port,
            resume_state: None,
            connections: self.connections.clone(),
            verifier: self.host_keys.clone(),
        }
    }

    async fn connect_via_proxy(
        &self,
        connection_id: &str,
        host: &str,
        port: u16,
        proxy: ProxyTransport<'_>,
        connect_timeout: Duration,
        keepalive_interval: Duration,
    ) -> Result<SshSession, AppError> {
        let normalized_host = normalize_host(host)?;
        let normalized_proxy_host = normalize_host(proxy.host)?;
        if port == 0 || proxy.port == 0 {
            return Err(validation("port", "errors.portOutOfRange"));
        }
        self.connections
            .transition(connection_id, ConnectionState::Resolving)?;
        let cancellation = self.connections.cancellation_token(connection_id)?;
        let deadline = tokio::time::Instant::now() + connect_timeout;
        let addresses =
            resolve_addresses(&normalized_proxy_host, proxy.port, &cancellation, deadline).await?;
        if addresses.is_empty() {
            return Err(AppError::new(ErrorCode::DnsFailed, "errors.dnsFailed")
                .with_stage("resolvingProxy"));
        }
        self.connections
            .transition(connection_id, ConnectionState::Connecting)?;
        let mut stream = connect_proxy_socket(addresses, &cancellation, deadline).await?;
        run_proxy_handshake(
            &mut stream,
            proxy.protocol,
            &normalized_host,
            port,
            &cancellation,
            deadline,
        )
        .await?;
        let handle = self
            .connect_stream_handle(
                StreamConnectOptions {
                    connection_id,
                    host: &normalized_host,
                    port,
                    keepalive_interval,
                    cancellation: &cancellation,
                    deadline,
                },
                stream,
            )
            .await?;
        Ok(SshSession {
            connection_id: connection_id.to_owned(),
            server_host: normalized_host,
            handle,
            upstream_handles: Vec::new(),
            connections: self.connections.clone(),
        })
    }

    async fn connect_over_stream<S>(
        &self,
        connection_id: &str,
        host: &str,
        port: u16,
        stream: S,
        connect_timeout: Duration,
        keepalive_interval: Duration,
    ) -> Result<SshSession, AppError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let normalized_host = normalize_host(host)?;
        if port == 0 {
            return Err(validation("port", "errors.portOutOfRange"));
        }
        let cancellation = self.connections.cancellation_token(connection_id)?;
        let deadline = tokio::time::Instant::now() + connect_timeout;
        let handle = self
            .connect_stream_handle(
                StreamConnectOptions {
                    connection_id,
                    host: &normalized_host,
                    port,
                    keepalive_interval,
                    cancellation: &cancellation,
                    deadline,
                },
                stream,
            )
            .await?;
        Ok(SshSession {
            connection_id: connection_id.to_owned(),
            server_host: normalized_host,
            handle,
            upstream_handles: Vec::new(),
            connections: self.connections.clone(),
        })
    }

    async fn connect_stream_handle<S>(
        &self,
        options: StreamConnectOptions<'_>,
        stream: S,
    ) -> Result<Handle<HostKeyHandler>, AppError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let connecting = client::connect_stream(
            client_config(options.keepalive_interval),
            stream,
            self.host_key_handler(options.connection_id, options.host, options.port),
        );
        connect_with_budget(
            connecting,
            &self.connections,
            options.connection_id,
            options.cancellation,
            options.deadline,
        )
        .await
    }

    pub async fn authenticate_password(
        &self,
        session: &mut SshSession,
        username: String,
        secret: Option<Secret>,
    ) -> Result<(), AppError> {
        let mut password = match secret {
            Some(secret) => {
                self.connections
                    .transition(&session.connection_id, ConnectionState::Authenticating)?;
                secret.into_utf8_string()?
            }
            None => self
                .connections
                .request_authentication_secret(&session.connection_id, CredentialKind::Password)
                .await?
                .into_utf8_string()?,
        };
        let cancellation = self
            .connections
            .cancellation_token(&session.connection_id)?;
        let result = run_authentication(
            &session.connection_id,
            &self.connections,
            &cancellation,
            session
                .handle
                .authenticate_password(username, std::mem::take(&mut *password)),
        )
        .await;
        self.finish_authentication(&session.connection_id, russh::MethodKind::Password, result)
    }

    pub async fn authenticate_private_key(
        &self,
        session: &mut SshSession,
        username: String,
        key_contents: Zeroizing<String>,
        passphrase: Option<Secret>,
    ) -> Result<(), AppError> {
        let was_encrypted = private_key_is_encrypted(key_contents.as_str());
        let passphrase = match passphrase {
            Some(secret) => Some(secret.into_utf8_string()?),
            None if was_encrypted => Some(
                self.connections
                    .request_authentication_secret(
                        &session.connection_id,
                        CredentialKind::Passphrase,
                    )
                    .await?
                    .into_utf8_string()?,
            ),
            None => {
                self.connections
                    .transition(&session.connection_id, ConnectionState::Authenticating)?;
                None
            }
        };
        if matches!(
            self.connections.get(&session.connection_id)?.state,
            ConnectionState::VerifyingHostKey | ConnectionState::Ready
        ) {
            self.connections
                .transition(&session.connection_id, ConnectionState::Authenticating)?;
        }
        let private_key = match keys::decode_secret_key(
            key_contents.as_str(),
            passphrase.as_deref().map(String::as_str),
        ) {
            Ok(key) => key,
            Err(error) => {
                let app_error = map_private_key_error(error, was_encrypted);
                let _ = self
                    .connections
                    .fail(&session.connection_id, app_error.clone());
                return Err(app_error);
            }
        };
        let hash = if private_key.algorithm().is_rsa() {
            let cancellation = self
                .connections
                .cancellation_token(&session.connection_id)?;
            match tokio::select! {
                _ = cancellation.cancelled() => Err(cancelled()),
                result = tokio::time::timeout(
                    DEFAULT_AUTH_TIMEOUT,
                    session.handle.best_supported_rsa_hash(),
                ) => match result {
                    Err(_) => Err(AppError::new(ErrorCode::AuthTimeout, "errors.authTimeout").with_stage("authenticating")),
                    Ok(Err(error)) => Err(map_ssh_error(error, "authenticating")),
                    Ok(Ok(Some(None))) => Err(AppError::new(
                        ErrorCode::AuthMethodUnsupported,
                        "errors.rsaSha2Unsupported",
                    ).with_stage("authenticating")),
                    Ok(Ok(Some(Some(hash)))) => Ok(hash),
                    Ok(Ok(None)) => Ok(russh::keys::HashAlg::Sha512),
                },
            } {
                Ok(hash) => Some(hash),
                Err(error) => {
                    let _ = self.connections.fail(&session.connection_id, error.clone());
                    return Err(error);
                }
            }
        } else {
            None
        };
        let key = PrivateKeyWithHashAlg::new(Arc::new(private_key), hash);
        let cancellation = self
            .connections
            .cancellation_token(&session.connection_id)?;
        let result = run_authentication(
            &session.connection_id,
            &self.connections,
            &cancellation,
            session.handle.authenticate_publickey(username, key),
        )
        .await;
        self.finish_authentication(&session.connection_id, russh::MethodKind::PublicKey, result)
    }

    fn finish_authentication(
        &self,
        connection_id: &str,
        attempted_method: russh::MethodKind,
        result: Result<russh::client::AuthResult, AppError>,
    ) -> Result<(), AppError> {
        match result {
            Ok(result) if result.success() => {
                self.connections
                    .transition(connection_id, ConnectionState::Ready)?;
                Ok(())
            }
            Ok(russh::client::AuthResult::Failure {
                partial_success: true,
                ..
            }) => {
                let error = AppError::new(
                    ErrorCode::AuthMethodUnsupported,
                    "errors.additionalAuthenticationRequired",
                )
                .with_stage("authenticating");
                let _ = self.connections.fail(connection_id, error.clone());
                Err(error)
            }
            Ok(russh::client::AuthResult::Failure {
                remaining_methods,
                partial_success: false,
            }) if !remaining_methods.contains(&attempted_method) => {
                let error = AppError::new(
                    ErrorCode::AuthMethodUnsupported,
                    "errors.authMethodUnsupported",
                )
                .with_stage("authenticating");
                let _ = self.connections.fail(connection_id, error.clone());
                Err(error)
            }
            Ok(_) => {
                let error = AppError::new(ErrorCode::AuthFailed, "errors.authFailed")
                    .with_stage("authenticating");
                let _ = self.connections.fail(connection_id, error.clone());
                Err(error)
            }
            Err(error) => {
                let _ = self.connections.fail(connection_id, error.clone());
                Err(error)
            }
        }
    }
}

impl SshConnectionManager {
    pub fn new(
        profiles: ProfileStore,
        credentials: CredentialManager,
        connections: ConnectionRegistry,
        connector: SshConnector,
    ) -> Self {
        Self {
            profiles,
            credentials,
            connections,
            connector,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            sessions_changed: Arc::new(Notify::new()),
            operations_lock: Arc::new(Mutex::new(())),
        }
    }

    pub async fn start(
        &self,
        server_id: String,
        expected_revision: u32,
        mode: ConnectionMode,
    ) -> Result<ConnectionSnapshot, AppError> {
        let _operation_guard = self.operations_lock.lock().await;
        let profile = self.profiles.get_server(server_id.clone()).await?;
        if profile.revision != expected_revision {
            return Err(
                AppError::new(ErrorCode::RevisionConflict, "errors.revisionConflict")
                    .with_param("expected", expected_revision.to_string())
                    .with_param("actual", profile.revision.to_string()),
            );
        }
        let private_key_path = if profile.auth_type == AuthType::PrivateKey {
            Some(
                self.profiles
                    .private_key_path_for_authentication(profile.id.clone(), profile.revision)
                    .await?,
            )
        } else {
            None
        };
        let attempt = ConnectionAttempt {
            profile: SshConnectionProfile {
                host: profile.host,
                port: profile.port,
                username: profile.username,
                auth_type: profile.auth_type,
                private_key_path,
                connect_timeout_ms: profile.connect_timeout_ms,
                keepalive_interval_seconds: profile.keepalive_interval_seconds,
                jump_host: profile.jump_host,
                jump_port: profile.jump_port,
                proxy_type: profile.proxy_type,
                proxy_host: profile.proxy_host,
                proxy_port: profile.proxy_port,
            },
            credential_server_id: Some(server_id.clone()),
            credential: None,
        };
        self.reserve_start(Some(&server_id)).await?;
        self.spawn_attempt(Some(server_id), attempt, mode)
    }

    pub async fn start_draft_test(
        &self,
        input: crate::ServerProfileInput,
        credential: Option<Secret>,
    ) -> Result<ConnectionSnapshot, AppError> {
        self.start_draft_test_from(input, credential, None).await
    }

    pub async fn start_draft_test_with_saved_profile(
        &self,
        input: crate::ServerProfileInput,
        server_id: String,
        expected_revision: u32,
        credential: Option<Secret>,
        use_saved_credential: bool,
    ) -> Result<ConnectionSnapshot, AppError> {
        self.start_draft_test_from(
            input,
            credential,
            Some((server_id, expected_revision, use_saved_credential)),
        )
        .await
    }

    async fn start_draft_test_from(
        &self,
        input: crate::ServerProfileInput,
        credential: Option<Secret>,
        saved_profile_source: Option<(String, u32, bool)>,
    ) -> Result<ConnectionSnapshot, AppError> {
        let _operation_guard = self.operations_lock.lock().await;
        let input = crate::profiles::validate_server_input(input)?;
        let saved_profile = if let Some((server_id, expected_revision, use_saved_credential)) =
            saved_profile_source
        {
            let server = self.profiles.get_server(server_id).await?;
            if server.revision != expected_revision {
                return Err(
                    AppError::new(ErrorCode::RevisionConflict, "errors.revisionConflict")
                        .with_param("expected", expected_revision.to_string())
                        .with_param("actual", server.revision.to_string()),
                );
            }
            if use_saved_credential
                && (server.host != input.host
                    || server.port != input.port
                    || server.username != input.username
                    || server.auth_type != input.auth_type)
            {
                return Err(AppError::new(
                    ErrorCode::ValidationFailed,
                    "errors.savedCredentialIdentityChanged",
                ));
            }
            Some((server, use_saved_credential))
        } else {
            None
        };
        let private_key_path = match input.private_key_path.clone() {
            Some(path) => Some(crate::profiles::native_path_from_stored(path)?),
            None if input.auth_type == AuthType::PrivateKey
                && saved_profile
                    .as_ref()
                    .is_some_and(|(server, _)| server.has_private_key) =>
            {
                let (server, _) = saved_profile.as_ref().expect("saved profile checked above");
                Some(
                    self.profiles
                        .private_key_path_for_authentication(server.id.clone(), server.revision)
                        .await?,
                )
            }
            None => None,
        };
        let attempt = ConnectionAttempt {
            profile: SshConnectionProfile {
                host: input.host,
                port: input.port,
                username: input.username,
                auth_type: input.auth_type,
                private_key_path,
                connect_timeout_ms: input.connect_timeout_ms,
                keepalive_interval_seconds: input.keepalive_interval_seconds,
                jump_host: input.jump_host,
                jump_port: input.jump_port,
                proxy_type: input.proxy_type,
                proxy_host: input.proxy_host,
                proxy_port: input.proxy_port,
            },
            credential_server_id: saved_profile
                .filter(|(_, use_saved_credential)| *use_saved_credential)
                .map(|(server, _)| server.id),
            credential,
        };
        self.reserve_start(None).await?;
        self.spawn_attempt(None, attempt, ConnectionMode::Test)
    }

    async fn reserve_start(&self, server_id: Option<&str>) -> Result<(), AppError> {
        self.prune_terminal_sessions().await;
        let active = self
            .connections
            .snapshots()?
            .into_iter()
            .filter(|entry| !entry.state.is_terminal())
            .collect::<Vec<_>>();
        if server_id.is_some_and(|server_id| {
            active
                .iter()
                .any(|entry| entry.server_id.as_deref() == Some(server_id))
        }) {
            return Err(
                AppError::new(ErrorCode::ServerInUse, "errors.serverAlreadyConnected")
                    .with_param("serverId", server_id.unwrap_or_default()),
            );
        }
        if active.len() >= MAX_ACTIVE_CONNECTIONS {
            return Err(
                AppError::new(ErrorCode::ResourceLimit, "errors.connectionLimitReached")
                    .with_param("limit", MAX_ACTIVE_CONNECTIONS.to_string()),
            );
        }
        let in_flight = active
            .iter()
            .filter(|entry| {
                matches!(
                    entry.state,
                    ConnectionState::Created
                        | ConnectionState::Resolving
                        | ConnectionState::Connecting
                        | ConnectionState::VerifyingHostKey
                        | ConnectionState::AwaitingHostTrust
                        | ConnectionState::AwaitingCredentials
                        | ConnectionState::Authenticating
                )
            })
            .count();
        if in_flight >= MAX_IN_FLIGHT_CONNECTIONS {
            return Err(AppError::new(
                ErrorCode::ResourceLimit,
                "errors.connectionAttemptLimitReached",
            )
            .with_param("limit", MAX_IN_FLIGHT_CONNECTIONS.to_string()));
        }
        Ok(())
    }

    fn spawn_attempt(
        &self,
        server_id: Option<String>,
        attempt: ConnectionAttempt,
        mode: ConnectionMode,
    ) -> Result<ConnectionSnapshot, AppError> {
        let connection = match server_id {
            Some(server_id) => self.connections.create(server_id, mode)?,
            None => self.connections.create_draft(mode)?,
        };
        let manager = self.clone();
        let connection_id = connection.connection_id.clone();
        tokio::spawn(async move {
            manager.run_attempt(connection_id, attempt, mode).await;
        });
        Ok(connection)
    }

    pub fn get(&self, connection_id: &str) -> Result<ConnectionSnapshot, AppError> {
        self.connections.get(connection_id)
    }

    pub(crate) fn connection_snapshots(&self) -> Result<Vec<ConnectionSnapshot>, AppError> {
        self.connections.snapshots()
    }

    pub(crate) async fn open_terminal_channel(
        &self,
        connection_id: &str,
        columns: u32,
        rows: u32,
        pixel_width: u32,
        pixel_height: u32,
    ) -> Result<(russh::ChannelReadHalf, russh::ChannelWriteHalf<client::Msg>), AppError> {
        let deadline = tokio::time::Instant::now() + TERMINAL_OPEN_TIMEOUT;
        loop {
            let snapshot = self.connections.get(connection_id)?;
            if snapshot.state != ConnectionState::Ready {
                return Err(AppError::new(
                    ErrorCode::ValidationFailed,
                    "errors.connectionNotReady",
                )
                .with_param("connectionId", connection_id));
            }
            let changed = self.sessions_changed.notified();
            let sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get(connection_id) {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    return Err(terminal_open_timeout());
                }
                return tokio::time::timeout(
                    remaining,
                    session.open_terminal_channel(columns, rows, pixel_width, pixel_height),
                )
                .await
                .map_err(|_| terminal_open_timeout())?;
            }
            drop(sessions);
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(terminal_open_timeout());
            }
            tokio::time::timeout(remaining, changed)
                .await
                .map_err(|_| terminal_open_timeout())?;
        }
    }

    pub(crate) async fn open_sftp_channel(
        &self,
        connection_id: &str,
    ) -> Result<russh::ChannelStream<client::Msg>, AppError> {
        let deadline = tokio::time::Instant::now() + SFTP_OPEN_TIMEOUT;
        loop {
            let snapshot = self.connections.get(connection_id)?;
            if snapshot.state != ConnectionState::Ready {
                return Err(AppError::new(
                    ErrorCode::ValidationFailed,
                    "errors.connectionNotReady",
                )
                .with_param("connectionId", connection_id));
            }
            let changed = self.sessions_changed.notified();
            let sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get(connection_id) {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    return Err(sftp_open_timeout());
                }
                return tokio::time::timeout(remaining, session.open_sftp_channel())
                    .await
                    .map_err(|_| sftp_open_timeout())?;
            }
            drop(sessions);
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(sftp_open_timeout());
            }
            tokio::time::timeout(remaining, changed)
                .await
                .map_err(|_| sftp_open_timeout())?;
        }
    }

    pub(crate) async fn run_fixed_command(
        &self,
        connection_id: &str,
        command: &str,
        timeout: Duration,
    ) -> Result<FixedExecOutput, AppError> {
        let cancellation = self.connections.cancellation_token(connection_id)?;
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let snapshot = self.connections.get(connection_id)?;
            if snapshot.state != ConnectionState::Ready {
                return Err(AppError::new(
                    ErrorCode::ValidationFailed,
                    "errors.connectionNotReady",
                )
                .with_param("connectionId", connection_id));
            }
            let changed = self.sessions_changed.notified();
            let sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get(connection_id) {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    return Err(monitor_timeout());
                }
                return tokio::select! {
                    _ = cancellation.cancelled() => Err(cancelled()),
                    result = tokio::time::timeout(
                        remaining,
                        session.execute_fixed_command(command, remaining, &cancellation),
                    ) => result.unwrap_or_else(|_| Err(monitor_timeout())),
                };
            }
            drop(sessions);
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(monitor_timeout());
            }
            tokio::select! {
                _ = cancellation.cancelled() => return Err(cancelled()),
                result = tokio::time::timeout(remaining, changed) => {
                    if result.is_err() {
                        return Err(monitor_timeout());
                    }
                },
            }
        }
    }

    pub(crate) async fn run_sudo_write(
        &self,
        connection_id: &str,
        command: &str,
        prompt_marker: &str,
        ready_marker: &str,
        password: Secret,
        content: &str,
    ) -> Result<(), AppError> {
        let cancellation = self.connections.cancellation_token(connection_id)?;
        let deadline = tokio::time::Instant::now() + SUDO_WRITE_TIMEOUT;
        let mut password = Some(password);
        loop {
            let snapshot = self.connections.get(connection_id)?;
            if snapshot.state != ConnectionState::Ready {
                return Err(AppError::new(
                    ErrorCode::ValidationFailed,
                    "errors.connectionNotReady",
                )
                .with_param("connectionId", connection_id));
            }
            let changed = self.sessions_changed.notified();
            let sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get(connection_id) {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    return Err(sudo_write_timeout());
                }
                return tokio::select! {
                    _ = cancellation.cancelled() => Err(cancelled()),
                    result = tokio::time::timeout(
                        remaining,
                        session.execute_sudo_write(
                            command,
                            prompt_marker,
                            ready_marker,
                            password.take().ok_or_else(sudo_authorization_failed)?,
                            content,
                            remaining,
                            &cancellation,
                        ),
                    ) => result.unwrap_or_else(|_| Err(sudo_write_timeout())),
                };
            }
            drop(sessions);
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(sudo_write_timeout());
            }
            tokio::select! {
                _ = cancellation.cancelled() => return Err(cancelled()),
                result = tokio::time::timeout(remaining, changed) => {
                    if result.is_err() {
                        return Err(sudo_write_timeout());
                    }
                },
            }
        }
    }

    pub async fn lock_profile_operations(&self) -> tokio::sync::OwnedMutexGuard<()> {
        self.operations_lock.clone().lock_owned().await
    }

    pub fn server_in_use(&self, server_id: &str) -> Result<bool, AppError> {
        Ok(self.connections.snapshots()?.into_iter().any(|entry| {
            entry.server_id.as_deref() == Some(server_id) && !entry.state.is_terminal()
        }))
    }

    pub fn ensure_server_idle(&self, server_id: &str) -> Result<(), AppError> {
        if self.server_in_use(server_id)? {
            return Err(AppError::new(ErrorCode::ServerInUse, "errors.serverInUse")
                .with_param("serverId", server_id));
        }
        Ok(())
    }

    pub fn respond_host_key(
        &self,
        connection_id: &str,
        challenge_id: &str,
        decision: HostKeyDecision,
    ) -> Result<(), AppError> {
        self.connections
            .respond_host_key(connection_id, challenge_id, decision)
    }

    pub fn respond_authentication(
        &self,
        connection_id: &str,
        challenge_id: &str,
        secret: Secret,
    ) -> Result<(), AppError> {
        self.connections
            .respond_authentication(connection_id, challenge_id, secret)
    }

    pub fn cancel(&self, connection_id: &str) -> Result<ConnectionSnapshot, AppError> {
        let snapshot = self.connections.get(connection_id)?;
        if snapshot.state.is_terminal() {
            return Ok(snapshot);
        }
        if snapshot.state == ConnectionState::Ready {
            return Err(AppError::new(
                ErrorCode::ValidationFailed,
                "errors.connectionAlreadyReady",
            ));
        }
        self.connections.cancel(connection_id)
    }

    pub async fn disconnect(&self, connection_id: &str) -> Result<ConnectionSnapshot, AppError> {
        let snapshot = self.connections.get(connection_id)?;
        if snapshot.state.is_terminal() {
            if let Some(session) = self.sessions.lock().await.remove(connection_id) {
                session.abort_transport().await;
            }
            return Ok(snapshot);
        }
        if snapshot.state != ConnectionState::Ready {
            return self.connections.cancel(connection_id);
        }
        let session = self.sessions.lock().await.remove(connection_id);
        match session {
            Some(session) => session.disconnect().await?,
            None => {
                self.connections.cancel(connection_id)?;
            }
        }
        self.connections.get(connection_id)
    }

    pub async fn shutdown(&self) {
        let snapshots = match self.connections.snapshots() {
            Ok(snapshots) => snapshots,
            Err(_) => return,
        };
        for snapshot in snapshots {
            if !snapshot.state.is_terminal() {
                let _ = self.disconnect(&snapshot.connection_id).await;
            }
        }
        let sessions = std::mem::take(&mut *self.sessions.lock().await);
        for (_, session) in sessions {
            session.abort_transport().await;
        }
    }

    async fn prune_terminal_sessions(&self) {
        let snapshots = match self.connections.snapshots() {
            Ok(snapshots) => snapshots,
            Err(_) => return,
        };
        let terminal_ids = snapshots
            .into_iter()
            .filter(|snapshot| snapshot.state.is_terminal())
            .map(|snapshot| snapshot.connection_id)
            .collect::<std::collections::HashSet<_>>();
        let mut sessions = self.sessions.lock().await;
        let expired = terminal_ids
            .into_iter()
            .filter_map(|id| sessions.remove(&id))
            .collect::<Vec<_>>();
        drop(sessions);
        for session in expired {
            session.abort_transport().await;
        }
    }

    async fn run_attempt(
        &self,
        connection_id: String,
        attempt: ConnectionAttempt,
        mode: ConnectionMode,
    ) {
        let result = self.connect_and_authenticate(&connection_id, attempt).await;
        let session = match result {
            Ok(session) => session,
            Err(error) => {
                let _ = self.connections.fail(&connection_id, error);
                return;
            }
        };
        if mode == ConnectionMode::Test {
            let _ = session.disconnect().await;
            return;
        }
        let mut sessions = self.sessions.lock().await;
        if self
            .connections
            .get(&connection_id)
            .is_ok_and(|snapshot| snapshot.state == ConnectionState::Ready)
        {
            sessions.insert(connection_id, session);
            self.sessions_changed.notify_one();
        } else {
            drop(sessions);
            let _ = session.disconnect().await;
        }
    }

    async fn connect_and_authenticate(
        &self,
        connection_id: &str,
        attempt: ConnectionAttempt,
    ) -> Result<SshSession, AppError> {
        let ConnectionAttempt {
            profile,
            credential_server_id,
            credential,
        } = attempt;
        if let Some(jump_host) = profile.jump_host.clone() {
            self.connect_through_jump(
                connection_id,
                profile,
                &jump_host,
                credential_server_id,
                credential,
            )
            .await
        } else {
            let proxy = proxy_transport(&profile)?;
            let mut session = match proxy {
                Some(proxy) => {
                    self.connector
                        .connect_via_proxy(
                            connection_id,
                            &profile.host,
                            profile.port,
                            proxy,
                            profile.connect_timeout(),
                            profile.keepalive_interval(),
                        )
                        .await?
                }
                None => {
                    self.connector
                        .connect(
                            connection_id,
                            &profile.host,
                            profile.port,
                            profile.connect_timeout(),
                            profile.keepalive_interval(),
                        )
                        .await?
                }
            };
            let authentication = async {
                let secret = self
                    .resolve_credential(credential, credential_server_id.as_deref())
                    .await?;
                let key_contents = self.read_profile_key(&profile).await?;
                self.authenticate_profile(
                    &mut session,
                    &profile,
                    profile.username.clone(),
                    secret,
                    key_contents.as_ref(),
                )
                .await
            }
            .await;
            match authentication {
                Ok(()) => Ok(session),
                Err(error) => {
                    session.abort_transport().await;
                    Err(error)
                }
            }
        }
    }

    async fn connect_through_jump(
        &self,
        connection_id: &str,
        profile: SshConnectionProfile,
        jump_host: &str,
        credential_server_id: Option<String>,
        credential: Option<Secret>,
    ) -> Result<SshSession, AppError> {
        let (jump_username, jump_address) = parse_jump_host(jump_host, &profile.username)?;
        let proxy = proxy_transport(&profile)?;
        let mut jump_session = match proxy {
            Some(proxy) => {
                self.connector
                    .connect_via_proxy(
                        connection_id,
                        &jump_address,
                        profile.jump_port,
                        proxy,
                        profile.connect_timeout(),
                        profile.keepalive_interval(),
                    )
                    .await?
            }
            None => {
                self.connector
                    .connect(
                        connection_id,
                        &jump_address,
                        profile.jump_port,
                        profile.connect_timeout(),
                        profile.keepalive_interval(),
                    )
                    .await?
            }
        };

        let authentication = async {
            let key_contents = self.read_profile_key(&profile).await?;
            let mut secret = self
                .resolve_credential(credential, credential_server_id.as_deref())
                .await?;
            let needs_secret = match profile.auth_type {
                AuthType::Password => true,
                AuthType::PrivateKey => key_contents
                    .as_ref()
                    .is_some_and(|contents| private_key_is_encrypted(contents.as_str())),
            };
            if secret.is_none() && needs_secret {
                let kind = match profile.auth_type {
                    AuthType::Password => CredentialKind::Password,
                    AuthType::PrivateKey => CredentialKind::Passphrase,
                };
                secret = Some(
                    self.connections
                        .request_authentication_secret(connection_id, kind)
                        .await?,
                );
            }
            self.authenticate_profile(
                &mut jump_session,
                &profile,
                jump_username,
                secret.as_ref().map(Secret::duplicate),
                key_contents.as_ref(),
            )
            .await?;

            let cancellation = self.connections.cancellation_token(connection_id)?;
            let timeout = profile.connect_timeout();
            let channel = tokio::select! {
                _ = cancellation.cancelled() => return Err(cancelled()),
                result = tokio::time::timeout(timeout, jump_session.handle.channel_open_direct_tcpip(
                    profile.host.clone(),
                    u32::from(profile.port),
                    "127.0.0.1",
                    0,
                )) => match result {
                    Err(_) => return Err(connection_timeout("openingJumpHostTunnel")),
                    Ok(Err(error)) => return Err(map_ssh_error(error, "openingJumpHostTunnel")),
                    Ok(Ok(channel)) => channel,
                }
            };
            let mut target_session = self
                .connector
                .connect_over_stream(
                    connection_id,
                    &profile.host,
                    profile.port,
                    channel.into_stream(),
                    timeout,
                    profile.keepalive_interval(),
                )
                .await?;
            if let Err(error) = self.authenticate_profile(
                &mut target_session,
                &profile,
                profile.username.clone(),
                secret.as_ref().map(Secret::duplicate),
                key_contents.as_ref(),
            )
            .await
            {
                target_session.abort_transport().await;
                return Err(error);
            }
            Ok(target_session)
        }
        .await;
        match authentication {
            Ok(mut session) => {
                session.upstream_handles.push(jump_session.handle);
                Ok(session)
            }
            Err(error) => {
                jump_session.abort_transport().await;
                Err(error)
            }
        }
    }

    async fn resolve_credential(
        &self,
        credential: Option<Secret>,
        credential_server_id: Option<&str>,
    ) -> Result<Option<Secret>, AppError> {
        match (credential, credential_server_id) {
            (Some(secret), _) => Ok(Some(secret)),
            (None, Some(server_id)) => match self
                .credentials
                .load_for_authentication(server_id.to_owned())
                .await
            {
                Ok(secret) => Ok(Some(secret)),
                Err(error)
                    if matches!(
                        error.code,
                        ErrorCode::CredentialNotFound | ErrorCode::CredentialAccessDenied
                    ) =>
                {
                    Ok(None)
                }
                Err(error) => Err(error.with_stage("authenticating")),
            },
            (None, None) => Ok(None),
        }
    }

    async fn read_profile_key(
        &self,
        profile: &SshConnectionProfile,
    ) -> Result<Option<Zeroizing<String>>, AppError> {
        match (profile.auth_type, profile.private_key_path.as_ref()) {
            (AuthType::PrivateKey, Some(path)) => read_private_key(path.clone()).await.map(Some),
            (AuthType::PrivateKey, None) => {
                Err(validation("privateKeyPath", "errors.privateKeyPathInvalid"))
            }
            (AuthType::Password, _) => Ok(None),
        }
    }

    async fn authenticate_profile(
        &self,
        session: &mut SshSession,
        profile: &SshConnectionProfile,
        username: String,
        secret: Option<Secret>,
        key_contents: Option<&Zeroizing<String>>,
    ) -> Result<(), AppError> {
        match profile.auth_type {
            AuthType::Password => {
                self.connector
                    .authenticate_password(session, username, secret)
                    .await
            }
            AuthType::PrivateKey => {
                let key_contents = key_contents
                    .ok_or_else(|| validation("privateKeyPath", "errors.privateKeyPathInvalid"))?;
                self.connector
                    .authenticate_private_key(
                        session,
                        username,
                        Zeroizing::new(key_contents.as_str().to_owned()),
                        secret,
                    )
                    .await
            }
        }
    }
}

fn proxy_transport(profile: &SshConnectionProfile) -> Result<Option<ProxyTransport<'_>>, AppError> {
    match (
        profile.proxy_type,
        profile.proxy_host.as_deref(),
        profile.proxy_port,
    ) {
        (None, None, None) => Ok(None),
        (Some(protocol), Some(host), Some(port)) if port > 0 => Ok(Some(ProxyTransport {
            protocol,
            host,
            port,
        })),
        _ => Err(validation("proxyHost", "errors.proxyConfigurationInvalid")),
    }
}

fn parse_jump_host(value: &str, fallback_username: &str) -> Result<(String, String), AppError> {
    let (username, host) = match value.split_once('@') {
        Some((username, host)) if !username.is_empty() && !host.is_empty() => {
            if username.chars().count() > 256 || username.contains('@') {
                return Err(validation("jumpHost", "errors.jumpHostInvalid"));
            }
            (username.to_owned(), host)
        }
        Some(_) => return Err(validation("jumpHost", "errors.jumpHostInvalid")),
        None => (fallback_username.to_owned(), value),
    };
    let host =
        normalize_host(host).map_err(|_| validation("jumpHost", "errors.jumpHostInvalid"))?;
    Ok((username, host))
}

impl SshSession {
    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    pub fn server_host(&self) -> &str {
        &self.server_host
    }

    pub async fn disconnect(self) -> Result<(), AppError> {
        let state = self.connections.get(&self.connection_id)?.state;
        if state.is_terminal() {
            let _ = self
                .handle
                .disconnect(Disconnect::ByApplication, "MauLink disconnected", "")
                .await;
            for handle in self.upstream_handles.iter().rev() {
                let _ = handle
                    .disconnect(Disconnect::ByApplication, "MauLink disconnected", "")
                    .await;
            }
            return Ok(());
        }
        self.connections
            .transition(&self.connection_id, ConnectionState::Disconnecting)?;
        let mut disconnect_error = self
            .handle
            .disconnect(Disconnect::ByApplication, "MauLink disconnected", "")
            .await;
        for handle in self.upstream_handles.iter().rev() {
            if let Err(error) = handle
                .disconnect(Disconnect::ByApplication, "MauLink disconnected", "")
                .await
                && disconnect_error.is_ok()
            {
                disconnect_error = Err(error);
            }
        }
        if let Err(error) = disconnect_error {
            let app_error = map_ssh_error(error, "disconnecting");
            let _ = self
                .connections
                .fail(&self.connection_id, app_error.clone());
            return Err(app_error);
        }
        self.connections
            .transition(&self.connection_id, ConnectionState::Closed)?;
        Ok(())
    }

    async fn open_terminal_channel(
        &self,
        columns: u32,
        rows: u32,
        pixel_width: u32,
        pixel_height: u32,
    ) -> Result<(russh::ChannelReadHalf, russh::ChannelWriteHalf<client::Msg>), AppError> {
        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(map_terminal_open_error)?;
        channel
            .request_pty(
                true,
                "xterm-256color",
                columns,
                rows,
                pixel_width,
                pixel_height,
                &[],
            )
            .await
            .map_err(map_terminal_open_error)?;
        wait_for_channel_success(&mut channel, "pty").await?;
        channel
            .request_shell(true)
            .await
            .map_err(map_terminal_open_error)?;
        wait_for_channel_success(&mut channel, "shell").await?;
        Ok(channel.split())
    }

    async fn open_sftp_channel(&self) -> Result<russh::ChannelStream<client::Msg>, AppError> {
        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|_| sftp_channel_open_failed())?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|_| sftp_channel_open_failed())?;
        wait_for_sftp_channel_success(&mut channel).await?;
        Ok(channel.into_stream())
    }

    async fn execute_fixed_command(
        &self,
        command: &str,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<FixedExecOutput, AppError> {
        let deadline = tokio::time::Instant::now() + timeout;
        let mut channel = tokio::select! {
            _ = cancellation.cancelled() => return Err(cancelled()),
            result = tokio::time::timeout(timeout, self.handle.channel_open_session()) => match result {
                Ok(Ok(channel)) => channel,
                Ok(Err(error)) => return Err(monitor_channel_error(error)),
                Err(_) => return Err(monitor_timeout()),
            },
        };

        let request = async {
            channel
                .exec(true, command.to_owned())
                .await
                .map_err(|error| map_ssh_error(error, "openingMonitorExec"))?;
            wait_for_exec_channel_success(&mut channel).await
        };
        let request_timeout = deadline.saturating_duration_since(tokio::time::Instant::now());
        let request_result = tokio::select! {
            _ = cancellation.cancelled() => Err(cancelled()),
            result = tokio::time::timeout(request_timeout, request) => {
                result.unwrap_or_else(|_| Err(monitor_timeout()))
            },
        };
        if let Err(error) = request_result {
            let _ = channel.close().await;
            return Err(error);
        }

        let read_output = async {
            let mut stdout = Vec::new();
            let mut total_output_bytes = 0_usize;
            let mut exit_status = None;
            loop {
                match channel.wait().await {
                    Some(russh::ChannelMsg::Data { data }) => {
                        total_output_bytes = total_output_bytes.saturating_add(data.len());
                        if total_output_bytes > MONITOR_EXEC_OUTPUT_LIMIT {
                            return Err(monitor_output_too_large());
                        }
                        stdout.extend_from_slice(&data);
                    }
                    Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                        total_output_bytes = total_output_bytes.saturating_add(data.len());
                        if total_output_bytes > MONITOR_EXEC_OUTPUT_LIMIT {
                            return Err(monitor_output_too_large());
                        }
                    }
                    Some(russh::ChannelMsg::ExitStatus {
                        exit_status: status,
                    }) => {
                        exit_status = Some(status);
                    }
                    Some(russh::ChannelMsg::Close) | None => break,
                    _ => {}
                }
            }
            let stdout = String::from_utf8(stdout).map_err(|_| {
                AppError::new(
                    ErrorCode::MonitorCollectionFailed,
                    "errors.monitorOutputEncodingInvalid",
                )
                .with_stage("readingMonitorOutput")
            })?;
            Ok(FixedExecOutput {
                stdout,
                exit_status,
            })
        };
        let read_timeout = deadline.saturating_duration_since(tokio::time::Instant::now());
        let result = tokio::select! {
            _ = cancellation.cancelled() => Err(cancelled()),
            result = tokio::time::timeout(read_timeout, read_output) => {
                result.unwrap_or_else(|_| Err(monitor_timeout()))
            },
        };
        if result.is_err() {
            let _ = channel.close().await;
        }
        result
    }

    async fn execute_sudo_write(
        &self,
        command: &str,
        prompt_marker: &str,
        ready_marker: &str,
        password: Secret,
        content: &str,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<(), AppError> {
        let password = password.into_utf8_string()?;
        if password.is_empty()
            || password
                .chars()
                .any(|character| matches!(character, '\n' | '\r' | '\0'))
        {
            return Err(validation("password", "errors.sudoPasswordInvalid"));
        }
        tokio::select! {
            _ = cancellation.cancelled() => Err(cancelled()),
            result = tokio::time::timeout(
                timeout,
                self.execute_sudo_write_inner(
                    command,
                    prompt_marker.as_bytes(),
                    ready_marker.as_bytes(),
                    password,
                    content,
                ),
            ) => result.unwrap_or_else(|_| Err(sudo_write_timeout())),
        }
    }

    async fn execute_sudo_write_inner(
        &self,
        command: &str,
        prompt_marker: &[u8],
        ready_marker: &[u8],
        password: Zeroizing<String>,
        content: &str,
    ) -> Result<(), AppError> {
        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(sudo_exec_error)?;
        channel
            .exec(true, command.to_owned())
            .await
            .map_err(|error| map_ssh_error(error, "openingSudoWrite"))?;
        wait_for_sudo_exec_success(&mut channel).await?;

        let mut password_line = Zeroizing::new(password.as_bytes().to_vec());
        password_line.push(b'\n');
        let mut password_sent = false;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        loop {
            let prompt_received = match channel.wait().await {
                Some(russh::ChannelMsg::Data { data }) => {
                    append_sudo_output(&mut stdout, &data)?;
                    contains_bytes(&stdout, prompt_marker)
                }
                Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                    append_sudo_output(&mut stderr, &data)?;
                    contains_bytes(&stderr, prompt_marker)
                }
                Some(russh::ChannelMsg::ExitStatus { .. })
                | Some(russh::ChannelMsg::Close)
                | None => return Err(sudo_authorization_failed()),
                _ => false,
            };
            if prompt_received {
                if password_sent {
                    let _ = channel.close().await;
                    return Err(sudo_authorization_failed());
                }
                let mut password_input = password_line.as_slice();
                channel
                    .data(&mut password_input)
                    .await
                    .map_err(|error| map_ssh_error(error, "sendingSudoPassword"))?;
                password_sent = true;
            }
            if contains_bytes(&stdout, ready_marker) {
                break;
            }
        }

        let mut content_input = content.as_bytes();
        channel
            .data(&mut content_input)
            .await
            .map_err(|error| map_ssh_error(error, "sendingSudoFileContent"))?;
        channel
            .eof()
            .await
            .map_err(|error| map_ssh_error(error, "closingSudoFileInput"))?;

        loop {
            match channel.wait().await {
                Some(russh::ChannelMsg::ExitStatus { exit_status }) => {
                    let _ = channel.close().await;
                    return if exit_status == 0 {
                        Ok(())
                    } else {
                        Err(sudo_write_failed())
                    };
                }
                Some(russh::ChannelMsg::Data { data }) => append_sudo_output(&mut stdout, &data)?,
                Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                    append_sudo_output(&mut stderr, &data)?;
                }
                Some(russh::ChannelMsg::Close) | None => return Err(sudo_write_failed()),
                _ => {}
            }
        }
    }

    async fn abort_transport(&self) {
        let _ = self
            .handle
            .disconnect(Disconnect::ByApplication, "MauLink connection closed", "")
            .await;
        for handle in self.upstream_handles.iter().rev() {
            let _ = handle
                .disconnect(Disconnect::ByApplication, "MauLink connection closed", "")
                .await;
        }
    }
}

impl Handler for HostKeyHandler {
    type Error = ClientHandlerError;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        if server_public_key.certificate().is_some() {
            return Err(AppError::new(
                ErrorCode::HostKeyAlgorithmUnsupported,
                "errors.hostCertificateUnsupported",
            )
            .with_stage("verifyingHostKey")
            .into());
        }
        let previous_state = self.connections.get(&self.connection_id)?.state;
        self.resume_state = match previous_state {
            ConnectionState::Connecting => None,
            ConnectionState::Ready | ConnectionState::Authenticating => Some(previous_state),
            ConnectionState::VerifyingHostKey => self.resume_state,
            _ => {
                return Err(AppError::new(
                    ErrorCode::ValidationFailed,
                    "errors.hostKeyChallengeStateInvalid",
                )
                .into());
            }
        };
        if matches!(
            previous_state,
            ConnectionState::Connecting | ConnectionState::Ready | ConnectionState::Authenticating
        ) {
            self.connections
                .transition(&self.connection_id, ConnectionState::VerifyingHostKey)?;
        }
        let public_key = server_public_key.public_key();
        let candidate = HostKeyCandidate::new(
            &self.host,
            self.port,
            public_key.algorithm().to_string(),
            public_key.to_bytes().map_err(russh::Error::from)?,
        )?;
        self.verifier.verify(&self.connection_id, candidate).await?;
        if let Some(resume_state) = self.resume_state.take() {
            self.connections
                .transition(&self.connection_id, resume_state)?;
        }
        Ok(true)
    }

    async fn kex_done(
        &mut self,
        _shared_secret: Option<&[u8]>,
        names: &russh::Names,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        self.connections.set_negotiated_algorithms(
            &self.connection_id,
            NegotiatedAlgorithms {
                key_exchange: names.kex.as_ref().to_owned(),
                host_key: names.key.to_string(),
                cipher: names.cipher.as_ref().to_owned(),
                client_mac: names.client_mac.as_ref().to_owned(),
                server_mac: names.server_mac.as_ref().to_owned(),
                client_compression: format!("{:?}", names.client_compression).to_ascii_lowercase(),
                server_compression: format!("{:?}", names.server_compression).to_ascii_lowercase(),
            },
        )?;
        Ok(())
    }

    async fn disconnected(
        &mut self,
        reason: client::DisconnectReason<Self::Error>,
    ) -> Result<(), Self::Error> {
        if let Ok(snapshot) = self.connections.get(&self.connection_id)
            && !snapshot.state.is_terminal()
            && snapshot.state != ConnectionState::Disconnecting
        {
            let error = AppError::new(ErrorCode::ConnectionLost, "errors.connectionLost")
                .with_retry()
                .with_stage(if snapshot.state == ConnectionState::Ready {
                    "connected"
                } else {
                    "connecting"
                });
            let _ = self.connections.fail(&self.connection_id, error);
        }
        match reason {
            client::DisconnectReason::ReceivedDisconnect(_) => Ok(()),
            client::DisconnectReason::Error(error) => Err(error),
        }
    }
}

async fn resolve_addresses(
    host: &str,
    port: u16,
    cancellation: &CancellationToken,
    deadline: tokio::time::Instant,
) -> Result<Vec<SocketAddr>, AppError> {
    let addresses = if let Ok(ip) = host.parse::<IpAddr>() {
        vec![SocketAddr::new(ip, port)]
    } else {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(connection_timeout("resolving"));
        }
        let lookup = tokio::select! {
            _ = cancellation.cancelled() => return Err(cancelled()),
            result = tokio::time::timeout(remaining, lookup_host((host, port))) => {
                match result {
                    Err(_) => return Err(connection_timeout("resolving")),
                    Ok(Err(_)) => return Err(AppError::new(ErrorCode::DnsFailed, "errors.dnsFailed").with_stage("resolving")),
                    Ok(Ok(addresses)) => addresses.collect::<Vec<_>>(),
                }
            }
        };
        lookup
    };
    Ok(addresses)
}

fn client_config(keepalive_interval: Duration) -> Arc<client::Config> {
    let mut config = client::Config {
        keepalive_interval: Some(keepalive_interval),
        keepalive_max: 3,
        nodelay: true,
        window_size: TERMINAL_CHANNEL_WINDOW_BYTES,
        maximum_packet_size: 32 * 1024,
        channel_buffer_size: TERMINAL_CHANNEL_BUFFER_MESSAGES,
        ..client::Config::default()
    };
    // Keep RSA SHA-2 while removing russh's legacy SHA-1 ssh-rsa fallback.
    config.preferred.key = std::borrow::Cow::Owned(
        config
            .preferred
            .key
            .iter()
            .filter(|algorithm| {
                !matches!(
                    algorithm,
                    russh::keys::ssh_key::Algorithm::Rsa { hash: None }
                )
            })
            .cloned()
            .collect(),
    );
    Arc::new(config)
}

async fn connect_proxy_socket(
    addresses: Vec<SocketAddr>,
    cancellation: &CancellationToken,
    deadline: tokio::time::Instant,
) -> Result<TcpStream, AppError> {
    let mut last_error = None;
    for address in addresses {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(connection_timeout("connectingProxy"));
        }
        let result = tokio::select! {
            _ = cancellation.cancelled() => return Err(cancelled()),
            result = tokio::time::timeout(remaining, TcpStream::connect(address)) => result,
        };
        match result {
            Err(_) => return Err(connection_timeout("connectingProxy")),
            Ok(Ok(stream)) => {
                stream.set_nodelay(true).map_err(proxy_io_error)?;
                return Ok(stream);
            }
            Ok(Err(error)) if error.kind() == io::ErrorKind::ConnectionRefused => {
                last_error = Some(proxy_connection_refused());
            }
            Ok(Err(error)) => return Err(proxy_io_error(error)),
        }
    }
    Err(last_error.unwrap_or_else(|| connection_timeout("connectingProxy")))
}

async fn run_proxy_handshake<S>(
    stream: &mut S,
    protocol: ProxyType,
    target_host: &str,
    target_port: u16,
    cancellation: &CancellationToken,
    deadline: tokio::time::Instant,
) -> Result<(), AppError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    if remaining.is_zero() {
        return Err(connection_timeout("connectingProxy"));
    }
    let handshake = async {
        match protocol {
            ProxyType::Socks5 => socks5_connect(stream, target_host, target_port).await,
            ProxyType::HttpConnect => http_connect(stream, target_host, target_port).await,
        }
    };
    tokio::select! {
        _ = cancellation.cancelled() => Err(cancelled()),
        result = tokio::time::timeout(remaining, handshake) => match result {
            Err(_) => Err(connection_timeout("connectingProxy")),
            Ok(result) => result,
        }
    }
}

async fn socks5_connect<S>(
    stream: &mut S,
    target_host: &str,
    target_port: u16,
) -> Result<(), AppError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    stream.write_all(&[5, 1, 0]).await.map_err(proxy_io_error)?;
    let mut greeting = [0; 2];
    stream
        .read_exact(&mut greeting)
        .await
        .map_err(proxy_io_error)?;
    if greeting != [5, 0] {
        return Err(proxy_handshake_failed());
    }

    let mut request = vec![5, 1, 0];
    if let Ok(address) = target_host.parse::<IpAddr>() {
        match address {
            IpAddr::V4(address) => {
                request.push(1);
                request.extend_from_slice(&address.octets());
            }
            IpAddr::V6(address) => {
                request.push(4);
                request.extend_from_slice(&address.octets());
            }
        }
    } else {
        let domain = proxy_domain_name(target_host)?;
        let domain = domain.as_bytes();
        let length = u8::try_from(domain.len()).map_err(|_| proxy_handshake_failed())?;
        request.extend_from_slice(&[3, length]);
        request.extend_from_slice(domain);
    }
    request.extend_from_slice(&target_port.to_be_bytes());
    stream.write_all(&request).await.map_err(proxy_io_error)?;

    let mut response = [0; 4];
    stream
        .read_exact(&mut response)
        .await
        .map_err(proxy_io_error)?;
    if response[0] != 5 || response[1] != 0 || response[2] != 0 {
        return Err(proxy_handshake_failed());
    }
    let address_length = match response[3] {
        1 => 4,
        4 => 16,
        3 => {
            let mut length = [0; 1];
            stream
                .read_exact(&mut length)
                .await
                .map_err(proxy_io_error)?;
            usize::from(length[0])
        }
        _ => return Err(proxy_handshake_failed()),
    };
    let mut bound_address = vec![0; address_length + 2];
    stream
        .read_exact(&mut bound_address)
        .await
        .map_err(proxy_io_error)?;
    Ok(())
}

async fn http_connect<S>(
    stream: &mut S,
    target_host: &str,
    target_port: u16,
) -> Result<(), AppError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let authority_host = match target_host.parse::<IpAddr>() {
        Ok(IpAddr::V4(address)) => address.to_string(),
        Ok(IpAddr::V6(address)) => format!("[{address}]"),
        Err(_) => proxy_domain_name(target_host)?,
    };
    let authority = format!("{authority_host}:{target_port}");
    let request = format!(
        "CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\nProxy-Connection: Keep-Alive\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(proxy_io_error)?;

    let mut headers = Vec::with_capacity(256);
    while headers.len() < 8192 {
        let byte = stream.read_u8().await.map_err(proxy_io_error)?;
        headers.push(byte);
        if headers.ends_with(b"\r\n\r\n") {
            let response = std::str::from_utf8(&headers).map_err(|_| proxy_handshake_failed())?;
            let status = response.lines().next().ok_or_else(proxy_handshake_failed)?;
            let mut status_parts = status.split_ascii_whitespace();
            let protocol = status_parts.next().unwrap_or_default();
            let code = status_parts.next().unwrap_or_default();
            if protocol.starts_with("HTTP/")
                && code
                    .parse::<u16>()
                    .is_ok_and(|status| (200..300).contains(&status))
            {
                return Ok(());
            }
            return Err(proxy_handshake_failed());
        }
    }
    Err(proxy_handshake_failed())
}

fn proxy_domain_name(host: &str) -> Result<String, AppError> {
    idna::domain_to_ascii(host).map_err(|_| proxy_handshake_failed())
}

fn proxy_connection_refused() -> AppError {
    AppError::new(ErrorCode::ConnectionRefused, "errors.proxyConnectionFailed")
        .with_retry()
        .with_stage("connectingProxy")
}

fn proxy_handshake_failed() -> AppError {
    AppError::new(ErrorCode::ConnectionLost, "errors.proxyHandshakeFailed")
        .with_stage("connectingProxy")
}

fn proxy_io_error(error: io::Error) -> AppError {
    if error.kind() == io::ErrorKind::TimedOut {
        connection_timeout("connectingProxy")
    } else if error.kind() == io::ErrorKind::ConnectionRefused {
        proxy_connection_refused()
    } else {
        proxy_handshake_failed()
    }
}

async fn connect_with_budget<F>(
    connecting: F,
    connections: &ConnectionRegistry,
    connection_id: &str,
    cancellation: &CancellationToken,
    deadline: tokio::time::Instant,
) -> Result<Handle<HostKeyHandler>, AppError>
where
    F: Future<Output = Result<Handle<HostKeyHandler>, ClientHandlerError>>,
{
    tokio::pin!(connecting);
    let mut remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    let mut last_tick = tokio::time::Instant::now();
    loop {
        if remaining.is_zero() {
            return Err(connection_timeout("connecting"));
        }
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => {
                let snapshot = connections.get(connection_id)?;
                return Err(snapshot.error.unwrap_or_else(cancelled));
            }
            result = &mut connecting => {
                return result.map_err(|error| match error {
                    ClientHandlerError::App(error) => error,
                    ClientHandlerError::Ssh(error) => map_ssh_error(error, "connecting"),
                });
            }
            _ = tokio::time::sleep(CONNECT_BUDGET_TICK.min(remaining)) => {
                let now = tokio::time::Instant::now();
                let current_state = connections.get(connection_id)?.state;
                if current_state != ConnectionState::AwaitingHostTrust {
                    remaining = remaining.saturating_sub(now.saturating_duration_since(last_tick));
                }
                last_tick = now;
            }
        }
    }
}

async fn run_authentication<F>(
    connection_id: &str,
    connections: &ConnectionRegistry,
    cancellation: &CancellationToken,
    authentication: F,
) -> Result<russh::client::AuthResult, AppError>
where
    F: Future<Output = Result<russh::client::AuthResult, russh::Error>>,
{
    tokio::select! {
        _ = cancellation.cancelled() => Err(cancelled()),
        result = tokio::time::timeout(DEFAULT_AUTH_TIMEOUT, authentication) => match result {
            Err(_) => Err(AppError::new(ErrorCode::AuthTimeout, "errors.authTimeout").with_stage("authenticating")),
            Ok(Err(error)) => Err(map_ssh_error(error, "authenticating")),
            Ok(Ok(result)) => {
                let snapshot = connections.get(connection_id)?;
                if snapshot.state.is_terminal() {
                    Err(snapshot.error.unwrap_or_else(cancelled))
                } else {
                    Ok(result)
                }
            }
        },
    }
}

fn map_private_key_error(error: keys::Error, was_encrypted: bool) -> AppError {
    let code = match error {
        keys::Error::KeyIsEncrypted => ErrorCode::PassphraseInvalid,
        keys::Error::UnsupportedKeyType { .. } => ErrorCode::KeyFormatUnsupported,
        _ if was_encrypted => ErrorCode::PassphraseInvalid,
        _ => ErrorCode::PrivateKeyUnreadable,
    };
    AppError::new(
        code,
        match code {
            ErrorCode::PassphraseInvalid => "errors.passphraseInvalid",
            ErrorCode::KeyFormatUnsupported => "errors.keyFormatUnsupported",
            _ => "errors.privateKeyUnreadable",
        },
    )
    .with_stage("authenticating")
}

fn private_key_is_encrypted(key_contents: &str) -> bool {
    if matches!(
        keys::decode_secret_key(key_contents, None),
        Err(keys::Error::KeyIsEncrypted)
    ) {
        return true;
    }
    key_contents.starts_with("-----BEGIN ENCRYPTED PRIVATE KEY-----")
        || (key_contents.starts_with("-----BEGIN RSA PRIVATE KEY-----")
            && key_contents.contains("Proc-Type: 4,ENCRYPTED"))
}

fn map_ssh_error(error: russh::Error, stage: &str) -> AppError {
    use std::io::ErrorKind;

    match error {
        russh::Error::IO(error) if error.kind() == ErrorKind::ConnectionRefused => {
            AppError::new(ErrorCode::ConnectionRefused, "errors.connectionRefused")
                .with_retry()
                .with_stage(stage)
        }
        russh::Error::IO(error) if error.kind() == ErrorKind::TimedOut => connection_timeout(stage),
        russh::Error::ConnectionTimeout => connection_timeout(stage),
        russh::Error::KeepaliveTimeout | russh::Error::InactivityTimeout | russh::Error::HUP => {
            AppError::new(ErrorCode::ConnectionLost, "errors.connectionLost")
                .with_retry()
                .with_stage(stage)
        }
        russh::Error::NoAuthMethod | russh::Error::UnsupportedAuthMethod => AppError::new(
            ErrorCode::AuthMethodUnsupported,
            "errors.authMethodUnsupported",
        )
        .with_stage(stage),
        _ => AppError::new(ErrorCode::ConnectionLost, "errors.sshProtocolFailed").with_stage(stage),
    }
}

async fn wait_for_channel_success(
    channel: &mut russh::Channel<client::Msg>,
    request: &'static str,
) -> Result<(), AppError> {
    loop {
        match channel.wait().await {
            Some(russh::ChannelMsg::Success) => return Ok(()),
            Some(russh::ChannelMsg::Failure) | Some(russh::ChannelMsg::Close) | None => {
                return Err(AppError::new(
                    ErrorCode::TerminalOpenFailed,
                    "errors.terminalOpenRejected",
                )
                .with_param("request", request)
                .with_stage("openingTerminal"));
            }
            Some(_) => {}
        }
    }
}

async fn wait_for_exec_channel_success(
    channel: &mut russh::Channel<client::Msg>,
) -> Result<(), AppError> {
    loop {
        match channel.wait().await {
            Some(russh::ChannelMsg::Success) => return Ok(()),
            Some(russh::ChannelMsg::Failure) | Some(russh::ChannelMsg::Close) | None => {
                return Err(AppError::new(
                    ErrorCode::ChannelOpenFailed,
                    "errors.monitorExecRejected",
                )
                .with_stage("openingMonitorExec"));
            }
            _ => {}
        }
    }
}

async fn wait_for_sudo_exec_success(
    channel: &mut russh::Channel<client::Msg>,
) -> Result<(), AppError> {
    loop {
        match channel.wait().await {
            Some(russh::ChannelMsg::Success) => return Ok(()),
            Some(russh::ChannelMsg::Failure) | Some(russh::ChannelMsg::Close) | None => {
                return Err(sudo_exec_rejected());
            }
            _ => {}
        }
    }
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn append_sudo_output(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), AppError> {
    if output.len().saturating_add(bytes.len()) > SUDO_EXEC_OUTPUT_LIMIT {
        return Err(sudo_write_failed());
    }
    output.extend_from_slice(bytes);
    Ok(())
}

fn monitor_channel_error(error: russh::Error) -> AppError {
    match error {
        russh::Error::ChannelOpenFailure(_) => AppError::new(
            ErrorCode::ChannelOpenFailed,
            "errors.monitorChannelOpenFailed",
        )
        .with_stage("openingMonitorExec"),
        error => map_ssh_error(error, "openingMonitorExec"),
    }
}

fn monitor_timeout() -> AppError {
    AppError::new(ErrorCode::MonitorTimeout, "errors.monitorTimeout")
        .with_stage("collectingMonitorMetrics")
        .with_retry()
}

fn monitor_output_too_large() -> AppError {
    AppError::new(
        ErrorCode::MonitorOutputTooLarge,
        "errors.monitorOutputTooLarge",
    )
    .with_stage("readingMonitorOutput")
}

fn sudo_exec_error(error: russh::Error) -> AppError {
    match error {
        russh::Error::ChannelOpenFailure(_) => sudo_exec_rejected(),
        error => map_ssh_error(error, "openingSudoWrite"),
    }
}

fn sudo_exec_rejected() -> AppError {
    AppError::new(ErrorCode::ChannelOpenFailed, "errors.sudoExecUnavailable")
        .with_stage("openingSudoWrite")
}

fn sudo_authorization_failed() -> AppError {
    AppError::new(
        ErrorCode::PermissionDenied,
        "errors.sudoAuthorizationFailed",
    )
    .with_stage("authorizingSudoWrite")
}

fn sudo_write_failed() -> AppError {
    AppError::new(ErrorCode::SftpOperationFailed, "errors.sudoWriteFailed")
        .with_stage("writingTextFileWithSudo")
}

fn sudo_write_timeout() -> AppError {
    AppError::new(ErrorCode::ConnectionTimeout, "errors.sudoWriteTimeout")
        .with_retry()
        .with_stage("writingTextFileWithSudo")
}

fn map_terminal_open_error(_error: russh::Error) -> AppError {
    AppError::new(ErrorCode::TerminalOpenFailed, "errors.terminalOpenFailed")
        .with_stage("openingTerminal")
}

fn terminal_open_timeout() -> AppError {
    AppError::new(ErrorCode::TerminalOpenFailed, "errors.terminalOpenTimeout")
        .with_stage("openingTerminal")
}

async fn wait_for_sftp_channel_success(
    channel: &mut russh::Channel<client::Msg>,
) -> Result<(), AppError> {
    loop {
        match channel.wait().await {
            Some(russh::ChannelMsg::Success) => return Ok(()),
            Some(russh::ChannelMsg::Failure) | Some(russh::ChannelMsg::Close) | None => {
                return Err(sftp_channel_open_failed());
            }
            Some(_) => {}
        }
    }
}

fn sftp_channel_open_failed() -> AppError {
    AppError::new(ErrorCode::ChannelOpenFailed, "errors.sftpChannelOpenFailed")
        .with_stage("openingSftp")
}

fn sftp_open_timeout() -> AppError {
    AppError::new(
        ErrorCode::ChannelOpenFailed,
        "errors.sftpChannelOpenTimeout",
    )
    .with_stage("openingSftp")
}

fn connection_timeout(stage: &str) -> AppError {
    AppError::new(ErrorCode::ConnectionTimeout, "errors.connectionTimeout")
        .with_retry()
        .with_stage(stage)
}

fn cancelled() -> AppError {
    AppError::new(ErrorCode::Cancelled, "errors.connectionCancelled")
}

fn validation(field: &'static str, message_key: &'static str) -> AppError {
    AppError::new(ErrorCode::ValidationFailed, message_key).with_param("field", field)
}

async fn read_private_key(path: PathBuf) -> Result<Zeroizing<String>, AppError> {
    tokio::task::spawn_blocking(move || {
        let metadata = fs::metadata(&path).map_err(|_| private_key_unreadable())?;
        if metadata.len() > MAX_PRIVATE_KEY_FILE_BYTES {
            return Err(private_key_unreadable());
        }
        let bytes = fs::read(path).map_err(|_| private_key_unreadable())?;
        match String::from_utf8(bytes) {
            Ok(contents) => Ok(Zeroizing::new(contents)),
            Err(error) => {
                let mut bytes = error.into_bytes();
                bytes.zeroize();
                Err(private_key_unreadable())
            }
        }
    })
    .await
    .map_err(|_| AppError::new(ErrorCode::Internal, "errors.privateKeyReadFailed"))?
}

fn private_key_unreadable() -> AppError {
    AppError::new(
        ErrorCode::PrivateKeyUnreadable,
        "errors.privateKeyUnreadable",
    )
    .with_stage("authenticating")
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn connect_cancellation_preserves_an_existing_terminal_error() {
        let connections = ConnectionRegistry::default();
        let connection = connections
            .create(Uuid::new_v4().to_string(), ConnectionMode::Test)
            .expect("create connection");
        connections
            .transition(&connection.connection_id, ConnectionState::Resolving)
            .expect("resolving");
        connections
            .transition(&connection.connection_id, ConnectionState::Connecting)
            .expect("connecting");
        let cancellation = connections
            .cancellation_token(&connection.connection_id)
            .expect("cancellation token");
        let connecting_id = connection.connection_id.clone();
        let connecting = connections.clone();
        let attempt = tokio::spawn(async move {
            connect_with_budget(
                std::future::pending::<Result<Handle<HostKeyHandler>, ClientHandlerError>>(),
                &connecting,
                &connecting_id,
                &cancellation,
                tokio::time::Instant::now() + Duration::from_secs(5),
            )
            .await
        });

        let rejection = AppError::new(ErrorCode::HostKeyRejected, "errors.hostKeyRejected");
        connections
            .fail(&connection.connection_id, rejection.clone())
            .expect("record host-key rejection");
        let result = attempt.await.expect("join connect attempt");
        let error = match result {
            Ok(_) => panic!("terminal error must stop connecting"),
            Err(error) => error,
        };
        assert_eq!(error.code, ErrorCode::HostKeyRejected);
        assert_eq!(
            connections
                .get(&connection.connection_id)
                .expect("failed snapshot")
                .error,
            Some(rejection)
        );
    }

    #[tokio::test]
    async fn socks5_proxy_opens_a_remote_dns_tunnel() {
        let (mut stream, mut proxy_stream) = tokio::io::duplex(4096);
        let proxy = tokio::spawn(async move {
            let mut greeting = [0; 3];
            proxy_stream
                .read_exact(&mut greeting)
                .await
                .expect("read greeting");
            assert_eq!(greeting, [5, 1, 0]);
            proxy_stream
                .write_all(&[5, 0])
                .await
                .expect("accept no-auth mode");

            let mut request_head = [0; 5];
            proxy_stream
                .read_exact(&mut request_head)
                .await
                .expect("read SOCKS request");
            assert_eq!(request_head, [5, 1, 0, 3, 11]);
            let mut destination = [0; 13];
            proxy_stream
                .read_exact(&mut destination)
                .await
                .expect("read destination and port");
            assert_eq!(&destination[..11], b"target.test");
            assert_eq!(&destination[11..], &22_u16.to_be_bytes());
            proxy_stream
                .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 22])
                .await
                .expect("accept tunnel");
        });
        run_proxy_handshake(
            &mut stream,
            ProxyType::Socks5,
            "target.test",
            22,
            &CancellationToken::new(),
            tokio::time::Instant::now() + Duration::from_secs(2),
        )
        .await
        .expect("complete SOCKS5 CONNECT");
        proxy.await.expect("SOCKS proxy task");
    }

    #[tokio::test]
    async fn http_connect_keeps_bytes_after_headers_for_the_ssh_handshake() {
        let (mut stream, mut proxy_stream) = tokio::io::duplex(4096);
        let proxy = tokio::spawn(async move {
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                request.push(proxy_stream.read_u8().await.expect("read CONNECT request"));
            }
            let request = String::from_utf8(request).expect("UTF-8 request");
            assert!(request.starts_with("CONNECT target.test:2222 HTTP/1.1\r\n"));
            proxy_stream
                .write_all(b"HTTP/1.1 200 Connection established\r\n\r\nSSH-")
                .await
                .expect("write proxy response and next protocol bytes");
        });
        run_proxy_handshake(
            &mut stream,
            ProxyType::HttpConnect,
            "target.test",
            2222,
            &CancellationToken::new(),
            tokio::time::Instant::now() + Duration::from_secs(2),
        )
        .await
        .expect("complete HTTP CONNECT");
        let mut ssh_prefix = [0; 4];
        stream
            .read_exact(&mut ssh_prefix)
            .await
            .expect("read first SSH bytes without proxy overread");
        assert_eq!(&ssh_prefix, b"SSH-");
        proxy.await.expect("HTTP proxy task");
    }

    #[tokio::test]
    async fn http_connect_rejects_non_success_responses() {
        let (mut stream, mut proxy_stream) = tokio::io::duplex(4096);
        let proxy = tokio::spawn(async move {
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                request.push(proxy_stream.read_u8().await.expect("read CONNECT request"));
            }
            proxy_stream
                .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
                .await
                .expect("write rejection");
        });
        let error = run_proxy_handshake(
            &mut stream,
            ProxyType::HttpConnect,
            "target.test",
            22,
            &CancellationToken::new(),
            tokio::time::Instant::now() + Duration::from_secs(2),
        )
        .await
        .expect_err("proxy authentication is unsupported");
        assert_eq!(error.message_key, "errors.proxyHandshakeFailed");
        proxy.await.expect("HTTP proxy task");
    }

    #[test]
    fn proxy_domain_names_use_idna_ascii() {
        assert_eq!(
            proxy_domain_name("bücher.example").expect("convert IDN host"),
            "xn--bcher-kva.example"
        );
    }
}
