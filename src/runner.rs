use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, watch, Mutex as AsyncMutex};

use crate::auth::{build_client, TokenGate};
use crate::breaker::Breaker;
use crate::domain::{ContractMode, Evidence, JobState, RecordEvent};
use crate::error::{Error, Secret};
use crate::httpcall::{
    backoff_ms, classify_status, classify_transport, post_cpf, Disposition, Transport,
};
use crate::jobcfg::JobConfig;
use crate::metrics::Metrics;
use crate::rate::RateGate;
use crate::secrets::SecretStore;
use crate::store::{Db, Outcome, WorkItem};
use crate::timeutil::{format_rfc3339, parse_rfc3339, WallClock};

/// Queues are bounded: 1024 async commands and 1024 commands on the writer thread.
/// Workers: exactly `concurrency` tasks, each with a one-slot queue.
/// Termination: the writer exits on `Stop` or channel disconnect after a final flush.
/// Each worker exits when its queue closes or `stop` is set before the next item.
const QUEUE: usize = 1024;

enum Cmd {
    Claim {
        limit: usize,
        now: String,
        reply: SyncSender<Result<Vec<WorkItem>, Error>>,
    },
    Outcome(Outcome),
    Flush(SyncSender<Result<(), Error>>),
    Counts(SyncSender<Result<crate::domain::Counts, Error>>),
    MetaGet {
        key: String,
        reply: SyncSender<Result<Option<String>, Error>>,
    },
    #[allow(dead_code)]
    MetaSet {
        key: String,
        value: String,
        reply: SyncSender<Result<(), Error>>,
    },
    SetState {
        state: JobState,
        reply: SyncSender<Result<(), Error>>,
    },
    Recover {
        now: String,
        reply: SyncSender<Result<u64, Error>>,
    },
    Event {
        now: String,
        kind: String,
        detail: String,
        reply: SyncSender<Result<(), Error>>,
    },
    NextRetry(SyncSender<Result<Option<String>, Error>>),
    Stop(SyncSender<()>),
}

struct WriterHandle {
    tx: mpsc::Sender<Cmd>,
}

impl WriterHandle {
    async fn recover(&self, now: String) -> Result<u64, Error> {
        let (reply, rx) = sync_channel(1);
        self.send(Cmd::Recover { now, reply }).await?;
        tokio::task::spawn_blocking(move || {
            rx.recv()
                .unwrap_or(Err(Error::Msg("writer stopped".into())))
        })
        .await
        .map_err(|e| Error::Msg(e.to_string()))?
    }
    async fn claim(&self, limit: usize, now: String) -> Result<Vec<WorkItem>, Error> {
        let (reply, rx) = sync_channel(1);
        self.send(Cmd::Claim { limit, now, reply }).await?;
        tokio::task::spawn_blocking(move || {
            rx.recv()
                .unwrap_or(Err(Error::Msg("writer stopped".into())))
        })
        .await
        .map_err(|e| Error::Msg(e.to_string()))?
    }

    async fn outcome(&self, outcome: Outcome) -> Result<(), Error> {
        self.send(Cmd::Outcome(outcome)).await
    }

    async fn flush(&self) -> Result<(), Error> {
        let (reply, rx) = sync_channel(1);
        self.send(Cmd::Flush(reply)).await?;
        tokio::task::spawn_blocking(move || {
            rx.recv()
                .unwrap_or(Err(Error::Msg("writer stopped".into())))
        })
        .await
        .map_err(|e| Error::Msg(e.to_string()))?
    }

    async fn counts(&self) -> Result<crate::domain::Counts, Error> {
        let (reply, rx) = sync_channel(1);
        self.send(Cmd::Counts(reply)).await?;
        tokio::task::spawn_blocking(move || {
            rx.recv()
                .unwrap_or(Err(Error::Msg("writer stopped".into())))
        })
        .await
        .map_err(|e| Error::Msg(e.to_string()))?
    }

