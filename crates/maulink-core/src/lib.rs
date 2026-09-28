//! MauLink's UI-independent application core.

pub mod app;
pub mod connections;
pub mod contracts;
pub mod credentials;
pub mod error;
pub mod host_keys;
pub mod local_files;
pub mod monitor;
pub mod profiles;
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
    ConnectionDisconnectPayload, ConnectionIdPayload, ConnectionStartPayload,
    ConnectionStartSource, DecimalU64, EmptyPayload, GroupUpdatePayload, HostKeyRespondPayload,
    LocalFileSelectPayload, MonitorCpuSnapshot, MonitorDiskSnapshot, MonitorGetHistoryPayload,
    MonitorGetSnapshotPayload, MonitorHistoryMetric, MonitorHistoryPage, MonitorHistorySample,
    MonitorLoadSnapshot, MonitorMemorySnapshot, MonitorMetricQuality, MonitorNetworkInterface,
    MonitorNetworkSnapshot, MonitorQualityStatus, MonitorRefreshPayload, MonitorSnapshot,
    MonitorSystemSnapshot, MonitorUptimeSnapshot, RemoteFileEntry, RemoteFileType,
    ResourceIdPayload, RevisionPayload, ServerCreatePayload, ServerDeletePayload,
    ServerProfileDraft, ServerUpdatePayload, SftpCursorPayload, SftpDeletePayload,
    SftpDirectoryPage, SftpDownloadPayload, SftpListStartPayload, SftpMkdirPayload,
    SftpRenamePayload, SftpStatPayload, SftpTransferDirection, SftpTransferIdPayload,
    SftpTransferListPayload, SftpTransferSnapshot, SftpTransferState, SftpUploadPayload,
    TerminalAckPayload, TerminalIdPayload, TerminalOpenPayload, TerminalResizePayload,
    TerminalWritePayload, WorkspaceActivityPayload,
};
pub use credentials::{
    CredentialDeleteResult, CredentialKind, CredentialManager, CredentialReplaceResult,
    CredentialUpdate, CredentialWorker, NativeSecureStore, RetainedCredential, Secret, SecureStore,
    SecureStoreError, ServerMutationResult,
};
pub use error::{AppError, ErrorAction, ErrorCode};
pub use host_keys::{HostKeyCandidate, HostKeyCheck, HostKeyRecord, HostKeyStore, HostKeyVerifier};
pub use local_files::{LocalFilePurpose, LocalFileRegistry, SelectedLocalFile};
pub use monitor::MonitorManager;
pub use profiles::{
    AuthType, Group, GroupCreate, GroupUpdate, PathEncoding, ProfileStore, ServerListPage,
    ServerListQuery, ServerProfile, ServerProfileInput, StoredPath,
};
pub use settings::{
    AppSettings, CursorStyle, Language, SettingsRecord, SettingsService, SettingsUpdate, Theme,
};
pub use sftp::{SftpManager, SftpTransferManager};
pub use ssh::{SshConnectionManager, SshConnector, SshSession};
pub use storage::Database;
pub use terminal::{
    TerminalChunk, TerminalManager, TerminalOpenResult, TerminalSize, TerminalSnapshot,
    TerminalState, TerminalWriteResult,
};
