use crate::state::DesktopState;
use maulink_core::{
    ApiRequest, AppError, EmptyPayload, GeoIpDatabaseConfigure, GeoIpDatabaseImport,
    GeoIpDatabaseStatus,
};

#[tauri::command]
pub fn geoip_database_get(
    request: ApiRequest<EmptyPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<GeoIpDatabaseStatus, AppError> {
    let (request_id, _) = request.validate()?;
    state
        .geoip
        .status()
        .map_err(|error| error.with_request_id(request_id))
}
#[tauri::command]
pub async fn geoip_database_import(
    request: ApiRequest<GeoIpDatabaseImport>,
    state: tauri::State<'_, DesktopState>,
) -> Result<GeoIpDatabaseStatus, AppError> {
    let (request_id, payload) = request.validate()?;
    let path = state
        .local_files
        .consume_geoip_database_path(&payload.token)
        .map_err(|error| error.with_request_id(request_id.clone()))?;
    state
        .geoip
        .import(path)
        .await
        .map_err(|error| error.with_request_id(request_id))
}
#[tauri::command]
pub async fn geoip_database_delete(
    request: ApiRequest<EmptyPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<GeoIpDatabaseStatus, AppError> {
    let (request_id, _) = request.validate()?;
    state
        .geoip
        .delete()
        .await
        .map_err(|error| error.with_request_id(request_id))
}
#[tauri::command]
pub async fn geoip_database_configure(
    request: ApiRequest<GeoIpDatabaseConfigure>,
    state: tauri::State<'_, DesktopState>,
) -> Result<GeoIpDatabaseStatus, AppError> {
    let (request_id, payload) = request.validate()?;
    state
        .geoip
        .configure(payload.update_interval_days)
        .await
        .map_err(|error| error.with_request_id(request_id))
}
#[tauri::command]
pub async fn geoip_database_update(
    request: ApiRequest<EmptyPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<GeoIpDatabaseStatus, AppError> {
    let (request_id, _) = request.validate()?;
    state
        .geoip
        .update(false)
        .await
        .map_err(|error| error.with_request_id(request_id))
}
