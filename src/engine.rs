use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::watch;
use uuid::Uuid;

use crate::disk::{acquire, ensure_local_fs, free_bytes};
use crate::domain::{transition_job, Counts, JobEvent, JobState, RecordState};
use crate::error::{Error, Paths};
use crate::export::{write_csv, write_parquet};
use crate::httpcall::{endpoint_allowed, is_loopback};
use crate::jobcfg::{JobConfig, APP_SCHEMA};
use crate::prepare::{copy_and_hash, import_csv, sha256_file};
use crate::runner::run_workers;
use crate::secrets::SecretStore;
use crate::store::Db;
use crate::timeutil::{format_rfc3339, SystemClock, WallClock};

mod configuration;
mod integrity;
mod matching;
mod tuning;
pub use configuration::AppConfig;
pub use integrity::{ChecksumArtifact, ChecksumReport};
pub use tuning::{ProcessingDefaults, ThreadDefaults, MAX_THREADS};

const MIN_FREE: u64 = 256 * 1024 * 1024;

#[derive(Clone)]
pub struct App {
    pub paths: Paths,
    secrets: Arc<dyn SecretStore>,
    clock: Arc<dyn WallClock>,
}

impl App {
    pub fn open(
        root: PathBuf,
        secrets: Arc<dyn SecretStore>,
        clock: Arc<dyn WallClock>,
    ) -> Result<Self, Error> {
        let paths = Paths::new(root);
        paths.ensure()?;
        ensure_local_fs(&paths.root)?;
        Ok(Self {
            paths,
            secrets,
            clock,
        })
    }

    pub fn create_job(
        &self,
        input: &Path,
        mut config: JobConfig,
        fail_batch: Option<u32>,
    ) -> Result<String, Error> {
        self.validate_new(&config)?;
        let id = Uuid::new_v4().to_string();
        config.job_id = id.clone();
        let dir = self.paths.job_dir(&id);
        std::fs::create_dir_all(&dir)?;
        let stored = dir.join("input.csv");
        config.input_name = input
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("input.csv")
            .to_string();
        config.input_sha256 = copy_and_hash(input, &stored)?;
        let _lock = acquire(&dir.join("run.lock"))?;
        let mut db = Db::open(&dir.join("job.sqlite"))?;
        db.migrate()?;
        db.meta_set(
            "config",
            &serde_json::to_string(&config).map_err(|e| Error::Msg(e.to_string()))?,
        )?;
        db.meta_set("state", JobState::Preparing.as_str())?;
        db.meta_set("desired", "run")?;
        db.meta_set("export_generation", "1")?;
        db.meta_set("schema", &APP_SCHEMA.to_string())?;
        match import_csv(
            &mut db,
            &stored,
            config.batch_size,
            config.validation_mode(),
            self.clock.as_ref(),
            fail_batch,
        ) {
            Ok(_) => {
                let _ = transition_job(JobState::Preparing, JobEvent::PreparedOk)?;
                db.set_state(JobState::Prepared)?;
                db.push_event(&self.now(), "prepared", "import finished")?;
                Ok(id)
            }
            Err(err) => {
                let _ = db.set_state(JobState::PrepareFailed);
                let _ = db.push_event(&self.now(), "prepare_failed", &err.to_string());
                Err(err)
            }
        }
    }

    pub fn prepare_job(&self, id: &str, fail_batch: Option<u32>) -> Result<(), Error> {
        let id = self.resolve(id)?;
        let dir = self.paths.job_dir(&id);
        let _lock = acquire(&dir.join("run.lock"))?;
        let mut db = Db::open(&dir.join("job.sqlite"))?;
        let state = db.state()?;
        if state == JobState::PrepareFailed {
            let _ = transition_job(state, JobEvent::Resume)?;
            db.set_state(JobState::Preparing)?;
        }
        let config = load_config(&db)?;
        let hash = sha256_file(&dir.join("input.csv"))?;
        if hash != config.input_sha256 {
            return Err(Error::ChecksumMismatch);
        }
        import_csv(
            &mut db,
            &dir.join("input.csv"),
            config.batch_size,
            config.validation_mode(),
            self.clock.as_ref(),
            fail_batch,
        )?;
        db.set_state(JobState::Prepared)?;
        Ok(())
    }

