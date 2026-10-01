use maulink_core::{AppError, AppIconStyle, ErrorCode};
use tauri::AppHandle;

fn icon_bytes(style: AppIconStyle) -> &'static [u8] {
    match style {
        AppIconStyle::Light => include_bytes!("../icons/128x128@2x.png"),
        AppIconStyle::Dark => include_bytes!("../icons/app-icon-dark.png"),
    }
}

fn apply_error() -> AppError {
    AppError::new(ErrorCode::Internal, "errors.appIconApplyFailed")
}

// AppKit calls must run on the application's main thread.
pub fn apply_on_main(app: &AppHandle, style: AppIconStyle) -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    {
        use objc2::{AllocAnyThread, MainThreadMarker};
        use objc2_app_kit::{NSApplication, NSImage};
        use objc2_foundation::NSData;
        let _ = app;
        let main = MainThreadMarker::new().ok_or_else(apply_error)?;
        let data = NSData::with_bytes(icon_bytes(style));
        let image = NSImage::initWithData(NSImage::alloc(), &data).ok_or_else(apply_error)?;
        // SAFETY: AppKit receives a valid, non-null NSImage on its main thread.
        unsafe { NSApplication::sharedApplication(main).setApplicationIconImage(Some(&image)) };
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        use tauri::Manager;
        let icon = tauri::image::Image::from_bytes(icon_bytes(style)).map_err(|_| apply_error())?;
        for window in app.webview_windows().values() {
            window.set_icon(icon.clone()).map_err(|_| apply_error())?;
        }
        Ok(())
    }
}

pub async fn apply(app: &AppHandle, style: AppIconStyle) -> Result<(), AppError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = sender.send(apply_on_main(&handle, style));
    })
    .map_err(|_| apply_error())?;
    receiver.await.map_err(|_| apply_error())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_variants_are_decodable_rgba_with_the_expected_backgrounds() {
        let light = tauri::image::Image::from_bytes(icon_bytes(AppIconStyle::Light)).unwrap();
        let dark = tauri::image::Image::from_bytes(icon_bytes(AppIconStyle::Dark)).unwrap();
        for icon in [&light, &dark] {
            assert_eq!((icon.width(), icon.height()), (256, 256));
            assert_eq!(icon.rgba().len(), 256 * 256 * 4);
            assert_eq!(icon.rgba()[3], 255);
        }
        assert!(light.rgba()[..3].iter().all(|value| *value > 200));
        assert!(dark.rgba()[..3].iter().all(|value| *value < 64));
        assert_ne!(light.rgba(), dark.rgba());
    }
}
