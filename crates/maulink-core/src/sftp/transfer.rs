use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    io,
    path::PathBuf,
    sync::{Arc, Mutex as StdMutex},
    time::{Duration, Instant},
};

use russh_sftp::{
    client::{RawSftpSession, error::Error as SftpError},
    protocol::{FileAttributes, FileType, OpenFlags, StatusCode},
};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{oneshot, watch},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::{
    join_remote_path, map_sftp_error, open_sftp_session, remote_parent_and_name,
    remote_path_exists, resolve_remote_path, target_exists, validate_remote_path,
};
use crate::{
    AppError, ConnectionState, DecimalU64, ErrorCode, LocalFileRegistry, SftpDownloadPayload,
    SftpTransferDirection, SftpTransferIdPayload, SftpTransferListPayload, SftpTransferSnapshot,
    SftpTransferState, SftpUploadPayload, SshConnectionManager,
};

const TRANSFER_CHUNK_BYTES: usize = 64 * 1024;
const MAX_ACTIVE_TRANSFERS: usize = 2;
const MAX_TRANSFER_HISTORY: usize = 50;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(200);
const RATE_WINDOW: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct SftpTransferManager {
    connections: SshConnectionManager,
    local_files: LocalFileRegistry,
    registry: Arc<StdMutex<TransferRegistry>>,
    shutdown: CancellationToken,
}

#[derive(Default)]
struct TransferRegistry {
    records: HashMap<String, TransferRecord>,
    order: VecDeque<String>,
    pending_global: usize,
    pending_by_connection: HashMap<String, usize>,
}

struct TransferRecord {
    snapshot: SftpTransferSnapshot,
    cancellation: CancellationToken,
    updates: watch::Sender<SftpTransferSnapshot>,
}

struct TransferReservation {
    registry: Arc<StdMutex<TransferRegistry>>,
    connection_id: String,
    committed: bool,
}

struct RateTracker {
    samples: VecDeque<(Instant, u64)>,
    last_published: Instant,
}

impl SftpTransferManager {
    pub fn new(connections: SshConnectionManager, local_files: LocalFileRegistry) -> Self {
        Self {
            connections,
            local_files,
            registry: Arc::new(StdMutex::new(TransferRegistry::default())),
            shutdown: CancellationToken::new(),
        }
    }

    pub async fn upload(
        &self,
        payload: SftpUploadPayload,
    ) -> Result<(SftpTransferSnapshot, watch::Receiver<SftpTransferSnapshot>), AppError> {
        self.ensure_connection_ready(&payload.connection_id)?;
        validate_remote_path(&payload.remote_path)?;
        let (parent_path, file_name) = remote_parent_and_name(&payload.remote_path)?;
        let reservation = self.reserve(&payload.connection_id)?;
        let local_path = self
            .local_files
            .consume_upload_path(&payload.local_file_token)?;
        let local_file = File::open(&local_path)
            .await
            .map_err(|error| map_local_error(error, "openingUploadSource"))?;
        let local_metadata = local_file
            .metadata()
            .await
            .map_err(|error| map_local_error(error, "readingUploadSource"))?;
        if !local_metadata.is_file() {
            return Err(validation(
                "localFileToken",
                "errors.localUploadFileInvalid",
            ));
        }
        let local_size = local_metadata.len();
        let session = open_sftp_session(&self.connections, &payload.connection_id).await?;
        let parent_path = resolve_remote_path(&session, &parent_path).await?;
        let final_path = join_remote_path(&parent_path, &file_name);
        validate_remote_path(&final_path)?;
        if remote_path_exists(&session, &final_path).await? {
            return Err(target_exists("checkingUploadTarget"));
        }

        let transfer_id = Uuid::new_v4().to_string();
        let temporary_path =
            join_remote_path(&parent_path, &format!(".maulink-{transfer_id}.part"));
        validate_remote_path(&temporary_path)?;
        let snapshot = initial_snapshot(
            transfer_id,
            payload.connection_id,
            SftpTransferDirection::Upload,
            file_name,
            final_path.clone(),
            Some(local_size),
        );
        let manager = self.clone();
        let cancellation = self.shutdown.child_token();
        let task_cancellation = cancellation.clone();
        let task_snapshot = snapshot.clone();
        let (updates, receiver) = watch::channel(snapshot.clone());
        self.spawn_task(
            reservation,
            snapshot.clone(),
            cancellation,
            updates,
            async move {
                run_upload(
                    manager,
                    task_snapshot,
                    task_cancellation,
                    session,
                    local_file,
                    final_path,
                    temporary_path,
                )
                .await;
            },
        )?;
        Ok((snapshot, receiver))
    }