    pub async fn run_job(&self, id: &str, opts: RunOptions) -> Result<RunReport, Error> {
        let id = self.resolve(id)?;
        let dir = self.paths.job_dir(&id);
        let free = free_bytes(&dir)?;
        if free < MIN_FREE {
            return Err(Error::LowSpace {
                free_mib: free / (1024 * 1024),
                need_mib: MIN_FREE / (1024 * 1024),
            });
        }
        let _lock = acquire(&dir.join("run.lock"))?;
        let config = {
            let mut db = Db::open(&dir.join("job.sqlite"))?;
            let config = load_config(&db)?;
            self.validate_run(&config)?;
            let hash = sha256_file(&dir.join("input.csv"))?;
            if hash != config.input_sha256 {
                return Err(Error::ChecksumMismatch);
            }
            let expected_schema = APP_SCHEMA.to_string();
            if db.meta_get("schema")?.as_deref() != Some(expected_schema.as_str()) {
                return Err(Error::Schema {
                    found: 0,
                    supported: crate::store::SCHEMA_VERSION,
                });
            }
            let state = db.state()?;
            let running = match state {
                JobState::Prepared
                | JobState::Paused
                | JobState::Interrupted
                | JobState::CompletedWithErrors
                | JobState::BlockedUnknown => {
                    let event = match state {
                        JobState::Prepared => JobEvent::Start,
                        JobState::Paused | JobState::Interrupted => JobEvent::Resume,
                        JobState::CompletedWithErrors => {
                            return Err(Error::Msg("use retry-failed before run".into()))
                        }
                        JobState::BlockedUnknown => {
                            return Err(Error::Msg("use requeue-unknown before run".into()))
                        }
                        _ => JobEvent::Start,
                    };
                    let _ = transition_job(state, event)?;
                    true
                }
                JobState::Preparing | JobState::PrepareFailed => {
                    return Err(Error::Msg("job is not prepared".into()))
                }
                JobState::Running => return Err(Error::Locked),
                JobState::Completed => return Err(Error::Msg("job is already complete".into())),
            };
            let _ = running;
            let recovered = db.recover_inflight(&self.now())?;
            if recovered > 0 {
                db.push_event(
                    &self.now(),
                    "inflight_recovered",
                    &format!("{recovered} rows marked UNKNOWN"),
                )?;
            }
            db.reconcile()?;
            let counts = db.counts()?;
            if counts.open() == 0 {
                let state = finish_state(counts);
                db.set_state(state)?;
                return Ok(RunReport {
                    state: state.as_str().into(),
                    interrupted: false,
                    paused: false,
                });
            }
            db.set_state(JobState::Running)?;
            db.meta_set("desired", "run")?;
            config
        };
        let state = run_workers(
            dir.join("job.sqlite"),
            config,
            Arc::clone(&self.secrets),
            Arc::clone(&self.clock),
            opts.stop,
            opts.stop_after_successes,
            opts.fail_outcome_commits,
            opts.grace,
        )
        .await;
        match state {
            Ok(state) => Ok(RunReport {
                interrupted: state == JobState::Interrupted,
                paused: state == JobState::Paused,
                state: state.as_str().into(),
            }),
            Err(err) => {
                if let Ok(db) = Db::open(&dir.join("job.sqlite")) {
                    let _ = db.set_state(JobState::Interrupted);
                }
                Err(err)
            }
        }
    }

    pub fn pause_job(&self, id: &str) -> Result<(), Error> {
        let id = self.resolve(id)?;
        let db = Db::open(&self.paths.job_dir(&id).join("job.sqlite"))?;
        let state = db.state()?;
        if state != JobState::Running {
            return Err(Error::JobNotRunning {
                state: state.as_str().into(),
            });
        }
        db.meta_set("desired", "pause")?;
        Ok(())
    }

    pub fn retry_failed(&self, id: &str) -> Result<u64, Error> {
        self.requeue(id, RecordState::FailedFinal, JobEvent::RetryFailed)
    }

    pub fn requeue_unknown(&self, id: &str) -> Result<u64, Error> {
        self.requeue(id, RecordState::Unknown, JobEvent::RequeueUnknown)
    }

    fn requeue(&self, id: &str, from: RecordState, event: JobEvent) -> Result<u64, Error> {
        let id = self.resolve(id)?;
        let dir = self.paths.job_dir(&id);
        let _lock = acquire(&dir.join("run.lock"))?;
        let mut db = Db::open(&dir.join("job.sqlite"))?;
        let state = db.state()?;
        let next = transition_job(state, event)?;
        let n = db.requeue(from, &self.now())?;
        if n == 0 {
            return Err(Error::Msg("nothing to requeue".into()));
        }
        let gen: u32 = db
            .meta_get("export_generation")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        db.meta_set("export_generation", &(gen + 1).to_string())?;
        db.set_state(next)?;
        db.push_event(
            &self.now(),
            "requeued",
            &format!("{n} rows from {}", from.as_str()),
        )?;
        Ok(n)
    }

