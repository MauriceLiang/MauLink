use std::path::PathBuf;

use maulink_core::{
    AccentColor, ApiRequest, AppError, AppInfo, AppSettings, AuthType, AuthenticationChallenge,
    AuthenticationRespondPayload, ConnectionDisconnectPayload, ConnectionIdPayload, ConnectionMode,
    ConnectionPreflightError, ConnectionPreflightPayload, ConnectionPreflightResult,
    ConnectionSnapshot, ConnectionStartPayload, ConnectionStartSource, ConnectionState,
    CredentialDeleteResult, CredentialKind, CredentialReplaceResult, CredentialUpdate, CursorStyle,
    DecimalU64, EmptyPayload, Group, GroupCreate, GroupUpdate, GroupUpdatePayload,
    HostKeyChallenge, HostKeyDecision, HostKeyGetPayload, HostKeyRecord, HostKeyRespondPayload,
    Language, LocalFilePurpose, LocalFileSelectPayload, MonitorCpuSnapshot, MonitorDiskSnapshot,
    MonitorGetHistoryPayload, MonitorGetSnapshotPayload, MonitorHistoryMetric, MonitorHistoryPage,
    MonitorHistorySample, MonitorLoadSnapshot, MonitorMemorySnapshot, MonitorMetricQuality,
    MonitorNetworkInterface, MonitorNetworkSnapshot, MonitorQualityStatus, MonitorRefreshPayload,
    MonitorSnapshot, MonitorSystemSnapshot, MonitorUptimeSnapshot, NegotiatedAlgorithms,
    NetworkGeo, NetworkHostKind, NetworkInspectPayload, NetworkInspection, NetworkInspectionSource,
    NetworkIpVersion, NetworkScope, ProxyType, RemoteFileEntry, RemoteFileType, ResourceIdPayload,
    RetainedCredential, RevisionPayload, SelectedLocalFile, ServerCreatePayload,
    ServerDeletePayload, ServerListPage, ServerListQuery, ServerMutationResult, ServerProfile,
    ServerProfileDraft, ServerUpdatePayload, SettingsRecord, SettingsUpdate, SftpCursorPayload,
    SftpDeletePayload, SftpDirectoryPage, SftpDownloadPayload, SftpListStartPayload,
    SftpMkdirPayload, SftpReadTextPayload, SftpReadTextResult, SftpRenamePayload, SftpStatPayload,
    SftpTransferDirection, SftpTransferIdPayload, SftpTransferListPayload, SftpTransferSnapshot,
    SftpTransferState, SftpUploadPayload, SftpWriteTextPayload, SftpWriteTextResult,
    SftpWriteTextWithSudoPayload, TerminalAckPayload, TerminalChunk, TerminalCustomColors,
    TerminalIdPayload, TerminalOpenPayload, TerminalOpenResult, TerminalResizePayload,
    TerminalSize, TerminalSnapshot, TerminalState, TerminalThemeMode, TerminalWritePayload,
    TerminalWriteResult, Theme, WorkspaceActivityPayload,
};
use ts_rs::{Config, TS};