    pub async fn download(
        &self,
        payload: SftpDownloadPayload,
    ) -> Result<(SftpTransferSnapshot, watch::Receiver<SftpTransferSnapshot>), AppError> {
        self.ensure_connection_ready(&payload.connection_id)?;
        validate_remote_path(&payload.remote_path)?;
        let (_, file_name) = remote_parent_and_name(&payload.remote_path)?;
        let reservation = self.reserve(&payload.connection_id)?;
        let local_target = self
            .local_files
            .consume_download_path(&payload.local_file_token)?;
        let session = open_sftp_session(&self.connections, &payload.connection_id).await?;
        let attributes = session
            .stat(payload.remote_path.clone())
            .await
            .map_err(|error| map_sftp_error(error, "checkingDownloadSource"))?
            .attrs;
        if attributes.file_type() != FileType::File {
            return Err(validation(
                "remotePath",
                "errors.sftpTransferFileTypeUnsupported",
            ));
        }
        let handle = session
            .open(
                payload.remote_path,
                OpenFlags::READ,
                FileAttributes::default(),
            )
            .await
            .map_err(|error| map_sftp_error(error, "openingDownloadSource"))?;
        let transfer_id = Uuid::new_v4().to_string();
        let temporary_path = local_temporary_path(&local_target, &transfer_id)?;
        let snapshot = initial_snapshot(
            transfer_id,
            payload.connection_id,
            SftpTransferDirection::Download,
            file_name,
            local_target.to_string_lossy().into_owned(),
            attributes.size,
        );
        let manager = self.clone();
        let cancellation = self.shutdown.child_token();
        let task_cancellation = cancellation.clone();
        let task_snapshot = snapshot.clone();
        let (updates, receiver) = watch::channel(snapshot.clone());
        self.spawn_task(
            reservation,
            snapshot.clone(),
            cancellation,
            updates,
            async move {
                run_download(
                    manager,
                    task_snapshot,
                    task_cancellation,
                    session,
                    handle.handle,
                    local_target,
                    temporary_path,
                )
                .await;
            },
        )?;
        Ok((snapshot, receiver))
    }

    pub fn get(&self, payload: SftpTransferIdPayload) -> Result<SftpTransferSnapshot, AppError> {
        validate_transfer_id(&payload.transfer_id)?;
        self.registry
            .lock()
            .map_err(|_| registry_unavailable())?
            .records
            .get(&payload.transfer_id)
            .map(|record| record.snapshot.clone())
            .ok_or_else(transfer_not_found)
    }

    pub fn list(
        &self,
        payload: SftpTransferListPayload,
    ) -> Result<Vec<SftpTransferSnapshot>, AppError> {
        let limit = usize::from(payload.limit.unwrap_or(50).min(100));
        let registry = self.registry.lock().map_err(|_| registry_unavailable())?;
        Ok(registry
            .order
            .iter()
            .rev()
            .filter_map(|id| registry.records.get(id))
            .filter(|record| {
                payload
                    .connection_id
                    .as_ref()
                    .is_none_or(|id| &record.snapshot.connection_id == id)
            })
            .take(limit)
            .map(|record| record.snapshot.clone())
            .collect())
    }

    pub fn subscribe(
        &self,
        payload: SftpTransferIdPayload,
    ) -> Result<watch::Receiver<SftpTransferSnapshot>, AppError> {
        validate_transfer_id(&payload.transfer_id)?;
        self.registry
            .lock()
            .map_err(|_| registry_unavailable())?
            .records
            .get(&payload.transfer_id)
            .map(|record| record.updates.subscribe())
            .ok_or_else(transfer_not_found)
    }

    pub fn cancel(&self, payload: SftpTransferIdPayload) -> Result<SftpTransferSnapshot, AppError> {
        validate_transfer_id(&payload.transfer_id)?;
        let registry = self.registry.lock().map_err(|_| registry_unavailable())?;
        let record = registry
            .records
            .get(&payload.transfer_id)
            .ok_or_else(transfer_not_found)?;
        if !record.snapshot.state.is_terminal() {
            record.cancellation.cancel();
        }
        Ok(record.snapshot.clone())
    }

