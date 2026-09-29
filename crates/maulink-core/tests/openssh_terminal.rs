#![cfg(unix)]

mod support;

use std::{path::Path, sync::Arc, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use maulink_core::{
    AuthType, ConnectionMode, ConnectionState, CredentialManager, CredentialWorker, Database,
    DecimalU64, ErrorCode, HostKeyDecision, HostKeyStore, HostKeyVerifier, PathEncoding,
    ProfileStore, Secret, SecureStore, SecureStoreError, ServerProfileInput, SshConnectionManager,
    SshConnector, StoredPath, TerminalAckPayload, TerminalIdPayload, TerminalManager,
    TerminalOpenPayload, TerminalResizePayload, TerminalState, TerminalWritePayload,
};
use support::OpenSshFixture;
use tokio::time::{sleep, timeout};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "starts an isolated loopback OpenSSH service"]
async fn opens_two_pty_channels_and_keeps_them_isolated() {
    let fixture = OpenSshFixture::start();
    let directory = tempfile::tempdir().expect("temporary database directory");
    let database = Database::open(directory.path().join("terminal.sqlite3")).expect("database");
    let profiles = ProfileStore::new(database.clone());
    let credentials = CredentialManager::new(
        database.clone(),
        CredentialWorker::new(Arc::new(EmptySecureStore)).expect("credential worker"),
    );
    let registry = maulink_core::ConnectionRegistry::default();
    let verifier = HostKeyVerifier::new(HostKeyStore::new(database), registry.clone());
    let connector = SshConnector::new(registry.clone(), verifier);
    let connections = SshConnectionManager::new(profiles.clone(), credentials, registry, connector);
    let profile = profiles
        .create_server(profile_input(&fixture))
        .await
        .expect("create terminal fixture profile");
    let started = connections
        .start(
            profile.id.clone(),
            profile.revision,
            ConnectionMode::Workspace,
        )
        .await
        .expect("start workspace connection");
    let challenge = wait_for_host_key(&connections, &started.connection_id).await;
    connections
        .respond_host_key(
            &started.connection_id,
            &challenge.challenge_id,
            HostKeyDecision::TrustAndSave,
        )
        .expect("trust fixture host key");
    wait_for_ready(&connections, &started.connection_id).await;

    let terminals = TerminalManager::new(connections.clone());
    let (first, mut first_output) = terminals
        .open(open_payload(&started.connection_id))
        .await
        .expect("open first terminal");
    let (second, mut second_output) = terminals
        .open(open_payload(&started.connection_id))
        .await
        .expect("open second terminal");
    wait_for_shell_prompt(&terminals, &second, &mut second_output).await;

    let size = terminals
        .resize(TerminalResizePayload {
            terminal_id: second.terminal_id.clone(),
            columns: 100,
            rows: 40,
            pixel_width: Some(1_000),
            pixel_height: Some(800),
        })
        .await
        .expect("resize second terminal");
    assert_eq!(size.columns, 100);
    assert_eq!(size.rows, 40);

    let first_closed = terminals
        .close(&TerminalIdPayload {
            terminal_id: first.terminal_id.clone(),
        })
        .await
        .expect("close first terminal");
    assert_eq!(first_closed.state, TerminalState::Closed);
    timeout(Duration::from_secs(5), async {
        while first_output.recv().await.is_some() {}
    })
    .await
    .expect("first output closes after terminal close");
    assert_eq!(
        connections
            .get(&started.connection_id)
            .expect("connection remains available")
            .state,
        ConnectionState::Ready
    );

    assert_eq!(
        write(&terminals, &second.terminal_id, 2, b"must-not-run\n")
            .await
            .expect_err("input sequence cannot skip")
            .code,
        ErrorCode::TerminalInputSequenceInvalid
    );
    assert_eq!(
        terminals
            .ack(TerminalAckPayload {
                terminal_id: second.terminal_id.clone(),
                stream_id: uuid::Uuid::new_v4().to_string(),
                seq: DecimalU64(0),
            })
            .await
            .expect_err("ACK belongs to the opened stream")
            .code,
        ErrorCode::TerminalAckInvalid
    );
    write(&terminals, &second.terminal_id, 1, b"stty -echo\n")
        .await
        .expect("disable terminal echo");
    wait_for_shell_prompt(&terminals, &second, &mut second_output).await;
    let command = b"printf '\\155\\141\\165\\154\\151\\156\\153\\055\\164\\145\\162\\155\\151\\156\\141\\154\\055\\164\\167\\157\\n'; exit\n";
    let accepted = write(&terminals, &second.terminal_id, 2, command)
        .await
        .expect("write terminal command");
    assert!(!accepted.duplicate);
    let duplicate = write(&terminals, &second.terminal_id, 2, command)
        .await
        .expect("retry accepted input");
    assert!(duplicate.duplicate);

    let output = collect_output(&terminals, &second, &mut second_output).await;
    let duplicate_after_close = write(&terminals, &second.terminal_id, 2, b"ignored retry")
        .await
        .expect("accepted input retry remains idempotent after remote exit");
    assert!(duplicate_after_close.duplicate);
    assert_eq!(
        output
            .windows(b"maulink-terminal-two".len())
            .filter(|window| *window == b"maulink-terminal-two")
            .count(),
        1,
        "retried input must not execute twice: {}",
        String::from_utf8_lossy(&output)
    );
    let second_snapshot = terminals
        .get(&TerminalIdPayload {
            terminal_id: second.terminal_id,
        })
        .expect("second terminal snapshot");
    assert_eq!(second_snapshot.state, TerminalState::Closed);
    assert_eq!(second_snapshot.exit_status, Some(0));
    assert_eq!(second_snapshot.size, size);

    let (pressure, mut pressure_output) = terminals
        .open(open_payload(&started.connection_id))
        .await
        .expect("open high-output terminal");
    wait_for_shell_prompt(&terminals, &pressure, &mut pressure_output).await;
    write(&terminals, &pressure.terminal_id, 1, b"stty -echo\n")
        .await
        .expect("disable pressure terminal echo");
    wait_for_shell_prompt(&terminals, &pressure, &mut pressure_output).await;
    write(
        &terminals,
        &pressure.terminal_id,
        2,
        b"python3 -c \"import os; os.write(1, b'\\x01'*1000000)\"; exit\n",
    )
    .await
    .expect("start bounded high output");
    let first_pressure_chunk =
        wait_for_byte_without_ack(&terminals, &pressure, &mut pressure_output, 1).await;
    let first_pressure_bytes = BASE64
        .decode(&first_pressure_chunk.data_base64)
        .expect("first pressure chunk Base64");
    let pressure_snapshot = wait_for_terminal_state_within(
        &terminals,
        &pressure.terminal_id,
        TerminalState::Failed,
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        pressure_snapshot
            .error
            .expect("slow consumer is reported")
            .code,
        ErrorCode::TerminalConsumerStalled
    );

    let (responsive, mut responsive_output) = timeout(
        Duration::from_secs(5),
        terminals.open(open_payload(&started.connection_id)),
    )
    .await
    .expect("another terminal opens while one output stream is not ACKed")
    .expect("open responsive terminal");
    wait_for_shell_prompt(&terminals, &responsive, &mut responsive_output).await;
    write(&terminals, &responsive.terminal_id, 1, b"stty -echo\n")
        .await
        .expect("disable responsive terminal echo");
    wait_for_shell_prompt(&terminals, &responsive, &mut responsive_output).await;
    write(
        &terminals,
        &responsive.terminal_id,
        2,
        b"printf '\\162\\145\\163\\160\\157\\156\\163\\151\\166\\145\\055\\157\\153\\n'; exit\n",
    )
    .await
    .expect("write responsive terminal command");
    let responsive_bytes = collect_output(&terminals, &responsive, &mut responsive_output).await;
    assert!(
        responsive_bytes
            .windows(b"responsive-ok".len())
            .any(|window| window == b"responsive-ok")
    );

    let mut pressure_bytes = first_pressure_bytes.len();
    let mut pressure_contains_payload = first_pressure_bytes.contains(&1);
    terminals
        .ack(TerminalAckPayload {
            terminal_id: pressure.terminal_id.clone(),
            stream_id: pressure.stream_id.clone(),
            seq: first_pressure_chunk.seq,
        })
        .await
        .expect("ack first emitted pressure chunk");
    while let Some(chunk) = timeout(Duration::from_secs(5), pressure_output.recv())
        .await
        .expect("failed stream drains bounded queued chunks")
    {
        let bytes = BASE64
            .decode(&chunk.data_base64)
            .expect("pressure chunk Base64");
        pressure_bytes += bytes.len();
        pressure_contains_payload |= bytes.contains(&1);
        terminals
            .ack(TerminalAckPayload {
                terminal_id: pressure.terminal_id.clone(),
                stream_id: pressure.stream_id.clone(),
                seq: chunk.seq,
            })
            .await
            .expect("ack already emitted pressure chunk");
    }
    assert!(
        pressure_contains_payload,
        "pressure output contains the remote bulk payload"
    );
    assert!(pressure_bytes <= 128 * 1024);

    let (bulk, mut bulk_output) = terminals
        .open(open_payload(&started.connection_id))
        .await
        .expect("open lossless high-output terminal");
    wait_for_shell_prompt(&terminals, &bulk, &mut bulk_output).await;
    write(&terminals, &bulk.terminal_id, 1, b"stty -echo\n")
        .await
        .expect("disable bulk terminal echo");
    wait_for_shell_prompt(&terminals, &bulk, &mut bulk_output).await;
    write(
        &terminals,
        &bulk.terminal_id,
        2,
        b"python3 -c \"import os; os.write(1, b'\\x01'*1000000)\"; exit\n",
    )
    .await
    .expect("start lossless high output");
    let bulk_bytes = collect_output(&terminals, &bulk, &mut bulk_output).await;
    assert_eq!(
        bulk_bytes.iter().filter(|byte| **byte == 1).count(),
        1_000_000,
        "acknowledged high terminal output must be lossless"
    );

    let (orphan, orphan_output) = terminals
        .open(open_payload(&started.connection_id))
        .await
        .expect("open terminal with detached consumer");
    drop(orphan_output);
    let orphan_snapshot =
        wait_for_terminal_state(&terminals, &orphan.terminal_id, TerminalState::Failed).await;
    assert_eq!(
        orphan_snapshot.error.expect("detached consumer error").code,
        ErrorCode::TerminalConsumerStalled
    );
    assert_eq!(
        connections
            .get(&started.connection_id)
            .expect("other channels remain connected")
            .state,
        ConnectionState::Ready
    );

    let (disconnecting, mut disconnecting_output) = terminals
        .open(open_payload(&started.connection_id))
        .await
        .expect("open terminal before connection disconnect");
    wait_for_shell_prompt(&terminals, &disconnecting, &mut disconnecting_output).await;
    let disconnected = connections
        .disconnect(&started.connection_id)
        .await
        .expect("disconnect fixture connection");
    assert_eq!(disconnected.state, ConnectionState::Closed);
    let terminal_after_disconnect = wait_for_terminal_state(
        &terminals,
        &disconnecting.terminal_id,
        TerminalState::Closed,
    )
    .await;
    assert!(terminal_after_disconnect.error.is_none());
}

