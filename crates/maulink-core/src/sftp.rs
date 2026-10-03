use std::{
    collections::{HashMap, VecDeque},
    io,
    pin::Pin,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use russh_sftp::{
    client::{RawSftpSession, error::Error as SftpError},
    protocol::{File, FileAttributes, FileType, OpenFlags, StatusCode},
};
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    sync::Mutex,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

mod transfer;
pub use transfer::SftpTransferManager;

use crate::{
    AppError, ConnectionState, DecimalU64, ErrorCode, RemoteFileEntry, RemoteFileType,
    SftpCursorPayload, SftpDirectoryPage, SftpListStartPayload, SshConnectionManager,
};

const MAX_DIRECTORY_ENTRIES: usize = 200;
const MAX_DIRECTORY_PAGE_BYTES: usize = 256 * 1024;
const MAX_EDITABLE_TEXT_FILE_BYTES: usize = 2 * 1024 * 1024;
const TEXT_FILE_IO_CHUNK_BYTES: usize = 64 * 1024;
pub(super) const MAX_SFTP_PACKET_BYTES: u32 = 256 * 1024;
pub(super) const MAX_REMOTE_PATH_BYTES: usize = 16 * 1024;
const MAX_CURSORS_PER_CONNECTION: usize = 4;
const CURSOR_TTL: Duration = Duration::from_secs(30);
const CURSOR_CLEANUP_INTERVAL: Duration = Duration::from_secs(1);
pub(super) const SFTP_REQUEST_TIMEOUT_SECONDS: u64 = 10;
const CURSOR_ID_FOR_SIZE_CHECK: &str = "00000000-0000-0000-0000-000000000000";
const MAX_FILTERED_READDIR_BATCHES: usize = 8;

#[derive(Clone)]
pub struct SftpManager {
    connections: SshConnectionManager,
    cursors: Arc<StdMutex<CursorRegistry>>,
    shutdown: CancellationToken,
    reaper_started: Arc<AtomicBool>,
}

#[derive(Default)]
struct CursorRegistry {
    entries: HashMap<String, CursorRecord>,
    opening_by_connection: HashMap<String, usize>,
}

struct CursorRecord {
    connection_id: String,
    cursor: Arc<Mutex<DirectoryCursor>>,
}

struct DirectoryCursor {
    connection_id: String,
    path: String,
    session: Option<RawSftpSession>,
    remote_handle: Option<String>,
    buffered_entries: VecDeque<RemoteFileEntry>,
    eof: bool,
    expires_at: Instant,
}

struct CursorSlotReservation {
    registry: Arc<StdMutex<CursorRegistry>>,
    connection_id: String,
    committed: bool,
}

impl SftpManager {
    pub fn new(connections: SshConnectionManager) -> Self {
        Self {
            connections,
            cursors: Arc::new(StdMutex::new(CursorRegistry::default())),
            shutdown: CancellationToken::new(),
            reaper_started: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn list_start(
        &self,
        payload: SftpListStartPayload,
    ) -> Result<SftpDirectoryPage, AppError> {
        self.ensure_reaper();
        self.reap_expired().await;
        validate_remote_path(&payload.path)?;
        let snapshot = self.connections.get(&payload.connection_id)?;
        if snapshot.state != ConnectionState::Ready {
            return Err(connection_not_ready(&payload.connection_id));
        }
        let _slot = self.reserve_cursor_slot(&payload.connection_id)?;

        let session = self.open_session(&payload.connection_id).await?;

        let requested_path = if payload.path.is_empty() {
            "."
        } else {
            payload.path.as_str()
        };
        let resolved = session
            .realpath(requested_path)
            .await
            .map_err(|error| map_sftp_error(error, "resolvingSftpPath"))?;
        let path = resolved
            .files
            .first()
            .map(|entry| entry.filename.clone())
            .ok_or_else(|| sftp_operation_failed("resolvingSftpPath"))?;
        validate_remote_path(&path)?;
        if !path.starts_with('/') {
            return Err(sftp_operation_failed("resolvingSftpPath"));
        }

        let handle = session
            .opendir(path.clone())
            .await
            .map_err(|error| map_sftp_error(error, "openingDirectory"))?;
        let mut cursor = DirectoryCursor {
            connection_id: payload.connection_id,
            path: path.clone(),
            session: Some(session),
            remote_handle: Some(handle.handle),
            buffered_entries: VecDeque::new(),
            eof: false,
            expires_at: Instant::now() + CURSOR_TTL,
        };

        let (entries, has_more) = match read_directory_page(&mut cursor).await {
            Ok(page) => page,
            Err(error) => {
                let _ = close_directory_cursor(&mut cursor).await;
                return Err(error);
            }
        };
        let mut page = SftpDirectoryPage {
            path,
            entries,
            cursor_id: None,
        };
        if !has_more {
            let _ = close_directory_cursor(&mut cursor).await;
            return Ok(page);
        }

        cursor.expires_at = Instant::now() + CURSOR_TTL;
        let cursor_id = Uuid::new_v4().to_string();
        page.cursor_id = Some(cursor_id.clone());
        let cursor = Arc::new(Mutex::new(cursor));
        _slot.commit(cursor_id, cursor)?;
        Ok(page)
    }

    pub async fn list_next(
        &self,
        payload: SftpCursorPayload,
    ) -> Result<SftpDirectoryPage, AppError> {
        self.reap_expired().await;
        validate_cursor_id(&payload.cursor_id)?;
        let cursor = self
            .lookup_cursor(&payload.cursor_id)?
            .ok_or_else(cursor_closed)?;
        let mut cursor_guard = cursor.lock().await;
        if !self.cursor_is_registered(&payload.cursor_id, &cursor)? {
            return Err(cursor_closed());
        }
        if cursor_guard.expires_at <= Instant::now() {
            self.remove_cursor(&payload.cursor_id, &cursor)?;
            let _ = close_directory_cursor(&mut cursor_guard).await;
            return Err(cursor_closed());
        }

        let (entries, has_more) = match read_directory_page(&mut cursor_guard).await {
            Ok(page) => page,
            Err(error) => {
                self.remove_cursor(&payload.cursor_id, &cursor)?;
                let _ = close_directory_cursor(&mut cursor_guard).await;
                return Err(error);
            }
        };
        let path = cursor_guard.path.clone();
        if has_more {
            cursor_guard.expires_at = Instant::now() + CURSOR_TTL;
            Ok(SftpDirectoryPage {
                path,
                entries,
                cursor_id: Some(payload.cursor_id),
            })
        } else {
            self.remove_cursor(&payload.cursor_id, &cursor)?;
            let _ = close_directory_cursor(&mut cursor_guard).await;
            Ok(SftpDirectoryPage {
                path,
                entries,
                cursor_id: None,
            })
        }
    }

    pub async fn list_close(&self, payload: SftpCursorPayload) -> Result<(), AppError> {
        validate_cursor_id(&payload.cursor_id)?;
        let Some(cursor) = self.lookup_cursor(&payload.cursor_id)? else {
            return Ok(());
        };
        let mut cursor_guard = cursor.lock().await;
        self.remove_cursor(&payload.cursor_id, &cursor)?;
        close_directory_cursor(&mut cursor_guard).await
    }

    pub async fn stat(&self, payload: crate::SftpStatPayload) -> Result<RemoteFileEntry, AppError> {
        validate_remote_path(&payload.path)?;
        let session = self.open_session(&payload.connection_id).await?;
        let attrs = if payload.follow_symlink {
            session.stat(payload.path.clone()).await
        } else {
            session.lstat(payload.path.clone()).await
        }
        .map_err(|error| map_sftp_error(error, "statPath"))?
        .attrs;
        remote_entry_from_attributes(&payload.path, remote_display_name(&payload.path), attrs)
    }

    pub async fn read_text(
        &self,
        payload: crate::SftpReadTextPayload,
    ) -> Result<crate::SftpReadTextResult, AppError> {
        validate_remote_path(&payload.path)?;
        let session = self.open_session(&payload.connection_id).await?;
        let result = read_editable_text_file(&session, &payload.path).await;
        let _ = session.close_session();
        result.map(|(content, revision)| crate::SftpReadTextResult { content, revision })
    }

    pub async fn write_text(
        &self,
        payload: crate::SftpWriteTextPayload,
    ) -> Result<crate::SftpWriteTextResult, AppError> {
        validate_remote_path(&payload.path)?;
        if payload.content.len() > MAX_EDITABLE_TEXT_FILE_BYTES {
            return Err(text_file_too_large());
        }
        let session = self.open_session(&payload.connection_id).await?;
        let result = async {
            let (_, current_revision) = read_editable_text_file(&session, &payload.path).await?;
            if current_revision != payload.expected_revision {
                return Err(AppError::new(
                    ErrorCode::RevisionConflict,
                    "errors.sftpFileChangedDuringEdit",
                ));
            }

            let handle = session
                .open(
                    payload.path.clone(),
                    OpenFlags::WRITE | OpenFlags::TRUNCATE,
                    FileAttributes::default(),
                )
                .await
                .map_err(|error| map_sftp_error(error, "openingTextFileForWrite"))?
                .handle;
            let write_result = async {
                for (index, chunk) in payload
                    .content
                    .as_bytes()
                    .chunks(TEXT_FILE_IO_CHUNK_BYTES)
                    .enumerate()
                {
                    session
                        .write(
                            handle.clone(),
                            (index * TEXT_FILE_IO_CHUNK_BYTES) as u64,
                            chunk.to_vec(),
                        )
                        .await
                        .map_err(|error| map_sftp_error(error, "writingTextFile"))?;
                }
                Ok::<(), AppError>(())
            }
            .await;
            let close_result = session.close(handle).await;
            write_result?;
            close_result.map_err(|error| map_sftp_error(error, "closingTextFile"))?;

            Ok(crate::SftpWriteTextResult {
                revision: text_revision(payload.content.as_bytes()),
            })
        }
        .await;
        let _ = session.close_session();
        result
    }

    pub async fn mkdir(
        &self,
        payload: crate::SftpMkdirPayload,
    ) -> Result<RemoteFileEntry, AppError> {
        validate_remote_path(&payload.parent_path)?;
        validate_basename(&payload.name, "name")?;
        let session = self.open_session(&payload.connection_id).await?;
        let parent = resolve_remote_path(&session, &payload.parent_path).await?;
        let path = join_remote_path(&parent, &payload.name);
        if remote_path_exists(&session, &path).await? {
            return Err(path_exists("creatingDirectory"));
        }
        if let Err(error) = session.mkdir(path.clone(), FileAttributes::default()).await {
            return Err(conflict_error(
                &session,
                &path,
                error,
                ErrorCode::PathExists,
                "creatingDirectory",
            )
            .await);
        }
        let attrs = session
            .lstat(path.clone())
            .await
            .map_err(|error| map_sftp_error(error, "readingCreatedDirectory"))?
            .attrs;
        remote_entry_from_attributes(&path, payload.name, attrs)
    }

    pub async fn rename(
        &self,
        payload: crate::SftpRenamePayload,
    ) -> Result<RemoteFileEntry, AppError> {
        validate_remote_path(&payload.source_path)?;
        validate_basename(&payload.new_name, "newName")?;
        let (source_parent, source_name) = remote_parent_and_name(&payload.source_path)?;
        let session = self.open_session(&payload.connection_id).await?;
        let parent = resolve_remote_path(&session, &source_parent).await?;
        let source = join_remote_path(&parent, &source_name);
        let destination = join_remote_path(&parent, &payload.new_name);
        session
            .lstat(source.clone())
            .await
            .map_err(|error| map_sftp_error(error, "checkingRenameSource"))?;
        if remote_path_exists(&session, &destination).await? {
            return Err(target_exists("renamingPath"));
        }
        if let Err(error) = session.rename(source, destination.clone()).await {
            return Err(conflict_error(
                &session,
                &destination,
                error,
                ErrorCode::TargetExists,
                "renamingPath",
            )
            .await);
        }
        let attrs = session
            .lstat(destination.clone())
            .await
            .map_err(|error| map_sftp_error(error, "readingRenamedPath"))?
            .attrs;
        remote_entry_from_attributes(&destination, payload.new_name, attrs)
    }

    pub async fn delete(&self, payload: crate::SftpDeletePayload) -> Result<(), AppError> {
        if !payload.confirmed {
            return Err(validation(
                "confirmed",
                "errors.sftpDeleteConfirmationRequired",
            ));
        }
        validate_remote_path(&payload.path)?;
        let (parent_path, name) = remote_parent_and_name(&payload.path)?;
        let session = self.open_session(&payload.connection_id).await?;
        let parent = resolve_remote_path(&session, &parent_path).await?;
        let path = join_remote_path(&parent, &name);
        let attrs = session
            .lstat(path.clone())
            .await
            .map_err(|error| map_sftp_error(error, "checkingDeleteTarget"))?
            .attrs;
        let actual_type = remote_file_type(&attrs);
        if actual_type != payload.expected_type {
            return Err(validation("expectedType", "errors.sftpEntryTypeChanged"));
        }
        match actual_type {
            RemoteFileType::File | RemoteFileType::Symlink => session
                .remove(path)
                .await
                .map(|_| ())
                .map_err(|error| map_sftp_error(error, "deletingFile")),
            RemoteFileType::Directory => {
                if !remote_directory_is_empty(&session, &path).await? {
                    return Err(directory_not_empty());
                }
                match session.rmdir(path.clone()).await {
                    Ok(_) => Ok(()),
                    Err(error)
                        if is_generic_sftp_failure(&error)
                            && remote_directory_is_empty(&session, &path)
                                .await
                                .is_ok_and(|empty| !empty) =>
                    {
                        Err(directory_not_empty())
                    }
                    Err(error) => Err(map_sftp_error(error, "deletingDirectory")),
                }
            }
            RemoteFileType::Other => Err(validation("path", "errors.sftpDeleteTypeUnsupported")),
        }
    }

    async fn open_session(&self, connection_id: &str) -> Result<RawSftpSession, AppError> {
        open_sftp_session(&self.connections, connection_id).await
    }

    pub async fn close_connection(&self, connection_id: &str) {
        let ids = self
            .cursors
            .lock()
            .map(|registry| {
                registry
                    .entries
                    .iter()
                    .filter(|(_, record)| record.connection_id == connection_id)
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for id in ids {
            if let Ok(Some(cursor)) = self.lookup_cursor(&id) {
                let mut cursor_guard = cursor.lock().await;
                self.remove_cursor(&id, &cursor).ok();
                let _ = close_directory_cursor(&mut cursor_guard).await;
            }
        }
    }

    pub async fn shutdown(&self) {
        self.shutdown.cancel();
        let ids = self
            .cursors
            .lock()
            .map(|registry| registry.entries.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for id in ids {
            if let Ok(Some(cursor)) = self.lookup_cursor(&id) {
                let mut cursor_guard = cursor.lock().await;
                self.remove_cursor(&id, &cursor).ok();
                let _ = close_directory_cursor(&mut cursor_guard).await;
            }
        }
    }

    async fn reap_expired(&self) {
        reap_cursors(&self.cursors, &self.connections).await;
    }

    fn ensure_reaper(&self) {
        if self
            .reaper_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            self.reaper_started.store(false, Ordering::Release);
            return;
        };
        let cursors = Arc::downgrade(&self.cursors);
        let connections = self.connections.clone();
        let shutdown = self.shutdown.clone();
        runtime.spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = tokio::time::sleep(CURSOR_CLEANUP_INTERVAL) => {
                        let Some(cursors) = cursors.upgrade() else { break };
                        reap_cursors(&cursors, &connections).await;
                    }
                }
            }
        });
    }

    fn reserve_cursor_slot(&self, connection_id: &str) -> Result<CursorSlotReservation, AppError> {
        if self.shutdown.is_cancelled() {
            return Err(AppError::shutting_down());
        }
        let mut registry = self
            .cursors
            .lock()
            .map_err(|_| sftp_registry_unavailable())?;
        let active = registry
            .entries
            .values()
            .filter(|cursor| cursor.connection_id == connection_id)
            .count();
        let opening = registry
            .opening_by_connection
            .get(connection_id)
            .copied()
            .unwrap_or_default();
        if active + opening >= MAX_CURSORS_PER_CONNECTION {
            return Err(AppError::new(
                ErrorCode::ResourceLimit,
                "errors.sftpCursorLimitReached",
            ));
        }
        *registry
            .opening_by_connection
            .entry(connection_id.to_owned())
            .or_default() += 1;
        Ok(CursorSlotReservation {
            registry: self.cursors.clone(),
            connection_id: connection_id.to_owned(),
            committed: false,
        })
    }

    fn lookup_cursor(
        &self,
        cursor_id: &str,
    ) -> Result<Option<Arc<Mutex<DirectoryCursor>>>, AppError> {
        self.cursors
            .lock()
            .map(|registry| {
                registry
                    .entries
                    .get(cursor_id)
                    .map(|record| record.cursor.clone())
            })
            .map_err(|_| sftp_registry_unavailable())
    }

    fn cursor_is_registered(
        &self,
        cursor_id: &str,
        cursor: &Arc<Mutex<DirectoryCursor>>,
    ) -> Result<bool, AppError> {
        self.cursors
            .lock()
            .map(|registry| {
                registry
                    .entries
                    .get(cursor_id)
                    .is_some_and(|registered| Arc::ptr_eq(&registered.cursor, cursor))
            })
            .map_err(|_| sftp_registry_unavailable())
    }

    fn remove_cursor(
        &self,
        cursor_id: &str,
        cursor: &Arc<Mutex<DirectoryCursor>>,
    ) -> Result<(), AppError> {
        let mut registry = self
            .cursors
            .lock()
            .map_err(|_| sftp_registry_unavailable())?;
        if registry
            .entries
            .get(cursor_id)
            .is_some_and(|registered| Arc::ptr_eq(&registered.cursor, cursor))
        {
            registry.entries.remove(cursor_id);
        }
        Ok(())
    }
}

impl CursorSlotReservation {
    fn commit(
        mut self,
        cursor_id: String,
        cursor: Arc<Mutex<DirectoryCursor>>,
    ) -> Result<(), AppError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| sftp_registry_unavailable())?;
        decrement_opening(&mut registry, &self.connection_id);
        registry.entries.insert(
            cursor_id,
            CursorRecord {
                connection_id: self.connection_id.clone(),
                cursor,
            },
        );
        self.committed = true;
        Ok(())
    }
}

