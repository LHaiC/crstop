use crate::model::{Snapshot, StatusResult};

pub fn build_status(snapshot: &Snapshot) -> StatusResult {
    let mut level = "OK".to_string();
    let mut messages = Vec::new();

    if snapshot.health.status == "healthy" {
        messages.push(format!(
            "service: healthy version={}",
            snapshot.health.version.as_deref().unwrap_or("-")
        ));
    } else {
        level = "FAIL".to_string();
        messages.push(format!("service: {}", snapshot.health.status));
    }

    match snapshot.health.redis_status.as_deref() {
        Some("healthy") => messages.push("redis: healthy".to_string()),
        Some(status) => {
            level = "FAIL".to_string();
            messages.push(format!("redis: {status}"));
        }
        None => messages.push("redis: unknown".to_string()),
    }

    if !snapshot.user.is_active {
        level = "FAIL".to_string();
        messages.push("key: inactive".to_string());
    } else if !snapshot.user.is_activated {
        level = "FAIL".to_string();
        messages.push("key: not activated".to_string());
    } else {
        messages.push("key: active".to_string());
    }

    if !snapshot.user.permissions.iter().any(|p| p == "openai") {
        if level == "OK" {
            level = "WARN".to_string();
        }
        messages.push(format!(
            "permissions: missing openai ({})",
            snapshot.user.permissions.join(",")
        ));
    } else {
        messages.push("permissions: openai".to_string());
    }

    check_cost_limit(
        "currentDailyCost",
        snapshot.user.limits.current_daily_cost,
        snapshot.user.limits.daily_cost_limit,
        &mut level,
        &mut messages,
    );
    check_cost_limit(
        "currentTotalCost",
        snapshot.user.limits.current_total_cost,
        snapshot.user.limits.total_cost_limit,
        &mut level,
        &mut messages,
    );

    if snapshot.pool_visible {
        messages.push("pool limits: visible".to_string());
    } else {
        messages.push("pool limits: not visible without admin token".to_string());
    }

    if let Some(err) = &snapshot.last_error {
        if level == "OK" {
            level = "WARN".to_string();
        }
        messages.push(format!("last error: {err}"));
    }

    StatusResult { level, messages }
}

fn check_cost_limit(
    name: &str,
    current: f64,
    limit: f64,
    level: &mut String,
    messages: &mut Vec<String>,
) {
    if limit <= 0.0 {
        messages.push(format!("{name}: ${current:.2} / unlimited"));
        return;
    }
    let ratio = current / limit;
    messages.push(format!("{name}: ${current:.2} / ${limit:.2}"));
    if ratio >= 1.0 {
        *level = "FAIL".to_string();
    } else if ratio >= 0.9 && level == "OK" {
        *level = "WARN".to_string();
    }
}

pub fn status_exit_code(level: &str) -> i32 {
    match level {
        "OK" => 0,
        "WARN" => 1,
        _ => 2,
    }
}
