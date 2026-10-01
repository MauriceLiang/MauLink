use maulink_core::{
    ApiRequest, AppError, AppInfo, AuthenticationRespondPayload, ConnectionDisconnectPayload,
    ConnectionIdPayload, ConnectionSnapshot, ConnectionStartPayload, CredentialDeleteResult,
    CredentialUpdate, EmptyPayload, ErrorCode, Group, GroupCreate, GroupUpdatePayload,
    HostKeyRespondPayload, LocalFilePurpose, LocalFileSelectPayload, MonitorGetHistoryPayload,
    MonitorGetSnapshotPayload, MonitorHistoryPage, MonitorRefreshPayload, MonitorSnapshot,
    RemoteFileEntry, ResourceIdPayload, RetainedCredential, RevisionPayload, SelectedLocalFile,
    ServerCreatePayload, ServerDeletePayload, ServerListPage, ServerListQuery,
    ServerMutationResult, ServerProfile, ServerProfileDraft, ServerProfileInput,
    ServerUpdatePayload, SettingsRecord, SettingsUpdate, SftpCursorPayload, SftpDeletePayload,
    SftpDirectoryPage, SftpDownloadPayload, SftpListStartPayload, SftpMkdirPayload,
    SftpRenamePayload, SftpStatPayload, SftpTransferIdPayload, SftpTransferListPayload,
    SftpTransferSnapshot, SftpUploadPayload, TerminalAckPayload, TerminalChunk, TerminalIdPayload,
    TerminalOpenPayload, TerminalOpenResult, TerminalResizePayload, TerminalSize, TerminalSnapshot,
    TerminalWritePayload, TerminalWriteResult, WorkspaceActivityPayload,
};
use tauri::{AppHandle, ipc::Channel};
use tauri_plugin_dialog::DialogExt;

use crate::state::DesktopState;

#[tauri::command]
pub fn app_get_info(state: tauri::State<'_, DesktopState>) -> AppInfo {
    state.core.info()
}

#[tauri::command]
pub async fn monitor_get_snapshot(
    request: ApiRequest<MonitorGetSnapshotPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<MonitorSnapshot, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.monitor.get_snapshot(payload).await)
}

#[tauri::command]
pub async fn monitor_get_history(
    request: ApiRequest<MonitorGetHistoryPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<MonitorHistoryPage, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.monitor.get_history(payload).await)
}

#[tauri::command]
pub async fn monitor_refresh(
    request: ApiRequest<MonitorRefreshPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<MonitorSnapshot, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.monitor.refresh(payload).await)
}

#[tauri::command]
pub async fn workspace_set_activity(
    request: ApiRequest<WorkspaceActivityPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<(), AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.monitor.set_activity(payload).await)
}

#[tauri::command]
pub async fn group_list(
    request: ApiRequest<EmptyPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<Group>, AppError> {
    let (request_id, _) = request.validate()?;
    attach_request_id(request_id, state.profiles.list_groups().await)
}

#[tauri::command]
pub async fn group_create(
    request: ApiRequest<GroupCreate>,
    state: tauri::State<'_, DesktopState>,
) -> Result<Group, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.profiles.create_group(payload).await)
}

#[tauri::command]
pub async fn group_update(
    request: ApiRequest<GroupUpdatePayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<Group, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(
        request_id,
        state
            .profiles
            .update_group(payload.group_id, payload.update)
            .await,
    )
}

#[tauri::command]
pub async fn group_delete(
    request: ApiRequest<RevisionPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<(), AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(
        request_id,
        state
            .profiles
            .delete_group(payload.id, payload.expected_revision)
            .await,
    )
}

#[tauri::command]
pub async fn server_list(
    request: ApiRequest<ServerListQuery>,
    state: tauri::State<'_, DesktopState>,
) -> Result<ServerListPage, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.profiles.list_servers(payload).await)
}

#[tauri::command]
pub async fn server_get(
    request: ApiRequest<ResourceIdPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<ServerProfile, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.profiles.get_server(payload.id).await)
}

