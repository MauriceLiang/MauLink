mod app_icon;
mod background_images;
mod commands;
mod security;
mod server_appearance;
mod state;
mod updater;

use std::fs;

use commands::{
    app_get_info, auth_respond, background_image_delete, background_image_get,
    background_image_import, connection_cancel, connection_disconnect, connection_get,
    connection_preflight, connection_start, credential_cleanup_retry, credential_delete_retained,
    credential_list_retained, credential_reveal, group_create, group_delete, group_list,
    group_update, host_key_get, host_key_respond, local_file_select, monitor_get_history,
    monitor_get_snapshot, monitor_refresh, network_inspect, reveal_policy_change_password,
    reveal_policy_enable_direct, reveal_policy_enable_protected, reveal_policy_get,
    reveal_policy_recover, reveal_policy_set_deny, server_create, server_delete, server_get,
    server_list, server_runtime_stats_get, server_update, settings_get, settings_update,
    sftp_delete, sftp_download, sftp_list_close, sftp_list_next, sftp_list_start, sftp_mkdir,
    sftp_read_text, sftp_rename, sftp_stat, sftp_transfer_cancel, sftp_transfer_get,
    sftp_transfer_list, sftp_upload, sftp_write_text, sftp_write_text_with_sudo, terminal_ack,
    terminal_close, terminal_get, terminal_get_working_directory, terminal_open, terminal_resize,
    terminal_write, workspace_set_activity,
};
use maulink_core::{
    AppCapabilities, AppCore, AppInfo, ConnectionRegistry, CredentialCipher, CredentialManager,
    CredentialRevealService, CredentialWorker, Database, HostKeyStore, HostKeyVerifier,
    LocalFileRegistry, MonitorManager, NetworkInspector, ProfileStore, ServerAppearanceStore,
    ServerRuntimeStatsStore, SettingsService, SftpManager, SftpTransferManager,
    SshConnectionManager, SshConnector, TerminalManager,
};
use server_appearance::{server_appearance_get, server_appearance_list, server_appearance_update};
use state::DesktopState;
use tauri::Manager;
use updater::{updater_cancel, updater_check, updater_download, updater_install};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init());
    if updater::is_configured(context.config().plugins.0.get("updater")) {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }
    builder
        .setup(|app| {
            let app_data_directory = app.path().app_data_dir()?;
            fs::create_dir_all(&app_data_directory)?;
            let background_images = background_images::BackgroundImageStore::new(
                app_data_directory.join("terminal-backgrounds"),
            )?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&app_data_directory, fs::Permissions::from_mode(0o700))?;
            }

            let database = Database::open(app_data_directory.join("maulink.sqlite3"))?;
            let settings = tauri::async_runtime::block_on(SettingsService::load(database.clone()))?;
            app_icon::apply_on_main(app.handle(), settings.current().value.app_icon_style)?;
            let credential_worker = CredentialWorker::native()?;
            let credential_cipher =
                CredentialCipher::open(app_data_directory.join("credential-encryption.key"))?;
            let credentials =
                CredentialManager::new(database.clone(), credential_worker, credential_cipher);
            let credential_reveal = CredentialRevealService::new(
                database.clone(),
                credentials.clone(),
                security::NativeIdentityGate::new(app.handle().clone()),
            );
            if let Err(error) = tauri::async_runtime::block_on(credentials.recover_pending()) {
                eprintln!(
                    "MauLink deferred credential cleanup: {}",
                    error.code.as_str()
                );
            }
            let profiles = ProfileStore::new(database.clone());
            let server_appearance = ServerAppearanceStore::new(database.clone());
            let runtime_stats = ServerRuntimeStatsStore::new(database.clone());
            let connection_registry = ConnectionRegistry::default();
            let host_keys = HostKeyStore::new(database);
            let network = NetworkInspector::default();
            let host_key_verifier =
                HostKeyVerifier::new(host_keys.clone(), connection_registry.clone());
            let ssh_connector = SshConnector::new(connection_registry.clone(), host_key_verifier);
            let connections = SshConnectionManager::new_with_runtime_stats(
                profiles.clone(),
                credentials.clone(),
                connection_registry,
                ssh_connector,
                runtime_stats.clone(),
            );
            let sftp = SftpManager::new(connections.clone());
            let local_files = LocalFileRegistry::default();
            let sftp_transfers = SftpTransferManager::new(connections.clone(), local_files.clone());
            let terminals = TerminalManager::new(connections.clone());
            let monitor = MonitorManager::new(connections.clone());
            let core = AppCore::with_info(AppInfo::with_capabilities(
                AppCapabilities::monitor_backend(true),
            ));
            app.manage(DesktopState {
                core,
                profiles,
                server_appearance,
                runtime_stats,
                settings,
                updater: updater::UpdaterState::default(),
                local_files,
                credentials,
                credential_reveal,
                host_keys,
                network,
                connections,
                sftp,
                sftp_transfers,
                terminals,
                monitor,
                background_images,
                background_image_mutation: tokio::sync::Mutex::new(()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_get_info,
            group_list,
            group_create,
            group_update,
            group_delete,
            server_list,
            server_get,
            server_create,
            server_update,
            server_delete,
            server_appearance_list,
            server_appearance_get,
            server_appearance_update,
            server_runtime_stats_get,
            credential_list_retained,
            credential_delete_retained,
            credential_cleanup_retry,
            reveal_policy_get,
            reveal_policy_enable_protected,
            reveal_policy_enable_direct,
            reveal_policy_set_deny,
            reveal_policy_change_password,
            reveal_policy_recover,
            credential_reveal,
            connection_start,
            connection_get,
            connection_cancel,
            connection_preflight,
            host_key_respond,
            host_key_get,
            network_inspect,
            auth_respond,
            connection_disconnect,
            terminal_open,
            terminal_get,
            terminal_get_working_directory,
            terminal_write,
            terminal_resize,
            terminal_ack,
            terminal_close,
            sftp_list_start,
            sftp_list_next,
            sftp_list_close,
            sftp_stat,
            sftp_read_text,
            sftp_write_text,
            sftp_write_text_with_sudo,
            sftp_mkdir,
            sftp_rename,
            sftp_delete,
            sftp_upload,
            sftp_download,
            sftp_transfer_get,
            sftp_transfer_list,
            sftp_transfer_cancel,
            monitor_get_snapshot,
            monitor_get_history,
            monitor_refresh,
            workspace_set_activity,
            settings_get,
            settings_update,
            updater_check,
            updater_download,
            updater_install,
            updater_cancel,
            local_file_select,
            background_image_import,
            background_image_get,
            background_image_delete,
        ])
        .build(context)
        .expect("error while building MauLink")
        .run(|app_handle, event| match event {
            tauri::RunEvent::Ready => {
                eprintln!(
                    "MauLink ready: main window visible={:?}",
                    app_handle
                        .get_webview_window("main")
                        .map(|window| window.is_visible())
                );
            }
            tauri::RunEvent::Exit => {
                if let Some(state) = app_handle.try_state::<DesktopState>() {
                    let core = state.core.clone();
                    let terminals = state.terminals.clone();
                    let sftp = state.sftp.clone();
                    let sftp_transfers = state.sftp_transfers.clone();
                    let monitor = state.monitor.clone();
                    let connections = state.connections.clone();
                    tauri::async_runtime::block_on(async move {
                        core.shutdown(std::time::Duration::from_secs(3)).await;
                        monitor.shutdown().await;
                        terminals.shutdown().await;
                        sftp_transfers.shutdown().await;
                        sftp.shutdown().await;
                        connections.shutdown().await;
                    });
                }
            }
            tauri::RunEvent::WindowEvent {
                label,
                event: tauri::WindowEvent::Focused(_),
                ..
            } if label == "main" => {
                if let (Some(window), Some(state)) = (
                    app_handle.get_webview_window("main"),
                    app_handle.try_state::<DesktopState>(),
                ) {
                    let minimized = window.is_minimized().unwrap_or(false);
                    let monitor = state.monitor.clone();
                    tauri::async_runtime::spawn(async move {
                        monitor.set_window_minimized(minimized).await;
                    });
                }
            }
            _ => {}
        });
}
