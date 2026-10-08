use crate::{AppError, ErrorCode, Language, NetworkInspection, storage};
use flate2::read::GzDecoder;
use maxminddb::Reader;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    net::IpAddr,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use ts_rs::TS;
use uuid::Uuid;

const MAX_DATABASE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DOWNLOAD_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum GeoIpDatabaseSource {
    DbIp,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GeoIpDatabaseInfo {
    pub file_name: String,
    pub database_type: String,
    #[ts(type = "number")]
    pub build_at_ms: i64,
    pub source: GeoIpDatabaseSource,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GeoIpDatabaseStatus {
    pub location: Option<GeoIpDatabaseInfo>,
    pub asn: Option<GeoIpDatabaseInfo>,
    pub automatic_updates: bool,
    pub update_interval_days: u16,
    #[ts(type = "number | null")]
    pub last_checked_at_ms: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GeoIpDatabaseConfigure {
    pub update_interval_days: u16,
}
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GeoIpDatabaseImport {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredDatabase {
    id: String,
    info: GeoIpDatabaseInfo,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    location: Option<StoredDatabase>,
    asn: Option<StoredDatabase>,
    automatic_updates: bool,
    update_interval_days: u16,
    release: Option<String>,
    last_checked_at_ms: Option<i64>,
    last_error: Option<String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            location: None,
            asn: None,
            automatic_updates: false,
            update_interval_days: 30,
            release: None,
            last_checked_at_ms: None,
            last_error: None,
        }
    }
}
struct State {
    config: Config,
    location: Option<Arc<Reader<Vec<u8>>>>,
    asn: Option<Arc<Reader<Vec<u8>>>>,
}
#[derive(Clone)]
pub struct GeoIpDatabase {
    directory: PathBuf,
    state: Arc<Mutex<State>>,
    mutation: Arc<tokio::sync::Mutex<()>>,
    generation: Arc<AtomicU64>,
}

impl GeoIpDatabase {
    pub fn new(directory: PathBuf) -> Result<Self, AppError> {
        fs::create_dir_all(&directory).map_err(|_| file_error())?;
        let path = directory.join("config.json");
        let mut config = match fs::read(path) {
            Ok(bytes) => serde_json::from_slice::<Config>(&bytes).map_err(|_| file_error())?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Config::default(),
            Err(_) => return Err(file_error()),
        };
        validate_interval(config.update_interval_days)?;
        let mut load = |stored: &mut Option<StoredDatabase>| -> Option<Arc<Reader<Vec<u8>>>> {
            let stored = stored.as_mut()?;
            match read_stored(&directory, stored) {
                Ok(reader) => {
                    stored.info.available = true;
                    Some(Arc::new(reader))
                }
                Err(error) => {
                    stored.info.available = false;
                    config.last_error = Some(error.message_key);
                    None
                }
            }
        };
        let location = load(&mut config.location);
        let asn = load(&mut config.asn);
        Ok(Self {
            directory,
            state: Arc::new(Mutex::new(State {
                config,
                location,
                asn,
            })),
            mutation: Arc::new(tokio::sync::Mutex::new(())),
            generation: Arc::new(AtomicU64::new(0)),
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }
    pub fn status(&self) -> Result<GeoIpDatabaseStatus, AppError> {
        let state = self.state.lock().map_err(|_| file_error())?;
        Ok(status(&state.config))
    }
    pub async fn configure(&self, interval: u16) -> Result<GeoIpDatabaseStatus, AppError> {
        validate_interval(interval)?;
        let _guard = self.mutation.lock().await;
        let mut config = self.config()?;
        config.update_interval_days = interval;
        self.publish_async(config).await
    }
    pub async fn import(&self, path: PathBuf) -> Result<GeoIpDatabaseStatus, AppError> {
        let _guard = self.mutation.lock().await;
        let database = self.clone();
        tokio::task::spawn_blocking(move || {
            let metadata = fs::symlink_metadata(&path).map_err(|_| invalid_database())?;
            if !metadata.file_type().is_file() || metadata.len() > MAX_DATABASE_BYTES {
                return Err(invalid_database());
            }
            let mut bytes = Vec::new();
            fs::File::open(&path)
                .map_err(|_| invalid_database())?
                .take(MAX_DATABASE_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| invalid_database())?;
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(invalid_database)?
                .to_owned();
            let stored = database.store(&bytes, name, GeoIpDatabaseSource::Local)?;
            let mut config = database.config()?;
            if is_asn(&stored.info.database_type) {
                config.asn = Some(stored);
            } else {
                config.location = Some(stored);
            }
            config.automatic_updates = false;
            config.release = None;
            config.last_error = None;
            database.publish(config)
        })
        .await
        .map_err(|_| file_error())?
    }
    pub async fn delete(&self) -> Result<GeoIpDatabaseStatus, AppError> {
        let _guard = self.mutation.lock().await;
        let mut config = self.config()?;
        config.location = None;
        config.asn = None;
        config.automatic_updates = false;
        config.release = None;
        config.last_error = None;
        config.last_checked_at_ms = None;
        self.publish_async(config).await
    }
    pub async fn update(&self, only_if_due: bool) -> Result<GeoIpDatabaseStatus, AppError> {
        let _guard = self.mutation.lock().await;
        let mut config = self.config()?;
        let now = storage::now_ms()?;
        if only_if_due
            && (!config.automatic_updates
                || config.update_interval_days == 0
                || config.last_checked_at_ms.is_some_and(|last| {
                    now.saturating_sub(last) < i64::from(config.update_interval_days) * 86_400_000
                }))
        {
            return Ok(status(&config));
        }
        config.last_checked_at_ms = Some(now);
        let result = self.download_release(&config).await;
        let result = match result {
            Ok(Some((release, city, asn))) => {
                let database = self.clone();
                let mut next = config.clone();
                tokio::task::spawn_blocking(move || {
                    // Validate both slots before publishing either database.
                    let city_reader = validated_reader(city.as_slice())?;
                    let asn_reader = validated_reader(asn.as_slice())?;
                    if is_asn(&city_reader.metadata().database_type)
                        || !is_asn(&asn_reader.metadata().database_type)
                    {
                        return Err(invalid_database());
                    }
                    next.location = Some(database.store(
                        &city,
                        format!("dbip-city-lite-{release}.mmdb"),
                        GeoIpDatabaseSource::DbIp,
                    )?);
                    next.asn = Some(database.store(
                        &asn,
                        format!("dbip-asn-lite-{release}.mmdb"),
                        GeoIpDatabaseSource::DbIp,
                    )?);
                    next.release = Some(release);
                    next.automatic_updates = true;
                    next.last_error = None;
                    database.publish(next)
                })
                .await
                .map_err(|_| file_error())?
            }
            Ok(None) => {
                config.last_error = None;
                self.publish_async(config.clone()).await
            }
            Err(error) => Err(error),
        };
        match result {
            Ok(result) => Ok(result),
            Err(error) => {
                let mut config = self.config()?;
                config.last_checked_at_ms = Some(now);
                config.last_error = Some(error.message_key.clone());
                self.publish_async(config).await?;
                Err(error)
            }
        }
    }

    pub fn enrich(
        &self,
        address: IpAddr,
        language: Language,
        inspection: &mut NetworkInspection,
    ) -> Result<(), AppError> {
        let state = self.state.lock().map_err(|_| file_error())?;
        let location = decode(state.location.as_deref(), address)?;
        let asn = decode(state.asn.as_deref(), address)?;
        if let Some(value) = location {
            inspection.geo.country_code = string_at(&value, "/country/iso_code");
            inspection.geo.country_name = localized_string_at(&value, "/country/names", language);
            inspection.geo.region = localized_string_at(&value, "/subdivisions/0/names", language);
            inspection.geo.city = localized_string_at(&value, "/city/names", language);
        }
        if let Some(value) = asn {
            inspection.asn = value
                .get("autonomous_system_number")
                .and_then(Value::as_u64)
                .map(|number| format!("AS{number}"));
            inspection.organization = value
                .get("autonomous_system_organization")
                .and_then(Value::as_str)
                .map(str::to_owned);
        }
        let available: Vec<_> = [&state.config.location, &state.config.asn]
            .into_iter()
            .flatten()
            .filter(|value| value.info.available)
            .collect();
        inspection.database_updated_at_ms =
            available.iter().map(|value| value.info.build_at_ms).min();
        inspection.database_source = if available.is_empty() {
            None
        } else if available
            .iter()
            .any(|value| value.info.source == GeoIpDatabaseSource::DbIp)
        {
            Some(GeoIpDatabaseSource::DbIp)
        } else {
            Some(GeoIpDatabaseSource::Local)
        };
        Ok(())
    }

    fn config(&self) -> Result<Config, AppError> {
        Ok(self.state.lock().map_err(|_| file_error())?.config.clone())
    }
    fn store(
        &self,
        bytes: &[u8],
        file_name: String,
        source: GeoIpDatabaseSource,
    ) -> Result<StoredDatabase, AppError> {
        let reader = validated_reader(bytes)?;
        let id = Uuid::new_v4().to_string();
        write_file(&self.directory.join(format!("{id}.mmdb")), bytes)?;
        let database_type = reader.metadata().database_type.clone();
        let source = if database_type.to_lowercase().contains("dbip") {
            GeoIpDatabaseSource::DbIp
        } else {
            source
        };
        Ok(StoredDatabase {
            id,
            info: GeoIpDatabaseInfo {
                file_name,
                database_type,
                build_at_ms: i64::try_from(reader.metadata().build_epoch)
                    .map_err(|_| invalid_database())?
                    .checked_mul(1000)
                    .ok_or_else(invalid_database)?,
                source,
                available: true,
            },
        })
    }
    async fn publish_async(&self, config: Config) -> Result<GeoIpDatabaseStatus, AppError> {
        let database = self.clone();
        tokio::task::spawn_blocking(move || database.publish(config))
            .await
            .map_err(|_| file_error())?
    }
    fn publish(&self, config: Config) -> Result<GeoIpDatabaseStatus, AppError> {
        let (location, asn) = {
            let state = self.state.lock().map_err(|_| file_error())?;
            let load = |next: &Option<StoredDatabase>,
                        previous: &Option<StoredDatabase>,
                        reader: &Option<Arc<Reader<Vec<u8>>>>|
             -> Result<_, AppError> {
                match next.as_ref().filter(|value| value.info.available) {
                    Some(next)
                        if previous.as_ref().is_some_and(|value| value.id == next.id)
                            && reader.is_some() =>
                    {
                        Ok(reader.clone())
                    }
                    Some(next) => {
                        read_stored(&self.directory, next).map(|reader| Some(Arc::new(reader)))
                    }
                    None => Ok(None),
                }
            };
            (
                load(&config.location, &state.config.location, &state.location)?,
                load(&config.asn, &state.config.asn, &state.asn)?,
            )
        };
        let temporary = self.directory.join("config.tmp");
        write_file(
            &temporary,
            &serde_json::to_vec(&config).map_err(|_| file_error())?,
        )?;
        fs::rename(&temporary, self.directory.join("config.json")).map_err(|_| file_error())?;
        let result = status(&config);
        *self.state.lock().map_err(|_| file_error())? = State {
            config: config.clone(),
            location,
            asn,
        };
        self.generation.fetch_add(1, Ordering::Release);
        for entry in fs::read_dir(&self.directory).map_err(|_| file_error())? {
            let entry = entry.map_err(|_| file_error())?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let Some(id) = name.strip_suffix(".mmdb") else {
                continue;
            };
            if Uuid::parse_str(id).is_ok()
                && ![&config.location, &config.asn]
                    .into_iter()
                    .flatten()
                    .any(|value| value.id == id)
            {
                fs::remove_file(entry.path()).map_err(|_| file_error())?;
            }
        }
        Ok(result)
    }
    async fn download_release(
        &self,
        config: &Config,
    ) -> Result<Option<(String, Vec<u8>, Vec<u8>)>, AppError> {
        let date = time::OffsetDateTime::now_utc();
        let year = date.year();
        let month = u8::from(date.month());
        let releases = [
            format!("{year}-{month:02}"),
            if month == 1 {
                format!("{}-12", year - 1)
            } else {
                format!("{year}-{:02}", month - 1)
            },
        ];
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            .https_only(true)
            .build()
            .map_err(|_| download_error())?;
        for release in releases {
            if config.release.as_deref() == Some(&release)
                && config
                    .location
                    .as_ref()
                    .is_some_and(|value| value.info.available)
                && config
                    .asn
                    .as_ref()
                    .is_some_and(|value| value.info.available)
            {
                return Ok(None);
            }
            let city = download(
                &client,
                &format!("https://download.db-ip.com/free/dbip-city-lite-{release}.mmdb.gz"),
            )
            .await?;
            let Some(city) = city else {
                continue;
            };
            let asn = download(
                &client,
                &format!("https://download.db-ip.com/free/dbip-asn-lite-{release}.mmdb.gz"),
            )
            .await?
            .ok_or_else(download_error)?;
            return Ok(Some((release, city, asn)));
        }
        Err(download_error())
    }
}

fn status(config: &Config) -> GeoIpDatabaseStatus {
    GeoIpDatabaseStatus {
        location: config.location.as_ref().map(|value| value.info.clone()),
        asn: config.asn.as_ref().map(|value| value.info.clone()),
        automatic_updates: config.automatic_updates,
        update_interval_days: config.update_interval_days,
        last_checked_at_ms: config.last_checked_at_ms,
        last_error: config.last_error.clone(),
    }
}
fn is_asn(database_type: &str) -> bool {
    database_type.to_lowercase().contains("asn")
}
fn validated_reader<T: AsRef<[u8]>>(bytes: T) -> Result<Reader<T>, AppError> {
    if bytes.as_ref().len() as u64 > MAX_DATABASE_BYTES {
        return Err(invalid_database());
    }
    let reader = Reader::from_source(bytes).map_err(|_| invalid_database())?;
    let kind = reader.metadata().database_type.to_lowercase();
    if !["city", "country", "asn"]
        .iter()
        .any(|value| kind.contains(value))
    {
        return Err(invalid_database());
    }
    Ok(reader)
}
fn read_stored(directory: &Path, stored: &StoredDatabase) -> Result<Reader<Vec<u8>>, AppError> {
    if Uuid::parse_str(&stored.id).map_or(true, |id| id.to_string() != stored.id) {
        return Err(invalid_database());
    }
    let path = directory.join(format!("{}.mmdb", stored.id));
    let metadata = fs::symlink_metadata(&path).map_err(|_| file_error())?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_DATABASE_BYTES {
        return Err(invalid_database());
    }
    validated_reader(fs::read(path).map_err(|_| file_error())?)
}
fn decode(reader: Option<&Reader<Vec<u8>>>, address: IpAddr) -> Result<Option<Value>, AppError> {
    reader
        .map(|reader| {
            reader
                .lookup(address)
                .map_err(|_| invalid_database())?
                .decode::<Value>()
                .map_err(|_| invalid_database())
        })
        .transpose()
        .map(Option::flatten)
}
fn string_at(value: &Value, path: &str) -> Option<String> {
    value
        .pointer(path)
        .and_then(Value::as_str)
        .map(str::to_owned)
}
fn localized_string_at(value: &Value, path: &str, language: Language) -> Option<String> {
    let preferred_language = match language {
        Language::ZhCn => "zh-CN",
        Language::En => "en",
    };
    string_at(value, &format!("{path}/{preferred_language}"))
        .or_else(|| string_at(value, &format!("{path}/en")))
}
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let mut file = fs::File::create(path).map_err(|_| file_error())?;
    file.write_all(bytes).map_err(|_| file_error())?;
    file.sync_all().map_err(|_| file_error())
}
fn validate_interval(days: u16) -> Result<(), AppError> {
    if [0, 1, 7, 30].contains(&days) {
        Ok(())
    } else {
        Err(AppError::new(
            ErrorCode::ValidationFailed,
            "errors.geoIpUpdateIntervalInvalid",
        ))
    }
}
fn invalid_database() -> AppError {
    AppError::new(ErrorCode::ValidationFailed, "errors.geoIpDatabaseInvalid")
}
fn file_error() -> AppError {
    AppError::new(
        ErrorCode::LocalFileOperationFailed,
        "errors.geoIpDatabaseFileFailed",
    )
}
fn download_error() -> AppError {
    AppError::new(ErrorCode::Internal, "errors.geoIpDatabaseDownloadFailed").with_retry()
}
async fn download(client: &reqwest::Client, url: &str) -> Result<Option<Vec<u8>>, AppError> {
    let mut response = client.get(url).send().await.map_err(|_| download_error())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(download_error());
    }
    let mut compressed = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| download_error())? {
        if compressed.len().saturating_add(chunk.len()) > MAX_DOWNLOAD_BYTES {
            return Err(invalid_database());
        }
        compressed.extend_from_slice(&chunk);
    }
    tokio::task::spawn_blocking(move || {
        let mut bytes = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(MAX_DATABASE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid_database())?;
        validated_reader(bytes.as_slice())?;
        Ok(Some(bytes))
    })
    .await
    .map_err(|_| download_error())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_the_requested_place_name_and_falls_back_to_english() {
        let place = serde_json::json!({
            "names": { "en": "Hong Kong", "zh-CN": "中国香港" }
        });
        assert_eq!(
            localized_string_at(&place, "/names", Language::ZhCn).as_deref(),
            Some("中国香港")
        );
        assert_eq!(
            localized_string_at(&place, "/names", Language::En).as_deref(),
            Some("Hong Kong")
        );

        let english_only = serde_json::json!({ "names": { "en": "Kowloon" } });
        assert_eq!(
            localized_string_at(&english_only, "/names", Language::ZhCn).as_deref(),
            Some("Kowloon")
        );
    }

    #[tokio::test]
    async fn persists_frequency_and_rejects_invalid_changes() {
        let directory = tempfile::tempdir().unwrap();
        let database = GeoIpDatabase::new(directory.path().to_owned()).unwrap();
        for interval in [0, 1, 7, 30] {
            database.configure(interval).await.unwrap();
            assert_eq!(
                GeoIpDatabase::new(directory.path().to_owned())
                    .unwrap()
                    .status()
                    .unwrap()
                    .update_interval_days,
                interval
            );
        }
        assert_eq!(
            database.configure(2).await.unwrap_err().message_key,
            "errors.geoIpUpdateIntervalInvalid"
        );
        assert_eq!(database.status().unwrap().update_interval_days, 30);
    }

    #[tokio::test]
    async fn does_not_download_without_opt_in_and_rejects_invalid_imports() {
        let directory = tempfile::tempdir().unwrap();
        let database = GeoIpDatabase::new(directory.path().join("databases")).unwrap();
        let state = database.update(true).await.unwrap();
        assert!(state.last_checked_at_ms.is_none());
        let source = directory.path().join("invalid.mmdb");
        fs::write(&source, b"invalid database").unwrap();
        assert_eq!(
            database
                .import(source.clone())
                .await
                .unwrap_err()
                .message_key,
            "errors.geoIpDatabaseInvalid"
        );
        assert!(database.import(directory.path().to_owned()).await.is_err());
        assert!(
            database
                .import(directory.path().join("missing.mmdb"))
                .await
                .is_err()
        );
        assert_eq!(fs::read(source).unwrap(), b"invalid database");
        assert!(database.status().unwrap().location.is_none());
        assert_eq!(database.generation(), 0);
    }

    #[tokio::test]
    async fn deletion_preserves_frequency_and_clears_update_state() {
        let directory = tempfile::tempdir().unwrap();
        let database = GeoIpDatabase::new(directory.path().to_owned()).unwrap();
        let mut config = database.config().unwrap();
        config.update_interval_days = 7;
        config.automatic_updates = true;
        config.last_error = Some("errors.geoIpDatabaseDownloadFailed".into());
        config.last_checked_at_ms = Some(1);
        database.publish(config).unwrap();
        let state = database.delete().await.unwrap();
        assert!(!state.automatic_updates);
        assert!(state.last_error.is_none() && state.last_checked_at_ms.is_none());
        let reloaded = GeoIpDatabase::new(directory.path().to_owned())
            .unwrap()
            .status()
            .unwrap();
        assert_eq!(reloaded.update_interval_days, 7);
        assert!(!reloaded.automatic_updates);
    }

    #[test]
    fn rejects_invalid_config_instead_of_resetting_it() {
        let directory = tempfile::tempdir().unwrap();
        let bytes = b"invalid config";
        fs::write(directory.path().join("config.json"), bytes).unwrap();
        assert!(GeoIpDatabase::new(directory.path().to_owned()).is_err());
        assert_eq!(
            fs::read(directory.path().join("config.json")).unwrap(),
            bytes
        );
    }
}
