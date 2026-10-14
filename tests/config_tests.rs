use crstop::config::{load_settings_from_path, mask_key, normalize_root_url};
use std::fs;

#[test]
fn parses_codex_config_and_normalizes_openai_url() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let auth_path = dir.path().join("auth.json");
    fs::write(
        &config_path,
        r#"
model_provider = "crs"
preferred_auth_method = "apikey"

[model_providers.crs]
base_url = "http://crs.example.invalid:6868/openai/"
name = "crs"
wire_api = "responses"
"#,
    )
    .unwrap();
    fs::write(
        &auth_path,
        r#"{"OPENAI_API_KEY":"test-api-key-for-unit-tests-abcdefghijklmnopqrstuvwxyz"}"#,
    )
    .unwrap();

    let settings = load_settings_from_path(&config_path).unwrap();

    assert_eq!(settings.root_url, "http://crs.example.invalid:6868");
    assert!(settings.api_key.starts_with("test-api"));
}

#[test]
fn loads_api_key_without_preferred_auth_method() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let auth_path = dir.path().join("auth.json");
    fs::write(
        &config_path,
        r#"
model_provider = "crs"

[model_providers.crs]
base_url = "http://crs.example.invalid:6868/openai/"
name = "crs"
wire_api = "responses"
"#,
    )
    .unwrap();
    fs::write(
        &auth_path,
        r#"{"OPENAI_API_KEY":"test-api-key-from-auth-json-abcdefghijklmnopqrstuvwxyz"}"#,
    )
    .unwrap();

    let settings = load_settings_from_path(&config_path).unwrap();

    assert_eq!(
        settings.api_key,
        "test-api-key-from-auth-json-abcdefghijklmnopqrstuvwxyz"
    );
}

#[test]
fn does_not_use_config_values_as_api_key() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
model_provider = "crs"
preferred_auth_method = "test-api-key-should-not-be-read-from-config"

[model_providers.crs]
base_url = "http://crs.example.invalid:6868/openai/"
"#,
    )
    .unwrap();

    let err = load_settings_from_path(&config_path)
        .unwrap_err()
        .to_string();

    assert!(err.contains("auth.json"));
    assert!(!err.contains("test-api-key"));
}

#[test]
fn normalizes_url_without_openai_suffix() {
    assert_eq!(
        normalize_root_url("http://host:6868/").unwrap(),
        "http://host:6868"
    );
    assert_eq!(
        normalize_root_url("http://host:6868/openai").unwrap(),
        "http://host:6868"
    );
}

#[test]
fn masks_api_key_without_leaking_middle() {
    assert_eq!(
        mask_key("test-api-key-for-unit-tests-abcdefghijklmnopqrstuvwxyz"),
        "test-ap...wxyz"
    );
}
