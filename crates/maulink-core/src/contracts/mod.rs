use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use ts_rs::TS;
use uuid::Uuid;

pub const API_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ApiRequest<T> {
    pub api_version: u16,
    pub request_id: String,
    pub payload: T,
}

impl<T> ApiRequest<T> {
    pub fn validate(self) -> Result<(String, T), crate::AppError> {
        if self.api_version != API_VERSION {
            return Err(crate::AppError::new(
                crate::ErrorCode::IpcVersionUnsupported,
                "errors.ipcVersionUnsupported",
            )
            .with_param("requestedVersion", self.api_version.to_string())
            .with_param("supportedVersion", API_VERSION.to_string())
            .with_request_id(self.request_id));
        }
        if Uuid::parse_str(&self.request_id).is_err() {
            return Err(crate::AppError::new(
                crate::ErrorCode::ValidationFailed,
                "errors.requestIdInvalid",
            )
            .with_param("field", "requestId"));
        }
        Ok((self.request_id, self.payload))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct EmptyPayload {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ResourceIdPayload {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RevisionPayload {
    pub id: String,
    pub expected_revision: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerDeletePayload {
    pub server_id: String,
    pub expected_revision: u32,
    pub remove_credentials: bool,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStartPayload {
    pub source: ConnectionStartSource,
    pub mode: crate::ConnectionMode,
}

#[derive(Debug, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ConnectionStartSource {
    Saved {
        server_id: String,
        expected_revision: u32,
    },
    Draft {
        profile: ServerProfileDraft,
        credential: Option<crate::Secret>,
    },
    DraftWithSavedProfile {
        profile: ServerProfileDraft,
        server_id: String,
        expected_revision: u32,
        credential: Option<crate::Secret>,
        use_saved_credential: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionIdPayload {
    pub connection_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDisconnectPayload {
    pub connection_id: String,
    #[serde(default)]
    pub stop_active_transfers: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOpenPayload {
    pub connection_id: String,
    pub columns: u32,
    pub rows: u32,
    pub pixel_width: Option<u32>,
    pub pixel_height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalIdPayload {
    pub terminal_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalWritePayload {
    pub terminal_id: String,
    pub input_seq: DecimalU64,
    pub data_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalAckPayload {
    pub terminal_id: String,
    pub stream_id: String,
    pub seq: DecimalU64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalResizePayload {
    pub terminal_id: String,
    pub columns: u32,
    pub rows: u32,
    pub pixel_width: Option<u32>,
    pub pixel_height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpListStartPayload {
    pub connection_id: String,
    /// Remote POSIX path. Empty paths are treated as `.`.
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpCursorPayload {
    pub cursor_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpStatPayload {
    pub connection_id: String,
    pub path: String,
    pub follow_symlink: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpReadTextPayload {
    pub connection_id: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpReadTextResult {
    pub content: String,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpWriteTextPayload {
    pub connection_id: String,
    pub path: String,
    pub content: String,
    pub expected_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpWriteTextResult {
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpMkdirPayload {
    pub connection_id: String,
    pub parent_path: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpRenamePayload {
    pub connection_id: String,
    pub source_path: String,
    pub new_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpDeletePayload {
    pub connection_id: String,
    pub path: String,
    pub expected_type: RemoteFileType,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpUploadPayload {
    pub connection_id: String,
    pub local_file_token: String,
    pub remote_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpDownloadPayload {
    pub connection_id: String,
    pub remote_path: String,
    pub local_file_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpTransferIdPayload {
    pub transfer_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpTransferListPayload {
    pub connection_id: Option<String>,
    pub limit: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SftpTransferDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SftpTransferState {
    Created,
    Transferring,
    Finalizing,
    Completed,
    Cancelled,
    Failed,
}

impl SftpTransferState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpTransferSnapshot {
    pub transfer_id: String,
    pub connection_id: String,
    pub direction: SftpTransferDirection,
    pub file_name: String,
    pub final_path: String,
    pub total_bytes: Option<DecimalU64>,
    pub transferred_bytes: DecimalU64,
    pub bytes_per_second: Option<f64>,
    pub remaining_seconds: Option<u64>,
    pub state: SftpTransferState,
    pub error: Option<crate::AppError>,
    pub cleanup_required: bool,
    pub temporary_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum MonitorQualityStatus {
    Ok,
    WarmingUp,
    Stale,
    Unsupported,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorMetricQuality {
    pub status: MonitorQualityStatus,
    #[ts(type = "number | null")]
    pub sampled_at_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub collection_duration_ms: Option<u64>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorCpuSnapshot {
    pub usage_percent: Option<f64>,
    pub logical_cores: Option<u32>,
    pub quality: MonitorMetricQuality,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorLoadSnapshot {
    pub one_minute: Option<f64>,
    pub five_minutes: Option<f64>,
    pub fifteen_minutes: Option<f64>,
    pub quality: MonitorMetricQuality,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorMemorySnapshot {
    pub used_bytes: Option<DecimalU64>,
    pub total_bytes: Option<DecimalU64>,
    pub available_bytes: Option<DecimalU64>,
    pub used_percent: Option<f64>,
    pub quality: MonitorMetricQuality,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorDiskSnapshot {
    pub used_bytes: Option<DecimalU64>,
    pub total_bytes: Option<DecimalU64>,
    pub available_bytes: Option<DecimalU64>,
    pub used_percent: Option<f64>,
    pub source: Option<String>,
    pub mount: Option<String>,
    pub quality: MonitorMetricQuality,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorNetworkInterface {
    pub name: String,
    pub received_bytes: DecimalU64,
    pub transmitted_bytes: DecimalU64,
    pub received_bytes_per_second: Option<f64>,
    pub transmitted_bytes_per_second: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorNetworkSnapshot {
    pub received_bytes_per_second: Option<f64>,
    pub transmitted_bytes_per_second: Option<f64>,
    pub interfaces: Vec<MonitorNetworkInterface>,
    pub quality: MonitorMetricQuality,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorUptimeSnapshot {
    pub seconds: Option<DecimalU64>,
    pub quality: MonitorMetricQuality,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSystemSnapshot {
    pub hostname: Option<String>,
    pub os: Option<String>,
    pub kernel: Option<String>,
    pub architecture: Option<String>,
    pub quality: MonitorMetricQuality,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSnapshot {
    pub connection_id: String,
    pub cpu: MonitorCpuSnapshot,
    pub load: MonitorLoadSnapshot,
    pub memory: MonitorMemorySnapshot,
    pub disk: MonitorDiskSnapshot,
    pub network: MonitorNetworkSnapshot,
    pub uptime: MonitorUptimeSnapshot,
    pub system: MonitorSystemSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum MonitorHistoryMetric {
    CpuUsage,
    MemoryUsage,
    DiskUsage,
    NetworkReceiveRate,
    NetworkTransmitRate,
    LoadOneMinute,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorHistorySample {
    #[ts(type = "number")]
    pub sampled_at_ms: i64,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorHistoryPage {
    pub connection_id: String,
    pub metric: MonitorHistoryMetric,
    pub samples: Vec<MonitorHistorySample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorGetSnapshotPayload {
    pub connection_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorGetHistoryPayload {
    pub connection_id: String,
    pub metric: MonitorHistoryMetric,
    #[ts(type = "number | null")]
    pub from_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub to_ms: Option<i64>,
    pub limit: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorRefreshPayload {
    pub connection_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceActivityPayload {
    pub active_connection_id: Option<String>,
    pub monitor_visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum RemoteFileType {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFileEntry {
    pub name: String,
    pub path: String,
    pub file_type: RemoteFileType,
    pub size_bytes: Option<DecimalU64>,
    #[ts(type = "number | null")]
    pub modified_at_ms: Option<i64>,
    pub is_symlink: bool,
    pub permissions: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SftpDirectoryPage {
    pub path: String,
    pub entries: Vec<RemoteFileEntry>,
    pub cursor_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HostKeyRespondPayload {
    pub connection_id: String,
    pub challenge_id: String,
    pub decision: crate::HostKeyDecision,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationRespondPayload {
    pub connection_id: String,
    pub challenge_id: String,
    pub secret: crate::Secret,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfileDraft {
    pub name: Option<String>,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: crate::AuthType,
    pub private_key_token: Option<String>,
    pub group_id: Option<String>,
    pub connect_timeout_ms: u32,
    pub keepalive_interval_seconds: u32,
    #[serde(default)]
    pub jump_host: Option<String>,
    #[serde(default = "default_jump_port")]
    pub jump_port: u16,
    #[serde(default)]
    pub proxy_type: Option<crate::ProxyType>,
    #[serde(default)]
    pub proxy_host: Option<String>,
    #[serde(default)]
    pub proxy_port: Option<u16>,
}

const fn default_jump_port() -> u16 {
    22
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerCreatePayload {
    pub profile: ServerProfileDraft,
    pub credential: crate::CredentialUpdate,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerUpdatePayload {
    pub server_id: String,
    pub expected_revision: u32,
    pub profile: ServerProfileDraft,
    pub credential: crate::CredentialUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GroupUpdatePayload {
    pub group_id: String,
    pub update: crate::GroupUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LocalFileSelectPayload {
    pub purpose: crate::LocalFilePurpose,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub api_version: u16,
    pub platform: String,
    pub architecture: String,
    pub capabilities: AppCapabilities,
}

impl AppInfo {
    pub fn current() -> Self {
        Self::with_capabilities(AppCapabilities::foundation())
    }

    pub fn with_capabilities(capabilities: AppCapabilities) -> Self {
        Self {
            name: "MauLink".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            api_version: API_VERSION,
            platform: std::env::consts::OS.to_owned(),
            architecture: std::env::consts::ARCH.to_owned(),
            capabilities,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppCapabilities {
    pub profile_storage: bool,
    pub secure_credentials: bool,
    pub ssh: bool,
    pub terminal: bool,
    pub sftp: bool,
    pub monitor: bool,
}

impl AppCapabilities {
    const fn foundation() -> Self {
        Self {
            profile_storage: false,
            secure_credentials: false,
            ssh: false,
            terminal: false,
            sftp: false,
            monitor: false,
        }
    }

    pub const fn data_backend(secure_credentials: bool) -> Self {
        Self {
            profile_storage: true,
            secure_credentials,
            ssh: false,
            terminal: false,
            sftp: false,
            monitor: false,
        }
    }

    pub const fn ssh_backend(secure_credentials: bool) -> Self {
        Self {
            profile_storage: true,
            secure_credentials,
            ssh: true,
            terminal: false,
            sftp: false,
            monitor: false,
        }
    }

    pub const fn terminal_backend(secure_credentials: bool) -> Self {
        Self {
            profile_storage: true,
            secure_credentials,
            ssh: true,
            terminal: true,
            sftp: false,
            monitor: false,
        }
    }

    pub const fn sftp_backend(secure_credentials: bool) -> Self {
        Self {
            profile_storage: true,
            secure_credentials,
            ssh: true,
            terminal: true,
            sftp: true,
            monitor: false,
        }
    }

    pub const fn monitor_backend(secure_credentials: bool) -> Self {
        Self {
            profile_storage: true,
            secure_credentials,
            ssh: true,
            terminal: true,
            sftp: true,
            monitor: true,
        }
    }
}

/// A JSON-safe wire representation for counters and byte sizes that may exceed
/// JavaScript's exact integer range.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, TS)]
#[ts(type = "string")]
pub struct DecimalU64(pub u64);

impl fmt::Display for DecimalU64 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl From<u64> for DecimalU64 {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl Serialize for DecimalU64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for DecimalU64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        u64::from_str(&value)
            .map(Self)
            .map_err(|_| D::Error::custom("expected an unsigned 64-bit decimal string"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_uses_camel_case() {
        let value = serde_json::to_value(AppInfo::current()).expect("serialize app info");
        assert_eq!(value["apiVersion"], API_VERSION);
        assert!(value.get("api_version").is_none());
        assert_eq!(value["capabilities"]["profileStorage"], false);
    }

    #[test]
    fn monitor_timestamps_use_safe_json_numbers_and_camel_case() {
        let sample = MonitorHistorySample {
            sampled_at_ms: 1_790_000_000_123,
            value: 42.5,
        };
        let value = serde_json::to_value(&sample).expect("serialize monitor history sample");
        assert_eq!(value["sampledAtMs"].as_i64(), Some(sample.sampled_at_ms));
        assert_eq!(value["value"], 42.5);
        assert!(value.get("sampled_at_ms").is_none());

        let payload: MonitorGetHistoryPayload = serde_json::from_value(serde_json::json!({
            "connectionId": "00000000-0000-0000-0000-000000000001",
            "metric": "cpuUsage",
            "fromMs": 1_790_000_000_000_i64,
            "toMs": 1_790_000_060_000_i64,
            "limit": 120
        }))
        .expect("deserialize monitor history request with JavaScript number timestamps");
        assert_eq!(payload.from_ms, Some(1_790_000_000_000));
        assert_eq!(payload.to_ms, Some(1_790_000_060_000));
    }

    #[test]
    fn decimal_u64_round_trips_above_javascript_safe_integer() {
        let value = DecimalU64(9_007_199_254_740_993);
        let json = serde_json::to_string(&value).expect("serialize counter");
        assert_eq!(json, "\"9007199254740993\"");
        assert_eq!(
            serde_json::from_str::<DecimalU64>(&json).expect("deserialize counter"),
            value
        );
    }

    #[test]
    fn decimal_u64_rejects_json_number() {
        assert!(serde_json::from_str::<DecimalU64>("42").is_err());
    }
}