impl Drop for CursorSlotReservation {
    fn drop(&mut self) {
        if !self.committed
            && let Ok(mut registry) = self.registry.lock()
        {
            decrement_opening(&mut registry, &self.connection_id);
        }
    }
}

async fn read_editable_text_file(
    session: &RawSftpSession,
    path: &str,
) -> Result<(String, String), AppError> {
    let attributes = session
        .lstat(path.to_owned())
        .await
        .map_err(|error| map_sftp_error(error, "checkingTextFile"))?
        .attrs;
    if attributes.file_type() != FileType::File {
        return Err(validation("path", "errors.sftpTextFileUnsupported"));
    }
    if attributes
        .size
        .is_some_and(|size| size > MAX_EDITABLE_TEXT_FILE_BYTES as u64)
    {
        return Err(text_file_too_large());
    }

    let handle = session
        .open(path.to_owned(), OpenFlags::READ, FileAttributes::default())
        .await
        .map_err(|error| map_sftp_error(error, "openingTextFile"))?
        .handle;
    let read_result = async {
        let mut bytes = Vec::with_capacity(
            attributes
                .size
                .unwrap_or_default()
                .min(MAX_EDITABLE_TEXT_FILE_BYTES as u64) as usize,
        );
        loop {
            let remaining = MAX_EDITABLE_TEXT_FILE_BYTES + 1 - bytes.len();
            let length = remaining.min(TEXT_FILE_IO_CHUNK_BYTES) as u32;
            let data = match session
                .read(handle.clone(), bytes.len() as u64, length)
                .await
            {
                Ok(data) if data.data.is_empty() => break,
                Ok(data) => data.data,
                Err(SftpError::Status(status)) if status.status_code == StatusCode::Eof => break,
                Err(error) => return Err(map_sftp_error(error, "readingTextFile")),
            };
            if bytes.len() + data.len() > MAX_EDITABLE_TEXT_FILE_BYTES {
                return Err(text_file_too_large());
            }
            bytes.extend_from_slice(&data);
        }
        if attributes
            .size
            .is_some_and(|size| size != bytes.len() as u64)
        {
            return Err(AppError::new(
                ErrorCode::SftpOperationFailed,
                "errors.sftpTextFileChangedDuringRead",
            ));
        }
        let content = String::from_utf8(bytes)
            .map_err(|_| validation("path", "errors.sftpTextFileInvalidEncoding"))?;
        let revision = text_revision(content.as_bytes());
        Ok((content, revision))
    }
    .await;
    let close_result = session.close(handle).await;
    match read_result {
        Ok(result) => {
            close_result.map_err(|error| map_sftp_error(error, "closingTextFile"))?;
            Ok(result)
        }
        Err(error) => {
            let _ = close_result;
            Err(error)
        }
    }
}