fn profile_input(fixture: &OpenSshFixture) -> ServerProfileInput {
    use std::os::unix::ffi::OsStrExt;

    ServerProfileInput {
        name: Some("terminal fixture".to_owned()),
        host: fixture.host.clone(),
        port: fixture.port,
        username: fixture.username.clone(),
        auth_type: AuthType::PrivateKey,
        private_key_path: Some(StoredPath {
            bytes: Path::new(&fixture.private_key)
                .as_os_str()
                .as_bytes()
                .to_vec(),
            encoding: PathEncoding::UnixBytes,
        }),
        group_id: None,
        connect_timeout_ms: 10_000,
        keepalive_interval_seconds: 30,
        jump_host: None,
        jump_port: 22,
        proxy_type: None,
        proxy_host: None,
        proxy_port: None,
    }
}

fn open_payload(connection_id: &str) -> TerminalOpenPayload {
    TerminalOpenPayload {
        connection_id: connection_id.to_owned(),
        columns: 80,
        rows: 24,
        pixel_width: None,
        pixel_height: None,
    }
}

async fn write(
    terminals: &TerminalManager,
    terminal_id: &str,
    seq: u64,
    data: &[u8],
) -> Result<maulink_core::TerminalWriteResult, maulink_core::AppError> {
    terminals
        .write(TerminalWritePayload {
            terminal_id: terminal_id.to_owned(),
            input_seq: DecimalU64(seq),
            data_base64: BASE64.encode(data),
        })
        .await
}