fn main() -> Result<(), ts_rs::ExportError> {
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../contracts/v1");
    std::fs::create_dir_all(&output)?;
    let config = Config::new().with_out_dir(output);

    AppInfo::export_all(&config)?;
    AppError::export_all(&config)?;
    DecimalU64::export_all(&config)?;
    Group::export_all(&config)?;
    GroupCreate::export_all(&config)?;
    GroupUpdate::export_all(&config)?;
    ServerProfile::export_all(&config)?;
    ServerListQuery::export_all(&config)?;
    ServerListPage::export_all(&config)?;
    AuthType::export_all(&config)?;
    AppSettings::export_all(&config)?;
    AccentColor::export_all(&config)?;
    SettingsRecord::export_all(&config)?;
    SettingsUpdate::export_all(&config)?;
    TerminalThemeMode::export_all(&config)?;
    TerminalCustomColors::export_all(&config)?;
    Theme::export_all(&config)?;
    Language::export_all(&config)?;
    CursorStyle::export_all(&config)?;
    LocalFilePurpose::export_all(&config)?;
    SelectedLocalFile::export_all(&config)?;
    CredentialKind::export_all(&config)?;
    maulink_core::Secret::export_all(&config)?;
    CredentialReplaceResult::export_all(&config)?;
    CredentialDeleteResult::export_all(&config)?;
    CredentialUpdate::export_all(&config)?;
    RetainedCredential::export_all(&config)?;
    ServerMutationResult::export_all(&config)?;
    HostKeyRecord::export_all(&config)?;
    HostKeyGetPayload::export_all(&config)?;
    NetworkHostKind::export_all(&config)?;
    NetworkIpVersion::export_all(&config)?;
    NetworkScope::export_all(&config)?;
    NetworkInspectionSource::export_all(&config)?;
    NetworkGeo::export_all(&config)?;
    NetworkInspection::export_all(&config)?;
    NetworkInspectPayload::export_all(&config)?;
    ConnectionPreflightError::export_all(&config)?;
    ConnectionPreflightResult::export_all(&config)?;
    ConnectionPreflightPayload::export_all(&config)?;
    HostKeyChallenge::export_all(&config)?;
    AuthenticationChallenge::export_all(&config)?;
    ConnectionStartPayload::export_all(&config)?;
    ConnectionStartSource::export_all(&config)?;
    ConnectionIdPayload::export_all(&config)?;
    ConnectionDisconnectPayload::export_all(&config)?;
    HostKeyRespondPayload::export_all(&config)?;
    AuthenticationRespondPayload::export_all(&config)?;
    HostKeyDecision::export_all(&config)?;
    ConnectionMode::export_all(&config)?;
    ConnectionState::export_all(&config)?;
    ConnectionSnapshot::export_all(&config)?;
    NegotiatedAlgorithms::export_all(&config)?;
    ApiRequest::<EmptyPayload>::export_all(&config)?;
    EmptyPayload::export_all(&config)?;
    ResourceIdPayload::export_all(&config)?;
    RevisionPayload::export_all(&config)?;
    ServerProfileDraft::export_all(&config)?;
    ProxyType::export_all(&config)?;
    ServerCreatePayload::export_all(&config)?;
    ServerUpdatePayload::export_all(&config)?;
    ServerDeletePayload::export_all(&config)?;
    GroupUpdatePayload::export_all(&config)?;
    LocalFileSelectPayload::export_all(&config)?;
    TerminalState::export_all(&config)?;
    TerminalSize::export_all(&config)?;
    TerminalSnapshot::export_all(&config)?;
    TerminalOpenResult::export_all(&config)?;
    TerminalChunk::export_all(&config)?;
    TerminalWriteResult::export_all(&config)?;
    TerminalOpenPayload::export_all(&config)?;
    TerminalIdPayload::export_all(&config)?;
    TerminalWritePayload::export_all(&config)?;
    TerminalResizePayload::export_all(&config)?;
    TerminalAckPayload::export_all(&config)?;
    RemoteFileType::export_all(&config)?;
    RemoteFileEntry::export_all(&config)?;
    SftpDirectoryPage::export_all(&config)?;
    SftpListStartPayload::export_all(&config)?;
    SftpCursorPayload::export_all(&config)?;
    SftpStatPayload::export_all(&config)?;
    SftpReadTextPayload::export_all(&config)?;
    SftpReadTextResult::export_all(&config)?;
    SftpWriteTextPayload::export_all(&config)?;
    SftpWriteTextResult::export_all(&config)?;
    SftpWriteTextWithSudoPayload::export_all(&config)?;
    SftpMkdirPayload::export_all(&config)?;
    SftpRenamePayload::export_all(&config)?;
    SftpDeletePayload::export_all(&config)?;
    SftpUploadPayload::export_all(&config)?;
    SftpDownloadPayload::export_all(&config)?;
    SftpTransferIdPayload::export_all(&config)?;
    SftpTransferListPayload::export_all(&config)?;
    SftpTransferDirection::export_all(&config)?;
    SftpTransferState::export_all(&config)?;
    SftpTransferSnapshot::export_all(&config)?;
    MonitorQualityStatus::export_all(&config)?;
    MonitorMetricQuality::export_all(&config)?;
    MonitorCpuSnapshot::export_all(&config)?;
    MonitorLoadSnapshot::export_all(&config)?;
    MonitorMemorySnapshot::export_all(&config)?;
    MonitorDiskSnapshot::export_all(&config)?;
    MonitorNetworkInterface::export_all(&config)?;
    MonitorNetworkSnapshot::export_all(&config)?;
    MonitorUptimeSnapshot::export_all(&config)?;
    MonitorSystemSnapshot::export_all(&config)?;
    MonitorSnapshot::export_all(&config)?;
    MonitorHistoryMetric::export_all(&config)?;
    MonitorHistorySample::export_all(&config)?;
    MonitorHistoryPage::export_all(&config)?;
    MonitorGetSnapshotPayload::export_all(&config)?;
    MonitorGetHistoryPayload::export_all(&config)?;
    MonitorRefreshPayload::export_all(&config)?;
    WorkspaceActivityPayload::export_all(&config)?;
    Ok(())
}
