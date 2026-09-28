#![cfg(unix)]

mod support;

use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    os::unix::ffi::OsStrExt,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::Arc,
    time::Duration,
};

use maulink_core::{
    AuthType, ConnectionMode, ConnectionState, CredentialManager, CredentialWorker, Database,
    ErrorCode, HostKeyDecision, HostKeyStore, HostKeyVerifier, LocalFilePurpose, LocalFileRegistry,
    MonitorManager, MonitorQualityStatus, MonitorRefreshPayload, PathEncoding, ProfileStore,
    RemoteFileType, Secret, SecureStore, SecureStoreError, ServerProfileInput, SftpCursorPayload,
    SftpDeletePayload, SftpDownloadPayload, SftpListStartPayload, SftpManager, SftpMkdirPayload,
    SftpRenamePayload, SftpStatPayload, SftpTransferDirection, SftpTransferIdPayload,
    SftpTransferManager, SftpTransferState, SftpUploadPayload, SshConnectionManager, SshConnector,
    StoredPath, TerminalIdPayload, TerminalManager, TerminalOpenPayload,
};
use sha2::{Digest, Sha256};
use support::OpenSshFixture;
use tokio::time::{sleep, timeout};

#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStringExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "starts an isolated loopback OpenSSH service"]
async fn openssh_sftp_browses_and_pages_remote_entries() {
    let fixture = OpenSshFixture::start();
    let directory = tempfile::tempdir().expect("remote directory fixture");
    for index in 0..205 {
        fs::write(
            directory.path().join(format!("entry-{index:03}")),
            b"payload",
        )
        .expect("create remote file");
    }
    fs::write(directory.path().join("中文 space ' file.txt"), b"unicode")
        .expect("create unicode filename");
    fs::write(directory.path().join("line\nbreak.txt"), b"newline")
        .expect("create newline filename");
    std::os::unix::fs::symlink("entry-000", directory.path().join("alias"))
        .expect("create remote symlink");

    let (connections, connection_id, _database_directory) = connect(&fixture).await;
    let sftp = SftpManager::new(connections.clone());
    let mut page = sftp
        .list_start(SftpListStartPayload {
            connection_id: connection_id.clone(),
            path: directory
                .path()
                .to_str()
                .expect("UTF-8 temp path")
                .to_owned(),
        })
        .await
        .expect("list first remote directory page");
    let mut names = HashSet::new();
    let mut total = 0;
    let mut alias_is_symlink = false;
    loop {
        assert!(page.entries.len() <= 200);
        assert!(
            serde_json::to_vec(&page)
                .expect("serialize directory page")
                .len()
                <= 256 * 1024
        );
        total += page.entries.len();
        names.extend(page.entries.iter().map(|entry| entry.name.clone()));
        alias_is_symlink |= page
            .entries
            .iter()
            .any(|entry| entry.name == "alias" && entry.is_symlink);
        let Some(cursor_id) = page.cursor_id.take() else {
            break;
        };
        page = sftp
            .list_next(SftpCursorPayload { cursor_id })
            .await
            .expect("read next remote directory page");
    }

    assert_eq!(total, 208);
    assert_eq!(names.len(), 208);
    assert!(names.contains("中文 space ' file.txt"));
    assert!(names.contains("line\nbreak.txt"));
    assert!(alias_is_symlink, "SFTP attributes identify symbolic links");

    let first = sftp
        .list_start(SftpListStartPayload {
            connection_id: connection_id.clone(),
            path: directory
                .path()
                .to_str()
                .expect("UTF-8 temp path")
                .to_owned(),
        })
        .await
        .expect("open cursor for lifecycle checks");
    let cursor_id = first.cursor_id.expect("large directory retains a cursor");
    let mut cursor_ids = vec![cursor_id.clone()];
    for _ in 0..3 {
        let cursor = sftp
            .list_start(SftpListStartPayload {
                connection_id: connection_id.clone(),
                path: directory
                    .path()
                    .to_str()
                    .expect("UTF-8 temp path")
                    .to_owned(),
            })
            .await
            .expect("open cursor within per-connection limit");
        cursor_ids.push(cursor.cursor_id.expect("large directory retains a cursor"));
    }
    let fifth_cursor = sftp
        .list_start(SftpListStartPayload {
            connection_id: connection_id.clone(),
            path: directory
                .path()
                .to_str()
                .expect("UTF-8 temp path")
                .to_owned(),
        })
        .await;
    assert!(matches!(
        fifth_cursor,
        Err(error) if error.code == ErrorCode::ResourceLimit
    ));
    for cursor_id in cursor_ids {
        sftp.list_close(SftpCursorPayload { cursor_id })
            .await
            .expect("close cursor");
    }
    sftp.list_close(SftpCursorPayload { cursor_id })
        .await
        .expect("idempotent close");

    let active_cursor = sftp
        .list_start(SftpListStartPayload {
            connection_id: connection_id.clone(),
            path: directory
                .path()
                .to_str()
                .expect("UTF-8 temp path")
                .to_owned(),
        })
        .await
        .expect("open cursor for connection cleanup");
    let active_cursor_id = active_cursor
        .cursor_id
        .expect("large directory has a cursor");
    sftp.close_connection(&connection_id).await;
    assert_eq!(
        sftp.list_next(SftpCursorPayload {
            cursor_id: active_cursor_id.clone(),
        })
        .await
        .expect_err("disconnect cleanup invalidates the cursor")
        .code,
        ErrorCode::ResourceClosed
    );
    sftp.list_close(SftpCursorPayload {
        cursor_id: active_cursor_id,
    })
    .await
    .expect("closing an already cleaned cursor is idempotent");

    sftp.shutdown().await;
    connections
        .disconnect(&connection_id)
        .await
        .expect("disconnect fixture SSH session");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "starts an isolated loopback OpenSSH service"]
