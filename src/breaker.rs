use std::sync::Mutex;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Clone, Debug)]
pub struct BreakerView {
    pub phase: Phase,
    pub limit: u32,
    pub wait: Option<Duration>,
    pub reason: Option<String>,
}

struct Inner {
    phase: Phase,
    failures: u32,
    threshold: u32,
    open_for: Duration,
    open_until: Option<tokio::time::Instant>,
    ramp: u32,
    max_in_flight: u32,
    half_open_used: bool,
    successes_since_ramp: u32,
}

/// Failure breaker. After it closes, `limit` starts at 1 and doubles every 8 successes.
/// Termination: no task. Callers poll `view`.
pub struct Breaker {
    inner: Mutex<Inner>,
}

impl Breaker {
    pub fn new(threshold: u32, open_for: Duration, max_in_flight: u32) -> Self {
        Self {
            inner: Mutex::new(Inner {
                phase: Phase::Closed,
                failures: 0,
                threshold: threshold.max(1),
                open_for,
                open_until: None,
                ramp: max_in_flight.max(1),
                max_in_flight: max_in_flight.max(1),
                half_open_used: false,
                successes_since_ramp: 0,
            }),
        }
    }

    pub fn view(&self) -> BreakerView {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.refresh();
        let wait = guard.open_until.and_then(|until| {
            let now = tokio::time::Instant::now();
            if until > now {
                Some(until.saturating_duration_since(now))
            } else {
                None
            }
        });
        BreakerView {
            phase: guard.phase,
            limit: guard.ramp,
            wait,
            reason: match guard.phase {
                Phase::Open => Some("circuit open".into()),
                Phase::HalfOpen => Some("circuit half-open".into()),
                Phase::Closed => None,
            },
        }
    }

    pub fn on_success(&self) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.failures = 0;
        if guard.phase == Phase::HalfOpen {
            guard.phase = Phase::Closed;
            guard.ramp = 1;
            guard.successes_since_ramp = 0;
            guard.half_open_used = false;
            guard.open_until = None;
        } else if guard.phase == Phase::Closed && guard.ramp < guard.max_in_flight {
            guard.successes_since_ramp = guard.successes_since_ramp.saturating_add(1);
            if guard.successes_since_ramp >= 8 {
                guard.ramp = guard.ramp.saturating_mul(2).min(guard.max_in_flight);
                guard.successes_since_ramp = 0;
            }
        }
    }

    pub fn on_failure(&self) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.failures = guard.failures.saturating_add(1);
        let trip = guard.phase == Phase::HalfOpen || guard.failures >= guard.threshold;
        if trip {
            guard.phase = Phase::Open;
            guard.open_until = Some(tokio::time::Instant::now() + guard.open_for);
            guard.ramp = 1;
            guard.half_open_used = false;
            guard.successes_since_ramp = 0;
        }
    }
}

impl Inner {
    fn refresh(&mut self) {
        if self.phase == Phase::Open {
            if let Some(until) = self.open_until {
                if tokio::time::Instant::now() >= until {
                    self.phase = Phase::HalfOpen;
                    self.half_open_used = false;
                    self.ramp = 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn opens_and_ramps_instead_of_releasing_everyone() {
        tokio::time::pause();
        let breaker = Breaker::new(3, Duration::from_secs(10), 8);
        breaker.on_failure();
        breaker.on_failure();
        assert_eq!(breaker.view().phase, Phase::Closed);
        breaker.on_failure();
        let open = breaker.view();
        assert_eq!(open.phase, Phase::Open);
        assert_eq!(open.limit, 1);
        tokio::time::advance(Duration::from_secs(10)).await;
        assert_eq!(breaker.view().phase, Phase::HalfOpen);
        breaker.on_success();
        assert_eq!(breaker.view().phase, Phase::Closed);
        assert_eq!(breaker.view().limit, 1);
        for _ in 0..8 {
            breaker.on_success();
        }
        assert_eq!(breaker.view().limit, 2);
    }
}
