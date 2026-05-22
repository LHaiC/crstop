use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Health {
    pub status: String,
    pub service: Option<String>,
    pub version: Option<String>,
    pub redis_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageTotal {
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub all_tokens: u64,
    pub cost: f64,
    pub formatted_cost: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Limits {
    pub daily_cost_limit: f64,
    pub total_cost_limit: f64,
    pub current_daily_cost: f64,
    pub current_total_cost: f64,
    pub concurrency_limit: u64,
    pub rate_limit_window: u64,
    pub rate_limit_requests: u64,
    pub rate_limit_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserStats {
    pub name: Option<String>,
    pub is_active: bool,
    pub is_activated: bool,
    pub expires_at: Option<String>,
    pub permissions: Vec<String>,
    pub total: UsageTotal,
    pub limits: Limits,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelStat {
    pub model: String,
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub all_tokens: u64,
    pub cost: f64,
    pub formatted_cost: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub health: Health,
    pub user: UserStats,
    pub daily: Vec<ModelStat>,
    pub monthly: Vec<ModelStat>,
    pub pool_visible: bool,
    pub last_error: Option<String>,
    pub fetched_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusResult {
    pub level: String,
    pub messages: Vec<String>,
}
