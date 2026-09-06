use crstop::{history::TrendHistory, model::Snapshot};
use std::time::{Duration, Instant};

fn sample_snapshot() -> Snapshot {
    serde_json::from_str(include_str!("fixtures/sample_snapshot.json")).unwrap()
}

#[test]
fn first_sample_has_no_per_refresh_delta() {
    let mut history = TrendHistory::new(4);
    history.push_snapshot(&sample_snapshot(), Instant::now());

    assert_eq!(history.len(), 1);
    assert!(history.request_deltas().is_empty());
    assert!(history.token_deltas().is_empty());
    assert!(history.cost_deltas().is_empty());
}

#[test]
fn computes_deltas_between_successive_snapshots() {
    let mut history = TrendHistory::new(4);
    let start = Instant::now();
    let mut first = sample_snapshot();
    let mut second = first.clone();
    second.user.total.requests += 7;
    second.user.total.all_tokens += 12_345;
    second.user.total.cost += 1.25;

    history.push_snapshot(&first, start);
    history.push_snapshot(&second, start + Duration::from_secs(5));

    assert_eq!(history.request_deltas(), vec![7]);
    assert_eq!(history.token_deltas(), vec![12_345]);
    assert_eq!(history.cost_deltas(), vec![125]);
    assert_eq!(history.latest_request_delta(), Some(7));
    assert_eq!(history.latest_token_delta(), Some(12_345));
    assert_eq!(history.latest_cost_delta_cents(), Some(125));

    first.user.total.requests += 100;
    assert_eq!(history.request_deltas(), vec![7]);
}

#[test]
fn clamps_counter_resets_to_zero_deltas() {
    let mut history = TrendHistory::new(4);
    let start = Instant::now();
    let first = sample_snapshot();
    let mut second = first.clone();
    second.user.total.requests = first.user.total.requests.saturating_sub(10);
    second.user.total.all_tokens = first.user.total.all_tokens.saturating_sub(10);
    second.user.total.cost = (first.user.total.cost - 1.0).max(0.0);

    history.push_snapshot(&first, start);
    history.push_snapshot(&second, start + Duration::from_secs(5));

    assert_eq!(history.request_deltas(), vec![0]);
    assert_eq!(history.token_deltas(), vec![0]);
    assert_eq!(history.cost_deltas(), vec![0]);
}

#[test]
fn keeps_only_configured_number_of_samples() {
    let mut history = TrendHistory::new(3);
    let start = Instant::now();

    for i in 0..5 {
        let mut snapshot = sample_snapshot();
        snapshot.user.total.requests += i;
        snapshot.user.total.all_tokens += i * 10;
        snapshot.user.total.cost += i as f64;
        history.push_snapshot(&snapshot, start + Duration::from_secs(i));
    }

    assert_eq!(history.len(), 3);
    assert_eq!(history.request_deltas(), vec![1, 1]);
    assert_eq!(history.token_deltas(), vec![10, 10]);
    assert_eq!(history.cost_deltas(), vec![100, 100]);
}

#[test]
fn buckets_long_history_into_visible_window_totals() {
    let mut history = TrendHistory::new(8);
    let start = Instant::now();
    let mut snapshot = sample_snapshot();
    history.push_snapshot(&snapshot, start);

    for i in 1..=6 {
        snapshot.user.total.requests += i;
        snapshot.user.total.all_tokens += i * 10;
        snapshot.user.total.cost += i as f64 / 100.0;
        history.push_snapshot(&snapshot, start + Duration::from_secs(i));
    }

    assert_eq!(history.request_buckets(3), vec![3, 7, 11]);
    assert_eq!(history.token_buckets(3), vec![30, 70, 110]);
    assert_eq!(history.cost_buckets(3), vec![3, 7, 11]);
    assert_eq!(history.window_request_delta(), 21);
    assert_eq!(history.window_token_delta(), 210);
    assert_eq!(history.window_cost_delta_cents(), 21);
}

