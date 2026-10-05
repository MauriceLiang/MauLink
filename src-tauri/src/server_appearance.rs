use maulink_core::{
    ApiRequest, AppError, EmptyPayload, ServerAppearance, ServerAppearancePayload,
    ServerAppearanceUpdate,
};

use crate::state::DesktopState;

#[tauri::command]
pub async fn server_appearance_list(
    request: ApiRequest<EmptyPayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<ServerAppearance>, AppError> {
    let (request_id, _) = request.validate()?;
    state
        .server_appearance
        .list()
        .await
        .map_err(|error| error.with_request_id(request_id))
}

#[tauri::command]
pub async fn server_appearance_get(
    request: ApiRequest<ServerAppearancePayload>,
    state: tauri::State<'_, DesktopState>,
) -> Result<ServerAppearance, AppError> {
    let (request_id, payload) = request.validate()?;
    state
        .server_appearance
        .get(payload.server_id)
        .await
        .map_err(|error| error.with_request_id(request_id))
}

#[tauri::command]
pub async fn server_appearance_update(
    request: ApiRequest<ServerAppearanceUpdate>,
    state: tauri::State<'_, DesktopState>,
) -> Result<ServerAppearance, AppError> {
    let (request_id, payload) = request.validate()?;
    let result: Result<ServerAppearance, AppError> = async {
        let _guard = state.background_image_mutation.lock().await;
        let previous = state
            .server_appearance
            .get(payload.server_id.clone())
            .await?;
        if let Some(image_id) = payload
            .terminal_appearance
            .background_image
            .image_id
            .as_deref()
        {
            state.background_images.get(image_id)?;
        }
        let updated = state.server_appearance.update(payload).await?;
        if let Some(previous_image_id) = previous.terminal_appearance.background_image.image_id
            && updated
                .terminal_appearance
                .background_image
                .image_id
                .as_deref()
                != Some(previous_image_id.as_str())
        {
            let referenced_by_global = state
                .settings
                .current()
                .value
                .terminal_background_image
                .image_id
                .as_deref()
                == Some(previous_image_id.as_str());
            match state
                .server_appearance
                .references_background_image(previous_image_id.clone())
                .await
            {
                Ok(false) if !referenced_by_global => {
                    if let Err(error) = state.background_images.delete(&previous_image_id) {
                        eprintln!(
                            "MauLink deferred terminal background cleanup: {}",
                            error.code.as_str()
                        );
                    }
                }
                Ok(_) => {}
                Err(error) => eprintln!(
                    "MauLink deferred terminal background cleanup check: {}",
                    error.code.as_str()
                ),
            }
        }
        Ok(updated)
    }
    .await;
    result.map_err(|error| error.with_request_id(request_id))
}