async fn openssh_monitor_exec_runs_alongside_terminal_and_transfer() {
    let fixture = OpenSshFixture::start();
    let directory = tempfile::tempdir().expect("monitor concurrency fixture");
    let upload_path = directory.path().join("monitor-concurrent-upload.bin");
    fs::File::create(&upload_path)
        .and_then(|file| file.set_len(64 * 1024 * 1024))
        .expect("create sparse transfer fixture");
    let remote_path = directory
        .path()
        .to_str()
        .expect("UTF-8 temporary directory")
        .to_owned();
    let (connections, connection_id, _database_directory) = connect(&fixture).await;
    let monitor = MonitorManager::new(connections.clone());
    let terminal_manager = TerminalManager::new(connections.clone());
    let local_files = LocalFileRegistry::default();
    let transfers = SftpTransferManager::new(connections.clone(), local_files.clone());
    let selected_upload = local_files
        .register(upload_path, LocalFilePurpose::Upload)
        .expect("register monitor concurrency upload");
    let (_started, mut updates) = transfers
        .upload(SftpUploadPayload {
            connection_id: connection_id.clone(),
            local_file_token: selected_upload.token,
            remote_path: format!("{remote_path}/monitor-concurrent-upload-published.bin"),
        })
        .await
        .expect("start SFTP transfer beside Monitor");
    timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = updates.borrow_and_update().clone();
            if snapshot.state == SftpTransferState::Transferring {
                break;
            }
            assert!(
                !snapshot.state.is_terminal(),
                "transfer ended before streaming: {snapshot:?}"
            );
            updates
                .changed()
                .await
                .expect("transfer remains observable");
        }
    })
    .await
    .expect("transfer enters streaming state");

    let (terminal, _terminal_output) = terminal_manager
        .open(TerminalOpenPayload {
            connection_id: connection_id.clone(),
            columns: 80,
            rows: 24,
            pixel_width: None,
            pixel_height: None,
        })
        .await
        .expect("open interactive terminal beside Monitor");

    let snapshot = monitor
        .refresh(MonitorRefreshPayload {
            connection_id: connection_id.clone(),
        })
        .await
        .expect("run fixed Monitor exec collection");

    assert_eq!(snapshot.connection_id, connection_id);
    assert_eq!(snapshot.system.quality.status, MonitorQualityStatus::Ok);
    assert!(snapshot.system.hostname.is_some());
    assert_eq!(snapshot.disk.quality.status, MonitorQualityStatus::Ok);
    if !std::path::Path::new("/proc/stat").exists() {
        assert_eq!(
            snapshot.cpu.quality.status,
            MonitorQualityStatus::Unsupported
        );
        assert_eq!(
            snapshot.memory.quality.status,
            MonitorQualityStatus::Unsupported
        );
        assert_eq!(
            snapshot.network.quality.status,
            MonitorQualityStatus::Unsupported
        );
        assert_eq!(
            snapshot.load.quality.status,
            MonitorQualityStatus::Unsupported
        );
        assert_eq!(
            snapshot.uptime.quality.status,
            MonitorQualityStatus::Unsupported
        );
    }

    let completed = wait_for_transfer_terminal(updates).await;
    assert_eq!(completed.state, SftpTransferState::Completed);
    assert_eq!(
        fs::metadata(
            directory
                .path()
                .join("monitor-concurrent-upload-published.bin")
        )
        .expect("uploaded file is published")
        .len(),
        64 * 1024 * 1024
    );
    terminal_manager
        .close(&TerminalIdPayload {
            terminal_id: terminal.terminal_id,
        })
        .await
        .expect("close test terminal");

    monitor.close_connection(&connection_id).await;
    terminal_manager.shutdown().await;
    transfers.shutdown().await;
    connections
        .disconnect(&connection_id)
        .await
        .expect("disconnect fixture connection");
    monitor.shutdown().await;
    connections.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "starts an isolated loopback OpenSSH service"]
