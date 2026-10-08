#![cfg(unix)]

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use maulink_core::{
    AuthType, ConnectionMode, ConnectionRegistry, ConnectionState, CredentialCipher,
    CredentialManager, CredentialWorker, Database, ErrorCode, HostKeyCandidate, HostKeyDecision,
    HostKeyStore, HostKeyVerifier, PathEncoding, ProfileStore, Secret, SecureStore,
    SecureStoreError, ServerProfileInput, SshConnectionManager, SshConnector, StoredPath,
};
mod support;
use support::OpenSshFixture;
use tokio::time::{sleep, timeout};
use uuid::Uuid;
use zeroize::Zeroizing;

#[tokio::test]
#[ignore = "requires an isolated local OpenSSH fixture"]
async fn connects_to_isolated_openssh_and_checks_authentication_errors() {
    let (host, port, username, private_key) = fixture_config()
        .expect("MAULINK_OPENSSH_FIXTURE_HOST/PORT/USERNAME/PRIVATE_KEY must be set");
    let directory = tempfile::tempdir().expect("temporary database directory");
    let database = Database::open(directory.path().join("openssh.sqlite3")).expect("database");
    let registry = ConnectionRegistry::default();
    let store = HostKeyStore::new(database);
    let verifier = HostKeyVerifier::new(store, registry.clone());
    let connector = SshConnector::new(registry.clone(), verifier);

    let connection = registry
        .create(Uuid::new_v4().to_string(), ConnectionMode::Test)
        .expect("create connection");
    let connection_id = connection.connection_id.clone();
    let connect_task = {
        let connector = connector.clone();
        let host = host.clone();
        tokio::spawn(async move { connector.connect_default(&connection_id, &host, port).await })
    };
    let challenge = wait_for_host_key(&registry, &connection.connection_id).await;
    assert_eq!(challenge.host, host);
    registry
        .respond_host_key(
            &connection.connection_id,
            &challenge.challenge_id,
            HostKeyDecision::TrustAndSave,
        )
        .expect("trust fixture host key");
    let mut session = connect_task
        .await
        .expect("connect task")
        .expect("SSH handshake");

    let key_contents = Zeroizing::new(fs::read_to_string(&private_key).expect("private key"));
    connector
        .authenticate_private_key(&mut session, username.clone(), key_contents, None)
        .await
        .expect("public-key authentication");
    let snapshot = registry
        .get(&connection.connection_id)
        .expect("ready snapshot");
    assert_eq!(snapshot.state, ConnectionState::Ready);
    let algorithms = snapshot
        .negotiated_algorithms
        .expect("negotiated algorithm snapshot");
    assert!(!algorithms.key_exchange.is_empty());
    assert!(!algorithms.host_key.is_empty());
    assert!(!algorithms.cipher.is_empty());
    session.disconnect().await.expect("disconnect");
    assert_eq!(
        registry
            .get(&connection.connection_id)
            .expect("closed snapshot")
            .state,
        ConnectionState::Closed
    );

    verify_trust_once_rejection_and_host_key_cancellation(&host, port).await;
    verify_changed_host_key_decisions(&host, port).await;
    verify_connection_refused().await;
    verify_manager_test_mode(&host, port, &username, &private_key).await;

    if let Some(bad_private_key) = optional_path("MAULINK_OPENSSH_INVALID_PRIVATE_KEY") {
        let (mut session, connection_id) =
            connect_with_trusted_host(&connector, &registry, &host, port).await;
        let bad_key =
            Zeroizing::new(fs::read_to_string(bad_private_key).expect("invalid test key"));
        let error = connector
            .authenticate_private_key(&mut session, username.clone(), bad_key, None)
            .await
            .expect_err("unlisted public key must be rejected");
        assert_eq!(error.code, ErrorCode::AuthFailed);
        assert_eq!(
            registry.get(&connection_id).expect("failed snapshot").state,
            ConnectionState::Failed
        );
        session.disconnect().await.expect("close failed session");
    }

    if env::var_os("MAULINK_OPENSSH_TEST_WRONG_PASSWORD").is_some() {
        let (mut session, connection_id) =
            connect_with_trusted_host(&connector, &registry, &host, port).await;
        let wrong_password =
            Secret::new(b"intentionally-wrong-password".to_vec()).expect("test password");
        let error = connector
            .authenticate_password(&mut session, username.clone(), Some(wrong_password))
            .await
            .expect_err("wrong password must be rejected");
        assert_eq!(error.code, ErrorCode::AuthFailed);
        assert_eq!(
            registry.get(&connection_id).expect("failed snapshot").state,
            ConnectionState::Failed
        );
        session.disconnect().await.expect("close password session");
    }

    if env::var_os("MAULINK_OPENSSH_TEST_ENCRYPTED_KEY_SUCCESS").is_some() {
        let encrypted_private_key = optional_path("MAULINK_OPENSSH_ENCRYPTED_PRIVATE_KEY")
            .expect("encrypted test key path");
        let (mut session, connection_id) =
            connect_with_trusted_host(&connector, &registry, &host, port).await;
        let key_contents =
            Zeroizing::new(fs::read_to_string(encrypted_private_key).expect("encrypted key"));
        let passphrase = Secret::new(b"fixture-passphrase".to_vec()).expect("test passphrase");
        connector
            .authenticate_private_key(
                &mut session,
                username.clone(),
                key_contents,
                Some(passphrase),
            )
            .await
            .expect("encrypted private-key authentication");
        assert_eq!(
            registry.get(&connection_id).expect("ready snapshot").state,
            ConnectionState::Ready
        );
        session
            .disconnect()
            .await
            .expect("close encrypted-key session");
    }

    if let Some(encrypted_private_key) = optional_path("MAULINK_OPENSSH_ENCRYPTED_PRIVATE_KEY") {
        let (mut session, connection_id) =
            connect_with_trusted_host(&connector, &registry, &host, port).await;
        let encrypted_key =
            Zeroizing::new(fs::read_to_string(encrypted_private_key).expect("encrypted test key"));
        let wrong_passphrase =
            Secret::new(b"intentionally-wrong-passphrase".to_vec()).expect("test passphrase");
        let error = connector
            .authenticate_private_key(
                &mut session,
                username,
                encrypted_key,
                Some(wrong_passphrase),
            )
            .await
            .expect_err("wrong passphrase must be classified");
        assert_eq!(error.code, ErrorCode::PassphraseInvalid);
        assert_eq!(
            registry.get(&connection_id).expect("failed snapshot").state,
            ConnectionState::Failed
        );
        session.disconnect().await.expect("close failed session");
    }
}

