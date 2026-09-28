use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Stable machine-readable error codes shared with the desktop frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ValidationFailed,
    RevisionConflict,
    ServerInUse,
    ResourceNotFound,
    ResourceLimit,
    ResourceClosed,
    Cancelled,
    IpcVersionUnsupported,
    StorageBusy,
    MigrationFailed,
    SchemaTooNew,
    CredentialAccessDenied,
    CredentialNotFound,
    DnsFailed,
    ConnectionRefused,
    ConnectionTimeout,
    ConnectionLost,
    HostKeyChanged,
    HostKeyRejected,
    HostKeyAlgorithmUnsupported,
    AuthFailed,
    AuthMethodUnsupported,
    AuthTimeout,
    ChallengeExpired,
    PrivateKeyUnreadable,
    KeyFormatUnsupported,
    PassphraseInvalid,
    TerminalOpenFailed,
    TerminalConsumerStalled,
    TerminalAckInvalid,
    TerminalInputSequenceInvalid,
    InputBackpressure,
    ChannelOpenFailed,
    PathNotFound,
    PathExists,
    TargetExists,
    DirectoryNotEmpty,
    PermissionDenied,
    SftpOperationFailed,
    SftpEntryTooLarge,
    UnsupportedPathEncoding,
    TransferBusy,
    TransferOutcomeUnknown,
    PublishUnsupported,
    LocalDiskFull,
    LocalFileOperationFailed,
    MonitorUnsupported,
    MonitorTimeout,
    MonitorOutputTooLarge,
    MonitorCollectionFailed,
    Internal,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ValidationFailed => "VALIDATION_FAILED",
            Self::RevisionConflict => "REVISION_CONFLICT",
            Self::ServerInUse => "SERVER_IN_USE",
            Self::ResourceNotFound => "RESOURCE_NOT_FOUND",
            Self::ResourceLimit => "RESOURCE_LIMIT",
            Self::ResourceClosed => "RESOURCE_CLOSED",
            Self::Cancelled => "CANCELLED",
            Self::IpcVersionUnsupported => "IPC_VERSION_UNSUPPORTED",
            Self::StorageBusy => "STORAGE_BUSY",
            Self::MigrationFailed => "MIGRATION_FAILED",
            Self::SchemaTooNew => "SCHEMA_TOO_NEW",
            Self::CredentialAccessDenied => "CREDENTIAL_ACCESS_DENIED",
            Self::CredentialNotFound => "CREDENTIAL_NOT_FOUND",
            Self::DnsFailed => "DNS_FAILED",
            Self::ConnectionRefused => "CONNECTION_REFUSED",
            Self::ConnectionTimeout => "CONNECTION_TIMEOUT",
            Self::ConnectionLost => "CONNECTION_LOST",
            Self::HostKeyChanged => "HOST_KEY_CHANGED",
            Self::HostKeyRejected => "HOST_KEY_REJECTED",
            Self::HostKeyAlgorithmUnsupported => "HOST_KEY_ALGORITHM_UNSUPPORTED",
            Self::AuthFailed => "AUTH_FAILED",
            Self::AuthMethodUnsupported => "AUTH_METHOD_UNSUPPORTED",
            Self::AuthTimeout => "AUTH_TIMEOUT",
            Self::ChallengeExpired => "CHALLENGE_EXPIRED",
            Self::PrivateKeyUnreadable => "PRIVATE_KEY_UNREADABLE",
            Self::KeyFormatUnsupported => "KEY_FORMAT_UNSUPPORTED",
            Self::PassphraseInvalid => "PASSPHRASE_INVALID",
            Self::TerminalOpenFailed => "TERMINAL_OPEN_FAILED",
            Self::TerminalConsumerStalled => "TERMINAL_CONSUMER_STALLED",
            Self::TerminalAckInvalid => "TERMINAL_ACK_INVALID",
            Self::TerminalInputSequenceInvalid => "TERMINAL_INPUT_SEQUENCE_INVALID",
            Self::InputBackpressure => "INPUT_BACKPRESSURE",
            Self::ChannelOpenFailed => "CHANNEL_OPEN_FAILED",
            Self::PathNotFound => "PATH_NOT_FOUND",
            Self::PathExists => "PATH_EXISTS",
            Self::TargetExists => "TARGET_EXISTS",
            Self::DirectoryNotEmpty => "DIRECTORY_NOT_EMPTY",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::SftpOperationFailed => "SFTP_OPERATION_FAILED",
            Self::SftpEntryTooLarge => "SFTP_ENTRY_TOO_LARGE",
            Self::UnsupportedPathEncoding => "UNSUPPORTED_PATH_ENCODING",
            Self::TransferBusy => "TRANSFER_BUSY",
            Self::TransferOutcomeUnknown => "TRANSFER_OUTCOME_UNKNOWN",
            Self::PublishUnsupported => "PUBLISH_UNSUPPORTED",
            Self::LocalDiskFull => "LOCAL_DISK_FULL",
            Self::LocalFileOperationFailed => "LOCAL_FILE_OPERATION_FAILED",
            Self::MonitorUnsupported => "MONITOR_UNSUPPORTED",
            Self::MonitorTimeout => "MONITOR_TIMEOUT",
            Self::MonitorOutputTooLarge => "MONITOR_OUTPUT_TOO_LARGE",
            Self::MonitorCollectionFailed => "MONITOR_COLLECTION_FAILED",
            Self::Internal => "INTERNAL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ErrorAction {
    None,
    Retry,
    Reload,
    ReviewHostKey,
}

/// Safe error payload crossing the IPC boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{code:?}: {message_key}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message_key: String,
    pub params: Box<BTreeMap<String, String>>,
    pub retryable: bool,
    pub action: ErrorAction,
    pub stage: Option<String>,
    pub request_id: Option<String>,
    pub details: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message_key: impl Into<String>) -> Self {
        Self {
            code,
            message_key: message_key.into(),
            params: Box::default(),
            retryable: false,
            action: ErrorAction::None,
            stage: None,
            request_id: None,
            details: None,
        }
    }

    pub fn shutting_down() -> Self {
        Self::new(ErrorCode::ResourceClosed, "errors.appShuttingDown")
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    pub fn with_retry(mut self) -> Self {
        self.retryable = true;
        self.action = ErrorAction::Retry;
        self
    }

    pub fn with_stage(mut self, stage: impl Into<String>) -> Self {
        self.stage = Some(stage.into());
        self
    }
}
