use crate::config::fingerprint;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt, path::PathBuf};

#[derive(Debug, Serialize, Deserialize, Default)]
struct CacheFile {
    entries: BTreeMap<String, String>,
}

pub fn default_cache_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache")
        .join("crstop")
        .join("api-id.json")
}

pub fn load_api_id(root_url: &str, api_key: &str) -> Option<String> {
    let path = default_cache_path();
    let text = fs::read_to_string(path).ok()?;
    let cache: CacheFile = serde_json::from_str(&text).ok()?;
    cache.entries.get(&fingerprint(root_url, api_key)).cloned()
}

pub fn save_api_id(root_url: &str, api_key: &str, api_id: &str) -> Result<()> {
    let path = default_cache_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let mut cache = fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<CacheFile>(&text).ok())
        .unwrap_or_default();
    cache
        .entries
        .insert(fingerprint(root_url, api_key), api_id.to_string());
    fs::write(&path, serde_json::to_string_pretty(&cache)? + "\n")
        .with_context(|| format!("write {}", path.display()))?;
    let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    Ok(())
}
