use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub http_p50_us: u64,
    pub http_p95_us: u64,
    pub commit_p50_us: u64,
    pub commit_p95_us: u64,
    pub rps: f64,
    pub successes: u64,
    pub commits: u64,
}

pub struct Metrics {
    http: Hist,
    commit: Hist,
    rps: Mutex<Rps>,
    successes: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            http: Hist::default(),
            commit: Hist::default(),
            rps: Mutex::new(Rps {
                samples: VecDeque::new(),
            }),
            successes: AtomicU64::new(0),
        }
    }

    pub fn observe_http(&self, micros: u64, now_ms: i64) {
        self.http.observe(micros);
        let mut rps = self.rps.lock().unwrap_or_else(|e| e.into_inner());
        rps.samples.push_back(now_ms);
        let cutoff = now_ms.saturating_sub(5_000);
        while rps.samples.front().is_some_and(|t| *t < cutoff) {
            rps.samples.pop_front();
        }
    }

    pub fn observe_commit(&self, micros: u64) {
        self.commit.observe(micros);
    }

    pub fn success(&self) {
        self.successes.fetch_add(1, Ordering::Relaxed);
    }

    pub fn successes(&self) -> u64 {
        self.successes.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self, now_ms: i64) -> Snapshot {
        let rps = {
            let guard = self.rps.lock().unwrap_or_else(|e| e.into_inner());
            let cutoff = now_ms.saturating_sub(5_000);
            let n = guard.samples.iter().filter(|t| **t >= cutoff).count();
            n as f64 / 5.0
        };
        Snapshot {
            http_p50_us: self.http.quantile(0.50),
            http_p95_us: self.http.quantile(0.95),
            commit_p50_us: self.commit.quantile(0.50),
            commit_p95_us: self.commit.quantile(0.95),
            rps,
            successes: self.successes(),
            commits: self.commit.count(),
        }
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

struct Hist {
    buckets: [AtomicU64; 64],
    count: AtomicU64,
}

impl Default for Hist {
    fn default() -> Self {
        Self {
            buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            count: AtomicU64::new(0),
        }
    }
}

impl Hist {
    fn observe(&self, micros: u64) {
        let idx = micros.max(1).ilog2().min(63) as usize;
        self.buckets[idx].fetch_add(1, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    fn quantile(&self, q: f64) -> u64 {
        let total = self.count();
        if total == 0 {
            return 0;
        }
        let target = ((total as f64) * q).ceil().max(1.0) as u64;
        let mut seen = 0_u64;
        for (idx, bucket) in self.buckets.iter().enumerate() {
            seen += bucket.load(Ordering::Relaxed);
            if seen >= target {
                return 1_u64 << idx;
            }
        }
        0
    }
}

struct Rps {
    samples: VecDeque<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantile_tracks_observed_bucket() {
        let hist = Hist::default();
        hist.observe(1_000);
        hist.observe(1_000);
        hist.observe(1_000);
        hist.observe(50_000);
        assert!(hist.quantile(0.50) <= 2_048);
        assert!(hist.quantile(0.95) >= 32_768);
    }
}