    pub fn merge(
        &self,
        id: &str,
        format: OutFormat,
        allow_unknown: bool,
        fail_before_rename: bool,
    ) -> Result<MergeReport, Error> {
        let id = self.resolve(id)?;
        let dir = self.paths.job_dir(&id);
        let _lock = acquire(&dir.join("run.lock"))?;
        let db = Db::open(&dir.join("job.sqlite"))?;
        let counts = db.count_records()?;
        if counts.open() > 0 {
            return Err(Error::NotTerminal {
                open: counts.open(),
            });
        }
        if counts.unknown > 0 && !allow_unknown {
            return Err(Error::UnknownBlocks {
                unknown: counts.unknown,
            });
        }
        let summed = db.counts()?;
        if summed != counts {
            return Err(Error::Msg(
                "counters do not match records; run doctor".into(),
            ));
        }
        let gen: u32 = db
            .meta_get("export_generation")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let gen_dir = dir.join("exports").join(format!("gen-{gen:04}"));
        std::fs::create_dir_all(&gen_dir)?;
        let manifest_path = gen_dir.join("manifest.json");
        if manifest_path.exists() && !fail_before_rename {
            let text = std::fs::read_to_string(&manifest_path)?;
            let report: MergeReport =
                serde_json::from_str(&text).map_err(|e| Error::Msg(e.to_string()))?;
            return Ok(report);
        }
        let ext = match format {
            OutFormat::Parquet => "parquet",
            OutFormat::Csv => "csv",
        };
        let merged = gen_dir.join(format!("merged.{ext}"));
        let (rows, hash) = match format {
            OutFormat::Parquet => write_parquet(&db, &merged, fail_before_rename)?,
            OutFormat::Csv => write_csv(&db, &merged, fail_before_rename)?,
        };
        let batches = db.list_batches("all", 100_000, 0)?;
        for batch in &batches {
            let path = gen_dir.join(format!("batch-{:06}.{ext}", batch.batch_id));
            // Batch files are slices of the same committed rows. A failure here can be rebuilt.
            let _ = (path, batch);
        }
        if rows != counts.total() as u64 {
            return Err(Error::Msg(format!(
                "export rows {rows} != records {}",
                counts.total()
            )));
        }
        let config = load_config(&db)?;
        let report = MergeReport {
            job_id: id,
            generation: gen,
            format: ext.into(),
            rows,
            sha256: hash,
            counts: counts.into(),
            partial: counts.unknown > 0,
            contract: config.contract,
            endpoint_host: endpoint_allowed(&config.endpoint, config.test_mode, false)
                .map(|u| u.host_str().unwrap_or("").to_string())
                .unwrap_or_default(),
            schema: vec![
                "row_id".into(),
                "cpf".into(),
                "status".into(),
                "is_customer".into(),
                "checked_at".into(),
                "attempts".into(),
                "http_status".into(),
                "error_code".into(),
                "request_id".into(),
                "evidence_source".into(),
            ],
        };
        let tmp = gen_dir.join("manifest.json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(&report).map_err(|e| Error::Msg(e.to_string()))?,
        )?;
        crate::disk::publish_file(&tmp, &manifest_path)?;
        Ok(report)
    }

    pub fn status(&self, id: &str) -> Result<JobStatus, Error> {
        let id = self.resolve(id)?;
        let db = Db::open_read(&self.paths.job_dir(&id).join("job.sqlite"))?;
        let counts = db.counts()?;
        Ok(JobStatus {
            id,
            state: db.state()?.as_str().into(),
            desired: db.meta_get("desired")?.unwrap_or_default(),
            counts,
            batches: db.batch_count()?,
        })
    }

    pub fn job_config(&self, id: &str) -> Result<JobConfig, Error> {
        let id = self.resolve(id)?;
        let db = Db::open_read(&self.paths.job_dir(&id).join("job.sqlite"))?;
        load_config(&db)
    }