#[cfg(target_os = "linux")]
async fn openssh_sftp_rejects_lossy_non_utf8_remote_names() {
    let fixture = OpenSshFixture::start();
    let directory = tempfile::tempdir().expect("remote directory fixture");
    let invalid_name = std::ffi::OsString::from_vec(b"invalid-\xff-name".to_vec());
    fs::write(directory.path().join(invalid_name), b"unsafe to round-trip")
        .expect("create non-UTF-8 filename");

    let (connections, connection_id, _database_directory) = connect(&fixture).await;
    let sftp = SftpManager::new(connections.clone());
    let error = sftp
        .list_start(SftpListStartPayload {
            connection_id: connection_id.clone(),
            path: directory
                .path()
                .to_str()
                .expect("UTF-8 temp path")
                .to_owned(),
        })
        .await
        .expect_err("lossy path conversion must fail closed");
    assert_eq!(error.code, ErrorCode::UnsupportedPathEncoding);

    sftp.shutdown().await;
    connections
        .disconnect(&connection_id)
        .await
        .expect("disconnect fixture SSH session");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "starts an isolated loopback OpenSSH service"]
async fn openssh_sftp_supports_safe_basic_file_operations() {
    let fixture = OpenSshFixture::start();
    let directory = tempfile::tempdir().expect("remote directory fixture");
    let canonical_root = fs::canonicalize(directory.path()).expect("canonical remote directory");
    let root = canonical_root.to_str().expect("UTF-8 canonical temp path");
    fs::write(directory.path().join("source.txt"), b"source").expect("create rename source");
    fs::write(directory.path().join("occupied.txt"), b"occupied").expect("create collision target");
    fs::write(directory.path().join("symlink-target.txt"), b"target")
        .expect("create symlink target");
    std::os::unix::fs::symlink("symlink-target.txt", directory.path().join("link.txt"))
        .expect("create symlink");
    fs::create_dir(directory.path().join("empty-dir")).expect("create empty directory");
    fs::create_dir(directory.path().join("nonempty-dir")).expect("create nonempty directory");
    fs::write(directory.path().join("nonempty-dir/child.txt"), b"child")
        .expect("create nonempty directory child");

    let (connections, connection_id, _database_directory) = connect(&fixture).await;
    let sftp = SftpManager::new(connections.clone());
    let path = |name: &str| format!("{root}/{name}");

    let followed = sftp
        .stat(SftpStatPayload {
            connection_id: connection_id.clone(),
            path: path("link.txt"),
            follow_symlink: true,
        })
        .await
        .expect("stat follows symlink");
    assert_eq!(followed.file_type, RemoteFileType::File);
    let link = sftp
        .stat(SftpStatPayload {
            connection_id: connection_id.clone(),
            path: path("link.txt"),
            follow_symlink: false,
        })
        .await
        .expect("lstat identifies symlink");
    assert_eq!(link.file_type, RemoteFileType::Symlink);
    assert!(link.is_symlink);

    let created = sftp
        .mkdir(SftpMkdirPayload {
            connection_id: connection_id.clone(),
            parent_path: root.to_owned(),
            name: "created-dir".to_owned(),
        })
        .await
        .expect("create a directory");
    assert_eq!(created.file_type, RemoteFileType::Directory);
    let duplicate = sftp
        .mkdir(SftpMkdirPayload {
            connection_id: connection_id.clone(),
            parent_path: root.to_owned(),
            name: "created-dir".to_owned(),
        })
        .await
        .expect_err("mkdir must report an existing path");
    assert_eq!(duplicate.code, ErrorCode::PathExists);

    let read_only_directory = directory.path().join("read-only-dir");
    fs::create_dir(&read_only_directory).expect("create permission fixture directory");
    fs::set_permissions(&read_only_directory, fs::Permissions::from_mode(0o555))
        .expect("make permission fixture read-only");
    let permission_denied = sftp
        .mkdir(SftpMkdirPayload {
            connection_id: connection_id.clone(),
            parent_path: path("read-only-dir"),
            name: "forbidden".to_owned(),
        })
        .await
        .expect_err("read-only remote directory rejects writes");
    assert_eq!(permission_denied.code, ErrorCode::PermissionDenied);
    fs::set_permissions(&read_only_directory, fs::Permissions::from_mode(0o755))
        .expect("restore fixture directory permissions");

    let collision = sftp
        .rename(SftpRenamePayload {
            connection_id: connection_id.clone(),
            source_path: path("source.txt"),
            new_name: "occupied.txt".to_owned(),
        })
        .await
        .expect_err("rename must not overwrite a target");
    assert_eq!(collision.code, ErrorCode::TargetExists);
    let renamed = sftp
        .rename(SftpRenamePayload {
            connection_id: connection_id.clone(),
            source_path: path("source.txt"),
            new_name: "renamed.txt".to_owned(),
        })
        .await
        .expect("rename within the source directory");
    assert_eq!(renamed.path, path("renamed.txt"));

    let unconfirmed = sftp
        .delete(SftpDeletePayload {
            connection_id: connection_id.clone(),
            path: path("renamed.txt"),
            expected_type: RemoteFileType::File,
            confirmed: false,
        })
        .await
        .expect_err("delete requires explicit confirmation");
    assert_eq!(unconfirmed.code, ErrorCode::ValidationFailed);
    let changed_type = sftp
        .delete(SftpDeletePayload {
            connection_id: connection_id.clone(),
            path: path("renamed.txt"),
            expected_type: RemoteFileType::Directory,
            confirmed: true,
        })
        .await
        .expect_err("delete must reject a stale entry type");
    assert_eq!(changed_type.code, ErrorCode::ValidationFailed);
    sftp.delete(SftpDeletePayload {
        connection_id: connection_id.clone(),
        path: path("renamed.txt"),
        expected_type: RemoteFileType::File,
        confirmed: true,
    })
    .await
    .expect("delete file after confirmation");

    sftp.delete(SftpDeletePayload {
        connection_id: connection_id.clone(),
        path: path("link.txt"),
        expected_type: RemoteFileType::Symlink,
        confirmed: true,
    })
    .await
    .expect("delete symlink itself");
    assert!(directory.path().join("symlink-target.txt").exists());

    let not_empty = sftp
        .delete(SftpDeletePayload {
            connection_id: connection_id.clone(),
            path: path("nonempty-dir"),
            expected_type: RemoteFileType::Directory,
            confirmed: true,
        })
        .await
        .expect_err("nonempty directory must be preserved");
    assert_eq!(not_empty.code, ErrorCode::DirectoryNotEmpty);
    sftp.delete(SftpDeletePayload {
        connection_id: connection_id.clone(),
        path: path("empty-dir"),
        expected_type: RemoteFileType::Directory,
        confirmed: true,
    })
    .await
    .expect("delete empty directory");

    let root_delete = sftp
        .delete(SftpDeletePayload {
            connection_id: connection_id.clone(),
            path: "/".to_owned(),
            expected_type: RemoteFileType::Directory,
            confirmed: true,
        })
        .await
        .expect_err("root directory must never be deleted");
    assert_eq!(root_delete.code, ErrorCode::ValidationFailed);

    sftp.shutdown().await;
    connections
        .disconnect(&connection_id)
        .await
        .expect("disconnect fixture SSH session");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "starts an isolated loopback OpenSSH service"]
