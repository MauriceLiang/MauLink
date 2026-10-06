use std::{
    collections::HashMap,
    io::{self, Cursor, Read},
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
pub const MAX_TERMINAL_BACKGROUND_BYTES: u64 = 25 * 1024 * 1024;
const MAX_TERMINAL_BACKGROUND_EDGE: u32 = 8_192;
const MAX_TERMINAL_BACKGROUND_PIXELS: u64 = 32 * 1024 * 1024;
const MAX_TERMINAL_BACKGROUND_DECODE_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LocalFilePurpose {
    PrivateKey,
    Upload,
    Download,
    TerminalBackground,
    GeoIpDatabase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalBackgroundImageInfo {
    pub media_type: String,
    pub width: u32,
    pub height: u32,
    pub byte_length: u64,
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

    pub fn consume_terminal_background_path(&self, token: &str) -> Result<PathBuf, AppError> {
        let path = self.consume_path(token, LocalFilePurpose::TerminalBackground)?;
        validate_terminal_background_path(&path)?;
        Ok(path)
    }

    pub fn consume_geoip_database_path(&self, token: &str) -> Result<PathBuf, AppError> {
        self.consume_path(token, LocalFilePurpose::GeoIpDatabase)
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

pub fn validate_terminal_background_image_bytes(
    bytes: &[u8],
) -> Result<TerminalBackgroundImageInfo, AppError> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_TERMINAL_BACKGROUND_BYTES {
        return Err(validation("file", "errors.terminalBackgroundImageTooLarge"));
    }

    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| terminal_background_invalid())?;
    let format = reader.format().ok_or_else(terminal_background_invalid)?;
    let media_type = match format {
        image::ImageFormat::Png => {
            if png_is_animated(bytes) {
                return Err(terminal_background_animated());
            }
            "image/png"
        }
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::WebP => {
            if webp_is_animated(bytes) {
                return Err(terminal_background_animated());
            }
            "image/webp"
        }
        _ => {
            return Err(validation(
                "file",
                "errors.terminalBackgroundImageFormatUnsupported",
            ));
        }
    };

    let dimensions_reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| terminal_background_invalid())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_TERMINAL_BACKGROUND_EDGE);
    limits.max_image_height = Some(MAX_TERMINAL_BACKGROUND_EDGE);
    limits.max_alloc = Some(MAX_TERMINAL_BACKGROUND_DECODE_BYTES);
    let (width, height) = dimensions_reader
        .into_dimensions()
        .map_err(|_| terminal_background_invalid())?;
    if width == 0
        || height == 0
        || width > MAX_TERMINAL_BACKGROUND_EDGE
        || height > MAX_TERMINAL_BACKGROUND_EDGE
        || u64::from(width) * u64::from(height) > MAX_TERMINAL_BACKGROUND_PIXELS
    {
        return Err(validation(
            "file",
            "errors.terminalBackgroundImageDimensionsOutOfRange",
        ));
    }

    reader.limits(limits);
    reader.decode().map_err(|_| terminal_background_invalid())?;

    Ok(TerminalBackgroundImageInfo {
        media_type: media_type.to_owned(),
        width,
        height,
        byte_length: bytes.len() as u64,
    })
}

fn validate_terminal_background_path(path: &Path) -> Result<(), AppError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| validation("token", "errors.terminalBackgroundImageUnreadable"))?;
    if !metadata.file_type().is_file() {
        return Err(validation(
            "token",
            "errors.terminalBackgroundImageUnreadable",
        ));
    }
    if metadata.len() == 0 || metadata.len() > MAX_TERMINAL_BACKGROUND_BYTES {
        return Err(validation(
            "token",
            "errors.terminalBackgroundImageTooLarge",
        ));
    }
    let file = std::fs::File::open(path)
        .map_err(|_| validation("token", "errors.terminalBackgroundImageUnreadable"))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_TERMINAL_BACKGROUND_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| validation("token", "errors.terminalBackgroundImageUnreadable"))?;
    if bytes.len() as u64 > MAX_TERMINAL_BACKGROUND_BYTES {
        return Err(validation(
            "token",
            "errors.terminalBackgroundImageTooLarge",
        ));
    }
    validate_terminal_background_image_bytes(&bytes).map(|_| ())
}

fn png_is_animated(bytes: &[u8]) -> bool {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return false;
    }
    let mut offset = 8usize;
    while offset.checked_add(12).is_some_and(|end| end <= bytes.len()) {
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let Some(end) = offset
            .checked_add(12)
            .and_then(|base| base.checked_add(length))
        else {
            return false;
        };
        if end > bytes.len() {
            return false;
        }
        if &bytes[offset + 4..offset + 8] == b"acTL" {
            return true;
        }
        offset = end;
    }
    false
}