async fn collect_output(
    terminals: &TerminalManager,
    opened: &maulink_core::TerminalOpenResult,
    output: &mut tokio::sync::mpsc::Receiver<maulink_core::TerminalChunk>,
) -> Vec<u8> {
    let mut collected = Vec::new();
    loop {
        let chunk = timeout(Duration::from_secs(10), output.recv())
            .await
            .expect("terminal output completes within 10 seconds");
        let Some(chunk) = chunk else {
            return collected;
        };
        assert_eq!(chunk.terminal_id, opened.terminal_id);
        assert_eq!(chunk.stream_id, opened.stream_id);
        let bytes = BASE64.decode(&chunk.data_base64).expect("chunk Base64");
        assert_eq!(bytes.len(), chunk.byte_length as usize);
        assert!(bytes.len() <= 32 * 1024);
        collected.extend_from_slice(&bytes);
        terminals
            .ack(TerminalAckPayload {
                terminal_id: opened.terminal_id.clone(),
                stream_id: opened.stream_id.clone(),
                seq: chunk.seq,
            })
            .await
            .expect("ack terminal chunk");
    }
}

async fn wait_for_byte_without_ack(
    terminals: &TerminalManager,
    opened: &maulink_core::TerminalOpenResult,
    output: &mut tokio::sync::mpsc::Receiver<maulink_core::TerminalChunk>,
    needle: u8,
) -> maulink_core::TerminalChunk {
    timeout(Duration::from_secs(10), async {
        loop {
            let chunk = output
                .recv()
                .await
                .expect("terminal output stays open until bulk payload arrives");
            assert_eq!(chunk.terminal_id, opened.terminal_id);
            assert_eq!(chunk.stream_id, opened.stream_id);
            let bytes = BASE64
                .decode(&chunk.data_base64)
                .expect("terminal chunk Base64");
            if bytes.contains(&needle) {
                return chunk;
            }
            terminals
                .ack(TerminalAckPayload {
                    terminal_id: opened.terminal_id.clone(),
                    stream_id: opened.stream_id.clone(),
                    seq: chunk.seq,
                })
                .await
                .expect("ack command echo before bulk payload");
        }
    })
    .await
    .expect("remote bulk payload within 10 seconds")
}