async fn openssh_sftp_streams_uploads_and_downloads_with_no_clobber_publish() {
    let fixture = OpenSshFixture::start();
    let directory = tempfile::tempdir().expect("remote and local fixture directory");
    let remote_directory = fs::canonicalize(directory.path()).expect("canonical remote directory");
    let remote_root = remote_directory
        .to_str()
        .expect("UTF-8 canonical remote directory");
    let upload_source = directory.path().join("source.bin");
    let bytes: Vec<u8> = (0..384 * 1024).map(|index| (index % 251) as u8).collect();
    fs::write(&upload_source, &bytes).expect("write upload source");

    let (connections, connection_id, _database_directory) = connect(&fixture).await;
    let local_files = LocalFileRegistry::default();
    let transfers = SftpTransferManager::new(connections.clone(), local_files.clone());
    let selected_upload = local_files
        .register(upload_source.clone(), LocalFilePurpose::Upload)
        .expect("register upload selection");
    let (started_upload, upload_updates) = transfers
        .upload(SftpUploadPayload {
            connection_id: connection_id.clone(),
            local_file_token: selected_upload.token.clone(),
            remote_path: format!("{remote_root}/uploaded.bin"),
        })
        .await
        .expect("start streaming upload");
    assert_eq!(started_upload.direction, SftpTransferDirection::Upload);
    let uploaded = wait_for_transfer_terminal(upload_updates).await;
    assert_eq!(uploaded.state, SftpTransferState::Completed);
    assert_eq!(uploaded.transferred_bytes.0, bytes.len() as u64);
    let remote_bytes = fs::read(directory.path().join("uploaded.bin")).expect("read uploaded file");
    assert_eq!(Sha256::digest(&remote_bytes), Sha256::digest(&bytes));

    let unusual_name = "中文 ' line\nbreak.bin";
    let unusual_selection = local_files
        .register(upload_source.clone(), LocalFilePurpose::Upload)
        .expect("register upload with an unusual remote name");
    let (unusual_start, unusual_updates) = transfers
        .upload(SftpUploadPayload {
            connection_id: connection_id.clone(),
            local_file_token: unusual_selection.token,
            remote_path: format!("{remote_root}/{unusual_name}"),
        })
        .await
        .expect("start upload with an unusual remote name");
    let unusual_upload = wait_for_transfer_terminal(unusual_updates).await;
    assert_eq!(unusual_upload.state, SftpTransferState::Completed);
    assert_eq!(unusual_start.file_name, unusual_name);
    let unusual_bytes =
        fs::read(directory.path().join(unusual_name)).expect("read upload with an unusual name");
    assert_eq!(Sha256::digest(&unusual_bytes), Sha256::digest(&bytes));

    let conflicting_upload = local_files
        .register(upload_source.clone(), LocalFilePurpose::Upload)
        .expect("register upload that collides remotely");
    let remote_collision = transfers
        .upload(SftpUploadPayload {
            connection_id: connection_id.clone(),
            local_file_token: conflicting_upload.token,
            remote_path: format!("{remote_root}/uploaded.bin"),
        })
        .await
        .expect_err("remote upload target must not be overwritten");
    assert_eq!(remote_collision.code, ErrorCode::TargetExists);

    let download_target = directory.path().join("downloaded.bin");
    let selected_download = local_files
        .register(download_target.clone(), LocalFilePurpose::Download)
        .expect("register download destination");
    let (started_download, download_updates) = transfers
        .download(SftpDownloadPayload {
            connection_id: connection_id.clone(),
            remote_path: format!("{remote_root}/uploaded.bin"),
            local_file_token: selected_download.token,
        })
        .await
        .expect("start streaming download");
    assert_eq!(started_download.direction, SftpTransferDirection::Download);
    let downloaded = wait_for_transfer_terminal(download_updates).await;
    assert_eq!(downloaded.state, SftpTransferState::Completed);
    let downloaded_bytes = fs::read(&download_target).expect("read downloaded file");
    assert_eq!(Sha256::digest(&downloaded_bytes), Sha256::digest(&bytes));

    let repeated_token = transfers
        .upload(SftpUploadPayload {
            connection_id: connection_id.clone(),
            local_file_token: selected_upload.token,
            remote_path: format!("{remote_root}/second.bin"),
        })
        .await
        .expect_err("upload selection token is one-time");
    assert_eq!(repeated_token.code, ErrorCode::ResourceClosed);

    let selected_existing_target = local_files
        .register(upload_source.clone(), LocalFilePurpose::Download)
        .expect("register existing local target");
    let local_collision = transfers
        .download(SftpDownloadPayload {
            connection_id: connection_id.clone(),
            remote_path: format!("{remote_root}/uploaded.bin"),
            local_file_token: selected_existing_target.token,
        })
        .await
        .expect_err("download must not overwrite an existing local target");
    assert_eq!(local_collision.code, ErrorCode::TargetExists);

    let history = transfers
        .list(maulink_core::SftpTransferListPayload {
            connection_id: Some(connection_id.clone()),
            limit: Some(10),
        })
        .expect("list transfer history");
    assert_eq!(history.len(), 3);
    assert_eq!(
        transfers
            .get(SftpTransferIdPayload {
                transfer_id: uploaded.transfer_id.clone(),
            })
            .expect("get completed transfer")
            .state,
        SftpTransferState::Completed
    );

    transfers.shutdown().await;
    connections
        .disconnect(&connection_id)
        .await
        .expect("disconnect fixture SSH session");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "starts an isolated loopback OpenSSH service"]