fn text_revision(bytes: &[u8]) -> String {
    STANDARD_NO_PAD.encode(Sha256::digest(bytes))
}

fn text_file_too_large() -> AppError {
    AppError::new(ErrorCode::SftpEntryTooLarge, "errors.sftpTextFileTooLarge")
        .with_param("maxSize", "2 MiB")
}

fn decrement_opening(registry: &mut CursorRegistry, connection_id: &str) {
    if let Some(opening) = registry.opening_by_connection.get_mut(connection_id) {
        *opening = opening.saturating_sub(1);
        if *opening == 0 {
            registry.opening_by_connection.remove(connection_id);
        }
    }
}

async fn reap_cursors(
    registry: &Arc<StdMutex<CursorRegistry>>,
    connections: &SshConnectionManager,
) {
    let candidates = match registry.lock() {
        Ok(registry) => registry
            .entries
            .iter()
            .map(|(id, record)| (id.clone(), record.cursor.clone()))
            .collect::<Vec<_>>(),
        Err(_) => return,
    };
    for (id, cursor) in candidates {
        let mut cursor_guard = cursor.lock().await;
        let expired = cursor_guard.expires_at <= Instant::now();
        let disconnected = connections
            .get(&cursor_guard.connection_id)
            .map_or(true, |snapshot| snapshot.state != ConnectionState::Ready);
        let removed = if expired || disconnected {
            match registry.lock() {
                Ok(mut registry)
                    if registry
                        .entries
                        .get(&id)
                        .is_some_and(|registered| Arc::ptr_eq(&registered.cursor, &cursor)) =>
                {
                    registry.entries.remove(&id);
                    true
                }
                _ => false,
            }
        } else {
            false
        };
        if removed {
            let _ = close_directory_cursor(&mut cursor_guard).await;
        }
    }
}

