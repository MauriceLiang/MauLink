use std::sync::Arc;

use maulink_core::{
    AvailabilityFuture, IdentityFuture, Language, OsIdentityError, OsIdentityGate,
    OsIdentityPurpose,
};
use tauri::{AppHandle, Manager};

use crate::state::DesktopState;

pub struct NativeIdentityGate {
    app: AppHandle,
}

impl NativeIdentityGate {
    pub fn new(app: AppHandle) -> Arc<Self> {
        Arc::new(Self { app })
    }

    fn reason(&self, purpose: OsIdentityPurpose) -> &'static str {
        let language = self
            .app
            .try_state::<DesktopState>()
            .map(|state| state.settings.current().value.language)
            .unwrap_or_default();
        let lowering = matches!(
            purpose,
            OsIdentityPurpose::DisableProtection | OsIdentityPurpose::RecoverToDeny
        );
        match (language, lowering) {
            (Language::En, false) => "Authenticate to change saved credential viewing permissions.",
            (Language::En, true) => "Authenticate to change saved credential viewing protection.",
            (_, false) => "验证身份后更改已保存凭据的查看权限。",
            (_, true) => "验证身份后更改已保存凭据的查看保护设置。",
        }
    }
}

impl OsIdentityGate for NativeIdentityGate {
    fn is_available(&self) -> AvailabilityFuture<'_> {
        let app = self.app.clone();
        Box::pin(async move { platform::is_available(app).await })
    }

    fn verify(&self, purpose: OsIdentityPurpose) -> IdentityFuture<'_> {
        let app = self.app.clone();
        let reason = self.reason(purpose).to_owned();
        Box::pin(async move { platform::verify(app, reason).await })
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::{cell::RefCell, collections::HashMap, sync::Mutex, time::Duration};

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::Bool;
    use objc2_foundation::{NSError, NSString};
    use objc2_local_authentication::{LAContext, LAError, LAPolicy};
    use tokio::sync::oneshot;
    use uuid::Uuid;

    use super::*;

    thread_local! {
        static PENDING_CONTEXTS: RefCell<HashMap<Uuid, Retained<LAContext>>> = RefCell::new(HashMap::new());
    }

    pub async fn is_available(app: AppHandle) -> bool {
        let (sender, receiver) = oneshot::channel();
        if app
            .run_on_main_thread(move || {
                let context = unsafe { LAContext::new() };
                let available = unsafe {
                    context
                        .canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthentication)
                        .is_ok()
                };
                let _ = sender.send(available);
            })
            .is_err()
        {
            return false;
        }
        receiver.await.unwrap_or(false)
    }

    pub async fn verify(app: AppHandle, reason: String) -> Result<(), OsIdentityError> {
        let (sender, receiver) = oneshot::channel();
        let sender = Arc::new(Mutex::new(Some(sender)));
        let context_id = Uuid::new_v4();
        let app_for_main = app.clone();
        let sender_for_main = sender.clone();
        app.run_on_main_thread(move || {
            let context = unsafe { LAContext::new() };
            if unsafe { context.canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthentication) }
                .is_err()
            {
                if let Ok(mut sender) = sender_for_main.lock()
                    && let Some(sender) = sender.take()
                {
                    let _ = sender.send(Err(OsIdentityError::Unavailable));
                }
                return;
            }
            PENDING_CONTEXTS.with(|contexts| {
                contexts.borrow_mut().insert(context_id, context.clone());
            });
            let reason = NSString::from_str(&reason);
            let app_for_callback = app_for_main.clone();
            let sender_for_callback = sender_for_main.clone();
            let callback: RcBlock<dyn Fn(Bool, *mut NSError)> =
                RcBlock::new(move |success: Bool, error: *mut NSError| {
                    let result = if success.as_bool() {
                        Ok(())
                    } else {
                        // SAFETY: LocalAuthentication supplies a valid NSError pointer for the
                        // duration of this callback, or null when no error object is available.
                        let cancelled = unsafe { error.as_ref() }.is_some_and(|error| {
                            let code = LAError(error.code());
                            code == LAError::UserCancel
                                || code == LAError::UserFallback
                                || code == LAError::SystemCancel
                                || code == LAError::AppCancel
                        });
                        Err(if cancelled {
                            OsIdentityError::Cancelled
                        } else {
                            OsIdentityError::Failed
                        })
                    };
                    if let Ok(mut sender) = sender_for_callback.lock()
                        && let Some(sender) = sender.take()
                    {
                        let _ = sender.send(result);
                    }
                    let _ = app_for_callback.run_on_main_thread(move || {
                        PENDING_CONTEXTS.with(|contexts| {
                            contexts.borrow_mut().remove(&context_id);
                        });
                    });
                });
            // SAFETY: The reply block captures only Send values. The LAContext remains retained in
            // thread-local storage on the main thread until the callback schedules its removal.
            unsafe {
                context.evaluatePolicy_localizedReason_reply(
                    LAPolicy::DeviceOwnerAuthentication,
                    &reason,
                    &callback,
                );
            }
        })
        .map_err(|_| OsIdentityError::Unavailable)?;

        match tokio::time::timeout(Duration::from_secs(120), receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(OsIdentityError::Failed),
            Err(_) => {
                let _ = app.run_on_main_thread(move || {
                    PENDING_CONTEXTS.with(|contexts| {
                        contexts.borrow_mut().remove(&context_id);
                    });
                });
                Err(OsIdentityError::Failed)
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use tauri::Manager;
    use windows::Win32::System::WinRT::IUserConsentVerifierInterop;
    use windows::{
        Foundation::IAsyncOperation,
        Security::Credentials::UI::{
            UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
        },
        Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
        core::{HSTRING, factory},
    };

    use super::*;

    pub async fn is_available(app: AppHandle) -> bool {
        tokio::task::spawn_blocking(move || with_winrt(|| available_blocking(&app)))
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or(false)
    }

    pub async fn verify(app: AppHandle, reason: String) -> Result<(), OsIdentityError> {
        tokio::task::spawn_blocking(move || with_winrt(|| verify_blocking(&app, &reason)))
            .await
            .map_err(|_| OsIdentityError::Failed)?
    }

    fn with_winrt<T>(
        operation: impl FnOnce() -> Result<T, OsIdentityError>,
    ) -> Result<T, OsIdentityError> {
        unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|_| OsIdentityError::Unavailable)?;
        struct Uninitialize;
        impl Drop for Uninitialize {
            fn drop(&mut self) {
                unsafe { RoUninitialize() };
            }
        }
        let _uninitialize = Uninitialize;
        operation()
    }

    fn available_blocking(app: &AppHandle) -> Result<bool, OsIdentityError> {
        let has_window = app
            .get_webview_window("main")
            .and_then(|window| window.hwnd().ok())
            .is_some();
        if !has_window || factory::<UserConsentVerifier, IUserConsentVerifierInterop>().is_err() {
            return Ok(false);
        }
        let availability = tauri::async_runtime::block_on(async {
            UserConsentVerifier::CheckAvailabilityAsync()
                .map_err(|_| OsIdentityError::Unavailable)?
                .await
                .map_err(|_| OsIdentityError::Unavailable)
        })?;
        Ok(availability == UserConsentVerifierAvailability::Available)
    }

    fn verify_blocking(app: &AppHandle, reason: &str) -> Result<(), OsIdentityError> {
        let window = app
            .get_webview_window("main")
            .ok_or(OsIdentityError::Unavailable)?;
        let hwnd = window.hwnd().map_err(|_| OsIdentityError::Unavailable)?;
        let interop = factory::<UserConsentVerifier, IUserConsentVerifierInterop>()
            .map_err(|_| OsIdentityError::Unavailable)?;
        let message = HSTRING::from(reason);
        let operation: IAsyncOperation<UserConsentVerificationResult> =
            unsafe { interop.RequestVerificationForWindowAsync(hwnd, &message) }
                .map_err(|_| OsIdentityError::Unavailable)?;
        let result =
            tauri::async_runtime::block_on(operation).map_err(|_| OsIdentityError::Failed)?;
        match result {
            UserConsentVerificationResult::Verified => Ok(()),
            UserConsentVerificationResult::Canceled => Err(OsIdentityError::Cancelled),
            _ => Err(OsIdentityError::Failed),
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
    use super::*;

    pub async fn is_available(_app: AppHandle) -> bool {
        false
    }

    pub async fn verify(_app: AppHandle, _reason: String) -> Result<(), OsIdentityError> {
        Err(OsIdentityError::Unavailable)
    }
}