#[tokio::test]
#[ignore = "starts an isolated loopback OpenSSH service"]
async fn manager_authenticates_the_destination_through_a_jump_host() {
    let fixture = OpenSshFixture::start();
    let directory = tempfile::tempdir().expect("temporary database directory");
    let database = Database::open(directory.path().join("jump-host.sqlite3")).expect("database");
    let profiles = ProfileStore::new(database.clone());
    let credentials = CredentialManager::new(
        database.clone(),
        CredentialWorker::new(Arc::new(EmptySecureStore)).expect("credential worker"),
        CredentialCipher::open(directory.path().join("credential.key")).expect("cipher"),
    );
    let registry = ConnectionRegistry::default();
    let verifier = HostKeyVerifier::new(HostKeyStore::new(database), registry.clone());
    let connector = SshConnector::new(registry.clone(), verifier);
    let manager = SshConnectionManager::new(profiles, credentials, registry.clone(), connector);
    let mut input = profile_input(
        &fixture.host,
        fixture.port,
        &fixture.username,
        &fixture.private_key,
    );
    input.jump_host = Some(format!("{}@{}", fixture.username, fixture.host));
    input.jump_port = fixture.port;

    let started = manager
        .start_draft_test(input, None)
        .await
        .expect("start jump-host connection test");
    let challenge = wait_for_manager_host_key(&manager, &started.connection_id).await;
    assert_eq!(challenge.host, fixture.host);
    assert_eq!(challenge.port, fixture.port);
    manager
        .respond_host_key(
            &started.connection_id,
            &challenge.challenge_id,
            HostKeyDecision::TrustAndSave,
        )
        .expect("trust jump and destination fixture host key");
    let finished = wait_for_manager_terminal(&manager, &started.connection_id).await;
    assert_eq!(finished.state, ConnectionState::Closed);
}