fn webp_is_animated(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return false;
    }
    let mut offset = 12usize;
    while offset.checked_add(8).is_some_and(|end| end <= bytes.len()) {
        let kind = &bytes[offset..offset + 4];
        let length = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let Some(data_end) = offset
            .checked_add(8)
            .and_then(|start| start.checked_add(length))
        else {
            return false;
        };
        if data_end > bytes.len() {
            return false;
        }
        if kind == b"ANIM" || kind == b"ANMF" {
            return true;
        }
        if kind == b"VP8X" && length >= 1 && bytes[offset + 8] & 0x02 != 0 {
            return true;
        }
        offset = data_end + (length & 1);
    }
    false
}

fn terminal_background_invalid() -> AppError {
    validation("file", "errors.terminalBackgroundImageInvalid")
}

fn terminal_background_animated() -> AppError {
    validation("file", "errors.terminalBackgroundImageAnimatedUnsupported")
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
    use image::{DynamicImage, ImageFormat};

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
    fn terminal_background_accepts_static_png_jpeg_and_webp() {
        for format in [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::WebP] {
            let mut encoded = Cursor::new(Vec::new());
            DynamicImage::new_rgb8(3, 2)
                .write_to(&mut encoded, format)
                .expect("encode fixture");
            let info = validate_terminal_background_image_bytes(&encoded.into_inner())
                .expect("supported static image");
            assert_eq!((info.width, info.height), (3, 2));
            assert_eq!(
                info.media_type,
                match format {
                    ImageFormat::Png => "image/png",
                    ImageFormat::Jpeg => "image/jpeg",
                    ImageFormat::WebP => "image/webp",
                    _ => unreachable!(),
                }
            );
        }
    }

    #[test]
    fn terminal_background_rejects_animation_unsupported_formats_and_invalid_bytes() {
        let mut apng = b"\x89PNG\r\n\x1a\n".to_vec();
        apng.extend_from_slice(&0u32.to_be_bytes());
        apng.extend_from_slice(b"acTL");
        apng.extend_from_slice(&0u32.to_be_bytes());
        assert_eq!(
            validate_terminal_background_image_bytes(&apng)
                .expect_err("APNG must not be imported")
                .message_key,
            "errors.terminalBackgroundImageAnimatedUnsupported"
        );

        let mut animated_webp = b"RIFF".to_vec();
        animated_webp.extend_from_slice(&22u32.to_le_bytes());
        animated_webp.extend_from_slice(b"WEBPVP8X");
        animated_webp.extend_from_slice(&10u32.to_le_bytes());
        animated_webp.extend_from_slice(&[0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            validate_terminal_background_image_bytes(&animated_webp)
                .expect_err("animated WebP must not be imported")
                .message_key,
            "errors.terminalBackgroundImageAnimatedUnsupported"
        );

        assert_eq!(
            validate_terminal_background_image_bytes(b"GIF89a")
                .expect_err("GIF must not be imported")
                .message_key,
            "errors.terminalBackgroundImageFormatUnsupported"
        );
        assert_eq!(
            validate_terminal_background_image_bytes(b"not an image")
                .expect_err("invalid image bytes must fail")
                .message_key,
            "errors.terminalBackgroundImageInvalid"
        );
        assert_eq!(
            validate_terminal_background_image_bytes(&vec![
                0;
                MAX_TERMINAL_BACKGROUND_BYTES as usize
                    + 1
            ])
            .expect_err("oversized images must fail")
            .message_key,
            "errors.terminalBackgroundImageTooLarge"
        );
        let mut oversized_dimensions = Cursor::new(Vec::new());
        DynamicImage::new_rgb8(MAX_TERMINAL_BACKGROUND_EDGE + 1, 1)
            .write_to(&mut oversized_dimensions, ImageFormat::Png)
            .expect("encode oversized dimensions");
        assert_eq!(
            validate_terminal_background_image_bytes(&oversized_dimensions.into_inner())
                .expect_err("large dimensions must fail")
                .message_key,
            "errors.terminalBackgroundImageDimensionsOutOfRange"
        );
    }

    #[test]
    fn terminal_background_token_is_single_use_and_purpose_bound() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("picture.png");
        let mut encoded = Cursor::new(Vec::new());
        DynamicImage::new_rgb8(2, 2)
            .write_to(&mut encoded, ImageFormat::Png)
            .expect("encode fixture");
        std::fs::write(&path, encoded.into_inner()).expect("write fixture");
        let registry = LocalFileRegistry::default();
        let selected = registry
            .register(path, LocalFilePurpose::TerminalBackground)
            .expect("register selection");
        let error = registry
            .consume_upload_path(&selected.token)
            .expect_err("upload cannot consume image token");
        assert_eq!(error.message_key, "errors.localFilePurposeMismatch");
        assert!(
            registry
                .consume_terminal_background_path(&selected.token)
                .expect("consume image token")
                .is_file()
        );
        assert_eq!(
            registry
                .consume_terminal_background_path(&selected.token)
                .expect_err("consumed token cannot be reused")
                .code,
            ErrorCode::ResourceClosed
        );
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
