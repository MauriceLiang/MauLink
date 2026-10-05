use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use maulink_core::{
    AppError, BackgroundImageAsset, BackgroundImageGetResult, ErrorCode, LocalFileRegistry,
    validate_terminal_background_image_bytes,
};
use uuid::Uuid;

const MAX_BACKGROUND_FILE_BYTES: u64 = 25 * 1024 * 1024;

#[derive(Clone)]
pub struct BackgroundImageStore {
    directory: PathBuf,
}

impl BackgroundImageStore {
    pub fn new(directory: PathBuf) -> Result<Self, AppError> {
        fs::create_dir_all(&directory).map_err(|_| local_file_error())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
                .map_err(|_| local_file_error())?;
        }
        Ok(Self { directory })
    }

    pub fn import(
        &self,
        registry: &LocalFileRegistry,
        token: &str,
    ) -> Result<BackgroundImageAsset, AppError> {
        let source = registry.consume_terminal_background_path(token)?;
        let metadata = fs::symlink_metadata(&source).map_err(|_| image_unreadable())?;
        if !metadata.file_type().is_file() {
            return Err(image_unreadable());
        }
        if metadata.len() == 0 || metadata.len() > MAX_BACKGROUND_FILE_BYTES {
            return Err(image_too_large());
        }
        let file = File::open(&source).map_err(|_| image_unreadable())?;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.take(MAX_BACKGROUND_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| image_unreadable())?;
        if bytes.len() as u64 > MAX_BACKGROUND_FILE_BYTES {
            return Err(image_too_large());
        }
        let info = validate_terminal_background_image_bytes(&bytes)?;
        let id = Uuid::new_v4().to_string();
        let extension = extension_for_media_type(&info.media_type)?;
        let asset = BackgroundImageAsset {
            id: id.clone(),
            file_name: safe_file_name(&source)?,
            media_type: info.media_type,
            width: info.width,
            height: info.height,
            byte_length: info.byte_length,
            created_at_ms: now_ms()?,
        };
        self.write_asset(&asset, extension, &bytes)?;
        Ok(asset)
    }

    pub fn get(&self, image_id: &str) -> Result<BackgroundImageGetResult, AppError> {
        let id = validate_image_id(image_id)?;
        let metadata_path = self.metadata_path(&id);
        let metadata_stat = fs::symlink_metadata(&metadata_path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                image_not_found()
            } else {
                local_file_error()
            }
        })?;
        if !metadata_stat.file_type().is_file() || metadata_stat.len() > 16 * 1024 {
            return Err(local_file_error());
        }
        let metadata = fs::read(&metadata_path).map_err(|_| local_file_error())?;
        let asset: BackgroundImageAsset =
            serde_json::from_slice(&metadata).map_err(|_| local_file_error())?;
        if asset.id != id || asset.byte_length == 0 || asset.byte_length > MAX_BACKGROUND_FILE_BYTES
        {
            return Err(local_file_error());
        }
        let image_path = self.image_path(&asset)?;
        let image_metadata = fs::symlink_metadata(&image_path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                image_not_found()
            } else {
                local_file_error()
            }
        })?;
        if !image_metadata.file_type().is_file() || image_metadata.len() != asset.byte_length {
            return Err(local_file_error());
        }
        Ok(BackgroundImageGetResult {
            asset,
            local_path: image_path.to_string_lossy().into_owned(),
        })
    }

    pub fn delete(&self, image_id: &str) -> Result<(), AppError> {
        let result = self.get(image_id)?;
        let path = PathBuf::from(result.local_path);
        fs::remove_file(path).map_err(|_| local_file_error())?;
        fs::remove_file(self.metadata_path(&result.asset.id)).map_err(|_| local_file_error())
    }

    fn write_asset(
        &self,
        asset: &BackgroundImageAsset,
        extension: &str,
        bytes: &[u8],
    ) -> Result<(), AppError> {
        let image_path = self.directory.join(format!("{}.{}", asset.id, extension));
        let image_temp = self.directory.join(format!(".{}.image.tmp", asset.id));
        let metadata_path = self.metadata_path(&asset.id);
        let metadata_temp = self.directory.join(format!(".{}.json.tmp", asset.id));

        write_new_file(&image_temp, bytes)?;
        if fs::rename(&image_temp, &image_path).is_err() {
            let _ = fs::remove_file(&image_temp);
            return Err(local_file_error());
        }
        let metadata = match serde_json::to_vec(asset) {
            Ok(metadata) => metadata,
            Err(_) => {
                let _ = fs::remove_file(&image_path);
                return Err(local_file_error());
            }
        };
        if let Err(error) = write_new_file(&metadata_temp, &metadata).and_then(|()| {
            fs::rename(&metadata_temp, &metadata_path).map_err(|_| local_file_error())
        }) {
            let _ = fs::remove_file(&metadata_temp);
            let _ = fs::remove_file(&image_path);
            return Err(error);
        }
        Ok(())
    }

    fn metadata_path(&self, id: &str) -> PathBuf {
        self.directory.join(format!("{id}.json"))
    }

    fn image_path(&self, asset: &BackgroundImageAsset) -> Result<PathBuf, AppError> {
        let extension = extension_for_media_type(&asset.media_type)?;
        Ok(self.directory.join(format!("{}.{}", asset.id, extension)))
    }
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|_| local_file_error())?;
    file.write_all(bytes).map_err(|_| local_file_error())?;
    file.sync_all().map_err(|_| local_file_error())
}