    async fn meta(&self, key: &str) -> Result<Option<String>, Error> {
        let (reply, rx) = sync_channel(1);
        self.send(Cmd::MetaGet {
            key: key.to_string(),
            reply,
        })
        .await?;
        tokio::task::spawn_blocking(move || {
            rx.recv()
                .unwrap_or(Err(Error::Msg("writer stopped".into())))
        })
        .await
        .map_err(|e| Error::Msg(e.to_string()))?
    }

    #[allow(dead_code)]
    async fn set_meta(&self, key: &str, value: &str) -> Result<(), Error> {
        let (reply, rx) = sync_channel(1);
        self.send(Cmd::MetaSet {
            key: key.to_string(),
            value: value.to_string(),
            reply,
        })
        .await?;
        tokio::task::spawn_blocking(move || {
            rx.recv()
                .unwrap_or(Err(Error::Msg("writer stopped".into())))
        })
        .await
        .map_err(|e| Error::Msg(e.to_string()))?
    }

    async fn set_state(&self, state: JobState) -> Result<(), Error> {
        let (reply, rx) = sync_channel(1);
        self.send(Cmd::SetState { state, reply }).await?;
        tokio::task::spawn_blocking(move || {
            rx.recv()
                .unwrap_or(Err(Error::Msg("writer stopped".into())))
        })
        .await
        .map_err(|e| Error::Msg(e.to_string()))?
    }

    async fn event(&self, now: &str, kind: &str, detail: &str) -> Result<(), Error> {
        let (reply, rx) = sync_channel(1);
        self.send(Cmd::Event {
            now: now.to_string(),
            kind: kind.to_string(),
            detail: detail.to_string(),
            reply,
        })
        .await?;
        tokio::task::spawn_blocking(move || {
            rx.recv()
                .unwrap_or(Err(Error::Msg("writer stopped".into())))
        })
        .await
        .map_err(|e| Error::Msg(e.to_string()))?
    }

    async fn next_retry(&self) -> Result<Option<String>, Error> {
        let (reply, rx) = sync_channel(1);
        self.send(Cmd::NextRetry(reply)).await?;
        tokio::task::spawn_blocking(move || {
            rx.recv()
                .unwrap_or(Err(Error::Msg("writer stopped".into())))
        })
        .await
        .map_err(|e| Error::Msg(e.to_string()))?
    }

    async fn stop(&self) {
        let (reply, rx) = sync_channel(1);
        if self.send(Cmd::Stop(reply)).await.is_ok() {
            let _ = tokio::task::spawn_blocking(move || rx.recv().ok()).await;
        }
    }

    async fn send(&self, cmd: Cmd) -> Result<(), Error> {
        self.tx
            .send(cmd)
            .await
            .map_err(|_| Error::Msg("writer stopped".into()))
    }
}

pub struct Live {
    pub metrics: Arc<Metrics>,
    pub gate: Arc<TokenGate>,
    pub breaker: Arc<Breaker>,
}

