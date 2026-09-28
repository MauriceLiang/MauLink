#![cfg(unix)]

mod support;

use std::{fs, time::Duration};

use maulink_core::{
    ConnectionMode, ConnectionRegistry, ConnectionState, Database, ErrorCode, HostKeyDecision,
    HostKeyStore, HostKeyVerifier, SshConnector,
};
use support::OpenSshFixture;
use tokio::time::{sleep, timeout};
use uuid::Uuid;
use zeroize::Zeroizing;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "starts an isolated loopback OpenSSH service"]
async fn rotates_host_key_and_handles_disconnect_and_keepalive_timeout() {
    let mut fixture = OpenSshFixture::start();
    let directory = tempfile::tempdir().expect("temporary database directory");
    let database = Database::open(directory.path().join("lifecycle.sqlite3")).expect("database");
    let registry = ConnectionRegistry::default();
    let verifier = HostKeyVerifier::new(HostKeyStore::new(database), registry.clone());
    let connector = SshConnector::new(registry.clone(), verifier);

    let (first, first_task) = spawn_connect(
        &connector,
        &registry,
        &fixture.host,
        fixture.port,
        Duration::from_secs(30),
    );
    let first_challenge = wait_for_host_key(&registry, &first).await;
    assert!(first_challenge.previous_fingerprint_sha256.is_none());
    registry
        .respond_host_key(
            &first,
            &first_challenge.challenge_id,
            HostKeyDecision::TrustAndSave,
        )
        .expect("save initial host key");
    let mut first_session = join_connect(first_task).await;
    authenticate(&connector, &mut first_session, &fixture).await;
    first_session
        .disconnect()
        .await
        .expect("disconnect initial session");

    fixture.rotate_host_key();
    let (rotated, rotated_task) = spawn_connect(
        &connector,
        &registry,
        &fixture.host,
        fixture.port,
        Duration::from_secs(30),
    );
    let rotated_challenge = wait_for_host_key(&registry, &rotated).await;
    assert_eq!(
        rotated_challenge.previous_fingerprint_sha256.as_deref(),
        Some(first_challenge.fingerprint_sha256.as_str())
    );
    assert_ne!(
        rotated_challenge.fingerprint_sha256,
        first_challenge.fingerprint_sha256
    );
    registry
        .respond_host_key(
            &rotated,
            &rotated_challenge.challenge_id,
            HostKeyDecision::TrustAndSave,
        )
        .expect("replace saved host key");
    let mut rotated_session = join_connect(rotated_task).await;
    authenticate(&connector, &mut rotated_session, &fixture).await;
    rotated_session
        .disconnect()
        .await
        .expect("disconnect rotated session");

    fixture.restart_with_rotated_host_key();
    let (trusted, trusted_task) = spawn_connect(
        &connector,
        &registry,
        &fixture.host,
        fixture.port,
        Duration::from_secs(30),
    );
    let mut trusted_session = join_connect(trusted_task).await;
    assert!(
        registry
            .get(&trusted)
            .expect("trusted snapshot")
            .host_key_challenge
            .is_none(),
        "saved rotated key must reconnect without a new challenge"
    );
    authenticate(&connector, &mut trusted_session, &fixture).await;
    fixture.stop();
    let disconnected = wait_for_state(&registry, &trusted, ConnectionState::Failed).await;
    assert_eq!(
        disconnected.error.expect("disconnect error").code,
        ErrorCode::ConnectionLost
    );

    fixture.restart_with_rotated_host_key();
    let (keepalive, keepalive_task) = spawn_connect(
        &connector,
        &registry,
        &fixture.host,
        fixture.port,
        Duration::from_millis(100),
    );
    let mut keepalive_session = join_connect(keepalive_task).await;
    authenticate(&connector, &mut keepalive_session, &fixture).await;
    fixture.freeze();
    let timed_out = wait_for_state(&registry, &keepalive, ConnectionState::Failed).await;
    assert_eq!(
        timed_out.error.expect("keepalive error").code,
        ErrorCode::ConnectionLost
    );
    fixture.kill_frozen();
}

fn spawn_connect(
    connector: &SshConnector,
    registry: &ConnectionRegistry,
    host: &str,
    port: u16,
    keepalive_interval: Duration,
) -> (
    String,
    tokio::task::JoinHandle<Result<maulink_core::SshSession, maulink_core::AppError>>,
) {
    let connection = registry
        .create(Uuid::new_v4().to_string(), ConnectionMode::Test)
        .expect("create connection");
    let connection_id = connection.connection_id.clone();
    let task_id = connection_id.clone();
    let connector = connector.clone();
    let host = host.to_owned();
    let task = tokio::spawn(async move {
        connector
            .connect(
                &task_id,
                &host,
                port,
                Duration::from_secs(10),
                keepalive_interval,
            )
            .await
    });
    (connection_id, task)
}

async fn join_connect(
    task: tokio::task::JoinHandle<Result<maulink_core::SshSession, maulink_core::AppError>>,
) -> maulink_core::SshSession {
    timeout(Duration::from_secs(10), task)
        .await
        .expect("connect within 10 seconds")
        .expect("join connect task")
        .expect("SSH handshake")
}

async fn authenticate(
    connector: &SshConnector,
    session: &mut maulink_core::SshSession,
    fixture: &OpenSshFixture,
) {
    let key = Zeroizing::new(fs::read_to_string(&fixture.private_key).expect("fixture key"));
    connector
        .authenticate_private_key(session, fixture.username.clone(), key, None)
        .await
        .expect("fixture authentication");
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

async fn wait_for_state(
    registry: &ConnectionRegistry,
    connection_id: &str,
    expected: ConnectionState,
) -> maulink_core::ConnectionSnapshot {
    timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = registry.get(connection_id).expect("connection snapshot");
            if snapshot.state == expected {
                return snapshot;
            }
            assert!(
                !snapshot.state.is_terminal(),
                "connection reached unexpected terminal state: {snapshot:?}"
            );
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("connection reaches expected state within 10 seconds")
}
