//! MauLink's UI-independent application core.

pub mod app;
pub mod connections;
pub mod contracts;
pub mod credentials;
pub mod error;
pub mod host_keys;
pub mod local_files;
pub mod monitor;
pub mod network;
pub mod preflight;
pub mod profiles;
pub mod server_appearance;
pub mod settings;
pub mod sftp;
pub mod ssh;
pub mod storage;
pub mod terminal;

pub use app::{AppCore, ShutdownReport};
pub use connections::{
    AuthenticationChallenge, ConnectionMode, ConnectionRegistry, ConnectionSnapshot,
    ConnectionState, HostKeyChallenge, HostKeyDecision, NegotiatedAlgorithms,
};
pub use contracts::{
    API_VERSION, ApiRequest, AppCapabilities, AppInfo, AuthenticationRespondPayload,
    BackgroundImageAsset, BackgroundImageGetResult, BackgroundImageImportPayload,
    BackgroundImagePayload, ConnectionDisconnectPayload, ConnectionIdPayload,
    ConnectionPreflightError, ConnectionPreflightPayload, ConnectionPreflightResult,
    ConnectionStartPayload, ConnectionStartSource, DecimalU64, EmptyPayload, GroupUpdatePayload,
    HostKeyGetPayload, HostKeyRespondPayload, LocalFileSelectPayload, MonitorCpuSnapshot,
    MonitorDiskSnapshot, MonitorGetHistoryPayload, MonitorGetSnapshotPayload, MonitorHistoryMetric,
    MonitorHistoryPage, MonitorHistorySample, MonitorLoadSnapshot, MonitorMemorySnapshot,
    MonitorMetricQuality, MonitorNetworkInterface, MonitorNetworkSnapshot, MonitorQualityStatus,
    MonitorRefreshPayload, MonitorSnapshot, MonitorSystemSnapshot, MonitorUptimeSnapshot,
    NetworkGeo, NetworkHostKind, NetworkInspectPayload, NetworkInspection, NetworkInspectionSource,
    NetworkIpVersion, NetworkScope, RemoteFileEntry, RemoteFileType, ResourceIdPayload,
    RevisionPayload, ServerAppearancePayload, ServerCreatePayload, ServerDeletePayload,
    ServerProfileDraft, ServerUpdatePayload, SftpCursorPayload, SftpDeletePayload,
    SftpDirectoryPage, SftpDownloadPayload, SftpListStartPayload, SftpMkdirPayload,
    SftpReadTextPayload, SftpReadTextResult, SftpRenamePayload, SftpStatPayload,
    SftpTransferDirection, SftpTransferIdPayload, SftpTransferListPayload, SftpTransferSnapshot,
    SftpTransferState, SftpUploadPayload, SftpWriteTextPayload, SftpWriteTextResult,
    SftpWriteTextWithSudoPayload, TerminalAckPayload, TerminalIdPayload, TerminalOpenPayload,
    TerminalResizePayload, TerminalWritePayload, WorkspaceActivityPayload,
};
pub use credentials::{
    CredentialDeleteResult, CredentialKind, CredentialManager, CredentialReplaceResult,
    CredentialUpdate, CredentialWorker, NativeSecureStore, RetainedCredential, Secret, SecureStore,
    SecureStoreError, ServerMutationResult,
};
pub use error::{AppError, ErrorAction, ErrorCode};
pub use host_keys::{HostKeyCandidate, HostKeyCheck, HostKeyRecord, HostKeyStore, HostKeyVerifier};
pub use local_files::{
    LocalFilePurpose, LocalFileRegistry, SelectedLocalFile, TerminalBackgroundImageInfo,
    validate_terminal_background_image_bytes,
};
pub use monitor::MonitorManager;
pub use network::NetworkInspector;
pub use profiles::{
    AuthType, Group, GroupCreate, GroupUpdate, PathEncoding, ProfileStore, ProxyType,
    ServerListPage, ServerListQuery, ServerProfile, ServerProfileInput, StoredPath,
};
pub use server_appearance::{
    ServerAppearance, ServerAppearanceStore, ServerAppearanceUpdate, ServerEnvironment,
    TerminalAppearanceSettings,
};
pub use settings::{
    AccentColor, AppIconStyle, AppSettings, CursorStyle, Language, SettingsRecord, SettingsService,
    SettingsUpdate, SidebarWidth, TerminalBackgroundFit, TerminalBackgroundImageSettings,
    TerminalBackgroundOverlayKind, TerminalBackgroundPosition, TerminalCustomColors,
    TerminalThemeMode, Theme, UiDensity,
};
pub use sftp::{SftpManager, SftpTransferManager};
pub use ssh::{SshConnectionManager, SshConnector, SshSession};
pub use storage::Database;
pub use terminal::{
    TerminalChunk, TerminalManager, TerminalOpenResult, TerminalSize, TerminalSnapshot,
    TerminalState, TerminalWriteResult,
};