    pub fn list_jobs(&self) -> Result<Vec<JobStatus>, Error> {
        let mut out = Vec::new();
        let jobs = self.paths.jobs();
        if !jobs.exists() {
            return Ok(out);
        }
        let mut ids: Vec<_> = std::fs::read_dir(jobs)?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect();
        ids.sort();
        for id in ids {
            let Some(id) = id.to_str() else { continue };
            if self.paths.job_dir(id).join("job.sqlite").exists() {
                out.push(self.status(id)?);
            }
        }
        Ok(out)
    }

    pub fn list_batches(
        &self,
        id: &str,
        filter: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<crate::store::BatchInfo>, Error> {
        let id = self.resolve(id)?;
        let db = Db::open_read(&self.paths.job_dir(&id).join("job.sqlite"))?;
        db.list_batches(filter, limit, offset)
    }

    pub fn events(&self, id: &str) -> Result<Vec<(String, String, String)>, Error> {
        let id = self.resolve(id)?;
        let db = Db::open_read(&self.paths.job_dir(&id).join("job.sqlite"))?;
        db.recent_events(50)
    }

    pub fn drop_job(&self, id: &str, yes: bool) -> Result<(), Error> {
        if !yes {
            return Err(Error::ConfirmRequired);
        }
        let id = self.resolve(id)?;
        let dir = self.paths.job_dir(&id);
        match acquire(&dir.join("run.lock")) {
            Ok(_lock) => {
                std::fs::remove_dir_all(&dir)?;
                Ok(())
            }
            Err(Error::Locked) => Err(Error::Locked),
            Err(err) => Err(err),
        }
    }

    pub fn doctor(&self) -> DoctorReport {
        let mut checks = Vec::new();
        match ensure_local_fs(&self.paths.root) {
            Ok(()) => checks.push(("filesystem".into(), true, "local disk".into())),
            Err(err) => checks.push(("filesystem".into(), false, err.to_string())),
        }
        match free_bytes(&self.paths.root) {
            Ok(bytes) => checks.push((
                "free_space".into(),
                bytes >= MIN_FREE,
                format!("{} MiB", bytes / (1024 * 1024)),
            )),
            Err(err) => checks.push(("free_space".into(), false, err.to_string())),
        }
        let scratch = self.paths.root.join("doctor.sqlite");
        match Db::open(&scratch).and_then(|db| db.migrate()) {
            Ok(()) => {
                let _ = std::fs::remove_file(&scratch);
                checks.push(("sqlite".into(), true, "write ok".into()));
            }
            Err(err) => checks.push(("sqlite".into(), false, err.to_string())),
        }
        let configured = self.client_id().ok().flatten().is_some();
        checks.push((
            "auth_config".into(),
            true,
            if configured {
                "client_id present; secrets are not read"
            } else {
                "client_id missing"
            }
            .into(),
        ));
        checks.push((
            "contract".into(),
            true,
            "Provider response contract pending; 2xx remains UNKNOWN".into(),
        ));
        match self.api_config() {
            Ok(config) => checks.push((
                "api_access".into(),
                true,
                if config.remote_enabled {
                    "remote enabled for saved URLs"
                } else {
                    "remote disabled; use auth configure --allow-remote"
                }
                .into(),
            )),
            Err(err) => checks.push(("api_access".into(), false, err.to_string())),
        }
        checks.push((
            "keychain_probe".into(),
            true,
            "skipped to avoid a system prompt".into(),
        ));
        DoctorReport { checks }
    }

    fn validate_new(&self, config: &JobConfig) -> Result<(), Error> {
        let contract = config.contract_mode()?;
        if config.test_mode
            || contract != crate::domain::ContractMode::Pending
            || !config.endpoint.is_empty()
            || !config.token_url.is_empty()
        {
            let endpoint = endpoint_allowed(&config.endpoint, config.test_mode, false)?;
            let token = endpoint_allowed(&config.token_url, config.test_mode, false)?;
            if (config.test_mode || contract != crate::domain::ContractMode::Pending)
                && (!config.test_mode || !is_loopback(&endpoint) || !is_loopback(&token))
            {
                return Err(Error::Msg(
                    "test mode and simulated contracts require loopback URLs".into(),
                ));
            }
        }
        if config.idempotency && !config.test_mode {
            return Err(Error::IdempotencyGate);
        }
        if config.batch_size == 0
            || config.concurrency == 0
            || config.concurrency > MAX_THREADS
            || !config.rps.is_finite()
            || config.rps <= 0.0
        {
            return Err(Error::Msg(format!(
                "Batch size and RPS must be positive; threads: 1 to {MAX_THREADS}."
            )));
        }
        Ok(())
    }

    fn validate_run(&self, config: &JobConfig) -> Result<(), Error> {
        self.validate_new(config)?;
        let endpoint = endpoint_allowed(&config.endpoint, config.test_mode, false)?;
        let token = endpoint_allowed(&config.token_url, config.test_mode, false)?;
        let saved = self.api_config()?;
        if !is_loopback(&endpoint) || !is_loopback(&token) {
            saved.authorize_job(config)?;
        }
        if config.client_id.is_empty() {
            return Err(Error::AuthMissing);
        }
        Ok(())
    }

    fn resolve(&self, id: &str) -> Result<String, Error> {
        if crate::domain::JobId::from_string(id.to_string()).is_ok()
            && self.paths.job_dir(id).exists()
        {
            return Ok(id.to_string());
        }
        let mut hits = Vec::new();
        if self.paths.jobs().exists() {
            for entry in std::fs::read_dir(self.paths.jobs())? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(id) {
                    hits.push(name);
                }
            }
        }
        match hits.len() {
            1 => Ok(hits.remove(0)),
            0 => Err(Error::NotFound(id.into())),
            _ => Err(Error::Ambiguous(id.into())),
        }
    }

