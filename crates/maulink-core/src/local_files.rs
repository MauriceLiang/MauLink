use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{AppError, ErrorCode, PathEncoding, StoredPath};

const DEFAULT_TOKEN_TTL: Duration = Duration::from_secs(10 * 60);
const DEFAULT_TOKEN_CAPACITY: usize = 64;
const MAX_PRIVATE_KEY_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LocalFilePurpose {
    PrivateKey,
    Upload,
    Download,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SelectedLocalFile {
    pub token: String,
    pub display_name: String,
    pub purpose: LocalFilePurpose,
    #[ts(type = "number")]
    pub expires_at_ms: i64,
}

struct FileEntry {
    path: PathBuf,
    purpose: LocalFilePurpose,
    expires_at: Instant,
}

/// Keeps native file selections out of the IPC payload by exchanging them for
/// short-lived, purpose-bound opaque tokens.
#[derive(Clone)]
pub struct LocalFileRegistry {
    entries: Arc<Mutex<HashMap<String, FileEntry>>>,
    ttl: Duration,
    capacity: usize,
}

impl Default for LocalFileRegistry {
    fn default() -> Self {
        Self::new(DEFAULT_TOKEN_TTL, DEFAULT_TOKEN_CAPACITY)
    }
}

impl LocalFileRegistry {
    pub fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            ttl,
            capacity,
        }
    }

    pub fn register(
        &self,
        path: PathBuf,
        purpose: LocalFilePurpose,
    ) -> Result<SelectedLocalFile, AppError> {
        if self.capacity == 0 {
            return Err(resource_limit());
        }
        let display_name = display_name(&path)?;
        if purpose == LocalFilePurpose::PrivateKey {
            validate_private_key_file(&path)?;
        }

        let mut entries = self
            .entries
            .lock()
            .map_err(|_| internal("errors.localFileRegistryUnavailable"))?;
        let now = Instant::now();
        entries.retain(|_, entry| entry.expires_at > now);
        if entries.len() >= self.capacity {
            return Err(resource_limit());
        }

        let token = Uuid::new_v4().to_string();
        let expires_at_ms = current_time_ms()?
            .checked_add(i64::try_from(self.ttl.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or(i64::MAX);
        entries.insert(
            token.clone(),
            FileEntry {
                path,
                purpose,
                expires_at: now + self.ttl,
            },
        );
        Ok(SelectedLocalFile {
            token,
            display_name,
            purpose,
            expires_at_ms,
        })
    }

    pub fn resolve_private_key(&self, token: &str) -> Result<StoredPath, AppError> {
        validate_token(token)?;
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| internal("errors.localFileRegistryUnavailable"))?;
        let now = Instant::now();
        entries.retain(|_, entry| entry.expires_at > now);
        let entry = entries.get(token).ok_or_else(token_expired)?;
        if entry.purpose != LocalFilePurpose::PrivateKey {
            return Err(validation("token", "errors.localFilePurposeMismatch"));
        }
        validate_private_key_file(&entry.path)?;
        encode_path(&entry.path)
    }

    pub fn consume_upload_path(&self, token: &str) -> Result<PathBuf, AppError> {
        let path = self.consume_path(token, LocalFilePurpose::Upload)?;
        let metadata = std::fs::metadata(&path)
            .map_err(|_| validation("token", "errors.localUploadFileUnreadable"))?;
        if !metadata.is_file() {
            return Err(validation("token", "errors.localUploadFileInvalid"));
        }
        Ok(path)
    }

    pub fn consume_download_path(&self, token: &str) -> Result<PathBuf, AppError> {
        let path = self.consume_path(token, LocalFilePurpose::Download)?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| validation("token", "errors.localDownloadPathInvalid"))?;
        if path.file_name().is_none() || !parent.is_dir() {
            return Err(validation("token", "errors.localDownloadPathInvalid"));
        }
        match std::fs::symlink_metadata(&path) {
            Ok(_) => Err(AppError::new(
                ErrorCode::TargetExists,
                "errors.targetExists",
            )),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(path),
            Err(_) => Err(AppError::new(
                ErrorCode::LocalFileOperationFailed,
                "errors.localFileOperationFailed",
            )),
        }
    }

    fn consume_path(&self, token: &str, purpose: LocalFilePurpose) -> Result<PathBuf, AppError> {
        validate_token(token)?;
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| internal("errors.localFileRegistryUnavailable"))?;
        let now = Instant::now();
        entries.retain(|_, entry| entry.expires_at > now);
        let entry = entries.get(token).ok_or_else(token_expired)?;
        if entry.purpose != purpose {
            return Err(validation("token", "errors.localFilePurposeMismatch"));
        }
        entries
            .remove(token)
            .map(|entry| entry.path)
            .ok_or_else(token_expired)
    }

    pub fn forget(&self, token: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(token);
        }
    }
}

fn current_time_ms() -> Result<i64, AppError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| internal("errors.systemClockInvalid"))?
        .as_millis();
    i64::try_from(millis).map_err(|_| internal("errors.systemClockInvalid"))
}