#[test]
fn time_buckets_include_in_progress_bucket() {
    let mut history = TrendHistory::new(8);
    let start = Instant::now();
    let mut snapshot = sample_snapshot();
    history.push_snapshot(&snapshot, start);

    snapshot.user.total.requests += 4;
    snapshot.user.total.all_tokens += 40;
    snapshot.user.total.cost += 0.04;
    history.push_snapshot(&snapshot, start + Duration::from_secs(2));

    let idle = snapshot.clone();
    history.push_snapshot(&idle, start + Duration::from_secs(29));

    assert_eq!(
        history.request_time_buckets(4, Duration::from_secs(30)),
        vec![0, 0, 0, 4]
    );

    history.push_snapshot(&idle, start + Duration::from_secs(31));

    assert_eq!(
        history.request_time_buckets(4, Duration::from_secs(30)),
        vec![0, 0, 4, 0]
    );
    assert_eq!(
        history.token_time_buckets(4, Duration::from_secs(30)),
        vec![0, 0, 40, 0]
    );
    assert_eq!(
        history.cost_time_buckets(4, Duration::from_secs(30)),
        vec![0, 0, 4, 0]
    );
}

#[test]
fn time_buckets_fill_idle_buckets_with_zeroes() {
    let mut history = TrendHistory::new(8);
    let start = Instant::now();
    let mut snapshot = sample_snapshot();
    history.push_snapshot(&snapshot, start);

    snapshot.user.total.requests += 4;
    history.push_snapshot(&snapshot, start + Duration::from_secs(2));
    history.push_snapshot(&snapshot, start + Duration::from_secs(62));

    assert_eq!(
        history.request_time_buckets(4, Duration::from_secs(30)),
        vec![0, 4, 0, 0]
    );
}

#[test]
fn time_buckets_keep_fixed_bucket_membership_during_idle_refreshes() {
    let mut history = TrendHistory::new(8);
    let start = Instant::now();
    let mut snapshot = sample_snapshot();
    history.push_snapshot(&snapshot, start);

    snapshot.user.total.requests += 4;
    history.push_snapshot(&snapshot, start + Duration::from_secs(2));
    snapshot.user.total.requests += 3;
    history.push_snapshot(&snapshot, start + Duration::from_secs(25));
    history.push_snapshot(&snapshot, start + Duration::from_secs(33));

    assert_eq!(
        history.request_time_buckets(4, Duration::from_secs(30)),
        vec![0, 0, 7, 0]
    );
}

#[test]
fn time_buckets_include_current_in_progress_bucket() {
    let mut history = TrendHistory::new(8);
    let start = Instant::now();
    let mut snapshot = sample_snapshot();
    history.push_snapshot(&snapshot, start);

    snapshot.user.total.requests += 4;
    history.push_snapshot(&snapshot, start + Duration::from_secs(2));
    snapshot.user.total.requests += 3;
    history.push_snapshot(&snapshot, start + Duration::from_secs(32));
    history.push_snapshot(&snapshot, start + Duration::from_secs(40));

    assert_eq!(
        history.request_time_buckets(4, Duration::from_secs(30)),
        vec![0, 0, 4, 3]
    );
}

#[test]
fn time_buckets_preserve_time_gaps_between_bars() {
    let mut history = TrendHistory::new(8);
    let start = Instant::now();
    let mut snapshot = sample_snapshot();
    history.push_snapshot(&snapshot, start);

    snapshot.user.total.requests += 4;
    history.push_snapshot(&snapshot, start + Duration::from_secs(2));
    snapshot.user.total.requests += 3;
    history.push_snapshot(&snapshot, start + Duration::from_secs(95));

    assert_eq!(
        history.request_time_buckets(4, Duration::from_secs(30)),
        vec![4, 0, 0, 3]
    );
}

#[test]
fn time_buckets_keep_stable_boundaries_after_sample_eviction() {
    let mut history = TrendHistory::new(3);
    let start = Instant::now();
    let mut snapshot = sample_snapshot();
    history.push_snapshot(&snapshot, start);

    for (offset, bump) in [(31, 7), (62, 11), (93, 13), (124, 17)] {
        snapshot.user.total.requests += bump;
        history.push_snapshot(&snapshot, start + Duration::from_secs(offset));
    }

    assert_eq!(
        history.request_time_buckets(4, Duration::from_secs(30)),
        vec![0, 0, 13, 17]
    );
}
