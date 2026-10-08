use maulink_core::{
    API_VERSION, AppError, AppInfo, AuthType, ConnectionPreflightError, ConnectionPreflightPayload,
    ConnectionPreflightResult, ErrorCode, ProxyType, ServerProfile, ServerProfileDraft,
};
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
        require_authentication: true,
        has_private_key: true,
        group_id: None,
        has_saved_credential: true,
        connect_timeout_ms: 15_000,
        keepalive_interval_seconds: 30,
        jump_host: None,
        jump_port: 22,
        proxy_type: None,
        proxy_host: None,
        proxy_port: None,
        revision: 1,
        created_at_ms: 1_795_000_000_000,
        updated_at_ms: 1_795_000_000_000,
    };
    let value = serde_json::to_value(profile).expect("serialize server profile");
    assert!(value["createdAtMs"].is_number());
    assert_eq!(value["hasPrivateKey"], true);
    assert_eq!(value["requireAuthentication"], true);
    assert!(value.get("privateKeyPath").is_none());
    assert_eq!(value["jumpPort"], 22);
    assert_eq!(value["proxyType"], serde_json::Value::Null);

    let declaration = ServerProfile::decl(&ts_rs::Config::new());
    assert!(declaration.contains("createdAtMs: number"));
    assert!(!declaration.contains("privateKeyPath"));
}

#[test]
fn server_profile_draft_accepts_legacy_and_advanced_proxy_contracts() {
    let base = serde_json::json!({
        "name": null,
        "host": "target.example.test",
        "port": 22,
        "username": "deploy",
        "authType": "password",
        "privateKeyToken": null,
        "groupId": null,
        "connectTimeoutMs": 15000,
        "keepaliveIntervalSeconds": 30
    });
    let legacy: ServerProfileDraft =
        serde_json::from_value(base.clone()).expect("legacy draft defaults advanced options");
    assert_eq!(legacy.jump_port, 22);
    assert_eq!(legacy.jump_host, None);
    assert_eq!(legacy.proxy_type, None);

    let mut advanced = base;
    advanced["jumpHost"] = serde_json::json!("bastion@bastion.example.test");
    advanced["jumpPort"] = serde_json::json!(2222);
    advanced["proxyType"] = serde_json::json!("httpConnect");
    advanced["proxyHost"] = serde_json::json!("proxy.example.test");
    advanced["proxyPort"] = serde_json::json!(8080);
    let advanced: ServerProfileDraft =
        serde_json::from_value(advanced).expect("deserialize advanced draft");
    assert_eq!(advanced.jump_port, 2222);
    assert_eq!(advanced.proxy_type, Some(ProxyType::HttpConnect));
    let serialized = serde_json::to_value(advanced).expect("serialize advanced draft");
    assert_eq!(serialized["proxyType"], "httpConnect");
}

#[test]
fn connection_preflight_contract_uses_camel_case_and_nullable_fields() {
    let payload = ConnectionPreflightPayload {
        server_id: "b83cba9d-aede-46fb-a1e4-689dc6e9d218".to_owned(),
        host: "example.test".to_owned(),
        port: 2222,
        timeout_ms: 10_000,
    };
    let payload = serde_json::to_value(payload).expect("serialize preflight payload");
    assert_eq!(payload["serverId"], "b83cba9d-aede-46fb-a1e4-689dc6e9d218");
    assert_eq!(payload["timeoutMs"], 10_000);

    let result = ConnectionPreflightResult {
        resolved_addresses: vec!["192.0.2.10".to_owned()],
        selected_address: None,
        dns_duration_ms: 12,
        tcp_reachable: Some(false),
        tcp_connect_duration_ms: Some(34),
        error: Some(ConnectionPreflightError::ConnectionRefused),
        checked_at_ms: 1_795_000_000_000,
    };
    let result = serde_json::to_value(result).expect("serialize preflight result");
    assert_eq!(result["resolvedAddresses"][0], "192.0.2.10");
    assert_eq!(result["selectedAddress"], serde_json::Value::Null);
    assert_eq!(result["tcpReachable"], false);
    assert_eq!(result["error"], "connectionRefused");
    assert!(result["checkedAtMs"].is_number());
}