async fn openssh_sftp_cancels_upload_without_publishing_the_target() {
    let mut fixture = OpenSshFixture::start();
    let directory = tempfile::tempdir().expect("remote directory fixture");
    let remote_directory = fs::canonicalize(directory.path()).expect("canonical remote directory");
    let remote_root = remote_directory
        .to_str()
        .expect("UTF-8 canonical remote directory");
    let upload_source = directory.path().join("large-source.bin");
    fs::File::create(&upload_source)
        .and_then(|file| file.set_len(128 * 1024 * 1024))
        .expect("create sparse large upload source");

    let (connections, connection_id, _database_directory) = connect(&fixture).await;
    let local_files = LocalFileRegistry::default();
    let transfers = SftpTransferManager::new(connections.clone(), local_files.clone());
    let selected = local_files
        .register(upload_source, LocalFilePurpose::Upload)
        .expect("register large upload");
    let (started, mut updates) = transfers
        .upload(SftpUploadPayload {
            connection_id: connection_id.clone(),
            local_file_token: selected.token,
            remote_path: format!("{remote_root}/cancelled.bin"),
        })
        .await
        .expect("start large upload");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let snapshot = updates.borrow_and_update().clone();
            if snapshot.state == SftpTransferState::Transferring {
                break;
            }
            assert!(
                !snapshot.state.is_terminal(),
                "transfer ended before cancellation"
            );
            updates
                .changed()
                .await
                .expect("transfer update sender remains open");
        }
    })
    .await
    .expect("upload starts streaming");
    fixture.freeze();
    let blocked_selection = local_files
        .register(
            directory.path().join("large-source.bin"),
            LocalFilePurpose::Upload,
        )
        .expect("register second upload selection");
    let busy = transfers
        .upload(SftpUploadPayload {
            connection_id: connection_id.clone(),
            local_file_token: blocked_selection.token,
            remote_path: format!("{remote_root}/second.bin"),
        })
        .await
        .expect_err("only one transfer may run on a connection");
    assert_eq!(busy.code, ErrorCode::TransferBusy);
    transfers
        .cancel(SftpTransferIdPayload {
            transfer_id: started.transfer_id.clone(),
        })
        .expect("cancel active upload");
    let cancelled = wait_for_transfer_terminal(updates).await;
    assert_eq!(cancelled.state, SftpTransferState::Cancelled);
    assert!(!directory.path().join("cancelled.bin").exists());
    fixture.kill_frozen();
    transfers.shutdown().await;
    connections
        .disconnect(&connection_id)
        .await
        .expect("disconnect fixture SSH session");
}

