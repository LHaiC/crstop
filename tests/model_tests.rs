use crstop::app::{build_status, status_exit_code};
use crstop::format::{compact_number, dollars, sorted_models};
use crstop::model::{Health, Limits, ModelStat, Snapshot, UsageTotal, UserStats};

fn healthy_snapshot() -> Snapshot {
    Snapshot {
        health: Health {
            status: "healthy".into(),
            service: Some("claude-relay-service".into()),
            version: Some("1.1.303".into()),
            redis_status: Some("healthy".into()),
        },
        user: UserStats {
            name: Some("demo-user".into()),
            is_active: true,
            is_activated: true,
            expires_at: None,
            permissions: vec!["openai".into()],
            total: UsageTotal {
                requests: 10,
                input_tokens: 20,
                output_tokens: 30,
                cache_read_tokens: 40,
                all_tokens: 90,
                cost: 1.25,
                formatted_cost: Some("$1.25".into()),
            },
            limits: Limits {
                daily_cost_limit: 0.0,
                total_cost_limit: 0.0,
                current_daily_cost: 2.5,
                current_total_cost: 9.5,
                concurrency_limit: 0,
                rate_limit_window: 0,
                rate_limit_requests: 0,
                rate_limit_cost: 0.0,
            },
        },
        daily: vec![],
        monthly: vec![],
        pool_visible: false,
        last_error: None,
        fetched_at: "now".into(),
    }
}

#[test]
fn status_ok_when_service_and_key_are_healthy() {
    let status = build_status(&healthy_snapshot());
    assert_eq!(status.level, "OK");
    assert_eq!(status_exit_code(&status.level), 0);
    assert!(
        status
            .messages
            .iter()
            .any(|m| m.contains("pool limits: not visible"))
    );
}

#[test]
fn status_warns_near_daily_limit() {
    let mut snap = healthy_snapshot();
    snap.user.limits.daily_cost_limit = 10.0;
    snap.user.limits.current_daily_cost = 9.1;
    let status = build_status(&snap);
    assert_eq!(status.level, "WARN");
    assert_eq!(status_exit_code(&status.level), 1);
}

#[test]
fn status_fails_for_inactive_key() {
    let mut snap = healthy_snapshot();
    snap.user.is_active = false;
    let status = build_status(&snap);
    assert_eq!(status.level, "FAIL");
    assert_eq!(status_exit_code(&status.level), 2);
}

#[test]
fn formats_numbers_for_tui() {
    assert_eq!(compact_number(1_234), "1.2K");
    assert_eq!(compact_number(1_234_567), "1.2M");
    assert_eq!(compact_number(1_234_567_890), "1.2B");
    assert_eq!(dollars(12.3), "$12.30");
}

#[test]
fn sorts_model_stats_by_cost_descending() {
    let rows = vec![
        ModelStat {
            model: "small".into(),
            requests: 1,
            input_tokens: 1,
            output_tokens: 1,
            cache_read_tokens: 1,
            all_tokens: 3,
            cost: 1.0,
            formatted_cost: None,
        },
        ModelStat {
            model: "big".into(),
            requests: 2,
            input_tokens: 2,
            output_tokens: 2,
            cache_read_tokens: 2,
            all_tokens: 6,
            cost: 9.0,
            formatted_cost: None,
        },
    ];
    let sorted = sorted_models(&rows);
    assert_eq!(sorted[0].model, "big");
}