async fn verify_trust_once_rejection_and_host_key_cancellation(host: &str, port: u16) {
    let directory = tempfile::tempdir().expect("temporary database directory");
    let database = Database::open(directory.path().join("trust-once.sqlite3")).expect("database");
    let registry = ConnectionRegistry::default();
    let verifier = HostKeyVerifier::new(HostKeyStore::new(database), registry.clone());
    let connector = SshConnector::new(registry.clone(), verifier);

    let first = registry
        .create(Uuid::new_v4().to_string(), ConnectionMode::Test)
        .expect("create trust-once connection");
    let first_task = spawn_connect(&connector, &first.connection_id, host, port);
    let first_challenge = wait_for_host_key(&registry, &first.connection_id).await;
    assert!(first_challenge.previous_fingerprint_sha256.is_none());
    registry
        .respond_host_key(
            &first.connection_id,
            &first_challenge.challenge_id,
            HostKeyDecision::TrustOnce,
        )
        .expect("trust host key once");
    let session = timeout(Duration::from_secs(10), first_task)
        .await
        .expect("trust-once connect within 10 seconds")
        .expect("join trust-once connection")
        .expect("trust-once SSH handshake");
    session
        .disconnect()
        .await
        .expect("disconnect trust-once session");

    let second = registry
        .create(Uuid::new_v4().to_string(), ConnectionMode::Test)
        .expect("create rejection connection");
    let second_task = spawn_connect(&connector, &second.connection_id, host, port);
    let second_challenge = wait_for_host_key(&registry, &second.connection_id).await;
    assert!(second_challenge.previous_fingerprint_sha256.is_none());
    registry
        .respond_host_key(
            &second.connection_id,
            &second_challenge.challenge_id,
            HostKeyDecision::Reject,
        )
        .expect("reject unknown host key");
    let result = timeout(Duration::from_secs(10), second_task)
        .await
        .expect("rejected connect within 10 seconds")
        .expect("join rejected connection");
    let error = expect_error(result, "rejected host key must stop the handshake");
    assert_eq!(error.code, ErrorCode::HostKeyRejected);
    assert_eq!(
        registry
            .get(&second.connection_id)
            .expect("rejected snapshot")
            .state,
        ConnectionState::Failed
    );

    let third = registry
        .create(Uuid::new_v4().to_string(), ConnectionMode::Test)
        .expect("create cancelled connection");
    let third_task = spawn_connect(&connector, &third.connection_id, host, port);
    let third_challenge = wait_for_host_key(&registry, &third.connection_id).await;
    registry
        .cancel(&third.connection_id)
        .expect("cancel pending trust");
    let result = timeout(Duration::from_secs(10), third_task)
        .await
        .expect("cancelled connect within 10 seconds")
        .expect("join cancelled connection");
    let error = expect_error(
        result,
        "cancelled host-key challenge must stop the handshake",
    );
    assert_eq!(error.code, ErrorCode::Cancelled);
    let cancelled_snapshot = registry
        .get(&third.connection_id)
        .expect("cancelled snapshot");
    assert_eq!(cancelled_snapshot.state, ConnectionState::Cancelled);
    assert!(cancelled_snapshot.host_key_challenge.is_none());
    assert_ne!(third_challenge.challenge_id, second_challenge.challenge_id);
}