async fn wait_for_transfer_terminal(
    mut updates: tokio::sync::watch::Receiver<maulink_core::SftpTransferSnapshot>,
) -> maulink_core::SftpTransferSnapshot {
    timeout(Duration::from_secs(15), async {
        loop {
            let snapshot = updates.borrow_and_update().clone();
            if snapshot.state.is_terminal() {
                return snapshot;
            }
            updates
                .changed()
                .await
                .expect("transfer update sender remains open");
        }
    })
    .await
    .expect("transfer reaches a terminal state")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "transfers 100 MiB, 1 GiB, and 10 GiB through an isolated loopback OpenSSH service"]
async fn sftp_large_file_matrix_streams_with_bounded_chunks() {
    let fixture = OpenSshFixture::start();
    let directory = tempfile::tempdir().expect("large transfer fixture directory");
    let remote_subdirectory = directory.path().join("remote");
    fs::create_dir(&remote_subdirectory).expect("create isolated remote directory");
    let remote_directory =
        fs::canonicalize(&remote_subdirectory).expect("canonical remote directory");
    let remote_root = remote_directory
        .to_str()
        .expect("UTF-8 canonical remote directory");
    let (connections, connection_id, _database_directory) = connect(&fixture).await;
    let local_files = LocalFileRegistry::default();
    let transfers = SftpTransferManager::new(connections.clone(), local_files.clone());
    let block: Vec<u8> = (0..64 * 1024)
        .map(|index| ((index * 31 + 17) % 251) as u8)
        .collect();

    for size in [
        100 * 1024 * 1024,
        1024 * 1024 * 1024,
        10 * 1024 * 1024 * 1024,
    ] {
        let file_name = format!("roundtrip-{size}.bin");
        let source = directory.path().join(&file_name);
        let mut file = fs::File::create(&source).expect("create large source file");
        let mut remaining = size;
        while remaining > 0 {
            let chunk_size = remaining.min(block.len() as u64) as usize;
            file.write_all(&block[..chunk_size])
                .expect("write deterministic large source block");
            remaining -= chunk_size as u64;
        }
        file.sync_all().expect("flush large source fixture");
        drop(file);
        let expected_hash = sha256_file(&source);

        let selected_upload = local_files
            .register(source.clone(), LocalFilePurpose::Upload)
            .expect("register large upload source");
        let (upload, upload_updates) = transfers
            .upload(SftpUploadPayload {
                connection_id: connection_id.clone(),
                local_file_token: selected_upload.token,
                remote_path: format!("{remote_root}/{file_name}"),
            })
            .await
            .expect("start large upload");
        let started_at = std::time::Instant::now();
        let uploaded = wait_for_transfer_terminal_long(upload_updates).await;
        assert_eq!(uploaded.state, SftpTransferState::Completed);
        assert_eq!(uploaded.transferred_bytes.0, size);
        assert_eq!(sha256_file(&source), expected_hash);

        let destination = directory.path().join(format!("download-{file_name}"));
        let selected_download = local_files
            .register(destination.clone(), LocalFilePurpose::Download)
            .expect("register large download destination");
        let (_, download_updates) = transfers
            .download(SftpDownloadPayload {
                connection_id: connection_id.clone(),
                remote_path: format!("{remote_root}/{file_name}"),
                local_file_token: selected_download.token,
            })
            .await
            .expect("start large download");
        let downloaded = wait_for_transfer_terminal_long(download_updates).await;
        assert_eq!(downloaded.state, SftpTransferState::Completed);
        assert_eq!(downloaded.transferred_bytes.0, size);
        assert_eq!(sha256_file(&destination), expected_hash);
        eprintln!(
            "round-tripped {} bytes through bounded SFTP chunks in {:.2}s",
            size,
            started_at.elapsed().as_secs_f64()
        );
        assert_eq!(upload.state, SftpTransferState::Created);
    }

    transfers.shutdown().await;
    connections
        .disconnect(&connection_id)
        .await
        .expect("disconnect large-transfer fixture");
}

