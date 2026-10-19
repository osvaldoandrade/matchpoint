//! One bounded, payload-free progress snapshot per observed operation.
use super::Service;
use std::time::{Duration, Instant};
use tokio::sync::watch;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Starting,
    CopyingInput,
    ScanningInput,
    RequestingToken,
    WaitingForDat,
    DownloadingDat,
    RetryWait,
    DecryptingDat,
    VerifyingBloom,
    VerifyingInput,
    VerifyingCheckpoint,
    Matching,
    Publishing,
}
impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Starting => "Starting operation",
            Self::CopyingInput => "Copying input",
            Self::ScanningInput => "Scanning dataset",
            Self::RequestingToken => "Requesting token",
            Self::WaitingForDat => "Waiting for DAT response",
            Self::DownloadingDat => "Downloading DAT",
            Self::RetryWait => "Waiting to retry DAT",
            Self::DecryptingDat => "Decrypting DAT",
            Self::VerifyingBloom => "Verifying Filtering data",
            Self::VerifyingInput => "Verifying input checksum",
            Self::VerifyingCheckpoint => "Verifying output checkpoint",
            Self::Matching => "Matching dataset",
            Self::Publishing => "Publishing results",
        }
    }
    fn bytes(self) -> bool {
        matches!(
            self,
            Self::CopyingInput | Self::DownloadingDat | Self::VerifyingCheckpoint
        )
    }
}
#[derive(Clone, Debug)]
pub struct Progress {
    pub phase: Phase,
    pub completed: u64,
    pub total: Option<u64>,
    pub started: Instant,
    pub phase_started: Instant,
    pub updated: Instant,
    pub baseline: u64,
    pub retry: u8,
    pub retry_limit: u8,
    pub last_http: Option<u16>,
    pub retry_http: Option<u16>,
    pub wait_until: Option<Instant>,
    pub job: Option<uuid::Uuid>,
}
impl Progress {
    pub fn new(phase: Phase) -> Self {
        let now = Instant::now();
        Self {
            phase,
            completed: 0,
            total: None,
            started: now,
            phase_started: now,
            updated: now,
            baseline: 0,
            retry: 0,
            retry_limit: 0,
            last_http: None,
            retry_http: None,
            wait_until: None,
            job: None,
        }
    }
    fn transition(&mut self, phase: Phase) {
        if self.phase != phase {
            self.phase = phase;
            self.phase_started = Instant::now();
            self.baseline = self.completed;
        }
    }
    pub fn ratio(&self) -> f64 {
        self.total
            .filter(|&n| n > 0)
            .map_or(0.0, |n| (self.completed as f64 / n as f64).clamp(0.0, 1.0))
    }
    pub fn headline(&self, now: Instant, compact: bool) -> String {
        let seconds = now.saturating_duration_since(self.phase_started).as_secs();
        match self.phase {
            Phase::WaitingForDat => format!(
                "{} · {seconds}s · {}/{}",
                if compact {
                    "Waiting for DAT"
                } else {
                    self.phase.label()
                },
                self.retry + 1,
                self.retry_limit + 1
            ),
            Phase::RetryWait => format!(
                "{} · retry {}/{}",
                self.retry_http
                    .map_or_else(|| "DAT interrupted".into(), |s| format!("Server HTTP {s}")),
                self.retry,
                self.retry_limit
            ),
            Phase::CopyingInput | Phase::DownloadingDat | Phase::VerifyingCheckpoint => {
                let label = if compact {
                    match self.phase {
                        Phase::CopyingInput => "Copying input",
                        Phase::DownloadingDat => "DAT",
                        _ => "Checkpoint",
                    }
                } else {
                    self.phase.label()
                };
                match self.total {
                    Some(total) => format!(
                        "{label} · {:.1}/{:.1} MiB · {:.0}%",
                        self.completed as f64 / 1048576.0,
                        total as f64 / 1048576.0,
                        self.ratio() * 100.0
                    ),
                    None => format!("{label} · {:.1} MiB", self.completed as f64 / 1048576.0),
                }
            }
            Phase::ScanningInput => format!("Scanning dataset · {} rows", self.completed),
            Phase::Matching => match self.total {
                Some(total) => format!(
                    "Matching · {}/{total} · {:.0}%",
                    self.completed,
                    self.ratio() * 100.0
                ),
                None => format!("Matching · {} rows", self.completed),
            },
            _ => format!("{} · {seconds}s", self.phase.label()),
        }
    }
    pub fn detail(&self, now: Instant) -> String {
        let elapsed = now.saturating_duration_since(self.started).as_secs();
        if matches!(self.phase, Phase::WaitingForDat | Phase::RetryWait) {
            let last = self
                .last_http
                .map_or_else(String::new, |s| format!("Last HTTP {s} · "));
            let wait = self
                .wait_until
                .filter(|_| self.phase == Phase::RetryWait)
                .map_or_else(String::new, |until| {
                    format!(
                        "retry in {:.1}s · ",
                        until.saturating_duration_since(now).as_secs_f64()
                    )
                });
            return format!("{last}{wait}elapsed {elapsed}s");
        }
        let seconds = now
            .saturating_duration_since(self.phase_started)
            .as_secs_f64();
        let rate = if seconds >= 0.1 {
            self.completed.saturating_sub(self.baseline) as f64 / seconds
        } else {
            0.0
        };
        let speed = if self.phase.bytes() {
            format!("{:.1} MiB/s · ", rate / 1048576.0)
        } else if self.phase == Phase::Matching || self.phase == Phase::ScanningInput {
            format!("{rate:.0} rows/s · ")
        } else {
            String::new()
        };
        let idle = now.saturating_duration_since(self.updated).as_secs();
        let quiet = if self.phase == Phase::DownloadingDat && idle > 1 {
            format!("no data {idle}s · ")
        } else {
            String::new()
        };
        format!("{speed}{quiet}elapsed {elapsed}s")
    }
}
#[derive(Clone)]
pub(super) struct Reporter(watch::Sender<Progress>);
impl Reporter {
    pub fn phase(&self, phase: Phase, completed: u64, total: Option<u64>) {
        let mut changed = false;
        self.0.send_modify(|p| {
            changed = p.phase != phase;
            p.transition(phase);
            p.completed = completed;
            p.total = total;
            if changed {
                p.baseline = completed;
            }
            p.updated = Instant::now();
            p.wait_until = None;
        });
        if changed {
            tracing::info!(
                event = "operation_phase",
                phase = phase.label(),
                "Operation phase changed"
            );
        }
    }
    pub fn count(&self, completed: u64, total: Option<u64>) {
        self.0.send_modify(|p| {
            p.completed = completed;
            p.total = total;
            p.updated = Instant::now();
        });
    }
    pub fn job(&self, id: &str) {
        self.0
            .send_modify(|p| p.job = uuid::Uuid::parse_str(id).ok());
    }
    pub fn download_start(&self, limit: u8) {
        self.phase(Phase::WaitingForDat, 0, None);
        self.0.send_modify(|p| {
            p.retry = 0;
            p.retry_limit = limit;
            p.last_http = None;
            p.retry_http = None;
        });
    }
    pub fn waiting(&self, parallel: bool) {
        self.0.send_modify(|p| {
            if !(parallel && p.phase == Phase::DownloadingDat) {
                p.transition(Phase::WaitingForDat);
            }
            p.wait_until = None;
        });
    }
    pub fn total(&self, total: Option<u64>) {
        self.0.send_modify(|p| p.total = total);
    }
    pub fn retry(&self, number: u8, status: Option<u16>, delay: Duration) {
        self.0.send_modify(|p| {
            p.transition(Phase::RetryWait);
            // Retry futures can finish out of order in parallel mode.
            p.retry = p.retry.max(number);
            p.retry_http = status;
            if status.is_some() {
                p.last_http = status;
            }
            p.wait_until = Some(Instant::now() + delay);
        });
    }
    fn add_bytes(&self, amount: u64) {
        self.0.send_modify(|p| {
            p.transition(Phase::DownloadingDat);
            p.completed = p.completed.saturating_add(amount);
            p.updated = Instant::now();
        });
    }
    fn rewind(&self, amount: u64) {
        self.0.send_modify(|p| {
            p.completed = p.completed.saturating_sub(amount);
            p.baseline = p.completed;
            p.phase_started = Instant::now();
        });
    }
}
pub(super) struct ByteCounter {
    reporter: Option<Reporter>,
    completed: u64,
}
impl ByteCounter {
    pub fn new(reporter: Option<Reporter>) -> Self {
        Self {
            reporter,
            completed: 0,
        }
    }
    pub fn add(&mut self, bytes: usize) {
        self.completed += bytes as u64;
        if let Some(r) = &self.reporter {
            r.add_bytes(bytes as u64);
        }
    }
    pub fn rewind(&mut self) {
        if let Some(r) = &self.reporter {
            r.rewind(self.completed);
        }
        self.completed = 0;
    }
}
impl Service {
    pub(crate) fn observed(&self) -> (Self, watch::Receiver<Progress>) {
        let (sender, receiver) = watch::channel(Progress::new(Phase::Starting));
        let mut owned = self.clone();
        owned.progress = Some(Reporter(sender));
        (owned, receiver)
    }
    pub(super) fn phase(&self, phase: Phase, completed: u64, total: Option<u64>) {
        if let Some(reporter) = &self.progress {
            reporter.phase(phase, completed, total);
        }
    }
    pub(super) fn count(&self, completed: u64, total: Option<u64>) {
        if let Some(reporter) = &self.progress {
            reporter.count(completed, total);
        }
    }
}