    pub async fn close_connection(&self, connection_id: &str) {
        let ids = self
            .registry
            .lock()
            .map(|registry| {
                registry
                    .records
                    .iter()
                    .filter(|(_, record)| {
                        record.snapshot.connection_id == connection_id
                            && !record.snapshot.state.is_terminal()
                    })
                    .map(|(id, record)| {
                        record.cancellation.cancel();
                        id.clone()
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.wait_for_terminal(&ids).await;
    }

    pub async fn shutdown(&self) {
        self.shutdown.cancel();
        let ids = self
            .registry
            .lock()
            .map(|registry| {
                registry
                    .records
                    .iter()
                    .filter(|(_, record)| !record.snapshot.state.is_terminal())
                    .map(|(id, record)| {
                        record.cancellation.cancel();
                        id.clone()
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.wait_for_terminal(&ids).await;
    }

    pub fn active_count(&self, connection_id: &str) -> usize {
        self.registry
            .lock()
            .map(|registry| {
                registry
                    .records
                    .values()
                    .filter(|record| {
                        record.snapshot.connection_id == connection_id
                            && !record.snapshot.state.is_terminal()
                    })
                    .count()
                    + registry
                        .pending_by_connection
                        .get(connection_id)
                        .copied()
                        .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    fn ensure_connection_ready(&self, connection_id: &str) -> Result<(), AppError> {
        let connection = self.connections.get(connection_id)?;
        if connection.state != ConnectionState::Ready {
            return Err(
                AppError::new(ErrorCode::ValidationFailed, "errors.connectionNotReady")
                    .with_param("connectionId", connection_id),
            );
        }
        Ok(())
    }

    fn reserve(&self, connection_id: &str) -> Result<TransferReservation, AppError> {
        if self.shutdown.is_cancelled() {
            return Err(AppError::shutting_down());
        }
        let mut registry = self.registry.lock().map_err(|_| registry_unavailable())?;
        let active_global = registry
            .records
            .values()
            .filter(|record| !record.snapshot.state.is_terminal())
            .count();
        let active_connection = registry
            .records
            .values()
            .filter(|record| {
                record.snapshot.connection_id == connection_id
                    && !record.snapshot.state.is_terminal()
            })
            .count();
        let pending_connection = registry
            .pending_by_connection
            .get(connection_id)
            .copied()
            .unwrap_or_default();
        if active_global + registry.pending_global >= MAX_ACTIVE_TRANSFERS
            || active_connection + pending_connection >= 1
        {
            return Err(AppError::new(
                ErrorCode::TransferBusy,
                "errors.transferBusy",
            ));
        }
        registry.pending_global += 1;
        *registry
            .pending_by_connection
            .entry(connection_id.to_owned())
            .or_default() += 1;
        Ok(TransferReservation {
            registry: self.registry.clone(),
            connection_id: connection_id.to_owned(),
            committed: false,
        })
    }

    fn spawn_task<F>(
        &self,
        reservation: TransferReservation,
        snapshot: SftpTransferSnapshot,
        cancellation: CancellationToken,
        updates: watch::Sender<SftpTransferSnapshot>,
        task: F,
    ) -> Result<(), AppError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let (start_tx, start_rx) = oneshot::channel();
        tokio::spawn(async move {
            if start_rx.await.is_ok() {
                task.await;
            }
        });
        reservation.commit(snapshot, cancellation, updates)?;
        let _ = start_tx.send(());
        Ok(())
    }

    fn publish(&self, snapshot: SftpTransferSnapshot) {
        let terminal = snapshot.state.is_terminal();
        if let Ok(mut registry) = self.registry.lock()
            && let Some(record) = registry.records.get_mut(&snapshot.transfer_id)
        {
            record.snapshot = snapshot.clone();
            record.updates.send_replace(snapshot);
            if terminal {
                trim_history(&mut registry);
            }
        }
    }

    async fn wait_for_terminal(&self, ids: &[String]) {
        if ids.is_empty() {
            return;
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let active = self
                .registry
                .lock()
                .map(|registry| {
                    ids.iter().any(|id| {
                        registry
                            .records
                            .get(id)
                            .is_some_and(|record| !record.snapshot.state.is_terminal())
                    })
                })
                .unwrap_or(false);
            if !active || Instant::now() >= deadline {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}

impl TransferReservation {
    fn commit(
        mut self,
        snapshot: SftpTransferSnapshot,
        cancellation: CancellationToken,
        updates: watch::Sender<SftpTransferSnapshot>,
    ) -> Result<(), AppError> {
        let mut registry = self.registry.lock().map_err(|_| registry_unavailable())?;
        release_reservation(&mut registry, &self.connection_id);
        registry.order.push_back(snapshot.transfer_id.clone());
        registry.records.insert(
            snapshot.transfer_id.clone(),
            TransferRecord {
                snapshot,
                cancellation,
                updates,
            },
        );
        self.committed = true;
        Ok(())
    }
}

impl Drop for TransferReservation {
    fn drop(&mut self) {
        if !self.committed
            && let Ok(mut registry) = self.registry.lock()
        {
            release_reservation(&mut registry, &self.connection_id);
        }
    }
}

fn release_reservation(registry: &mut TransferRegistry, connection_id: &str) {
    registry.pending_global = registry.pending_global.saturating_sub(1);
    if let Some(pending) = registry.pending_by_connection.get_mut(connection_id) {
        *pending = pending.saturating_sub(1);
        if *pending == 0 {
            registry.pending_by_connection.remove(connection_id);
        }
    }
}

fn trim_history(registry: &mut TransferRegistry) {
    while registry.records.len() > MAX_TRANSFER_HISTORY {
        let Some(position) = registry.order.iter().position(|id| {
            registry
                .records
                .get(id)
                .is_some_and(|record| record.snapshot.state.is_terminal())
        }) else {
            break;
        };
        if let Some(id) = registry.order.remove(position) {
            registry.records.remove(&id);
        }
    }
}

async fn run_upload(
    manager: SftpTransferManager,
    mut snapshot: SftpTransferSnapshot,
    cancellation: CancellationToken,
    session: RawSftpSession,
    mut local_file: File,
    final_path: String,
    temporary_path: String,
) {
    let mut transferred = 0u64;
    let mut rate = RateTracker::new();
    let open = cancellable(
        &cancellation,
        session.open(
            temporary_path.clone(),
            OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::EXCLUDE,
            FileAttributes::default(),
        ),
    )
    .await;
    let remote_handle = match open {
        None => {
            finish_cancelled_remote(&manager, snapshot, &session, &temporary_path, None).await;
            let _ = session.close_session();
            return;
        }
        Some(Err(error)) => {
            let error = map_sftp_error(error, "openingUploadTemporaryFile");
            let cleanup = cleanup_remote(&session, &temporary_path).await;
            fail_snapshot(
                &manager,
                &mut snapshot,
                error,
                !cleanup,
                (!cleanup).then_some(temporary_path),
            );
            let _ = session.close_session();
            return;
        }
        Some(Ok(handle)) => handle.handle,
    };
    let mut remote_handle = Some(remote_handle);

    snapshot.state = SftpTransferState::Transferring;
    manager.publish(snapshot.clone());
    let mut buffer = vec![0u8; TRANSFER_CHUNK_BYTES];
    let mut transfer_error = None;
    let mut cancelled = false;
    loop {
        let read = match cancellable(&cancellation, local_file.read(&mut buffer)).await {
            None => {
                cancelled = true;
                break;
            }
            Some(Err(error)) => {
                transfer_error = Some(map_local_error(error, "readingUploadSource"));
                break;
            }
            Some(Ok(0)) => break,
            Some(Ok(read)) => read,
        };
        let handle = remote_handle.as_deref().expect("remote upload handle");
        match cancellable(
            &cancellation,
            session.write(handle, transferred, buffer[..read].to_vec()),
        )
        .await
        {
            None => {
                cancelled = true;
                break;
            }
            Some(Err(error)) => {
                transfer_error = Some(map_sftp_error(error, "writingUploadChunk"));
                break;
            }
            Some(Ok(_)) => {
                transferred = transferred.saturating_add(read as u64);
                update_progress(&manager, &mut snapshot, transferred, &mut rate);
            }
        }
    }

    if transfer_error.is_none()
        && !cancelled
        && snapshot
            .total_bytes
            .is_some_and(|total| transferred != total.0)
    {
        transfer_error = Some(AppError::new(
            ErrorCode::LocalFileOperationFailed,
            "errors.localUploadFileChanged",
        ));
    }

    snapshot.transferred_bytes = DecimalU64(transferred);
    if cancellation.is_cancelled() {
        cancelled = true;
    }
    if cancelled {
        session.set_timeout(1);
    }
    if cancelled || transfer_error.is_some() {
        let close_ok = close_remote_handle(&session, remote_handle.take())
            .await
            .is_ok();
        let cleanup = close_ok && cleanup_remote(&session, &temporary_path).await;
        let _ = session.close_session();
        if cancelled {
            finish_cancelled(
                &manager,
                snapshot,
                !cleanup,
                (!cleanup).then_some(temporary_path),
            );
        } else {
            let error = transfer_error.expect("transfer error is present");
            fail_snapshot(
                &manager,
                &mut snapshot,
                error,
                !cleanup,
                (!cleanup).then_some(temporary_path),
            );
        }
        return;
    }

    let local_final_size = match local_file.metadata().await {
        Ok(metadata) => metadata.len(),
        Err(error) => {
            let close_ok = close_remote_handle(&session, remote_handle.take())
                .await
                .is_ok();
            let cleanup = close_ok && cleanup_remote(&session, &temporary_path).await;
            let _ = session.close_session();
            fail_snapshot(
                &manager,
                &mut snapshot,
                map_local_error(error, "checkingUploadSource"),
                !cleanup,
                (!cleanup).then_some(temporary_path),
            );
            return;
        }
    };
    if local_final_size != transferred {
        let close_ok = close_remote_handle(&session, remote_handle.take())
            .await
            .is_ok();
        let cleanup = close_ok && cleanup_remote(&session, &temporary_path).await;
        let _ = session.close_session();
        fail_snapshot(
            &manager,
            &mut snapshot,
            AppError::new(
                ErrorCode::LocalFileOperationFailed,
                "errors.localUploadFileChanged",
            ),
            !cleanup,
            (!cleanup).then_some(temporary_path),
        );
        return;
    }

    snapshot.state = SftpTransferState::Finalizing;
    manager.publish(snapshot.clone());
    if cancellation.is_cancelled() {
        session.set_timeout(1);
        let close_ok = close_remote_handle(&session, remote_handle.take())
            .await
            .is_ok();
        let cleanup = close_ok && cleanup_remote(&session, &temporary_path).await;
        let _ = session.close_session();
        finish_cancelled(
            &manager,
            snapshot,
            !cleanup,
            (!cleanup).then_some(temporary_path),
        );
        return;
    }
    if let Err(error) = close_remote_handle(&session, remote_handle.take()).await {
        let cleanup = cleanup_remote(&session, &temporary_path).await;
        let _ = session.close_session();
        fail_snapshot(
            &manager,
            &mut snapshot,
            map_sftp_error(error, "closingUploadTemporaryFile"),
            !cleanup,
            (!cleanup).then_some(temporary_path),
        );
        return;
    }
    match remote_path_exists(&session, &final_path).await {
        Ok(true) => {
            let cleanup = cleanup_remote(&session, &temporary_path).await;
            let _ = session.close_session();
            fail_snapshot(
                &manager,
                &mut snapshot,
                target_exists("publishingUpload"),
                !cleanup,
                (!cleanup).then_some(temporary_path),
            );
            return;
        }
        Ok(false) => {}
        Err(error) => {
            let cleanup = cleanup_remote(&session, &temporary_path).await;
            let _ = session.close_session();
            fail_snapshot(
                &manager,
                &mut snapshot,
                error,
                !cleanup,
                (!cleanup).then_some(temporary_path),
            );
            return;
        }
    }
    match session.rename(temporary_path.clone(), final_path).await {
        Ok(_) => {
            let _ = session.close_session();
            snapshot.state = SftpTransferState::Completed;
            snapshot.transferred_bytes = DecimalU64(transferred);
            snapshot.temporary_path = None;
            manager.publish(snapshot);
        }
        Err(error) => {
            let publish_unsupported = matches!(
                &error,
                SftpError::Status(status) if status.status_code == StatusCode::OpUnsupported
            );
            let mapped = map_sftp_error(error, "publishingUpload");
            let target_present = remote_path_exists(&session, &snapshot.final_path)
                .await
                .is_ok_and(|exists| exists);
            let (error, preserve_temporary_file) =
                classify_remote_publish_failure(mapped, publish_unsupported, target_present);
            if !preserve_temporary_file {
                let removed = cleanup_remote(&session, &temporary_path).await;
                let _ = session.close_session();
                fail_snapshot(
                    &manager,
                    &mut snapshot,
                    error,
                    !removed,
                    (!removed).then_some(temporary_path),
                );
            } else {
                let _ = session.close_session();
                fail_snapshot(&manager, &mut snapshot, error, true, Some(temporary_path));
            }
        }
    }
}

async fn run_download(
    manager: SftpTransferManager,
    mut snapshot: SftpTransferSnapshot,
    cancellation: CancellationToken,
    session: RawSftpSession,
    remote_handle: String,
    local_target: PathBuf,
    temporary_path: PathBuf,
) {
    let mut local_file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)
        .await
    {
        Ok(file) => file,
        Err(error) => {
            let _ = session.close(remote_handle).await;
            let _ = session.close_session();
            fail_snapshot(
                &manager,
                &mut snapshot,
                map_local_error(error, "creatingDownloadTemporaryFile"),
                false,
                None,
            );
            return;
        }
    };
    if cancellation.is_cancelled() {
        drop(local_file);
        let cleanup = cleanup_local(&temporary_path).await;
        let _ = session.close(remote_handle).await;
        let _ = session.close_session();
        finish_cancelled(
            &manager,
            snapshot,
            !cleanup,
            (!cleanup).then(|| temporary_path.to_string_lossy().into_owned()),
        );
        return;
    }

    snapshot.state = SftpTransferState::Transferring;
    manager.publish(snapshot.clone());
    let mut transferred = 0u64;
    let mut rate = RateTracker::new();
    let mut transfer_error = None;
    let mut cancelled = false;
    loop {
        let data = match cancellable(
            &cancellation,
            session.read(
                remote_handle.clone(),
                transferred,
                TRANSFER_CHUNK_BYTES as u32,
            ),
        )
        .await
        {
            None => {
                cancelled = true;
                break;
            }
            Some(Ok(data)) if data.data.is_empty() => break,
            Some(Ok(data)) => data.data,
            Some(Err(SftpError::Status(status))) if status.status_code == StatusCode::Eof => {
                break;
            }
            Some(Err(error)) => {
                transfer_error = Some(map_sftp_error(error, "readingDownloadChunk"));
                break;
            }
        };
        if let Err(error) = local_file.write_all(&data).await {
            transfer_error = Some(map_local_error(error, "writingDownloadTemporaryFile"));
            break;
        }
        transferred = transferred.saturating_add(data.len() as u64);
        update_progress(&manager, &mut snapshot, transferred, &mut rate);
    }
    snapshot.transferred_bytes = DecimalU64(transferred);
    if cancellation.is_cancelled() {
        cancelled = true;
    }
    if transfer_error.is_none()
        && !cancelled
        && snapshot
            .total_bytes
            .is_some_and(|total| transferred != total.0)
    {
        transfer_error = Some(AppError::new(
            ErrorCode::SftpOperationFailed,
            "errors.sftpDownloadSourceChanged",
        ));
    }
    if transfer_error.is_none()
        && !cancelled
        && let Err(error) = local_file.flush().await
    {
        transfer_error = Some(map_local_error(error, "flushingDownloadTemporaryFile"));
    }
    if transfer_error.is_none()
        && !cancelled
        && let Err(error) = local_file.sync_all().await
    {
        transfer_error = Some(map_local_error(error, "syncingDownloadTemporaryFile"));
    }
    drop(local_file);
    if cancelled {
        session.set_timeout(1);
    }
    let close_result = session.close(remote_handle).await;
    if transfer_error.is_none()
        && !cancelled
        && let Err(error) = close_result
    {
        transfer_error = Some(map_sftp_error(error, "closingDownloadSource"));
    }
    if cancelled || transfer_error.is_some() {
        if cancelled {
            session.set_timeout(1);
        }
        let _ = session.close_session();
        let cleanup = cleanup_local(&temporary_path).await;
        if cancelled {
            finish_cancelled(
                &manager,
                snapshot,
                !cleanup,
                (!cleanup).then(|| temporary_path.to_string_lossy().into_owned()),
            );
        } else {
            fail_snapshot(
                &manager,
                &mut snapshot,
                transfer_error.expect("transfer error is present"),
                !cleanup,
                (!cleanup).then(|| temporary_path.to_string_lossy().into_owned()),
            );
        }
        return;
    }

    snapshot.state = SftpTransferState::Finalizing;
    manager.publish(snapshot.clone());
    if cancellation.is_cancelled() {
        session.set_timeout(1);
        let _ = session.close_session();
        let cleanup = cleanup_local(&temporary_path).await;
        finish_cancelled(
            &manager,
            snapshot,
            !cleanup,
            (!cleanup).then(|| temporary_path.to_string_lossy().into_owned()),
        );
        return;
    }
    let _ = session.close_session();
    match publish_local_no_clobber(&temporary_path, &local_target).await {
        Ok(()) => {
            let cleanup = cleanup_local(&temporary_path).await;
            snapshot.state = SftpTransferState::Completed;
            snapshot.transferred_bytes = DecimalU64(transferred);
            snapshot.cleanup_required = !cleanup;
            snapshot.temporary_path =
                (!cleanup).then(|| temporary_path.to_string_lossy().into_owned());
            manager.publish(snapshot);
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let cleanup = cleanup_local(&temporary_path).await;
            fail_snapshot(
                &manager,
                &mut snapshot,
                target_exists("publishingDownload"),
                !cleanup,
                (!cleanup).then(|| temporary_path.to_string_lossy().into_owned()),
            );
        }
        Err(error) => {
            let cleanup = map_local_error(error, "publishingDownload");
            fail_snapshot(
                &manager,
                &mut snapshot,
                if cleanup.code == ErrorCode::LocalDiskFull {
                    cleanup
                } else {
                    AppError::new(ErrorCode::PublishUnsupported, "errors.publishUnsupported")
                        .with_stage("publishingDownload")
                },
                true,
                Some(temporary_path.to_string_lossy().into_owned()),
            );
        }
    }
}

async fn cancellable<F: Future>(cancellation: &CancellationToken, future: F) -> Option<F::Output> {
    tokio::select! {
        _ = cancellation.cancelled() => None,
        output = future => Some(output),
    }
}

fn initial_snapshot(
    transfer_id: String,
    connection_id: String,
    direction: SftpTransferDirection,
    file_name: String,
    final_path: String,
    total_bytes: Option<u64>,
) -> SftpTransferSnapshot {
    SftpTransferSnapshot {
        transfer_id,
        connection_id,
        direction,
        file_name,
        final_path,
        total_bytes: total_bytes.map(DecimalU64),
        transferred_bytes: DecimalU64(0),
        bytes_per_second: None,
        remaining_seconds: None,
        state: SftpTransferState::Created,
        error: None,
        cleanup_required: false,
        temporary_path: None,
    }
}

impl RateTracker {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            samples: VecDeque::from([(now, 0)]),
            last_published: now,
        }
    }

    fn update(&mut self, snapshot: &mut SftpTransferSnapshot, transferred: u64) -> bool {
        let now = Instant::now();
        snapshot.transferred_bytes = DecimalU64(transferred);
        if now.duration_since(self.last_published) < PROGRESS_INTERVAL {
            return false;
        }
        self.last_published = now;
        self.samples.push_back((now, transferred));
        while self.samples.len() > 2
            && now.duration_since(self.samples.front().expect("rate sample").0) > RATE_WINDOW
        {
            self.samples.pop_front();
        }
        snapshot.bytes_per_second = self.samples.front().and_then(|(start, start_bytes)| {
            let seconds = now.duration_since(*start).as_secs_f64();
            (seconds > 0.0).then(|| transferred.saturating_sub(*start_bytes) as f64 / seconds)
        });
        snapshot.remaining_seconds = snapshot.total_bytes.and_then(|total| {
            snapshot
                .bytes_per_second
                .filter(|rate| *rate > 0.0)
                .map(|rate| total.0.saturating_sub(transferred) as f64 / rate)
                .map(|seconds| seconds.ceil() as u64)
        });
        true
    }
}

fn update_progress(
    manager: &SftpTransferManager,
    snapshot: &mut SftpTransferSnapshot,
    transferred: u64,
    rate: &mut RateTracker,
) {
    if rate.update(snapshot, transferred) {
        manager.publish(snapshot.clone());
    } else {
        snapshot.transferred_bytes = DecimalU64(transferred);
    }
}

async fn close_remote_handle(
    session: &RawSftpSession,
    handle: Option<String>,
) -> Result<(), SftpError> {
    let Some(handle) = handle else {
        return Ok(());
    };
    session.close(handle).await.map(|_| ())
}

async fn cleanup_remote(session: &RawSftpSession, path: &str) -> bool {
    session.set_timeout(1);
    match session.remove(path.to_owned()).await {
        Ok(_) => true,
        Err(SftpError::Status(status)) if status.status_code == StatusCode::NoSuchFile => true,
        Err(_) => false,
    }
}

async fn cleanup_local(path: &PathBuf) -> bool {
    match tokio::fs::remove_file(path).await {
        Ok(()) => true,
        Err(error) if error.kind() == io::ErrorKind::NotFound => true,
        Err(_) => false,
    }
}

async fn finish_cancelled_remote(
    manager: &SftpTransferManager,
    snapshot: SftpTransferSnapshot,
    session: &RawSftpSession,
    path: &str,
    handle: Option<String>,
) {
    session.set_timeout(1);
    let close_ok = close_remote_handle(session, handle).await.is_ok();
    let cleanup = close_ok && cleanup_remote(session, path).await;
    finish_cancelled(
        manager,
        snapshot,
        !cleanup,
        (!cleanup).then(|| path.to_owned()),
    );
}

fn finish_cancelled(
    manager: &SftpTransferManager,
    mut snapshot: SftpTransferSnapshot,
    cleanup_required: bool,
    temporary_path: Option<String>,
) {
    snapshot.state = SftpTransferState::Cancelled;
    snapshot.cleanup_required = cleanup_required;
    snapshot.temporary_path = temporary_path;
    manager.publish(snapshot);
}

fn fail_snapshot(
    manager: &SftpTransferManager,
    snapshot: &mut SftpTransferSnapshot,
    error: AppError,
    cleanup_required: bool,
    temporary_path: Option<String>,
) {
    snapshot.state = SftpTransferState::Failed;
    snapshot.error = Some(error);
    snapshot.cleanup_required = cleanup_required;
    snapshot.temporary_path = temporary_path;
    manager.publish(snapshot.clone());
}

fn local_temporary_path(target: &PathBuf, transfer_id: &str) -> Result<PathBuf, AppError> {
    let parent = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| validation("localFileToken", "errors.localDownloadPathInvalid"))?;
    target
        .file_name()
        .ok_or_else(|| validation("localFileToken", "errors.localDownloadPathInvalid"))?;
    Ok(parent.join(format!(".maulink-{transfer_id}.part")))
}

async fn publish_local_no_clobber(temporary_path: &PathBuf, target: &PathBuf) -> io::Result<()> {
    tokio::fs::hard_link(temporary_path, target).await
}

fn map_local_error(error: io::Error, stage: &'static str) -> AppError {
    let (code, key) = if error.kind() == io::ErrorKind::StorageFull {
        (ErrorCode::LocalDiskFull, "errors.localDiskFull")
    } else {
        (
            ErrorCode::LocalFileOperationFailed,
            "errors.localFileOperationFailed",
        )
    };
    AppError::new(code, key).with_stage(stage)
}

fn classify_remote_publish_failure(
    mapped_error: AppError,
    publish_unsupported: bool,
    target_present: bool,
) -> (AppError, bool) {
    if matches!(
        mapped_error.code,
        ErrorCode::ConnectionLost | ErrorCode::ConnectionTimeout
    ) {
        (
            AppError::new(
                ErrorCode::TransferOutcomeUnknown,
                "errors.transferOutcomeUnknown",
            )
            .with_stage("publishingUpload"),
            true,
        )
    } else if target_present {
        (target_exists("publishingUpload"), false)
    } else if publish_unsupported {
        (
            AppError::new(ErrorCode::PublishUnsupported, "errors.publishUnsupported")
                .with_stage("publishingUpload"),
            true,
        )
    } else {
        (mapped_error, false)
    }
}

fn validate_transfer_id(transfer_id: &str) -> Result<(), AppError> {
    Uuid::parse_str(transfer_id)
        .map(|_| ())
        .map_err(|_| validation("transferId", "errors.transferIdInvalid"))
}

fn registry_unavailable() -> AppError {
    AppError::new(ErrorCode::Internal, "errors.transferRegistryUnavailable")
        .with_stage("sftpTransfer")
}

fn transfer_not_found() -> AppError {
    AppError::new(ErrorCode::ResourceNotFound, "errors.transferNotFound")
}

fn validation(field: &'static str, message_key: &'static str) -> AppError {
    super::validation(field, message_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn local_publish_never_replaces_a_concurrent_target() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let first_temporary = directory.path().join("first.part");
        let second_temporary = directory.path().join("second.part");
        let target = directory.path().join("result.bin");
        tokio::fs::write(&first_temporary, b"first")
            .await
            .expect("write first temporary file");
        tokio::fs::write(&second_temporary, b"second")
            .await
            .expect("write second temporary file");

        let (first, second) = tokio::join!(
            publish_local_no_clobber(&first_temporary, &target),
            publish_local_no_clobber(&second_temporary, &target),
        );
        assert_ne!(first.is_ok(), second.is_ok());
        let collision = first
            .err()
            .or_else(|| second.err())
            .expect("one target collision");
        assert_eq!(collision.kind(), io::ErrorKind::AlreadyExists);
        let published = tokio::fs::read(target)
            .await
            .expect("read published target");
        assert!(published.as_slice() == b"first" || published.as_slice() == b"second");
    }

    #[test]
    fn local_file_errors_keep_disk_full_distinct_from_other_failures() {
        assert_eq!(
            map_local_error(io::Error::from(io::ErrorKind::StorageFull), "writing").code,
            ErrorCode::LocalDiskFull
        );
        assert_eq!(
            map_local_error(io::Error::from(io::ErrorKind::PermissionDenied), "writing").code,
            ErrorCode::LocalFileOperationFailed
        );
    }

    #[test]
    fn remote_publish_failures_keep_conflict_unsupported_and_unknown_distinct() {
        let (conflict, preserve_conflict_temporary) = classify_remote_publish_failure(
            AppError::new(ErrorCode::SftpOperationFailed, "errors.sftpOperationFailed"),
            false,
            true,
        );
        assert_eq!(conflict.code, ErrorCode::TargetExists);
        assert!(!preserve_conflict_temporary);

        let (unsupported, preserve_unsupported_temporary) = classify_remote_publish_failure(
            AppError::new(ErrorCode::SftpOperationFailed, "errors.sftpOperationFailed"),
            true,
            false,
        );
        assert_eq!(unsupported.code, ErrorCode::PublishUnsupported);
        assert!(preserve_unsupported_temporary);

        let (unknown, preserve_unknown_temporary) = classify_remote_publish_failure(
            AppError::new(ErrorCode::ConnectionLost, "errors.connectionLost"),
            false,
            false,
        );
        assert_eq!(unknown.code, ErrorCode::TransferOutcomeUnknown);
        assert!(preserve_unknown_temporary);
    }
}
