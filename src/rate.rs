use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::Notify;

/// Global request rate, including retries. Waiters sleep until a token exists.
/// Termination: `acquire` returns when a token is granted or `cancel` is true.
pub struct RateGate {
    inner: Mutex<Bucket>,
    notify: Notify,
}

struct Bucket {
    rate: f64,
    tokens: f64,
    burst: f64,
    updated: tokio::time::Instant,
}

impl RateGate {
    pub fn new(rate: f64) -> Self {
        let burst = rate.max(1.0);
        Self {
            inner: Mutex::new(Bucket {
                rate,
                tokens: burst,
                burst,
                updated: tokio::time::Instant::now(),
            }),
            notify: Notify::new(),
        }
    }

    pub async fn acquire(&self, cancel: &tokio::sync::watch::Receiver<bool>) -> bool {
        loop {
            if *cancel.borrow() {
                return false;
            }
            let wait = {
                let mut bucket = self.inner.lock().unwrap_or_else(|e| e.into_inner());
                bucket.refill();
                if bucket.tokens >= 1.0 {
                    bucket.tokens -= 1.0;
                    return true;
                }
                bucket.wait_for_one()
            };
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = changed(cancel) => return false,
            }
            self.notify.notify_waiters();
        }
    }
}

impl Bucket {
    fn refill(&mut self) {
        let now = tokio::time::Instant::now();
        let elapsed = now.saturating_duration_since(self.updated).as_secs_f64();
        self.updated = now;
        self.tokens = (self.tokens + elapsed * self.rate).min(self.burst);
    }

    fn wait_for_one(&self) -> Duration {
        let missing = (1.0 - self.tokens).max(0.0);
        let secs = missing / self.rate.max(0.001);
        Duration::from_secs_f64(secs.clamp(0.001, 5.0))
    }
}

async fn changed(cancel: &tokio::sync::watch::Receiver<bool>) {
    let mut cancel = cancel.clone();
    loop {
        if *cancel.borrow() {
            return;
        }
        if cancel.changed().await.is_err() {
            return;
        }
    }
}