async fn read_directory_page(
    cursor: &mut DirectoryCursor,
) -> Result<(Vec<RemoteFileEntry>, bool), AppError> {
    let mut entries = Vec::with_capacity(MAX_DIRECTORY_ENTRIES);
    let mut entries_json_bytes = 0usize;
    let mut filtered_batches = 0usize;
    let empty_page_size = serialized_page_size(&cursor.path, &[], true)?;

    while entries.len() < MAX_DIRECTORY_ENTRIES {
        while cursor.buffered_entries.is_empty() && !cursor.eof {
            fetch_directory_batch(cursor).await?;
            if cursor.buffered_entries.is_empty() && !cursor.eof {
                filtered_batches += 1;
                if filtered_batches >= MAX_FILTERED_READDIR_BATCHES {
                    return Err(sftp_operation_failed("readingDirectory"));
                }
            }
        }
        let Some(entry) = cursor.buffered_entries.pop_front() else {
            break;
        };
        let entry_size = serde_json::to_vec(&entry)
            .map_err(|_| sftp_operation_failed("serializingDirectory"))?
            .len();
        let next_count = entries.len() + 1;
        let next_size = empty_page_size
            .saturating_add(entries_json_bytes)
            .saturating_add(usize::from(next_count > 1))
            .saturating_add(entry_size);
        if next_size > MAX_DIRECTORY_PAGE_BYTES {
            cursor.buffered_entries.push_front(entry);
            if entries.is_empty() {
                return Err(AppError::new(
                    ErrorCode::SftpEntryTooLarge,
                    "errors.sftpEntryTooLarge",
                )
                .with_stage("readingDirectory"));
            }
            break;
        }
        entries_json_bytes += entry_size + usize::from(!entries.is_empty());
        entries.push(entry);
    }

    if entries.len() == MAX_DIRECTORY_ENTRIES && cursor.buffered_entries.is_empty() && !cursor.eof {
        while cursor.buffered_entries.is_empty() && !cursor.eof {
            fetch_directory_batch(cursor).await?;
            if cursor.buffered_entries.is_empty() && !cursor.eof {
                filtered_batches += 1;
                if filtered_batches >= MAX_FILTERED_READDIR_BATCHES {
                    return Err(sftp_operation_failed("readingDirectory"));
                }
            }
        }
    }

    let has_more = !cursor.buffered_entries.is_empty();
    Ok((entries, has_more))
}