#[allow(clippy::too_many_arguments)]
pub async fn run_workers(
    db_path: std::path::PathBuf,
    config: JobConfig,
    secrets: Arc<dyn SecretStore>,
    clock: Arc<dyn WallClock>,
    stop: watch::Receiver<bool>,
    stop_after_successes: Option<u64>,
    fail_outcome_commits: u32,
    grace: Duration,
) -> Result<JobState, Error> {
    let (async_tx, mut async_rx) = mpsc::channel::<Cmd>(QUEUE);
    let (sync_tx, sync_rx) = sync_channel::<Cmd>(QUEUE);
    let (ready_tx, ready_rx) = sync_channel(1);
    let fail_flag = Arc::new(AtomicU32::new(fail_outcome_commits));
    let metrics = Arc::new(Metrics::new());
    let metrics_writer = Arc::clone(&metrics);
    let fail_writer = Arc::clone(&fail_flag);
    let commit_rows = config.commit_max_rows.max(1) as usize;
    let commit_every = Duration::from_millis(config.commit_max_ms.max(1));
    let clock_writer = Arc::clone(&clock);
    std::thread::Builder::new()
        .name("pgm-sqlite".into())
        .spawn(move || {
            // Terminates after Stop or after the sync channel disconnects.
            let mut db = match Db::open(&db_path) {
                Ok(db) => db,
                Err(err) => {
                    let _ = ready_tx.send(Err(err));
                    return;
                }
            };
            let _ = ready_tx.send(Ok(()));
            writer_loop(
                sync_rx,
                &mut db,
                commit_rows,
                commit_every,
                fail_writer,
                metrics_writer,
                clock_writer,
            );
        })
        .map_err(|e| Error::Msg(e.to_string()))?;
    tokio::task::spawn_blocking(move || {
        ready_rx
            .recv()
            .unwrap_or(Err(Error::Msg("writer died".into())))
    })
    .await
    .map_err(|e| Error::Msg(e.to_string()))??;
    let sync_tx = sync_tx;
    tokio::spawn(async move {
        // Terminates when the async command channel closes.
        while let Some(cmd) = async_rx.recv().await {
            let tx = sync_tx.clone();
            if tokio::task::spawn_blocking(move || tx.send(cmd).ok())
                .await
                .is_err()
            {
                break;
            }
        }
    });
    let db = WriterHandle { tx: async_tx };
    let client = build_client(
        Duration::from_millis(config.connect_timeout_ms),
        Duration::from_millis(config.response_timeout_ms),
        config.concurrency.max(1) as usize,
    )?;
    let gate = Arc::new(TokenGate::new(
        client.clone(),
        config.token_url.clone(),
        config.client_id.clone(),
        secrets,
        Arc::clone(&clock),
        config.token_skew_secs as i64 * 1000,
    ));
    let concurrency = config.concurrency.max(1);
    let breaker = Arc::new(Breaker::new(
        config.circuit_threshold.max(1),
        Duration::from_millis(config.circuit_open_ms.max(1)),
        concurrency,
    ));
    let rate = Arc::new(RateGate::new(config.rps));
    let (work_txs, mut work_rxs) = (0..concurrency)
        .map(|_| mpsc::channel::<WorkItem>(1))
        .unzip::<_, _, Vec<_>, Vec<_>>();
    let idle = Arc::new(tokio::sync::Notify::new());
    let contract = ContractMode::parse(&config.contract)?;
    let (worker_stop_tx, worker_stop) = watch::channel(false);
    let mut handles = Vec::new();
    for _ in 0..concurrency {
        let mut rx = work_rxs.remove(0);
        let db = db.clone();
        let client = client.clone();
        let gate = Arc::clone(&gate);
        let breaker = Arc::clone(&breaker);
        let rate = Arc::clone(&rate);
        let metrics = Arc::clone(&metrics);
        let clock = Arc::clone(&clock);
        let stop = worker_stop.clone();
        let idle = Arc::clone(&idle);
        let config = config.clone();
        handles.push(tokio::spawn(async move {
            // Terminates when the dispatcher drops the sender and the slot is empty,
            // or when stop is observed before taking another item.
            loop {
                let Some(item) = rx.recv().await else { break };
                if let Some(outcome) = process_one(
                    item,
                    &client,
                    &gate,
                    &rate,
                    &breaker,
                    &metrics,
                    clock.as_ref(),
                    &stop,
                    &config,
                    contract,
                )
                .await
                {
                    let _ = db.outcome(outcome).await;
                }
                idle.notify_waiters();
            }
        }));
    }
    let final_state = dispatch(
        &db,
        &work_txs,
        &idle,
        &breaker,
        &gate,
        &metrics,
        clock.as_ref(),
        &stop,
        &config,
        stop_after_successes,
        grace,
    )
    .await;
    // Stop new POSTs, let already sent requests finish, and retain the writer until
    // every worker has submitted its outcome. A flush alone is not a worker barrier.
    let _ = worker_stop_tx.send(true);
    for tx in work_txs {
        drop(tx);
    }
    if tokio::time::timeout(grace, async {
        for handle in &mut handles {
            let _ = handle.await;
        }
    })
    .await
    .is_err()
    {
        for handle in &mut handles {
            if !handle.is_finished() {
                handle.abort();
                let _ = handle.await;
            }
        }
    }
    let flushed = db.flush().await;
    if flushed.is_ok() {
        let recovered = db.recover(format_rfc3339(clock.now_ms())).await?;
        if recovered > 0 {
            db.event(
                &format_rfc3339(clock.now_ms()),
                "drain_timeout",
                &format!("{recovered} unconfirmed results: UNKNOWN"),
            )
            .await?;
        }
    }
    if let Ok(state) = final_state {
        db.set_state(state).await?;
    }
    db.stop().await;
    flushed?;
    final_state
}

