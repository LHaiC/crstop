use crstop::config::{load_settings_from_path, mask_key, normalize_root_url};
use std::fs;

#[test]
fn parses_codex_config_and_normalizes_openai_url() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(
        &path,
        r#"
model_provider = "crs"
preferred_auth_method = "test-api-key-for-unit-tests-abcdefghijklmnopqrstuvwxyz"

[model_providers.crs]
base_url = "http://crs.example.invalid:6868/openai/"
name = "crs"
wire_api = "responses"
"#,
    )
    .unwrap();

    let settings = load_settings_from_path(&path).unwrap();

    assert_eq!(settings.provider, "crs");
    assert_eq!(settings.root_url, "http://crs.example.invalid:6868");
    assert_eq!(
        settings.raw_base_url,
        "http://crs.example.invalid:6868/openai/"
    );
    assert!(settings.api_key.starts_with("test-api"));
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