async fn fetch_directory_batch(cursor: &mut DirectoryCursor) -> Result<(), AppError> {
    let session = cursor.session.as_ref().ok_or_else(cursor_closed)?;
    let handle = cursor.remote_handle.as_deref().ok_or_else(cursor_closed)?;
    match session.readdir(handle).await {
        Ok(batch) => {
            if batch.files.is_empty() {
                cursor.eof = true;
                return Ok(());
            }
            for file in batch.files {
                if let Some(entry) = remote_entry(&cursor.path, file)? {
                    cursor.buffered_entries.push_back(entry);
                }
            }
            Ok(())
        }
        Err(SftpError::Status(status)) if status.status_code == StatusCode::Eof => {
            cursor.eof = true;
            Ok(())
        }
        Err(error) => Err(map_sftp_error(error, "readingDirectory")),
    }
}

fn remote_entry(parent: &str, file: File) -> Result<Option<RemoteFileEntry>, AppError> {
    if file.filename == "." || file.filename == ".." {
        return Ok(None);
    }
    if file.filename.contains('\u{fffd}') {
        return Err(unsupported_path_encoding());
    }
    if file.filename.is_empty() || file.filename.contains('/') || file.filename.contains('\0') {
        return Err(sftp_operation_failed("readingDirectory"));
    }
    let path = join_remote_path(parent, &file.filename);
    if path.len() > MAX_REMOTE_PATH_BYTES {
        return Err(sftp_operation_failed("readingDirectory"));
    }
    remote_entry_from_attributes(&path, file.filename, file.attrs).map(Some)
}

fn remote_entry_from_attributes(
    path: &str,
    name: String,
    attrs: FileAttributes,
) -> Result<RemoteFileEntry, AppError> {
    validate_remote_path(path)?;
    validate_remote_path(&name)?;
    if name.contains('/') && path != "/" {
        return Err(sftp_operation_failed("readingFileAttributes"));
    }
    let file_type = remote_file_type(&attrs);
    Ok(RemoteFileEntry {
        name,
        path: path.to_owned(),
        file_type,
        size_bytes: attrs.size.map(DecimalU64),
        modified_at_ms: attrs.mtime.map(|seconds| i64::from(seconds) * 1_000),
        is_symlink: file_type == RemoteFileType::Symlink,
        permissions: attrs.permissions,
    })
}