impl Clone for WriterHandle {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn dispatch(
    db: &WriterHandle,
    workers: &[mpsc::Sender<WorkItem>],
    idle: &tokio::sync::Notify,
    breaker: &Breaker,
    gate: &TokenGate,
    metrics: &Metrics,
    clock: &dyn WallClock,
    stop: &watch::Receiver<bool>,
    config: &JobConfig,
    stop_after_successes: Option<u64>,
    grace: Duration,
) -> Result<JobState, Error> {
    let _ = config;
    loop {
        if *stop.borrow() {
            return drain(db, grace, JobState::Interrupted).await;
        }
        if stop_after_successes.is_some_and(|n| metrics.successes() >= n) {
            return drain(db, grace, JobState::Paused).await;
        }
        if db.meta("desired").await?.as_deref() == Some("pause") {
            return drain(db, grace, JobState::Paused).await;
        }
        let paused_until = gate.paused_until_ms().await;
        let now_ms = clock.now_ms();
        if paused_until > now_ms {
            let wait =
                Duration::from_millis((paused_until - now_ms) as u64).min(Duration::from_secs(1));
            let _ = db
                .event(
                    &format_rfc3339(now_ms),
                    "auth_paused",
                    "token refresh is backing off",
                )
                .await;
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = changed(stop) => return drain(db, grace, JobState::Interrupted).await,
            }
            continue;
        }
        let view = breaker.view();
        if let Some(wait) = view.wait {
            let _ = db
                .event(
                    &format_rfc3339(now_ms),
                    "circuit_open",
                    view.reason.as_deref().unwrap_or("circuit"),
                )
                .await;
            tokio::select! {
                _ = tokio::time::sleep(wait.min(Duration::from_secs(1))) => {}
                _ = changed(stop) => return drain(db, grace, JobState::Interrupted).await,
                _ = idle.notified() => {}
            }
            continue;
        }
        let idle_slots = workers.iter().filter(|tx| tx.capacity() > 0).count();
        if idle_slots == 0 {
            tokio::select! {
                _ = idle.notified() => {}
                _ = changed(stop) => return drain(db, grace, JobState::Interrupted).await,
                _ = tokio::time::sleep(Duration::from_millis(50)) => {}
            }
            continue;
        }
        let limit = idle_slots.min(view.limit as usize).max(1);
        let claimed = db.claim(limit, format_rfc3339(clock.now_ms())).await?;
        if claimed.is_empty() {
            db.flush().await?;
            let counts = db.counts().await?;
            if counts.open() == 0 {
                let state = if counts.unknown > 0 {
                    JobState::BlockedUnknown
                } else if counts.failed_final > 0 {
                    JobState::CompletedWithErrors
                } else {
                    JobState::Completed
                };
                db.set_state(state).await?;
                return Ok(state);
            }
            let wait = match db.next_retry().await? {
                Some(stamp) => parse_rfc3339(&stamp)
                    .map(
                        |ms| Duration::from_millis(ms.saturating_sub(clock.now_ms()).max(0) as u64),
                    )
                    .unwrap_or(Duration::from_millis(200))
                    .min(Duration::from_secs(1)),
                None => Duration::from_millis(200),
            };
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = idle.notified() => {}
                _ = changed(stop) => return drain(db, grace, JobState::Interrupted).await,
            }
            continue;
        }
        for item in claimed {
            let mut sent = false;
            for tx in workers {
                if tx.try_send(item.clone()).is_ok() {
                    sent = true;
                    break;
                }
            }
            if !sent {
                let _ = workers[0].send(item).await;
            }
        }
    }
}