#[tauri::command]
pub async fn server_create(
    request: ApiRequest<ServerCreatePayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<ServerMutationResult, AppError> {
    let (request_id, payload) = request.validate()?;
    let token = payload.profile.private_key_token.clone();
    let input = profile_input(payload.profile, &state.local_files)
        .map_err(|error| error.with_request_id(request_id.clone()))?;
    let result = state
        .credentials
        .create_server(input, payload.credential)
        .await;
    if result.is_ok()
        && let Some(token) = token
    {
        state.local_files.forget(&token);
    }
    attach_request_id(request_id, result)
}

#[tauri::command]
pub async fn server_update(
    request: ApiRequest<ServerUpdatePayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<ServerMutationResult, AppError> {
    let (request_id, payload) = request.validate()?;
    let _connection_guard = state.connections.lock_profile_operations().await;
    if attach_request_id(
        request_id.clone(),
        state.connections.server_in_use(&payload.server_id),
    )? {
        let current = attach_request_id(
            request_id.clone(),
            state.profiles.get_server(payload.server_id.clone()).await,
        )?;
        let connection_fields_changed = current.host != payload.profile.host
            || current.port != payload.profile.port
            || current.username != payload.profile.username
            || current.auth_type != payload.profile.auth_type
            || current.connect_timeout_ms != payload.profile.connect_timeout_ms
            || current.keepalive_interval_seconds != payload.profile.keepalive_interval_seconds
            || current.jump_host != payload.profile.jump_host
            || current.jump_port != payload.profile.jump_port
            || current.proxy_type != payload.profile.proxy_type
            || current.proxy_host != payload.profile.proxy_host
            || current.proxy_port != payload.profile.proxy_port
            || payload.profile.private_key_token.is_some()
            || !matches!(&payload.credential, CredentialUpdate::Keep);
        if connection_fields_changed {
            return Err(AppError::new(ErrorCode::ServerInUse, "errors.serverInUse")
                .with_param("serverId", payload.server_id)
                .with_request_id(request_id));
        }
    }
    let token = payload.profile.private_key_token.clone();
    let private_key_path = match payload.profile.private_key_token.as_deref() {
        Some(token) => state.local_files.resolve_private_key(token).map(Some),
        None if payload.profile.auth_type == maulink_core::AuthType::PrivateKey => state
            .profiles
            .private_key_path_for_update(payload.server_id.clone(), payload.expected_revision)
            .await
            .map(Some),
        None => Ok(None),
    }
    .map_err(|error| error.with_request_id(request_id.clone()))?;
    let input = profile_input_with_path(payload.profile, private_key_path);
    let result = state
        .credentials
        .update_server(
            payload.server_id,
            payload.expected_revision,
            input,
            payload.credential,
        )
        .await;
    if result.is_ok()
        && let Some(token) = token
    {
        state.local_files.forget(&token);
    }
    attach_request_id(request_id, result)
}

#[tauri::command]
pub async fn server_delete(
    request: ApiRequest<ServerDeletePayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<CredentialDeleteResult, AppError> {
    let (request_id, payload) = request.validate()?;
    let _connection_guard = state.connections.lock_profile_operations().await;
    attach_request_id(
        request_id.clone(),
        state.connections.ensure_server_idle(&payload.server_id),
    )?;
    attach_request_id(
        request_id,
        state
            .credentials
            .delete_server(
                payload.server_id,
                payload.expected_revision,
                payload.remove_credentials,
            )
            .await,
    )
}

#[tauri::command]
pub async fn credential_list_retained(
    request: ApiRequest<EmptyPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<RetainedCredential>, AppError> {
    let (request_id, _) = request.validate()?;
    attach_request_id(request_id, state.credentials.list_retained().await)
}

#[tauri::command]
pub async fn credential_delete_retained(
    request: ApiRequest<ResourceIdPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<CredentialDeleteResult, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(
        request_id,
        state.credentials.delete_retained(payload.id).await,
    )
}

#[tauri::command]
pub async fn credential_cleanup_retry(
    request: ApiRequest<ResourceIdPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<CredentialDeleteResult, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(
        request_id,
        state.credentials.retry_cleanup(payload.id).await,
    )
}

#[tauri::command]
pub async fn connection_start(
    request: ApiRequest<ConnectionStartPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<ConnectionSnapshot, AppError> {
    let (request_id, payload) = request.validate()?;
    let result = match payload.source {
        maulink_core::ConnectionStartSource::Saved {
            server_id,
            expected_revision,
        } => {
            state
                .connections
                .start(server_id, expected_revision, payload.mode)
                .await
        }
        maulink_core::ConnectionStartSource::Draft {
            profile,
            credential,
        } => {
            if payload.mode != maulink_core::ConnectionMode::Test {
                return Err(AppError::new(
                    ErrorCode::ValidationFailed,
                    "errors.draftConnectionMustBeTestMode",
                )
                .with_request_id(request_id));
            }
            let token = profile.private_key_token.clone();
            let input = profile_input(profile, &state.local_files)
                .map_err(|error| error.with_request_id(request_id.clone()))?;
            let result = state.connections.start_draft_test(input, credential).await;
            if result.is_ok()
                && let Some(token) = token
            {
                state.local_files.forget(&token);
            }
            result
        }
    };
    attach_request_id(request_id, result)
}

#[tauri::command]
pub fn connection_get(
    request: ApiRequest<ConnectionIdPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<ConnectionSnapshot, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.connections.get(&payload.connection_id))
}

#[tauri::command]
pub fn connection_cancel(
    request: ApiRequest<ConnectionIdPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<ConnectionSnapshot, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.connections.cancel(&payload.connection_id))
}

#[tauri::command]
pub fn host_key_respond(
    request: ApiRequest<HostKeyRespondPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<(), AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(
        request_id,
        state.connections.respond_host_key(
            &payload.connection_id,
            &payload.challenge_id,
            payload.decision,
        ),
    )
}

#[tauri::command]
pub fn auth_respond(
    request: ApiRequest<AuthenticationRespondPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<(), AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(
        request_id,
        state.connections.respond_authentication(
            &payload.connection_id,
            &payload.challenge_id,
            payload.secret,
        ),
    )
}

#[tauri::command]
pub async fn connection_disconnect(
    request: ApiRequest<ConnectionDisconnectPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<ConnectionSnapshot, AppError> {
    let (request_id, payload) = request.validate()?;
    let active_transfers = state.sftp_transfers.active_count(&payload.connection_id);
    if active_transfers > 0 && !payload.stop_active_transfers {
        return Err(AppError::new(
            ErrorCode::TransferBusy,
            "errors.activeTransfersRequireConfirmation",
        )
        .with_param("count", active_transfers.to_string())
        .with_request_id(request_id));
    }
    state
        .sftp_transfers
        .close_connection(&payload.connection_id)
        .await;
    state.sftp.close_connection(&payload.connection_id).await;
    let result = state.connections.disconnect(&payload.connection_id).await;
    state.monitor.close_connection(&payload.connection_id).await;
    attach_request_id(request_id, result)
}

#[tauri::command]
pub async fn sftp_list_start(
    request: ApiRequest<SftpListStartPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<SftpDirectoryPage, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.sftp.list_start(payload).await)
}

#[tauri::command]
pub async fn sftp_list_next(
    request: ApiRequest<SftpCursorPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<SftpDirectoryPage, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.sftp.list_next(payload).await)
}

#[tauri::command]
pub async fn sftp_list_close(
    request: ApiRequest<SftpCursorPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<(), AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.sftp.list_close(payload).await)
}

#[tauri::command]
pub async fn sftp_stat(
    request: ApiRequest<SftpStatPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<RemoteFileEntry, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.sftp.stat(payload).await)
}

