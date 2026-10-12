use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use maulink_core::{ApiRequest, AppError, EmptyPayload, SettingsService, UpdateChannel};
use semver::Version;
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, State, ipc::Channel};
use tauri_plugin_updater::{Config, Error as UpdaterError, Update, UpdaterExt};
use tokio::sync::Mutex;
use url::Url;

use crate::state::DesktopState;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateErrorCode {
    Network,
    Metadata,
    Signature,
    Storage,
    Permission,
    Install,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateTransition {
    Upgrade,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateBlockedReason {
    CompatibilityUnverified,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UpdateCheckResult {
    NotConfigured,
    ChannelEmpty {
        channel: UpdateChannel,
    },
    UpToDate {
        channel: UpdateChannel,
        #[serde(rename = "installedVersion")]
        installed_version: String,
    },
    Available {
        channel: UpdateChannel,
        #[serde(rename = "installedVersion")]
        installed_version: String,
        #[serde(rename = "targetVersion")]
        target_version: String,
        transition: UpdateTransition,
        #[serde(rename = "releaseNotes")]
        release_notes: Option<String>,
        #[serde(rename = "publishedAt")]
        published_at: Option<String>,
    },
    Blocked {
        channel: UpdateChannel,
        #[serde(rename = "installedVersion")]
        installed_version: String,
        #[serde(rename = "targetVersion")]
        target_version: String,
        reason: UpdateBlockedReason,
    },
    Failed {
        code: UpdateErrorCode,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UpdateOperationResult {
    Completed,
    Failed { code: UpdateErrorCode },
}

#[derive(Clone, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum UpdateDownloadEvent {
    #[serde(rename = "Started")]
    Started {
        #[serde(rename = "contentLength")]
        content_length: Option<u64>,
    },
    #[serde(rename = "Progress")]
    Progress {
        #[serde(rename = "chunkLength")]
        chunk_length: usize,
    },
    #[serde(rename = "Finished")]
    Finished,
}

struct PendingUpdate {
    channel: UpdateChannel,
    update: Update,
    downloaded: Option<Vec<u8>>,
}

#[derive(Default)]
pub struct UpdaterState {
    operation: Mutex<()>,
    pending: Mutex<Option<PendingUpdate>>,
}

pub fn is_configured(value: Option<&Value>) -> bool {
    value
        .and_then(|value| serde_json::from_value::<Config>(value.clone()).ok())
        .is_some_and(|config| valid_config(&config))
}

fn valid_config(config: &Config) -> bool {
    !config.pubkey.trim().is_empty()
        && !config.dangerous_insecure_transport_protocol
        && !config.dangerous_accept_invalid_certs
        && !config.dangerous_accept_invalid_hostnames
        && config.endpoints.len() == 2
        && config.endpoints[0] != config.endpoints[1]
        && config
            .endpoints
            .iter()
            .all(|endpoint| endpoint.scheme() == "https")
}

fn preferred_channel(settings: &SettingsService, installed_version: &str) -> UpdateChannel {
    settings
        .current()
        .value
        .update_channel
        .unwrap_or_else(|| default_channel(installed_version))
}

fn default_channel(version: &str) -> UpdateChannel {
    match Version::parse(version) {
        Ok(version) if version.pre.is_empty() => UpdateChannel::Stable,
        Ok(_) => UpdateChannel::Preview,
        Err(_) => UpdateChannel::Stable,
    }
}

fn channel_endpoint(config: &Config, channel: UpdateChannel) -> Option<Url> {
    let index = match channel {
        UpdateChannel::Stable => 0,
        UpdateChannel::Preview => 1,
    };
    config.endpoints.get(index).cloned()
}

fn version_matches_channel(version: &str, channel: UpdateChannel) -> bool {
    let Ok(parsed) = Version::parse(version) else {
        return false;
    };
    if parsed.to_string() != version || !parsed.build.is_empty() {
        return false;
    }

    match channel {
        UpdateChannel::Stable => parsed.pre.is_empty(),
        UpdateChannel::Preview => {
            let Some(sequence) = parsed.pre.as_str().strip_prefix("rc.") else {
                return false;
            };
            !sequence.is_empty()
                && sequence != "0"
                && !sequence.starts_with('0')
                && sequence.bytes().all(|byte| byte.is_ascii_digit())
        }
    }
}

async fn endpoint_status(endpoint: Url) -> Result<reqwest::StatusCode, UpdateErrorCode> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| UpdateErrorCode::Network)?
        .get(endpoint)
        .send()
        .await
        .map(|response| response.status())
        .map_err(|_| UpdateErrorCode::Network)
}

fn endpoint_status_result(
    status: reqwest::StatusCode,
    channel: UpdateChannel,
) -> Option<UpdateCheckResult> {
    if status == reqwest::StatusCode::NO_CONTENT
        || status == reqwest::StatusCode::NOT_FOUND
        || status == reqwest::StatusCode::GONE
    {
        return Some(UpdateCheckResult::ChannelEmpty { channel });
    }
    let code = if status == reqwest::StatusCode::UNAUTHORIZED
        || status == reqwest::StatusCode::FORBIDDEN
    {
        Some(UpdateErrorCode::Permission)
    } else if status == reqwest::StatusCode::REQUEST_TIMEOUT
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
    {
        Some(UpdateErrorCode::Network)
    } else if !status.is_success() {
        Some(UpdateErrorCode::Metadata)
    } else {
        None
    };
    code.map(|code| UpdateCheckResult::Failed { code })
}

fn check_error_code(error: &UpdaterError) -> UpdateErrorCode {
    match error {
        UpdaterError::Reqwest(_) | UpdaterError::Network(_) => UpdateErrorCode::Network,
        UpdaterError::Minisign(_)
        | UpdaterError::Base64(_)
        | UpdaterError::SignatureUtf8(_)
        | UpdaterError::SignedVersionMismatch { .. }
        | UpdaterError::MissingSignedVersion => UpdateErrorCode::Signature,
        UpdaterError::Io(_) => UpdateErrorCode::Storage,
        UpdaterError::UnsupportedArch | UpdaterError::UnsupportedOs => UpdateErrorCode::Unsupported,
        UpdaterError::TargetNotFound(_)
        | UpdaterError::TargetsNotFound(_)
        | UpdaterError::Serialization(_)
        | UpdaterError::Semver(_) => UpdateErrorCode::Metadata,
        _ => UpdateErrorCode::Unknown,
    }
}

async fn check_for_updates(
    app: &AppHandle,
    updater_state: &UpdaterState,
    settings: &SettingsService,
) -> UpdateCheckResult {
    let _operation = updater_state.operation.lock().await;
    *updater_state.pending.lock().await = None;

    let installed_version = app.package_info().version.to_string();
    let channel = preferred_channel(settings, &installed_version);
    let config_value = app.config().plugins.0.get("updater");
    let Some(config) = config_value
        .cloned()
        .and_then(|value| serde_json::from_value::<Config>(value).ok())
        .filter(valid_config)
    else {
        return UpdateCheckResult::NotConfigured;
    };
    let Some(endpoint) = channel_endpoint(&config, channel) else {
        return UpdateCheckResult::NotConfigured;
    };

    match endpoint_status(endpoint.clone()).await {
        Ok(status) => {
            if let Some(result) = endpoint_status_result(status, channel) {
                return result;
            }
        }
        Err(code) => return UpdateCheckResult::Failed { code },
    }

    let updater = match app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map(|builder| {
            builder
                .version_comparator(|current, remote| remote.version != current)
                .build()
        }) {
        Ok(Ok(updater)) => updater,
        Ok(Err(error)) | Err(error) => {
            return UpdateCheckResult::Failed {
                code: check_error_code(&error),
            };
        }
    };

    let update = match updater.check().await {
        Ok(Some(update)) => update,
        Ok(None) => {
            return UpdateCheckResult::UpToDate {
                channel,
                installed_version,
            };
        }
        Err(error) => {
            return UpdateCheckResult::Failed {
                code: check_error_code(&error),
            };
        }
    };

    let current = match Version::parse(&installed_version) {
        Ok(version) => version,
        Err(_) => {
            return UpdateCheckResult::Failed {
                code: UpdateErrorCode::Metadata,
            };
        }
    };
    let target = match Version::parse(&update.version) {
        Ok(version) => version,
        Err(_) => {
            return UpdateCheckResult::Failed {
                code: UpdateErrorCode::Metadata,
            };
        }
    };
    if !version_matches_channel(&update.version, channel) {
        return UpdateCheckResult::Failed {
            code: UpdateErrorCode::Metadata,
        };
    }

    if target == current {
        return UpdateCheckResult::UpToDate {
            channel,
            installed_version,
        };
    }
    if target < current {
        return UpdateCheckResult::Blocked {
            channel,
            installed_version,
            target_version: update.version,
            reason: UpdateBlockedReason::CompatibilityUnverified,
        };
    }

    let response = UpdateCheckResult::Available {
        channel,
        installed_version,
        target_version: update.version.clone(),
        transition: UpdateTransition::Upgrade,
        release_notes: update.body.clone(),
        published_at: update.date.map(|date| date.to_string()),
    };
    *updater_state.pending.lock().await = Some(PendingUpdate {
        channel,
        update,
        downloaded: None,
    });
    response
}

#[tauri::command]
pub async fn updater_check(
    request: ApiRequest<EmptyPayload>,
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<UpdateCheckResult, AppError> {
    request.validate()?;
    Ok(check_for_updates(&app, &state.updater, &state.settings).await)
}

#[tauri::command]
pub async fn updater_download(
    request: ApiRequest<EmptyPayload>,
    output_channel: Channel<UpdateDownloadEvent>,
    state: State<'_, DesktopState>,
) -> Result<UpdateOperationResult, AppError> {
    Ok(download_update_inner(request, output_channel, state).await)
}

async fn download_update_inner(
    request: ApiRequest<EmptyPayload>,
    output_channel: Channel<UpdateDownloadEvent>,
    state: State<'_, DesktopState>,
) -> UpdateOperationResult {
    if request.validate().is_err() {
        return UpdateOperationResult::Failed {
            code: UpdateErrorCode::Unknown,
        };
    }
    let _operation = state.updater.operation.lock().await;
    let Some(mut pending) = state.updater.pending.lock().await.take() else {
        return UpdateOperationResult::Failed {
            code: UpdateErrorCode::Metadata,
        };
    };
    let installed_version = state.settings.current().value;
    let expected_channel = installed_version
        .update_channel
        .unwrap_or_else(|| default_channel(&state.core.info().version));
    if pending.channel != expected_channel {
        return UpdateOperationResult::Failed {
            code: UpdateErrorCode::Metadata,
        };
    }

    let started = Arc::new(AtomicBool::new(false));
    let chunk_started = Arc::clone(&started);
    let finish_started = Arc::clone(&started);
    let chunk_channel = output_channel.clone();
    let finish_channel = output_channel;
    let download = pending
        .update
        .download(
            move |chunk_length, content_length| {
                if !chunk_started.swap(true, Ordering::Relaxed) {
                    let _ = chunk_channel.send(UpdateDownloadEvent::Started { content_length });
                }
                let _ = chunk_channel.send(UpdateDownloadEvent::Progress { chunk_length });
            },
            move || {
                if !finish_started.swap(true, Ordering::Relaxed) {
                    let _ = finish_channel.send(UpdateDownloadEvent::Started {
                        content_length: None,
                    });
                }
                let _ = finish_channel.send(UpdateDownloadEvent::Finished);
            },
        )
        .await;

    match download {
        Ok(bytes) => pending.downloaded = Some(bytes),
        Err(error) => {
            *state.updater.pending.lock().await = Some(pending);
            return UpdateOperationResult::Failed {
                code: check_error_code(&error),
            };
        }
    }
    *state.updater.pending.lock().await = Some(pending);
    UpdateOperationResult::Completed
}

#[tauri::command]
pub async fn updater_install(
    request: ApiRequest<EmptyPayload>,
    state: State<'_, DesktopState>,
) -> Result<UpdateOperationResult, AppError> {
    Ok(install_update_inner(request, state).await)
}

async fn install_update_inner(
    request: ApiRequest<EmptyPayload>,
    state: State<'_, DesktopState>,
) -> UpdateOperationResult {
    if request.validate().is_err() {
        return UpdateOperationResult::Failed {
            code: UpdateErrorCode::Unknown,
        };
    }
    let _operation = state.updater.operation.lock().await;
    let Some(mut pending) = state.updater.pending.lock().await.take() else {
        return UpdateOperationResult::Failed {
            code: UpdateErrorCode::Metadata,
        };
    };
    let Some(bytes) = pending.downloaded.take() else {
        *state.updater.pending.lock().await = Some(pending);
        return UpdateOperationResult::Failed {
            code: UpdateErrorCode::Metadata,
        };
    };
    let transition_is_upgrade = match (
        Version::parse(&pending.update.version),
        Version::parse(&pending.update.current_version),
    ) {
        (Ok(target), Ok(current)) => target > current,
        _ => false,
    };
    if !transition_is_upgrade {
        *state.updater.pending.lock().await = Some(pending);
        return UpdateOperationResult::Failed {
            code: UpdateErrorCode::Permission,
        };
    }

    match pending.update.install(&bytes) {
        Ok(()) => UpdateOperationResult::Completed,
        Err(_) => {
            pending.downloaded = Some(bytes);
            *state.updater.pending.lock().await = Some(pending);
            UpdateOperationResult::Failed {
                code: UpdateErrorCode::Install,
            }
        }
    }
}

#[tauri::command]
pub async fn updater_cancel(
    request: ApiRequest<EmptyPayload>,
    state: State<'_, DesktopState>,
) -> Result<(), AppError> {
    request.validate()?;
    let _operation = state.updater.operation.lock().await;
    *state.updater.pending.lock().await = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn two_channel_config(stable: &str, preview: &str) -> Value {
        json!({
            "pubkey": "test-public-key",
            "endpoints": [stable, preview]
        })
    }

    #[test]
    fn updater_is_disabled_without_both_trusted_https_channels_and_a_public_key() {
        assert!(!is_configured(None));
        assert!(!is_configured(Some(&Value::Null)));
        assert!(!is_configured(Some(&json!({}))));
        assert!(!is_configured(Some(&json!({
            "pubkey": "test-public-key",
            "endpoints": ["https://updates.test/stable.json"]
        }))));
        assert!(!is_configured(Some(&json!({
            "pubkey": " ",
            "endpoints": ["https://updates.test/stable.json", "https://updates.test/preview.json"]
        }))));
        assert!(!is_configured(Some(&json!({
            "pubkey": "test-public-key",
            "endpoints": ["http://updates.test/stable.json", "https://updates.test/preview.json"]
        }))));
        assert!(!is_configured(Some(&two_channel_config(
            "https://updates.test/releases.json",
            "https://updates.test/releases.json"
        ))));
    }

    #[test]
    fn accepts_exactly_two_https_channel_indexes_with_a_public_key() {
        let config = two_channel_config(
            "https://updates.test/stable.json",
            "https://updates.test/preview.json",
        );
        assert!(is_configured(Some(&config)));
        let parsed: Config = serde_json::from_value(config).expect("updater config");
        assert_eq!(
            channel_endpoint(&parsed, UpdateChannel::Stable)
                .unwrap()
                .path(),
            "/stable.json"
        );
        assert_eq!(
            channel_endpoint(&parsed, UpdateChannel::Preview)
                .unwrap()
                .path(),
            "/preview.json"
        );
    }

    #[test]
    fn defaults_release_candidates_to_preview_and_final_versions_to_stable() {
        assert_eq!(default_channel("0.1.1-rc.8"), UpdateChannel::Preview);
        assert_eq!(default_channel("0.1.1"), UpdateChannel::Stable);
    }

    #[test]
    fn release_versions_must_match_the_selected_channel_and_canonical_format() {
        for (version, channel, expected) in [
            ("0.1.1", UpdateChannel::Stable, true),
            ("0.1.1-rc.10", UpdateChannel::Preview, true),
            ("0.1.1-rc.0", UpdateChannel::Preview, false),
            ("0.1.1-rc.01", UpdateChannel::Preview, false),
            ("0.1.1-beta.1", UpdateChannel::Preview, false),
            ("0.1.1+build.1", UpdateChannel::Stable, false),
            ("0.1.1-rc.1", UpdateChannel::Stable, false),
            ("0.1.1", UpdateChannel::Preview, false),
        ] {
            assert_eq!(
                version_matches_channel(version, channel),
                expected,
                "{version}"
            );
        }
    }

    #[test]
    fn update_index_statuses_keep_empty_permission_and_network_results_distinct() {
        assert!(matches!(
            endpoint_status_result(reqwest::StatusCode::NOT_FOUND, UpdateChannel::Stable),
            Some(UpdateCheckResult::ChannelEmpty {
                channel: UpdateChannel::Stable
            })
        ));
        assert!(matches!(
            endpoint_status_result(reqwest::StatusCode::FORBIDDEN, UpdateChannel::Preview),
            Some(UpdateCheckResult::Failed {
                code: UpdateErrorCode::Permission
            })
        ));
        assert!(matches!(
            endpoint_status_result(
                reqwest::StatusCode::TOO_MANY_REQUESTS,
                UpdateChannel::Stable
            ),
            Some(UpdateCheckResult::Failed {
                code: UpdateErrorCode::Network
            })
        ));
        assert!(matches!(
            endpoint_status_result(
                reqwest::StatusCode::TEMPORARY_REDIRECT,
                UpdateChannel::Stable
            ),
            Some(UpdateCheckResult::Failed {
                code: UpdateErrorCode::Metadata
            })
        ));
        assert!(matches!(
            endpoint_status_result(reqwest::StatusCode::OK, UpdateChannel::Preview),
            None
        ));
    }
}