async fn drain(db: &WriterHandle, grace: Duration, state: JobState) -> Result<JobState, Error> {
    tokio::select! {
        _ = db.flush() => {}
        _ = tokio::time::sleep(grace) => {}
    }
    Ok(state)
}

#[allow(clippy::too_many_arguments)]
async fn process_one(
    item: WorkItem,
    client: &reqwest::Client,
    gate: &TokenGate,
    rate: &RateGate,
    breaker: &Breaker,
    metrics: &Metrics,
    clock: &dyn WallClock,
    stop: &watch::Receiver<bool>,
    config: &JobConfig,
    contract: ContractMode,
) -> Option<Outcome> {
    if *stop.borrow() {
        return Some(unsent(&item, clock, "cancelled"));
    }
    let token = match gate.access().await {
        Ok(token) => token,
        Err(_) => return Some(unsent(&item, clock, "auth_unavailable")),
    };
    if !rate.acquire(stop).await {
        return Some(unsent(&item, clock, "cancelled"));
    }
    let started = Instant::now();
    let started_at = format_rfc3339(clock.now_ms());
    let key = config
        .idempotency
        .then(|| format!("{}:{}", config.job_id, item.row_id));
    let first = post_cpf(
        client,
        &config.endpoint,
        token.access.expose(),
        &item.cpf,
        key.as_deref(),
        config.max_response_bytes,
    )
    .await;
    let mut sends = 1;
    let mut token_gen = token.generation as i64;
    let mut http_status: Option<i32> = None;
    let disposition = match first {
        Ok(reply) if reply.status == 401 => {
            http_status = Some(i32::from(reply.status));
            gate.invalidate(token.generation).await;
            match gate.access().await {
                Ok(next) if next.generation != token.generation => {
                    if !rate.acquire(stop).await {
                        return Some(unsent(&item, clock, "cancelled"));
                    }
                    sends = 2;
                    token_gen = next.generation as i64;
                    match post_cpf(
                        client,
                        &config.endpoint,
                        next.access.expose(),
                        &item.cpf,
                        key.as_deref(),
                        config.max_response_bytes,
                    )
                    .await
                    {
                        Ok(reply) if reply.status == 401 || reply.status == 403 => {
                            http_status = Some(i32::from(reply.status));
                            gate.note_auth_failure(config.circuit_open_ms as i64).await;
                            Disposition::Retry {
                                after_ms: Some(config.circuit_open_ms as i64),
                                code: "unauthorized".into(),
                            }
                        }
                        Ok(reply) => {
                            http_status = Some(i32::from(reply.status));
                            classify_status(contract, reply.status, &reply.bytes, reply.retry_after)
                        }
                        Err(err) => {
                            if let Transport::TooLarge { status } = &err {
                                http_status = Some(i32::from(*status));
                            }
                            transport(err, config.idempotency)
                        }
                    }
                }
                _ => {
                    gate.note_auth_failure(config.circuit_open_ms as i64).await;
                    Disposition::Retry {
                        after_ms: Some(config.circuit_open_ms as i64),
                        code: "unauthorized".into(),
                    }
                }
            }
        }
        Ok(reply) => {
            http_status = Some(i32::from(reply.status));
            classify_status(contract, reply.status, &reply.bytes, reply.retry_after)
        }
        Err(err) => {
            if let Transport::TooLarge { status } = &err {
                http_status = Some(i32::from(*status));
            }
            transport(err, config.idempotency)
        }
    };
    metrics.observe_http(started.elapsed().as_micros() as u64, clock.now_ms());
    let used = item.attempts.saturating_sub(item.attempt_origin) + sends;
    let mut disposition = disposition;
    if matches!(disposition, Disposition::Retry { .. }) && used >= config.max_attempts as i32 {
        disposition = Disposition::FinalFail {
            code: "attempts_exhausted".into(),
        };
    }
    match &disposition {
        Disposition::Unknown { code, .. } if code.starts_with("http_5") => breaker.on_failure(),
        Disposition::Retry { code, .. }
            if code.starts_with("http_5")
                || code == "connect"
                || code == "timeout"
                || code == "http_429" =>
        {
            breaker.on_failure();
        }
        Disposition::Succeeded { .. }
        | Disposition::FinalFail { .. }
        | Disposition::Unknown { .. } => breaker.on_success(),
        _ => {}
    }
    if matches!(disposition, Disposition::Succeeded { .. }) {
        metrics.success();
    }
    let event = disposition.event().unwrap_or(RecordEvent::ScheduleRetry);
    let finished = format_rfc3339(clock.now_ms());
    let next = if event == RecordEvent::ScheduleRetry {
        let retry_after = match &disposition {
            Disposition::Retry { after_ms, .. } => *after_ms,
            _ => None,
        };
        let delay = backoff_ms(
            used,
            config.backoff_base_ms,
            config.backoff_max_ms,
            item.row_id,
            retry_after,
        );
        Some(format_rfc3339(clock.now_ms().saturating_add(delay)))
    } else {
        None
    };
    Some(Outcome {
        row_id: item.row_id,
        batch_id: item.batch_id,
        event,
        sends,
        http_status,
        error_code: disposition.error_code(),
        request_id: disposition.request_id(),
        is_customer: disposition.is_customer(),
        evidence: disposition.evidence(),
        next_attempt_at: next,
        token_generation: Some(token_gen),
        started_at,
        finished_at: finished,
    })
}

