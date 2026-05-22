use crate::{
    cache,
    config::Settings,
    model::{Health, Limits, ModelStat, Snapshot, UsageTotal, UserStats},
};
use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct CrsClient {
    root_url: String,
}

impl CrsClient {
    pub fn new(root_url: String) -> Self {
        Self {
            root_url: root_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn snapshot(&self, settings: &Settings, no_cache: bool) -> Result<Snapshot> {
        let api_id = if no_cache {
            None
        } else {
            cache::load_api_id(&settings.root_url, &settings.api_key)
        }
        .map(Ok)
        .unwrap_or_else(|| {
            let id = self.get_api_id(&settings.api_key)?;
            if !no_cache {
                let _ = cache::save_api_id(&settings.root_url, &settings.api_key, &id);
            }
            Ok::<_, anyhow::Error>(id)
        })?;
        let health = self.health()?;
        let user = self.user_stats(&api_id)?;
        let daily = self.model_stats(&api_id, "daily")?;
        let monthly = self.model_stats(&api_id, "monthly")?;
        Ok(Snapshot {
            health,
            user,
            daily,
            monthly,
            pool_visible: std::env::var_os("CRS_ADMIN_TOKEN").is_some(),
            last_error: None,
            fetched_at: now_string(),
        })
    }

    pub fn health(&self) -> Result<Health> {
        let v = self.get_json("/health")?;
        Ok(Health {
            status: v
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            service: v.get("service").and_then(Value::as_str).map(str::to_string),
            version: v.get("version").and_then(Value::as_str).map(str::to_string),
            redis_status: v
                .pointer("/components/redis/status")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }

    pub fn get_api_id(&self, api_key: &str) -> Result<String> {
        let v = self.post_json("/apiStats/api/get-key-id", &json!({ "apiKey": api_key }))?;
        v.pointer("/data/id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| anyhow!("get-key-id response missing data.id"))
    }

    pub fn user_stats(&self, api_id: &str) -> Result<UserStats> {
        let v = self.post_json("/apiStats/api/user-stats", &json!({ "apiId": api_id }))?;
        let data = v
            .get("data")
            .ok_or_else(|| anyhow!("user-stats response missing data"))?;
        parse_user_stats(data)
    }

    pub fn model_stats(&self, api_id: &str, period: &str) -> Result<Vec<ModelStat>> {
        let v = self.post_json(
            "/apiStats/api/user-model-stats",
            &json!({ "apiId": api_id, "period": period }),
        )?;
        let data = v
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("model-stats response missing data array"))?;
        data.iter().map(parse_model_stat).collect()
    }

    fn get_json(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.root_url, path);
        let mut response = ureq::get(&url)
            .call()
            .with_context(|| format!("GET {url}"))?;
        let text = response
            .body_mut()
            .read_to_string()
            .with_context(|| format!("read {url}"))?;
        parse_success_json(&text, &url)
    }

    fn post_json(&self, path: &str, body: &Value) -> Result<Value> {
        let url = format!("{}{}", self.root_url, path);
        let mut response = ureq::post(&url)
            .header("Content-Type", "application/json")
            .send_json(body)
            .with_context(|| format!("POST {url}"))?;
        let text = response
            .body_mut()
            .read_to_string()
            .with_context(|| format!("read {url}"))?;
        parse_success_json(&text, &url)
    }
}

fn parse_success_json(text: &str, url: &str) -> Result<Value> {
    let v: Value = serde_json::from_str(text).with_context(|| format!("parse JSON from {url}"))?;
    if v.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(anyhow!(
            v.get("message")
                .or_else(|| v.get("error"))
                .and_then(Value::as_str)
                .unwrap_or("CRS request failed")
                .to_string()
        ));
    }
    Ok(v)
}

fn parse_user_stats(data: &Value) -> Result<UserStats> {
    let total = data
        .pointer("/usage/total")
        .ok_or_else(|| anyhow!("usage.total missing"))?;
    let limits = data.get("limits").unwrap_or(&Value::Null);
    Ok(UserStats {
        name: data.get("name").and_then(Value::as_str).map(str::to_string),
        is_active: data
            .get("isActive")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        is_activated: data
            .get("isActivated")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        expires_at: data
            .get("expiresAt")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        permissions: parse_permissions(data.get("permissions")),
        total: UsageTotal {
            requests: u64_at(total, "requests"),
            input_tokens: u64_at(total, "inputTokens"),
            output_tokens: u64_at(total, "outputTokens"),
            cache_read_tokens: u64_at(total, "cacheReadTokens"),
            all_tokens: u64_at(total, "allTokens").max(u64_at(total, "tokens")),
            cost: f64_at(total, "cost"),
            formatted_cost: total
                .get("formattedCost")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        limits: Limits {
            daily_cost_limit: f64_at(limits, "dailyCostLimit"),
            total_cost_limit: f64_at(limits, "totalCostLimit"),
            current_daily_cost: f64_at(limits, "currentDailyCost"),
            current_total_cost: f64_at(limits, "currentTotalCost"),
            concurrency_limit: u64_at(limits, "concurrencyLimit"),
            rate_limit_window: u64_at(limits, "rateLimitWindow"),
            rate_limit_requests: u64_at(limits, "rateLimitRequests"),
            rate_limit_cost: f64_at(limits, "rateLimitCost"),
        },
    })
}

fn parse_model_stat(v: &Value) -> Result<ModelStat> {
    Ok(ModelStat {
        model: v
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("-")
            .to_string(),
        requests: u64_at(v, "requests"),
        input_tokens: u64_at(v, "inputTokens"),
        output_tokens: u64_at(v, "outputTokens"),
        cache_read_tokens: u64_at(v, "cacheReadTokens"),
        all_tokens: u64_at(v, "allTokens"),
        cost: v
            .pointer("/costs/total")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        formatted_cost: v
            .pointer("/formatted/total")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn parse_permissions(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::Array(xs)) => xs
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        Some(Value::String(s)) => {
            serde_json::from_str::<Vec<String>>(s).unwrap_or_else(|_| vec![s.clone()])
        }
        _ => Vec::new(),
    }
}

fn u64_at(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn f64_at(v: &Value, key: &str) -> f64 {
    v.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn now_string() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}