async fn wait_for_shell_prompt(
    terminals: &TerminalManager,
    opened: &maulink_core::TerminalOpenResult,
    output: &mut tokio::sync::mpsc::Receiver<maulink_core::TerminalChunk>,
) {
    timeout(Duration::from_secs(10), async {
        let mut received = Vec::new();
        loop {
            let chunk = output
                .recv()
                .await
                .expect("terminal output stays open while waiting for shell prompt");
            assert_eq!(chunk.terminal_id, opened.terminal_id);
            assert_eq!(chunk.stream_id, opened.stream_id);
            received.extend_from_slice(
                &BASE64
                    .decode(&chunk.data_base64)
                    .expect("shell startup output Base64"),
            );
            terminals
                .ack(TerminalAckPayload {
                    terminal_id: opened.terminal_id.clone(),
                    stream_id: opened.stream_id.clone(),
                    seq: chunk.seq,
                })
                .await
                .expect("ack shell startup output");
            if [b"% ".as_slice(), b"$ ", b"# ", b"> "]
                .iter()
                .any(|prompt| {
                    received
                        .windows(prompt.len())
                        .any(|window| window == *prompt)
                })
            {
                return;
            }
        }
    })
    .await
    .expect("remote shell prompt within 10 seconds");
}

async fn wait_for_terminal_state(
    terminals: &TerminalManager,
    terminal_id: &str,
    expected: TerminalState,
) -> maulink_core::TerminalSnapshot {
    wait_for_terminal_state_within(terminals, terminal_id, expected, Duration::from_secs(5)).await
}

async fn wait_for_terminal_state_within(
    terminals: &TerminalManager,
    terminal_id: &str,
    expected: TerminalState,
    duration: Duration,
) -> maulink_core::TerminalSnapshot {
    let result = timeout(duration, async {
        loop {
            let snapshot = terminals
                .get(&TerminalIdPayload {
                    terminal_id: terminal_id.to_owned(),
                })
                .expect("terminal snapshot");
            if snapshot.state == expected {
                return snapshot;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    match result {
        Ok(snapshot) => snapshot,
        Err(error) => panic!(
            "terminal did not reach {expected:?} within {duration:?}; latest snapshot: {:?}; timeout: {error}",
            terminals
                .get(&TerminalIdPayload {
                    terminal_id: terminal_id.to_owned(),
                })
                .expect("terminal snapshot after timeout")
        ),
    }
}

async fn wait_for_host_key(
    connections: &SshConnectionManager,
    connection_id: &str,
) -> maulink_core::HostKeyChallenge {
    timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = connections.get(connection_id).expect("connection snapshot");
            if let Some(challenge) = snapshot.host_key_challenge {
                return challenge;
            }
            assert!(
                !snapshot.state.is_terminal(),
                "connection failed: {snapshot:?}"
            );
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Host Key challenge within 10 seconds")
}

async fn wait_for_ready(connections: &SshConnectionManager, connection_id: &str) {
    timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = connections.get(connection_id).expect("connection snapshot");
            if snapshot.state == ConnectionState::Ready {
                return;
            }
            assert!(
                !snapshot.state.is_terminal(),
                "connection failed: {snapshot:?}"
            );
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("connection ready within 10 seconds");
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