fn display_name(path: &Path) -> Result<String, AppError> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| validation("path", "errors.localFileNameInvalid"))
}

fn validate_private_key_file(path: &Path) -> Result<(), AppError> {
    let metadata = path
        .metadata()
        .map_err(|_| validation("path", "errors.privateKeyUnreadable"))?;
    if !metadata.is_file() || metadata.len() > MAX_PRIVATE_KEY_BYTES {
        return Err(validation("path", "errors.privateKeyUnreadable"));
    }
    Ok(())
}

#[cfg(unix)]
fn encode_path(path: &Path) -> Result<StoredPath, AppError> {
    use std::os::unix::ffi::OsStrExt;
    Ok(StoredPath {
        bytes: path.as_os_str().as_bytes().to_vec(),
        encoding: PathEncoding::UnixBytes,
    })
}

#[cfg(windows)]
fn encode_path(path: &Path) -> Result<StoredPath, AppError> {
    use std::os::windows::ffi::OsStrExt;
    let bytes = path
        .as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect();
    Ok(StoredPath {
        bytes,
        encoding: PathEncoding::WindowsUtf16le,
    })
}

fn validate_token(token: &str) -> Result<(), AppError> {
    Uuid::parse_str(token)
        .map(|_| ())
        .map_err(|_| validation("token", "errors.localFileTokenInvalid"))
}

fn token_expired() -> AppError {
    AppError::new(ErrorCode::ResourceClosed, "errors.localFileTokenExpired")
}

fn resource_limit() -> AppError {
    AppError::new(ErrorCode::ResourceLimit, "errors.localFileTokenLimit")
}

fn validation(field: &'static str, message_key: &'static str) -> AppError {
    AppError::new(ErrorCode::ValidationFailed, message_key).with_param("field", field)
}

fn internal(message_key: &'static str) -> AppError {
    AppError::new(ErrorCode::Internal, message_key).with_stage("localFile")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_purpose_bound_and_expires() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("id_ed25519");
        std::fs::write(&path, "test-only-private-key").expect("write fixture");
        let registry = LocalFileRegistry::new(Duration::from_millis(1), 2);
        let selected = registry
            .register(path, LocalFilePurpose::PrivateKey)
            .expect("register selection");
        let stored = registry
            .resolve_private_key(&selected.token)
            .expect("resolve selection");
        assert!(!stored.bytes.is_empty());
        assert!(!selected.token.contains("id_ed25519"));

        std::thread::sleep(Duration::from_millis(3));
        let error = registry
            .resolve_private_key(&selected.token)
            .expect_err("expired token must fail");
        assert_eq!(error.code, ErrorCode::ResourceClosed);
    }

    #[test]
    fn registry_enforces_capacity_without_exposing_paths() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let first = directory.path().join("first-key");
        let second = directory.path().join("second-key");
        std::fs::write(&first, "first").expect("write first fixture");
        std::fs::write(&second, "second").expect("write second fixture");
        let registry = LocalFileRegistry::new(Duration::from_secs(60), 1);
        let selected = registry
            .register(first, LocalFilePurpose::PrivateKey)
            .expect("register first selection");
        let json = serde_json::to_string(&selected).expect("serialize selection");
        assert!(!json.contains(directory.path().to_string_lossy().as_ref()));

        let error = registry
            .register(second, LocalFilePurpose::PrivateKey)
            .expect_err("capacity must be bounded");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn transfer_tokens_are_purpose_bound_single_use_and_no_clobber() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let upload = directory.path().join("upload.txt");
        std::fs::write(&upload, "upload fixture").expect("write upload fixture");
        let download = directory.path().join("download.txt");
        let registry = LocalFileRegistry::default();

        let upload_token = registry
            .register(upload.clone(), LocalFilePurpose::Upload)
            .expect("register upload source");
        let wrong_purpose = registry
            .consume_download_path(&upload_token.token)
            .expect_err("upload token cannot authorize a download");
        assert_eq!(wrong_purpose.code, ErrorCode::ValidationFailed);
        assert_eq!(
            registry
                .consume_upload_path(&upload_token.token)
                .expect("consume upload token once"),
            upload
        );
        assert_eq!(
            registry
                .consume_upload_path(&upload_token.token)
                .expect_err("upload token cannot be replayed")
                .code,
            ErrorCode::ResourceClosed
        );

        let download_token = registry
            .register(download.clone(), LocalFilePurpose::Download)
            .expect("register download target");
        assert_eq!(
            registry
                .consume_download_path(&download_token.token)
                .expect("consume download token once"),
            download
        );
        assert_eq!(
            registry
                .consume_download_path(&download_token.token)
                .expect_err("download token cannot be replayed")
                .code,
            ErrorCode::ResourceClosed
        );

        let existing_target = registry
            .register(upload, LocalFilePurpose::Download)
            .expect("register existing destination");
        assert_eq!(
            registry
                .consume_download_path(&existing_target.token)
                .expect_err("existing target must never be overwritten")
                .code,
            ErrorCode::TargetExists
        );
    }
}