fn safe_file_name(path: &Path) -> Result<String, AppError> {
    path.file_name()
        .map(|name| {
            name.to_string_lossy()
                .chars()
                .filter(|character| !character.is_control())
                .take(255)
                .collect::<String>()
        })
        .filter(|name| !name.is_empty())
        .ok_or_else(image_unreadable)
}

fn extension_for_media_type(media_type: &str) -> Result<&'static str, AppError> {
    match media_type {
        "image/png" => Ok("png"),
        "image/jpeg" => Ok("jpg"),
        "image/webp" => Ok("webp"),
        _ => Err(local_file_error()),
    }
}

fn validate_image_id(image_id: &str) -> Result<String, AppError> {
    let parsed = Uuid::parse_str(image_id).map_err(|_| {
        AppError::new(
            ErrorCode::ValidationFailed,
            "errors.terminalBackgroundImageIdInvalid",
        )
    })?;
    let id = parsed.to_string();
    if id != image_id {
        return Err(AppError::new(
            ErrorCode::ValidationFailed,
            "errors.terminalBackgroundImageIdInvalid",
        ));
    }
    Ok(id)
}

fn now_ms() -> Result<i64, AppError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| local_file_error())?
        .as_millis();
    i64::try_from(millis).map_err(|_| local_file_error())
}

fn image_not_found() -> AppError {
    AppError::new(
        ErrorCode::ResourceNotFound,
        "errors.terminalBackgroundImageNotFound",
    )
}

fn image_unreadable() -> AppError {
    AppError::new(
        ErrorCode::ValidationFailed,
        "errors.terminalBackgroundImageUnreadable",
    )
}

fn image_too_large() -> AppError {
    AppError::new(
        ErrorCode::ValidationFailed,
        "errors.terminalBackgroundImageTooLarge",
    )
}

fn local_file_error() -> AppError {
    AppError::new(
        ErrorCode::LocalFileOperationFailed,
        "errors.localFileOperationFailed",
    )
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{DynamicImage, ImageFormat};
    use maulink_core::{LocalFilePurpose, LocalFileRegistry};

    use super::*;

    #[test]
    fn imports_gets_and_deletes_a_private_app_data_copy() {
        let root = tempfile::tempdir().expect("temporary directory");
        let source_directory = root.path().join("pictures");
        fs::create_dir(&source_directory).expect("source directory");
        let source = source_directory.join("wallpaper.png");
        let mut encoded = Cursor::new(Vec::new());
        DynamicImage::new_rgb8(4, 3)
            .write_to(&mut encoded, ImageFormat::Png)
            .expect("encode image");
        fs::write(&source, encoded.into_inner()).expect("write image");

        let registry = LocalFileRegistry::default();
        let selected = registry
            .register(source.clone(), LocalFilePurpose::TerminalBackground)
            .expect("register image");
        let store = BackgroundImageStore::new(root.path().join("app-data/terminal-backgrounds"))
            .expect("create store");
        let asset = store
            .import(&registry, &selected.token)
            .expect("import image");
        assert_eq!(asset.file_name, "wallpaper.png");
        assert_eq!(asset.media_type, "image/png");
        assert_eq!((asset.width, asset.height), (4, 3));
        assert!(!asset.id.contains(source.to_string_lossy().as_ref()));
        let stored_path = store.get(&asset.id).expect("get imported image");
        assert_eq!(stored_path.asset, asset);
        assert!(Path::new(&stored_path.local_path).is_file());
        let metadata_path = store.metadata_path(&asset.id);
        let metadata = fs::read_to_string(metadata_path).expect("read asset metadata");
        assert!(!metadata.contains(source_directory.to_string_lossy().as_ref()));
        assert!(!stored_path.local_path.contains("wallpaper.png"));

        store.delete(&asset.id).expect("delete unused image");
        assert_eq!(
            store
                .get(&asset.id)
                .expect_err("deleted image is gone")
                .code,
            ErrorCode::ResourceNotFound
        );
        assert_eq!(
            registry
                .consume_terminal_background_path(&selected.token)
                .expect_err("import consumes the selection token")
                .code,
            ErrorCode::ResourceClosed
        );
    }
}
