use crate::model::Snapshot;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
struct TrendSample {
    fetched_at: Instant,
    requests: u64,
    all_tokens: u64,
    cost_cents: u64,
}

#[derive(Debug, Clone)]
pub struct TrendHistory {
    max_samples: usize,
    samples: VecDeque<TrendSample>,
}

impl TrendHistory {
    pub fn new(max_samples: usize) -> Self {
        Self {
            max_samples: max_samples.max(2),
            samples: VecDeque::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn push_snapshot(&mut self, snapshot: &Snapshot, fetched_at: Instant) {
        self.samples.push_back(TrendSample {
            fetched_at,
            requests: snapshot.user.total.requests,
            all_tokens: snapshot.user.total.all_tokens,
            cost_cents: dollars_to_cents(snapshot.user.total.cost),
        });
        while self.samples.len() > self.max_samples {
            self.samples.pop_front();
        }
    }

    pub fn request_deltas(&self) -> Vec<u64> {
        self.deltas(|sample| sample.requests)
    }

    pub fn token_deltas(&self) -> Vec<u64> {
        self.deltas(|sample| sample.all_tokens)
    }

    pub fn cost_deltas(&self) -> Vec<u64> {
        self.deltas(|sample| sample.cost_cents)
    }

    pub fn request_buckets(&self, max_bars: usize) -> Vec<u64> {
        bucket_values(&self.request_deltas(), max_bars)
    }

    pub fn token_buckets(&self, max_bars: usize) -> Vec<u64> {
        bucket_values(&self.token_deltas(), max_bars)
    }

    pub fn cost_buckets(&self, max_bars: usize) -> Vec<u64> {
        bucket_values(&self.cost_deltas(), max_bars)
    }

    pub fn window_request_delta(&self) -> u64 {
        self.request_deltas().into_iter().sum()
    }

    pub fn window_token_delta(&self) -> u64 {
        self.token_deltas().into_iter().sum()
    }

    pub fn window_cost_delta_cents(&self) -> u64 {
        self.cost_deltas().into_iter().sum()
    }

    pub fn request_time_buckets(&self, max_bars: usize, bucket: Duration) -> Vec<u64> {
        self.time_buckets(max_bars, bucket, |sample| sample.requests)
    }

    pub fn token_time_buckets(&self, max_bars: usize, bucket: Duration) -> Vec<u64> {
        self.time_buckets(max_bars, bucket, |sample| sample.all_tokens)
    }

    pub fn cost_time_buckets(&self, max_bars: usize, bucket: Duration) -> Vec<u64> {
        self.time_buckets(max_bars, bucket, |sample| sample.cost_cents)
    }

    pub fn latest_request_delta(&self) -> Option<u64> {
        self.latest_delta(|sample| sample.requests)
    }

    pub fn latest_token_delta(&self) -> Option<u64> {
        self.latest_delta(|sample| sample.all_tokens)
    }

    pub fn latest_cost_delta_cents(&self) -> Option<u64> {
        self.latest_delta(|sample| sample.cost_cents)
    }

    pub fn latest_interval(&self) -> Option<Duration> {
        let latest = self.samples.back()?;
        let previous = self.samples.iter().rev().nth(1)?;
        Some(
            latest
                .fetched_at
                .saturating_duration_since(previous.fetched_at),
        )
    }

    fn deltas(&self, value: impl Fn(&TrendSample) -> u64) -> Vec<u64> {
        self.samples
            .iter()
            .zip(self.samples.iter().skip(1))
            .map(|(previous, current)| value(current).saturating_sub(value(previous)))
            .collect()
    }

    fn latest_delta(&self, value: impl Fn(&TrendSample) -> u64) -> Option<u64> {
        let latest = self.samples.back()?;
        let previous = self.samples.iter().rev().nth(1)?;
        Some(value(latest).saturating_sub(value(previous)))
    }

    fn time_buckets(
        &self,
        max_bars: usize,
        bucket: Duration,
        value: impl Fn(&TrendSample) -> u64,
    ) -> Vec<u64> {
        if max_bars == 0 || bucket.is_zero() || self.samples.len() < 2 {
            return Vec::new();
        }
        let latest = self.samples.back().expect("len checked").fetched_at;
        let bucket_secs = bucket.as_secs_f64();
        let mut buckets = vec![0; max_bars];
        for (previous, current) in self.samples.iter().zip(self.samples.iter().skip(1)) {
            let seconds_ago = latest
                .saturating_duration_since(current.fetched_at)
                .as_secs_f64();
            let buckets_from_right = (seconds_ago / bucket_secs).floor() as usize;
            if buckets_from_right >= max_bars {
                continue;
            }
            let index = max_bars - 1 - buckets_from_right;
            buckets[index] += value(current).saturating_sub(value(previous));
        }
        match buckets.iter().position(|value| *value > 0) {
            Some(first_nonzero) => buckets[first_nonzero..].to_vec(),
            None => vec![0],
        }
    }
}

fn dollars_to_cents(value: f64) -> u64 {
    if value.is_finite() && value > 0.0 {
        (value * 100.0).round() as u64
    } else {
        0
    }
}

fn bucket_values(values: &[u64], max_bars: usize) -> Vec<u64> {
    if max_bars == 0 || values.is_empty() {
        return Vec::new();
    }
    if values.len() <= max_bars {
        return values.to_vec();
    }
    let chunk_size = values.len().div_ceil(max_bars);
    values
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().copied().sum())
        .collect()
}