async fn verify_changed_host_key_decisions(host: &str, port: u16) {
    let directory = tempfile::tempdir().expect("temporary database directory");
    let database = Database::open(directory.path().join("changed-key.sqlite3")).expect("database");
    let store = HostKeyStore::new(database);
    let stale_key = HostKeyCandidate::new(
        host,
        port,
        "ssh-ed25519".to_owned(),
        b"stale fixture host key".to_vec(),
    )
    .expect("stale key candidate");
    let old_fingerprint = stale_key.fingerprint_sha256.clone();
    store
        .trust(stale_key, None)
        .await
        .expect("pin stale host key");

    let registry = ConnectionRegistry::default();
    let verifier = HostKeyVerifier::new(store, registry.clone());
    let connector = SshConnector::new(registry.clone(), verifier);
    let connection = registry
        .create(Uuid::new_v4().to_string(), ConnectionMode::Test)
        .expect("create changed-key connection");
    let task = spawn_connect(&connector, &connection.connection_id, host, port);
    let challenge = wait_for_host_key(&registry, &connection.connection_id).await;
    assert_eq!(
        challenge.previous_fingerprint_sha256.as_deref(),
        Some(old_fingerprint.as_str())
    );
    registry
        .respond_host_key(
            &connection.connection_id,
            &challenge.challenge_id,
            HostKeyDecision::Reject,
        )
        .expect("reject changed host key");
    let result = timeout(Duration::from_secs(10), task)
        .await
        .expect("changed-key rejection within 10 seconds")
        .expect("join changed-key connection");
    let error = expect_error(result, "changed host key must stop the handshake");
    assert_eq!(error.code, ErrorCode::HostKeyChanged);
}

async fn verify_connection_refused() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("reserve local port");
    let port = listener.local_addr().expect("reserved address").port();
    drop(listener);

    let directory = tempfile::tempdir().expect("temporary database directory");
    let database = Database::open(directory.path().join("refused.sqlite3")).expect("database");
    let registry = ConnectionRegistry::default();
    let verifier = HostKeyVerifier::new(HostKeyStore::new(database), registry.clone());
    let connector = SshConnector::new(registry.clone(), verifier);
    let connection = registry
        .create(Uuid::new_v4().to_string(), ConnectionMode::Test)
        .expect("create refused connection");
    let result = timeout(
        Duration::from_secs(10),
        connector.connect(
            &connection.connection_id,
            "127.0.0.1",
            port,
            Duration::from_secs(2),
            Duration::from_secs(1),
        ),
    )
    .await
    .expect("refused connection within 10 seconds");
    let error = expect_error(result, "unused port must be refused");
    assert_eq!(error.code, ErrorCode::ConnectionRefused);
    assert_eq!(
        registry
            .get(&connection.connection_id)
            .expect("refused snapshot")
            .state,
        ConnectionState::Failed
    );
}

async fn verify_manager_test_mode(host: &str, port: u16, username: &str, private_key: &Path) {
    let directory = tempfile::tempdir().expect("temporary database directory");
    let database = Database::open(directory.path().join("manager-test.sqlite3")).expect("database");
    let profiles = ProfileStore::new(database.clone());
    let credentials = CredentialManager::new(
        database.clone(),
        CredentialWorker::new(Arc::new(EmptySecureStore)).expect("credential worker"),
        CredentialCipher::open(directory.path().join("credential.key")).expect("cipher"),
    );
    let registry = ConnectionRegistry::default();
    let verifier = HostKeyVerifier::new(HostKeyStore::new(database), registry.clone());
    let connector = SshConnector::new(registry.clone(), verifier);
    let manager = SshConnectionManager::new(profiles.clone(), credentials, registry, connector);
    let input = profile_input(host, port, username, private_key);
    let profile = profiles
        .create_server(input.clone())
        .await
        .expect("create temporary test profile");

    let started = manager
        .start(profile.id.clone(), profile.revision, ConnectionMode::Test)
        .await
        .expect("start saved-profile test connection");
    let challenge = wait_for_manager_host_key(&manager, &started.connection_id).await;
    manager
        .respond_host_key(
            &started.connection_id,
            &challenge.challenge_id,
            HostKeyDecision::TrustAndSave,
        )
        .expect("trust manager fixture host key");
    let finished = wait_for_manager_terminal(&manager, &started.connection_id).await;
    assert_eq!(finished.state, ConnectionState::Closed);
    assert!(!manager.server_in_use(&profile.id).expect("server usage"));

    let draft = manager
        .start_draft_test(input, None)
        .await
        .expect("start draft test connection");
    assert!(draft.server_id.is_none());
    let finished = wait_for_manager_terminal(&manager, &draft.connection_id).await;
    assert_eq!(finished.state, ConnectionState::Closed);
}

