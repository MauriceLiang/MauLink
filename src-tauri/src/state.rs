use crate::background_images::BackgroundImageStore;
use maulink_core::{
    AppCore, CredentialManager, HostKeyStore, LocalFileRegistry, MonitorManager, NetworkInspector,
    ProfileStore, ServerAppearanceStore, ServerRuntimeStatsStore, SettingsService, SftpManager,
    SftpTransferManager, SshConnectionManager, TerminalManager,
};

pub struct DesktopState {
    pub core: AppCore,
    pub profiles: ProfileStore,
    pub server_appearance: ServerAppearanceStore,
    pub runtime_stats: ServerRuntimeStatsStore,
    pub settings: SettingsService,
    pub local_files: LocalFileRegistry,
    pub credentials: CredentialManager,
    pub host_keys: HostKeyStore,
    pub network: NetworkInspector,
    pub geoip: maulink_core::GeoIpDatabase,
    pub connections: SshConnectionManager,
    pub sftp: SftpManager,
    pub sftp_transfers: SftpTransferManager,
    pub terminals: TerminalManager,
    pub monitor: MonitorManager,
    pub background_images: BackgroundImageStore,
    pub background_image_mutation: tokio::sync::Mutex<()>,
}