pub(super) fn remote_file_type(attrs: &FileAttributes) -> RemoteFileType {
    match attrs.file_type() {
        FileType::File => RemoteFileType::File,
        FileType::Dir => RemoteFileType::Directory,
        FileType::Symlink => RemoteFileType::Symlink,
        FileType::Other => RemoteFileType::Other,
    }
}

pub(super) async fn resolve_remote_path(
    session: &RawSftpSession,
    path: &str,
) -> Result<String, AppError> {
    let resolved = session
        .realpath(path.to_owned())
        .await
        .map_err(|error| map_sftp_error(error, "resolvingSftpPath"))?;
    let path = resolved
        .files
        .first()
        .map(|entry| entry.filename.clone())
        .ok_or_else(|| sftp_operation_failed("resolvingSftpPath"))?;
    validate_remote_path(&path)?;
    if !path.starts_with('/') {
        return Err(sftp_operation_failed("resolvingSftpPath"));
    }
    Ok(path)
}

pub(super) async fn remote_path_exists(
    session: &RawSftpSession,
    path: &str,
) -> Result<bool, AppError> {
    match session.lstat(path.to_owned()).await {
        Ok(_) => Ok(true),
        Err(SftpError::Status(status)) if status.status_code == StatusCode::NoSuchFile => Ok(false),
        Err(error) => Err(map_sftp_error(error, "checkingSftpPath")),
    }
}

async fn conflict_error(
    session: &RawSftpSession,
    path: &str,
    error: SftpError,
    code: ErrorCode,
    stage: &'static str,
) -> AppError {
    if is_generic_sftp_failure(&error) && session.lstat(path.to_owned()).await.is_ok() {
        return match code {
            ErrorCode::PathExists => path_exists(stage),
            _ => target_exists(stage),
        };
    }
    map_sftp_error(error, stage)
}

async fn remote_directory_is_empty(session: &RawSftpSession, path: &str) -> Result<bool, AppError> {
    let handle = session
        .opendir(path.to_owned())
        .await
        .map_err(|error| map_sftp_error(error, "checkingDirectory"))?;
    let mut filtered_batches = 0;
    loop {
        match session.readdir(handle.handle.clone()).await {
            Ok(batch) if batch.files.is_empty() => {
                let _ = session.close(handle.handle.clone()).await;
                return Ok(true);
            }
            Ok(batch)
                if batch
                    .files
                    .iter()
                    .any(|file| file.filename != "." && file.filename != "..") =>
            {
                let _ = session.close(handle.handle.clone()).await;
                return Ok(false);
            }
            Ok(_) => {
                filtered_batches += 1;
                if filtered_batches >= MAX_FILTERED_READDIR_BATCHES {
                    let _ = session.close(handle.handle.clone()).await;
                    return Err(sftp_operation_failed("checkingDirectory"));
                }
            }
            Err(SftpError::Status(status)) if status.status_code == StatusCode::Eof => {
                let _ = session.close(handle.handle.clone()).await;
                return Ok(true);
            }
            Err(error) => {
                let _ = session.close(handle.handle.clone()).await;
                return Err(map_sftp_error(error, "checkingDirectory"));
            }
        }
    }
}

pub(super) fn remote_parent_and_name(path: &str) -> Result<(String, String), AppError> {
    validate_remote_path(path)?;
    let path = path.trim_end_matches('/');
    if path.is_empty() {
        return Err(validation("path", "errors.sftpRootOperationDenied"));
    }
    let separator = path.rfind('/');
    let (parent, name) = match separator {
        Some(0) => ("/", &path[1..]),
        Some(index) => (&path[..index], &path[index + 1..]),
        None => (".", path),
    };
    validate_basename(name, "path")?;
    Ok((parent.to_owned(), name.to_owned()))
}

fn remote_display_name(path: &str) -> String {
    let path = path.trim_end_matches('/');
    if path.is_empty() {
        "/".to_owned()
    } else {
        path.rsplit('/').next().unwrap_or(path).to_owned()
    }
}

pub(super) fn validate_basename(name: &str, field: &'static str) -> Result<(), AppError> {
    validate_remote_path(name)?;
    if name.is_empty() || matches!(name, "." | "..") || name.contains('/') {
        return Err(validation(field, "errors.sftpNameInvalid"));
    }
    Ok(())
}

fn is_generic_sftp_failure(error: &SftpError) -> bool {
    matches!(error, SftpError::Status(status) if status.status_code == StatusCode::Failure)
}

async fn close_directory_cursor(cursor: &mut DirectoryCursor) -> Result<(), AppError> {
    if let Some(session) = cursor.session.as_ref() {
        session.set_timeout(1);
    }
    let result = match (cursor.session.as_ref(), cursor.remote_handle.take()) {
        (Some(session), Some(handle)) => session
            .close(handle)
            .await
            .map(|_| ())
            .map_err(|error| map_sftp_error(error, "closingDirectory")),
        _ => Ok(()),
    };
    if let Some(session) = cursor.session.take() {
        let _ = session.close_session();
    }
    result
}

fn serialized_page_size(
    path: &str,
    entries: &[RemoteFileEntry],
    with_cursor: bool,
) -> Result<usize, AppError> {
    let page = SftpDirectoryPage {
        path: path.to_owned(),
        entries: entries.to_vec(),
        cursor_id: with_cursor.then(|| CURSOR_ID_FOR_SIZE_CHECK.to_owned()),
    };
    serde_json::to_vec(&page)
        .map(|bytes| bytes.len())
        .map_err(|_| sftp_operation_failed("serializingDirectory"))
}