#[tauri::command]
pub async fn sftp_mkdir(
    request: ApiRequest<SftpMkdirPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<RemoteFileEntry, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.sftp.mkdir(payload).await)
}

#[tauri::command]
pub async fn sftp_rename(
    request: ApiRequest<SftpRenamePayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<RemoteFileEntry, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.sftp.rename(payload).await)
}

#[tauri::command]
pub async fn sftp_delete(
    request: ApiRequest<SftpDeletePayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<(), AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.sftp.delete(payload).await)
}

#[tauri::command]
pub async fn sftp_upload(
    request: ApiRequest<SftpUploadPayload>,
    output_channel: Channel<SftpTransferSnapshot>,
    state: tauri::State<'_, DesktopState>,
) -> Result<SftpTransferSnapshot, AppError> {
    let (request_id, payload) = request.validate()?;
    let (snapshot, updates) =
        attach_request_id(request_id, state.sftp_transfers.upload(payload).await)?;
    forward_transfer_updates(updates, output_channel);
    Ok(snapshot)
}

#[tauri::command]
pub async fn sftp_download(
    request: ApiRequest<SftpDownloadPayload>,
    output_channel: Channel<SftpTransferSnapshot>,
    state: tauri::State<'_, DesktopState>,
) -> Result<SftpTransferSnapshot, AppError> {
    let (request_id, payload) = request.validate()?;
    let (snapshot, updates) =
        attach_request_id(request_id, state.sftp_transfers.download(payload).await)?;
    forward_transfer_updates(updates, output_channel);
    Ok(snapshot)
}

#[tauri::command]
pub fn sftp_transfer_get(
    request: ApiRequest<SftpTransferIdPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<SftpTransferSnapshot, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.sftp_transfers.get(payload))
}

#[tauri::command]
pub fn sftp_transfer_list(
    request: ApiRequest<SftpTransferListPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<SftpTransferSnapshot>, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.sftp_transfers.list(payload))
}

#[tauri::command]
pub fn sftp_transfer_cancel(
    request: ApiRequest<SftpTransferIdPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<SftpTransferSnapshot, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.sftp_transfers.cancel(payload))
}