async fn wait_for_transfer_terminal_long(
    mut updates: tokio::sync::watch::Receiver<maulink_core::SftpTransferSnapshot>,
) -> maulink_core::SftpTransferSnapshot {
    timeout(Duration::from_secs(20 * 60), async {
        loop {
            let snapshot = updates.borrow_and_update().clone();
            if snapshot.state.is_terminal() {
                return snapshot;
            }
            updates
                .changed()
                .await
                .expect("transfer update sender remains open");
        }
    })
    .await
    .expect("large transfer reaches a terminal state")
}

fn sha256_file(path: &std::path::Path) -> [u8; 32] {
    let mut file = fs::File::open(path).expect("open file for streaming checksum");
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut hasher = Sha256::new();
    loop {
        let read = file.read(&mut buffer).expect("read checksum block");
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    hasher.finalize().into()
}

async fn connect(fixture: &OpenSshFixture) -> (SshConnectionManager, String, tempfile::TempDir) {
    let directory = tempfile::tempdir().expect("database directory");
    let database = Database::open(directory.path().join("sftp.sqlite3")).expect("database");
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
        .create_server(profile_input(fixture))
        .await
        .expect("create SFTP fixture profile");
    let started = connections
        .start(profile.id, profile.revision, ConnectionMode::Workspace)
        .await
        .expect("start SFTP fixture connection");
    let challenge = wait_for_host_key(&connections, &started.connection_id).await;
    connections
        .respond_host_key(
            &started.connection_id,
            &challenge.challenge_id,
            HostKeyDecision::TrustAndSave,
        )
        .expect("trust isolated fixture host key");
    wait_for_ready(&connections, &started.connection_id).await;
    (connections, started.connection_id, directory)
}

fn profile_input(fixture: &OpenSshFixture) -> ServerProfileInput {
    ServerProfileInput {
        name: Some("SFTP fixture".to_owned()),
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