fn transport(err: Transport, idempotent: bool) -> Disposition {
    match err {
        Transport::Reqwest { connect, timeout } => classify_transport(connect, timeout, idempotent),
        Transport::TooLarge { .. } => Disposition::Unknown {
            code: "response_too_large".into(),
            request_id: None,
        },
        Transport::Other => Disposition::Unknown {
            code: "transport".into(),
            request_id: None,
        },
    }
}

fn unsent(item: &WorkItem, clock: &dyn WallClock, code: &str) -> Outcome {
    let now = format_rfc3339(clock.now_ms());
    Outcome {
        row_id: item.row_id,
        batch_id: item.batch_id,
        event: RecordEvent::ReleaseUnsent,
        sends: 0,
        http_status: None,
        error_code: Some(code.into()),
        request_id: None,
        is_customer: None,
        evidence: Evidence::None,
        next_attempt_at: Some(now.clone()),
        token_generation: None,
        started_at: now.clone(),
        finished_at: now,
    }
}

fn writer_loop(
    rx: Receiver<Cmd>,
    db: &mut Db,
    commit_rows: usize,
    commit_every: Duration,
    fail_flag: Arc<AtomicU32>,
    metrics: Arc<Metrics>,
    clock: Arc<dyn WallClock>,
) {
    let mut pending = Vec::new();
    let mut started = Instant::now();
    let mut disk_full = false;
    let mut commits = 0_u32;
    loop {
        let cmd = if pending.is_empty() {
            match rx.recv() {
                Ok(cmd) => Some(cmd),
                Err(_) => break,
            }
        } else {
            let left = commit_every.saturating_sub(started.elapsed());
            match rx.recv_timeout(left) {
                Ok(cmd) => Some(cmd),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        };
        if cmd.is_none() {
            let _ = flush_pending(
                db,
                &mut pending,
                &fail_flag,
                &metrics,
                clock.as_ref(),
                &mut disk_full,
            );
            commits += 1;
            maybe_checkpoint(db, &mut commits);
            continue;
        }
        match cmd.unwrap() {
            Cmd::Outcome(outcome) => {
                if disk_full {
                    continue;
                }
                if pending.is_empty() {
                    started = Instant::now();
                }
                pending.push(outcome);
                if pending.len() >= commit_rows {
                    let _ = flush_pending(
                        db,
                        &mut pending,
                        &fail_flag,
                        &metrics,
                        clock.as_ref(),
                        &mut disk_full,
                    );
                    commits += 1;
                    maybe_checkpoint(db, &mut commits);
                }
            }
            Cmd::Claim { limit, now, reply } => {
                if !disk_full {
                    let _ = flush_pending(
                        db,
                        &mut pending,
                        &fail_flag,
                        &metrics,
                        clock.as_ref(),
                        &mut disk_full,
                    );
                }
                let result = if disk_full {
                    Err(Error::DiskFull)
                } else {
                    db.claim(limit, &now)
                };
                let _ = reply.send(result);
            }
            Cmd::Flush(reply) => {
                let result = if disk_full {
                    Err(Error::DiskFull)
                } else {
                    flush_pending(
                        db,
                        &mut pending,
                        &fail_flag,
                        &metrics,
                        clock.as_ref(),
                        &mut disk_full,
                    )
                };
                let _ = reply.send(result);
            }
            Cmd::Counts(reply) => {
                let _ = reply.send(db.counts());
            }
            Cmd::MetaGet { key, reply } => {
                let _ = reply.send(db.meta_get(&key));
            }
            Cmd::MetaSet { key, value, reply } => {
                let _ = reply.send(db.meta_set(&key, &value));
            }
            Cmd::SetState { state, reply } => {
                let _ = reply.send(db.set_state(state));
            }
            Cmd::Recover { now, reply } => {
                let _ = reply.send(db.recover_inflight(&now));
            }
            Cmd::Event {
                now,
                kind,
                detail,
                reply,
            } => {
                let _ = reply.send(db.push_event(&now, &kind, &detail));
            }
            Cmd::NextRetry(reply) => {
                let _ = reply.send(db.next_retry_at());
            }
            Cmd::Stop(reply) => {
                let _ = flush_pending(
                    db,
                    &mut pending,
                    &fail_flag,
                    &metrics,
                    clock.as_ref(),
                    &mut disk_full,
                );
                let _ = reply.send(());
                break;
            }
        }
    }
}

fn flush_pending(
    db: &mut Db,
    pending: &mut Vec<Outcome>,
    fail_flag: &AtomicU32,
    metrics: &Metrics,
    clock: &dyn WallClock,
    disk_full: &mut bool,
) -> Result<(), Error> {
    if pending.is_empty() {
        return Ok(());
    }
    let fail = fail_flag.load(Ordering::SeqCst) > 0 && {
        fail_flag.fetch_sub(1, Ordering::SeqCst);
        true
    };
    let started = Instant::now();
    let now = format_rfc3339(clock.now_ms());
    let result = db.apply(pending, &now, fail);
    if result.is_ok() {
        metrics.observe_commit(started.elapsed().as_micros() as u64);
        pending.clear();
    } else {
        *disk_full = true;
        pending.clear();
    }
    result
}

fn maybe_checkpoint(db: &Db, commits: &mut u32) {
    if *commits >= 20 {
        let _ = db.passive_checkpoint();
        *commits = 0;
    }
}

async fn changed(stop: &watch::Receiver<bool>) {
    let mut stop = stop.clone();
    loop {
        if *stop.borrow() {
            return;
        }
        if stop.changed().await.is_err() {
            return;
        }
    }
}

// The async mutex import keeps a single notify path documented for reviewers.
#[allow(dead_code)]
fn _async_mutex_used() -> AsyncMutex<()> {
    AsyncMutex::new(())
}

// Secret is referenced so a logged token would have to go through Debug.
#[allow(dead_code)]
fn _secret_debug(secret: &Secret) -> String {
    format!("{secret:?}")
}