pub(super) fn join_remote_path(parent: &str, name: &str) -> String {
    let parent = parent.trim_end_matches('/');
    if parent.is_empty() {
        format!("/{name}")
    } else {
        format!("{parent}/{name}")
    }
}

pub(super) fn validate_remote_path(path: &str) -> Result<(), AppError> {
    if path.contains('\0') || path.len() > MAX_REMOTE_PATH_BYTES {
        return Err(validation("path", "errors.sftpPathInvalid"));
    }
    if path.contains('\u{fffd}') {
        return Err(unsupported_path_encoding());
    }
    Ok(())
}

fn validate_cursor_id(cursor_id: &str) -> Result<(), AppError> {
    Uuid::parse_str(cursor_id)
        .map(|_| ())
        .map_err(|_| validation("cursorId", "errors.sftpCursorIdInvalid"))
}

pub(super) fn map_sftp_error(error: SftpError, stage: &'static str) -> AppError {
    let (code, message_key) = match error {
        SftpError::Status(status) => match status.status_code {
            StatusCode::NoSuchFile => (ErrorCode::PathNotFound, "errors.pathNotFound"),
            StatusCode::PermissionDenied => {
                (ErrorCode::PermissionDenied, "errors.permissionDenied")
            }
            StatusCode::ConnectionLost | StatusCode::NoConnection => {
                (ErrorCode::ConnectionLost, "errors.connectionLost")
            }
            _ => (ErrorCode::SftpOperationFailed, "errors.sftpOperationFailed"),
        },
        SftpError::Timeout => (ErrorCode::ConnectionTimeout, "errors.sftpTimeout"),
        _ => (ErrorCode::SftpOperationFailed, "errors.sftpOperationFailed"),
    };
    let mut app_error = AppError::new(code, message_key).with_stage(stage);
    if code == ErrorCode::ConnectionTimeout {
        app_error = app_error.with_retry();
    }
    app_error
}

pub(super) fn sftp_operation_failed(stage: &'static str) -> AppError {
    AppError::new(ErrorCode::SftpOperationFailed, "errors.sftpOperationFailed").with_stage(stage)
}

fn path_exists(stage: &'static str) -> AppError {
    AppError::new(ErrorCode::PathExists, "errors.pathExists").with_stage(stage)
}

pub(super) fn target_exists(stage: &'static str) -> AppError {
    AppError::new(ErrorCode::TargetExists, "errors.targetExists").with_stage(stage)
}

fn directory_not_empty() -> AppError {
    AppError::new(ErrorCode::DirectoryNotEmpty, "errors.directoryNotEmpty")
        .with_stage("deletingDirectory")
}

fn unsupported_path_encoding() -> AppError {
    AppError::new(
        ErrorCode::UnsupportedPathEncoding,
        "errors.unsupportedPathEncoding",
    )
    .with_stage("readingDirectory")
}

fn connection_not_ready(connection_id: &str) -> AppError {
    AppError::new(ErrorCode::ValidationFailed, "errors.connectionNotReady")
        .with_param("connectionId", connection_id)
}

fn cursor_closed() -> AppError {
    AppError::new(ErrorCode::ResourceClosed, "errors.sftpCursorClosed")
}

fn sftp_registry_unavailable() -> AppError {
    AppError::new(ErrorCode::Internal, "errors.sftpRegistryUnavailable").with_stage("sftp")
}

pub(super) fn validation(field: &'static str, message_key: &'static str) -> AppError {
    AppError::new(ErrorCode::ValidationFailed, message_key).with_param("field", field)
}

pub(super) async fn open_sftp_session(
    connections: &SshConnectionManager,
    connection_id: &str,
) -> Result<RawSftpSession, AppError> {
    let channel = connections.open_sftp_channel(connection_id).await?;
    let session = RawSftpSession::new(BoundedSftpStream::new(channel, MAX_SFTP_PACKET_BYTES));
    session.set_timeout(SFTP_REQUEST_TIMEOUT_SECONDS);
    session
        .init()
        .await
        .map_err(|error| map_sftp_error(error, "initializingSftp"))?;
    Ok(session)
}

struct BoundedSftpStream<S> {
    inner: S,
    max_packet_len: u32,
    header: [u8; 4],
    header_read: usize,
    header_sent: usize,
    payload_remaining: usize,
}