fn forward_transfer_updates(
    mut updates: tokio::sync::watch::Receiver<SftpTransferSnapshot>,
    output_channel: Channel<SftpTransferSnapshot>,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            let snapshot = updates.borrow_and_update().clone();
            if output_channel.send(snapshot.clone()).is_err() || snapshot.state.is_terminal() {
                break;
            }
            if updates.changed().await.is_err() {
                break;
            }
        }
    });
}

#[tauri::command]
pub async fn terminal_open(
    request: ApiRequest<TerminalOpenPayload>,
    output_channel: Channel<TerminalChunk>,
    state: tauri::State<'_, DesktopState>,
) -> Result<TerminalOpenResult, AppError> {
    let (request_id, payload) = request.validate()?;
    let (opened, mut output) = attach_request_id(request_id, state.terminals.open(payload).await)?;
    tauri::async_runtime::spawn(async move {
        while let Some(chunk) = output.recv().await {
            if output_channel.send(chunk).is_err() {
                break;
            }
        }
    });
    Ok(opened)
}

#[tauri::command]
pub fn terminal_get(
    request: ApiRequest<TerminalIdPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<TerminalSnapshot, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.terminals.get(&payload))
}

#[tauri::command]
pub async fn terminal_write(
    request: ApiRequest<TerminalWritePayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<TerminalWriteResult, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.terminals.write(payload).await)
}

#[tauri::command]
pub async fn terminal_resize(
    request: ApiRequest<TerminalResizePayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<TerminalSize, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.terminals.resize(payload).await)
}

#[tauri::command]
pub async fn terminal_ack(
    request: ApiRequest<TerminalAckPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<(), AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.terminals.ack(payload).await)
}

#[tauri::command]
pub async fn terminal_close(
    request: ApiRequest<TerminalIdPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<TerminalSnapshot, AppError> {
    let (request_id, payload) = request.validate()?;
    attach_request_id(request_id, state.terminals.close(&payload).await)
}

#[tauri::command]
pub fn settings_get(
    request: ApiRequest<EmptyPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<SettingsRecord, AppError> {
    let (_, _) = request.validate()?;
    Ok(state.settings.current())
}

#[tauri::command]
pub async fn settings_update(
    request: ApiRequest<SettingsUpdate>,
    app: AppHandle,
    state: tauri::State<'_, DesktopState>,
) -> Result<SettingsRecord, AppError> {
    let (request_id, payload) = request.validate()?;
    let result = async {
        let stored = state.settings.update(payload).await?;
        crate::app_icon::apply(&app, stored.value.app_icon_style).await?;
        Ok(stored)
    }
    .await;
    attach_request_id(request_id, result)
}

#[tauri::command]
pub async fn local_file_select(
    request: ApiRequest<LocalFileSelectPayload>,
    app: AppHandle,
    state: tauri::State<'_, DesktopState>,
) -> Result<Option<SelectedLocalFile>, AppError> {
    let (request_id, payload) = request.validate()?;
    let selected = match payload.purpose {
        LocalFilePurpose::PrivateKey | LocalFilePurpose::Upload => {
            app.dialog().file().blocking_pick_file()
        }
        LocalFilePurpose::Download => app.dialog().file().blocking_save_file(),
    };
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|_| {
        AppError::new(ErrorCode::ValidationFailed, "errors.localFilePathInvalid")
            .with_request_id(request_id.clone())
    })?;
    state
        .local_files
        .register(path, payload.purpose)
        .map(Some)
        .map_err(|error| error.with_request_id(request_id))
}

fn attach_request_id<T>(request_id: String, result: Result<T, AppError>) -> Result<T, AppError> {
    result.map_err(|error| error.with_request_id(request_id))
}

fn profile_input(
    profile: ServerProfileDraft,
    local_files: &maulink_core::LocalFileRegistry,
) -> Result<ServerProfileInput, AppError> {
    let private_key_path = profile
        .private_key_token
        .as_deref()
        .map(|token| local_files.resolve_private_key(token))
        .transpose()?;
    Ok(profile_input_with_path(profile, private_key_path))
}

fn profile_input_with_path(
    profile: ServerProfileDraft,
    private_key_path: Option<maulink_core::StoredPath>,
) -> ServerProfileInput {
    ServerProfileInput {
        name: profile.name,
        host: profile.host,
        port: profile.port,
        username: profile.username,
        auth_type: profile.auth_type,
        private_key_path,
        group_id: profile.group_id,
        connect_timeout_ms: profile.connect_timeout_ms,
        keepalive_interval_seconds: profile.keepalive_interval_seconds,
        jump_host: profile.jump_host,
        jump_port: profile.jump_port,
        proxy_type: profile.proxy_type,
        proxy_host: profile.proxy_host,
        proxy_port: profile.proxy_port,
    }
}
