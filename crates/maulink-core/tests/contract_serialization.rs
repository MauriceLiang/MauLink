use maulink_core::{API_VERSION, AppError, AppInfo, AuthType, ErrorCode, ServerProfile};
use ts_rs::TS;

#[test]
fn app_info_contract_fixture_is_stable() {
    let value = serde_json::to_value(AppInfo::current()).expect("serialize app info");
    assert_eq!(value["name"], "MauLink");
    assert_eq!(value["apiVersion"], API_VERSION);
    assert_eq!(value["capabilities"]["ssh"], false);
}

#[test]
fn app_error_code_is_screaming_snake_case() {
    let value = serde_json::to_value(AppError::new(
        ErrorCode::IpcVersionUnsupported,
        "errors.ipcVersionUnsupported",
    ))
    .expect("serialize app error");
    assert_eq!(value["code"], "IPC_VERSION_UNSUPPORTED");
    assert_eq!(value["messageKey"], "errors.ipcVersionUnsupported");
}

#[test]
fn server_profile_contract_uses_safe_timestamps_and_hides_private_key_path() {
    let profile = ServerProfile {
        id: "b83cba9d-aede-46fb-a1e4-689dc6e9d218".to_owned(),
        name: "deploy@example.test".to_owned(),
        host: "example.test".to_owned(),
        port: 22,
        username: "deploy".to_owned(),
        auth_type: AuthType::PrivateKey,
        has_private_key: true,
        group_id: None,
        has_saved_credential: true,
        connect_timeout_ms: 15_000,
        keepalive_interval_seconds: 30,
        revision: 1,
        created_at_ms: 1_795_000_000_000,
        updated_at_ms: 1_795_000_000_000,
    };
    let value = serde_json::to_value(profile).expect("serialize server profile");
    assert!(value["createdAtMs"].is_number());
    assert_eq!(value["hasPrivateKey"], true);
    assert!(value.get("privateKeyPath").is_none());

    let declaration = ServerProfile::decl(&ts_rs::Config::new());
    assert!(declaration.contains("createdAtMs: number"));
    assert!(!declaration.contains("privateKeyPath"));
}