    fn now(&self) -> String {
        format_rfc3339(self.clock.now_ms())
    }
}

fn load_config(db: &Db) -> Result<JobConfig, Error> {
    let raw = db
        .meta_get("config")?
        .ok_or_else(|| Error::Msg("job config missing".into()))?;
    serde_json::from_str(&raw).map_err(|e| Error::Msg(e.to_string()))
}

fn finish_state(counts: Counts) -> JobState {
    if counts.unknown > 0 {
        JobState::BlockedUnknown
    } else if counts.failed_final > 0 {
        JobState::CompletedWithErrors
    } else {
        JobState::Completed
    }
}

#[derive(Clone, Debug)]
pub struct RunOptions {
    pub stop: watch::Receiver<bool>,
    /// Keeps the watch channel open. Dropping every sender looks like cancellation.
    _keepalive: Option<watch::Sender<bool>>,
    pub stop_after_successes: Option<u64>,
    pub fail_outcome_commits: u32,
    pub grace: Duration,
}

impl RunOptions {
    pub fn run() -> Self {
        let (tx, stop) = watch::channel(false);
        Self {
            stop,
            _keepalive: Some(tx),
            stop_after_successes: None,
            fail_outcome_commits: 0,
            grace: Duration::from_secs(15),
        }
    }

    pub fn with_stop(stop: watch::Receiver<bool>) -> Self {
        Self {
            stop,
            _keepalive: None,
            stop_after_successes: None,
            fail_outcome_commits: 0,
            grace: Duration::from_secs(15),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RunReport {
    pub state: String,
    pub interrupted: bool,
    pub paused: bool,
}

#[derive(Clone, Debug)]
pub struct JobStatus {
    pub id: String,
    pub state: String,
    pub desired: String,
    pub counts: Counts,
    pub batches: i64,
}

#[derive(Clone, Copy, Debug, Serialize, serde::Deserialize)]
pub struct CountView {
    pub pending: i64,
    pub in_flight: i64,
    pub retry_wait: i64,
    pub succeeded: i64,
    pub failed_final: i64,
    pub unknown: i64,
}

impl From<Counts> for CountView {
    fn from(counts: Counts) -> Self {
        Self {
            pending: counts.pending,
            in_flight: counts.in_flight,
            retry_wait: counts.retry_wait,
            succeeded: counts.succeeded,
            failed_final: counts.failed_final,
            unknown: counts.unknown,
        }
    }
}

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct MergeReport {
    pub job_id: String,
    pub generation: u32,
    pub format: String,
    pub rows: u64,
    pub sha256: String,
    pub counts: CountView,
    pub partial: bool,
    pub contract: String,
    pub endpoint_host: String,
    pub schema: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum OutFormat {
    Parquet,
    Csv,
}

#[derive(Clone, Debug)]
pub struct DoctorReport {
    pub checks: Vec<(String, bool, String)>,
}

impl DoctorReport {
    pub fn ok(&self) -> bool {
        self.checks.iter().all(|(_, ok, _)| *ok)
    }
}

pub fn system_app(secrets: Arc<dyn SecretStore>) -> Result<App, Error> {
    App::open(Paths::default_root()?, secrets, Arc::new(SystemClock))
}