impl<S> BoundedSftpStream<S> {
    fn new(inner: S, max_packet_len: u32) -> Self {
        Self {
            inner,
            max_packet_len,
            header: [0; 4],
            header_read: 0,
            header_sent: 0,
            payload_remaining: 0,
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for BoundedSftpStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.as_mut().get_mut();
        if output.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        loop {
            if this.header_read < this.header.len() {
                let start = this.header_read;
                let read = {
                    let mut header = ReadBuf::new(&mut this.header[start..]);
                    match Pin::new(&mut this.inner).poll_read(cx, &mut header) {
                        Poll::Ready(Ok(())) => Poll::Ready(Ok(header.filled().len())),
                        Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
                        Poll::Pending => Poll::Pending,
                    }
                };
                match read {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                    Poll::Ready(Ok(0)) if this.header_read == 0 => return Poll::Ready(Ok(())),
                    Poll::Ready(Ok(0)) => {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "truncated SFTP packet length",
                        )));
                    }
                    Poll::Ready(Ok(read)) => this.header_read += read,
                }
                if this.header_read == this.header.len() {
                    let packet_len = u32::from_be_bytes(this.header);
                    if packet_len > this.max_packet_len {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "SFTP packet exceeds configured limit",
                        )));
                    }
                    this.payload_remaining = packet_len as usize;
                }
                continue;
            }

            if this.header_sent < this.header.len() {
                let bytes = (this.header.len() - this.header_sent).min(output.remaining());
                output.put_slice(&this.header[this.header_sent..this.header_sent + bytes]);
                this.header_sent += bytes;
                return Poll::Ready(Ok(()));
            }

            if this.payload_remaining == 0 {
                this.header_read = 0;
                this.header_sent = 0;
                continue;
            }

            let limit = this.payload_remaining.min(output.remaining());
            let read = {
                let mut limited = output.take(limit);
                match Pin::new(&mut this.inner).poll_read(cx, &mut limited) {
                    Poll::Ready(Ok(())) => Poll::Ready(Ok(limited.filled().len())),
                    Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
                    Poll::Pending => Poll::Pending,
                }
            };
            match read {
                Poll::Ready(Ok(read)) => {
                    output.advance(read);
                    this.payload_remaining -= read;
                    return Poll::Ready(Ok(()));
                }
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for BoundedSftpStream<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.as_mut().get_mut().inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.as_mut().get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.as_mut().get_mut().inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use russh_sftp::protocol::{FileAttributes, Status, StatusCode};
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    #[test]
    fn remote_permission_denial_has_a_stable_error_code() {
        let error = map_sftp_error(
            SftpError::Status(Status {
                id: 1,
                status_code: StatusCode::PermissionDenied,
                error_message: String::new(),
                language_tag: String::new(),
            }),
            "readingRemoteFile",
        );
        assert_eq!(error.code, ErrorCode::PermissionDenied);
        assert_eq!(error.message_key, "errors.permissionDenied");
    }

    #[test]
    fn remote_paths_use_posix_joining_and_preserve_control_characters() {
        assert_eq!(join_remote_path("/", "中文 name"), "/中文 name");
        assert_eq!(join_remote_path("/tmp/", "line\nbreak"), "/tmp/line\nbreak");
        assert_eq!(validate_remote_path("/tmp/quoted ' name").unwrap(), ());
        assert_eq!(
            validate_remote_path("/tmp/\0bad").unwrap_err().code,
            ErrorCode::ValidationFailed
        );
        assert_eq!(
            validate_remote_path("/tmp/�").unwrap_err().code,
            ErrorCode::UnsupportedPathEncoding
        );
    }

    #[test]
    fn remote_entry_maps_optional_metadata_and_detects_symlinks() {
        let entry = remote_entry(
            "/srv",
            File {
                filename: "alias".to_owned(),
                longname: String::new(),
                attrs: FileAttributes {
                    size: Some(9_007_199_254_740_993),
                    permissions: Some(0o120777),
                    mtime: Some(42),
                    ..FileAttributes::default()
                },
            },
        )
        .expect("convert remote file")
        .expect("file entry");
        assert_eq!(entry.path, "/srv/alias");
        assert_eq!(entry.file_type, RemoteFileType::Symlink);
        assert!(entry.is_symlink);
        assert_eq!(entry.size_bytes, Some(DecimalU64(9_007_199_254_740_993)));
        assert_eq!(entry.modified_at_ms, Some(42_000));
        let json = serde_json::to_value(entry).expect("serialize file entry");
        assert_eq!(json["sizeBytes"], "9007199254740993");
        assert_eq!(
            remote_entry(
                "/srv",
                File {
                    filename: "invalid-�-name".to_owned(),
                    longname: String::new(),
                    attrs: FileAttributes::default(),
                },
            )
            .expect_err("ambiguous replacement character is rejected")
            .code,
            ErrorCode::UnsupportedPathEncoding
        );
    }

    #[test]
    fn directory_page_size_includes_the_cursor_and_remote_path() {
        let entry = RemoteFileEntry {
            name: "a".repeat(MAX_DIRECTORY_PAGE_BYTES),
            path: "/tmp/name".to_owned(),
            file_type: RemoteFileType::File,
            size_bytes: None,
            modified_at_ms: None,
            is_symlink: false,
            permissions: None,
        };
        assert!(
            serialized_page_size("/tmp", &[entry], true).expect("serialize page")
                > MAX_DIRECTORY_PAGE_BYTES
        );
    }

    #[tokio::test]
    async fn bounded_sftp_stream_rejects_oversized_packet_before_payload_allocation() {
        let (mut writer, reader) = duplex(16);
        writer
            .write_all(&(u32::MAX).to_be_bytes())
            .await
            .expect("write packet length");
        let mut reader = BoundedSftpStream::new(reader, 32);
        let error = reader
            .read_u32()
            .await
            .expect_err("oversized length is rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn bounded_sftp_stream_preserves_consecutive_packet_framing() {
        let (mut writer, reader) = duplex(32);
        writer
            .write_all(&[0, 0, 0, 3, b'a', b'b', b'c', 0, 0, 0, 1, b'z'])
            .await
            .expect("write SFTP frames");
        let mut reader = BoundedSftpStream::new(reader, 8);
        assert_eq!(reader.read_u32().await.expect("first frame length"), 3);
        let mut first = [0; 3];
        reader
            .read_exact(&mut first)
            .await
            .expect("first frame body");
        assert_eq!(&first, b"abc");
        assert_eq!(reader.read_u32().await.expect("second frame length"), 1);
        let mut second = [0; 1];
        reader
            .read_exact(&mut second)
            .await
            .expect("second frame body");
        assert_eq!(&second, b"z");
    }
}
