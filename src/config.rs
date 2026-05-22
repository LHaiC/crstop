use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub config_path: PathBuf,
    pub provider: String,
    pub root_url: String,
    pub raw_base_url: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
struct CodexConfig {
    model_provider: Option<String>,
    preferred_auth_method: Option<String>,
    model_providers: Option<HashMap<String, ProviderConfig>>,
}

#[derive(Debug, Deserialize)]
struct ProviderConfig {
    base_url: Option<String>,
}

pub fn default_config_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("config.toml")
}

pub fn load_settings_from_path(path: &Path) -> Result<Settings> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let cfg: CodexConfig = toml::from_str(&text).context("parse Codex config TOML")?;
    let provider = cfg
        .model_provider
        .ok_or_else(|| anyhow!("Codex config missing model_provider"))?;
    let providers = cfg
        .model_providers
        .ok_or_else(|| anyhow!("Codex config missing model_providers"))?;
    let provider_cfg = providers
        .get(&provider)
        .ok_or_else(|| anyhow!("Codex config missing [model_providers.{provider}]"))?;
    let raw_base_url = provider_cfg
        .base_url
        .clone()
        .ok_or_else(|| anyhow!("[model_providers.{provider}] missing base_url"))?;
    let api_key = cfg
        .preferred_auth_method
        .ok_or_else(|| anyhow!("Codex config missing preferred_auth_method"))?;
    Ok(Settings {
        config_path: path.to_path_buf(),
        provider,
        root_url: normalize_root_url(&raw_base_url)?,
        raw_base_url,
        api_key,
    })
}

pub fn load_settings(path: Option<PathBuf>) -> Result<Settings> {
    load_settings_from_path(&path.unwrap_or_else(default_config_path))
}

pub fn normalize_root_url(input: &str) -> Result<String> {
    let mut url = input.trim().trim_end_matches('/').to_string();
    if url.ends_with("/openai") {
        url.truncate(url.len() - "/openai".len());
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(anyhow!("invalid CRS base_url: {input}"));
    }
    Ok(url.trim_end_matches('/').to_string())
}

pub fn mask_key(key: &str) -> String {
    if key.len() <= 12 {
        return "***".to_string();
    }
    format!("{}...{}", &key[..7], &key[key.len() - 4..])
}

pub fn fingerprint(root_url: &str, api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(root_url.as_bytes());
    hasher.update([0]);
    hasher.update(api_key.as_bytes());
    format!("{:x}", hasher.finalize())
}
