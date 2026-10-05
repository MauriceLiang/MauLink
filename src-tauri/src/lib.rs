mod app_icon;
mod background_images;
mod commands;
mod state;

use std::fs;

use commands::{
    app_get_info, auth_respond, background_image_delete, background_image_get,
    background_image_import, connection_cancel, connection_disconnect, connection_get,
    connection_preflight, connection_start, credential_cleanup_retry, credential_delete_retained,
    credential_list_retained, group_create, group_delete, group_list, group_update, host_key_get,
    host_key_respond, local_file_select, monitor_get_history, monitor_get_snapshot,
    monitor_refresh, network_inspect, server_create, server_delete, server_get, server_list,
    server_update, settings_get, settings_update, sftp_delete, sftp_download, sftp_list_close,
    sftp_list_next, sftp_list_start, sftp_mkdir, sftp_read_text, sftp_rename, sftp_stat,
    sftp_transfer_cancel, sftp_transfer_get, sftp_transfer_list, sftp_upload, sftp_write_text,
    sftp_write_text_with_sudo, terminal_ack, terminal_close, terminal_get, terminal_open,
    terminal_resize, terminal_write, workspace_set_activity,
};
use maulink_core::{
    AppCapabilities, AppCore, AppInfo, ConnectionRegistry, CredentialManager, CredentialWorker,
    Database, HostKeyStore, HostKeyVerifier, LocalFileRegistry, MonitorManager, NetworkInspector,
    ProfileStore, SettingsService, SftpManager, SftpTransferManager, SshConnectionManager,
    SshConnector, TerminalManager,
};
use state::DesktopState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
            let credentials = CredentialManager::new(database.clone(), credential_worker);
            if let Err(error) = tauri::async_runtime::block_on(credentials.recover_pending()) {
                eprintln!(
                    "MauLink deferred credential cleanup: {}",
                    error.code.as_str()
                );
            }
            let profiles = ProfileStore::new(database.clone());
            let connection_registry = ConnectionRegistry::default();
            let host_keys = HostKeyStore::new(database);
            let network = NetworkInspector::default();
            let host_key_verifier =
                HostKeyVerifier::new(host_keys.clone(), connection_registry.clone());
            let ssh_connector = SshConnector::new(connection_registry.clone(), host_key_verifier);
            let connections = SshConnectionManager::new(
                profiles.clone(),
                credentials.clone(),
                connection_registry,
                ssh_connector,
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
                settings,
                local_files,
                credentials,
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
            credential_list_retained,
            credential_delete_retained,
            credential_cleanup_retry,
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
            local_file_select,
            background_image_import,
            background_image_get,
            background_image_delete,
        ])
        .build(tauri::generate_context!())
        .expect("error while building MauLink")
        .run(|app_handle, event| match event {
            tauri::RunEvent::Exit => {
                if let Some(state) = app_handle.try_state::<DesktopState>() {
                    let terminals = state.terminals.clone();
                    let sftp = state.sftp.clone();
                    let sftp_transfers = state.sftp_transfers.clone();
                    let monitor = state.monitor.clone();
                    let connections = state.connections.clone();
                    tauri::async_runtime::block_on(async move {
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