fn profile_input(host: &str, port: u16, username: &str, private_key: &Path) -> ServerProfileInput {
    use std::os::unix::ffi::OsStrExt;

    ServerProfileInput {
        name: Some("isolated OpenSSH fixture".to_owned()),
        host: host.to_owned(),
        port,
        username: username.to_owned(),
        auth_type: AuthType::PrivateKey,
        require_authentication: true,
        private_key_path: Some(StoredPath {
            bytes: private_key.as_os_str().as_bytes().to_vec(),
            encoding: PathEncoding::UnixBytes,
        }),
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

async fn wait_for_manager_host_key(
    manager: &SshConnectionManager,
    connection_id: &str,
) -> maulink_core::HostKeyChallenge {
    timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = manager.get(connection_id).expect("manager snapshot");
            if let Some(challenge) = snapshot.host_key_challenge {
                return challenge;
            }
            assert!(
                !snapshot.state.is_terminal(),
                "manager test failed before Host Key challenge: {snapshot:?}"
            );
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("manager Host Key challenge within 10 seconds")
}

async fn wait_for_manager_terminal(
    manager: &SshConnectionManager,
    connection_id: &str,
) -> maulink_core::ConnectionSnapshot {
    timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = manager.get(connection_id).expect("manager snapshot");
            if snapshot.state.is_terminal() {
                return snapshot;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("manager Test Connection completes within 10 seconds")
}

struct EmptySecureStore;

impl SecureStore for EmptySecureStore {
    fn save(&self, _credential_id: &str, _secret: &Secret) -> Result<(), SecureStoreError> {
        Ok(())
    }

    fn load(&self, _credential_id: &str) -> Result<Secret, SecureStoreError> {
        Err(SecureStoreError::NotFound)
    }

    fn delete(&self, _credential_id: &str) -> Result<(), SecureStoreError> {
        Ok(())
    }
}

fn expect_error<T>(
    result: Result<T, maulink_core::AppError>,
    message: &str,
) -> maulink_core::AppError {
    match result {
        Ok(_) => panic!("{message}"),
        Err(error) => error,
    }
}

fn spawn_connect(
    connector: &SshConnector,
    connection_id: &str,
    host: &str,
    port: u16,
) -> tokio::task::JoinHandle<Result<maulink_core::SshSession, maulink_core::AppError>> {
    let connector = connector.clone();
    let connection_id = connection_id.to_owned();
    let host = host.to_owned();
    tokio::spawn(async move { connector.connect_default(&connection_id, &host, port).await })
}

fn fixture_config() -> Option<(String, u16, String, PathBuf)> {
    Some((
        env::var("MAULINK_OPENSSH_FIXTURE_HOST").ok()?,
        env::var("MAULINK_OPENSSH_FIXTURE_PORT")
            .ok()?
            .parse()
            .ok()?,
        env::var("MAULINK_OPENSSH_FIXTURE_USERNAME").ok()?,
        PathBuf::from(env::var_os("MAULINK_OPENSSH_FIXTURE_PRIVATE_KEY")?),
    ))
}

fn optional_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
}

async fn wait_for_host_key(
    registry: &ConnectionRegistry,
    connection_id: &str,
) -> maulink_core::HostKeyChallenge {
    timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = registry.get(connection_id).expect("connection snapshot");
            if let Some(challenge) = snapshot.host_key_challenge {
                return challenge;
            }
            assert!(
                !snapshot.state.is_terminal(),
                "SSH failed before Host Key challenge: {snapshot:?}"
            );
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Host Key challenge within 10 seconds")
}

async fn connect_with_trusted_host(
    connector: &SshConnector,
    registry: &ConnectionRegistry,
    host: &str,
    port: u16,
) -> (maulink_core::SshSession, String) {
    let connection = registry
        .create(Uuid::new_v4().to_string(), ConnectionMode::Test)
        .expect("create follow-up connection");
    let session = timeout(
        Duration::from_secs(10),
        connector.connect_default(&connection.connection_id, host, port),
    )
    .await
    .expect("previously trusted host key within 10 seconds")
    .expect("trusted connection");
    (session, connection.connection_id)
}
