use serde_json::Value;

pub fn is_configured(value: Option<&Value>) -> bool {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return false;
    };
    match serde_json::from_value::<tauri_plugin_updater::Config>(value.clone()) {
        Ok(config) => !config.pubkey.trim().is_empty() && !config.endpoints.is_empty(),
        Err(error) => {
            eprintln!("MauLink updater disabled: invalid configuration: {error}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_without_update_configuration_keep_the_plugin_disabled() {
        assert!(!is_configured(None));
        assert!(!is_configured(Some(&Value::Null)));
        assert!(!is_configured(Some(&json!({}))));
    }

    #[test]
    fn incomplete_or_invalid_update_configuration_keeps_the_plugin_disabled() {
        for config in [
            json!({ "pubkey": "fixture-public-key" }),
            json!({ "pubkey": " ", "endpoints": ["https://updates.example.com/latest.json"] }),
            json!({ "pubkey": "fixture-public-key", "endpoints": ["not-a-url"] }),
        ] {
            assert!(!is_configured(Some(&config)));
        }
    }

    #[test]
    fn complete_update_configuration_enables_the_plugin() {
        let config = json!({
            "pubkey": "fixture-public-key",
            "endpoints": ["https://updates.example.com/latest.json"]
        });
        assert!(is_configured(Some(&config)));
    }
}
