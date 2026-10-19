use super::parallel::{Pool, Row, WAVE_ROWS};
use super::progress::{Phase, Reporter};
use super::{
    bad, dataset::Dataset, format::Bloom, hex, read_json, rules::output_key, save_json, Rule,
    Service, Snapshot,
};
use crate::{error::Error, prepare::sha256_file};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::watch;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub state: String,
    pub input_name: String,
    pub input_sha256: String,
    pub rule: Rule,
    pub filter: Option<String>,
    pub total: u64,
    pub processed: u64,
    pub positives: u64,
    pub negatives: u64,
    pub byte: u64,
    pub line: u64,
    pub record: u64,
    pub output_bytes: u64,
    pub output_sha256: String,
    pub created_at: u64,
    pub error: Option<String>,
    #[serde(default = "legacy_threads")]
    pub threads: u16,
    /// Absolute primary destination; absent only on legacy/library-internal jobs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<PathBuf>,
}
fn legacy_threads() -> u16 {
    1
}
pub struct RunControl {
    pub stop: watch::Receiver<bool>,
    /// Optional deterministic row budget; reaching it pauses at a committed checkpoint.
    pub pause_after: Option<u64>,
    pub threads: Option<u16>,
    _keepalive: Option<watch::Sender<bool>>,
}
impl Default for RunControl {
    fn default() -> Self {
        let (tx, rx) = watch::channel(false);
        Self {
            stop: rx,
            pause_after: None,
            threads: None,
            _keepalive: Some(tx),
        }
    }
}
impl RunControl {
    pub fn with_stop(stop: watch::Receiver<bool>) -> Self {
        Self {
            stop,
            pause_after: None,
            threads: None,
            _keepalive: None,
        }
    }
}
impl Service {
    pub fn create_job(&self, input: &Path, rule: Rule) -> Result<Job, Error> {
        self.create_job_cancellable(input, rule, &RunControl::default().stop)
    }
    pub fn create_job_cancellable(
        &self,
        input: &Path,
        rule: Rule,
        stop: &watch::Receiver<bool>,
    ) -> Result<Job, Error> {
        self.prepare_job(input, rule, stop, None)
    }
    /// Pin a primary output file/directory before importing; existing files are refused.
    pub fn create_job_cancellable_to(
        &self,
        input: &Path,
        rule: Rule,
        stop: &watch::Receiver<bool>,
        output: &Path,
    ) -> Result<Job, Error> {
        self.prepare_job(input, rule, stop, Some(output))
    }
    fn prepare_job(
        &self,
        input: &Path,
        rule: Rule,
        stop: &watch::Receiver<bool>,
        output: Option<&Path>,
    ) -> Result<Job, Error> {
        rule.validate()?;
        let id = uuid::Uuid::new_v4().to_string();
        let output_path = output
            .map(|path| super::output::target(&id, Some(path)))
            .transpose()?;
        if let Some(path) = &output_path {
            match std::fs::symlink_metadata(path) {
                Ok(_) => return Err(bad("Output already exists; choose a new path.")),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        let dir = self.root.join("jobs").join(&id);
        std::fs::create_dir(&dir)?;
        super::private_dir(&dir)?;
        let result = (|| {
            if let Some(r) = &self.progress {
                r.job(&id);
            }
            let hash = copy_input(input, &dir.join("input.csv"), stop, self.progress.as_ref())?;
            self.phase(Phase::ScanningInput, 0, None);
            let mut keys = Dataset::open(&dir.join("input.csv"), &rule)?;
            let mut total: u64 = 0;
            while let Some(key) = keys.next_key()? {
                if *stop.borrow() {
                    return Err(bad("Import cancelled."));
                }
                if rule.lookup(&key).is_empty() {
                    return Err(bad("Dataset contains an empty transformed key."));
                }
                total += 1;
                if total.is_multiple_of(WAVE_ROWS as u64) {
                    self.count(total, None);
                }
            }
            self.count(total, Some(total));
            let job = Job {
                id,
                state: "PREPARED".into(),
                input_name: input
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
                input_sha256: hash,
                rule,
                filter: None,
                total,
                processed: 0,
                positives: 0,
                negatives: 0,
                byte: 0,
                line: 1,
                record: 0,
                output_bytes: 0,
                output_sha256: hex(&Sha256::digest([])),
                created_at: now(),
                error: None,
                threads: self.settings()?.threads,
                output_path,
            };
            self.save_job(&job)?;
            tracing::info!(
                event = "matching_job_created",
                rows = total,
                "Local job prepared"
            );
            Ok(job)
        })();
        if result.is_err() {
            let _ = std::fs::remove_dir_all(&dir);
        }
        result
    }
    pub fn jobs(&self) -> Result<Vec<Job>, Error> {
        let mut jobs = Vec::new();
        for entry in std::fs::read_dir(self.root.join("jobs"))? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if uuid::Uuid::parse_str(&name).is_err() {
                continue;
            }
            let path = entry.path().join("job.json");
            if path.exists() && std::fs::symlink_metadata(&path)?.file_type().is_file() {
                let job = read_json::<Job>(&path)?;
                if job.id != name {
                    return Err(bad("Job metadata identity mismatch."));
                }
                jobs.push(job);
            }
        }
        jobs.sort_by_key(|job| std::cmp::Reverse((job.created_at, job.id.clone())));
        Ok(jobs)
    }
    pub fn job(&self, id: &str) -> Result<Job, Error> {
        if id.is_empty() || id.len() > 36 || !id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
        {
            return Err(bad("Use a job UUID or an unambiguous UUID prefix."));
        }
        let jobs = self.jobs()?;
        let found = jobs
            .into_iter()
            .filter(|job| job.id.starts_with(id))
            .collect::<Vec<_>>();
        match found.len() {
            0 => Err(bad("Matching job not found.")),
            1 => Ok(found.into_iter().next().unwrap()),
            _ => Err(bad("Matching job prefix is ambiguous.")),
        }
    }
    pub(crate) fn job_dir(&self, id: &str) -> Result<PathBuf, Error> {
        uuid::Uuid::parse_str(id).map_err(|_| bad("Invalid matching job ID."))?;
        Ok(self.root.join("jobs").join(id))
    }
    pub(crate) fn save_job(&self, job: &Job) -> Result<(), Error> {
        save_json(&self.job_dir(&job.id)?.join("job.json"), job)
    }
    pub fn result_path(&self, id: &str) -> Result<PathBuf, Error> {
        let job = self.job(id)?;
        if let Some(path) = job.output_path {
            if !path.is_absolute() {
                return Err(bad("Job output destination must be absolute."));
            }
            return Ok(path);
        }
        Ok(std::path::absolute(
            self.job_dir(&job.id)?.join("result.txt"),
        )?)
    }
    pub fn verify(&self, id: &str) -> Result<bool, Error> {
        let job = self.job(id)?;
        let dir = self.job_dir(&job.id)?;
        if sha256_file(&dir.join("input.csv"))? != job.input_sha256 {
            return Ok(false);
        }
        let output = if job.state == "COMPLETED" {
            self.result_path(&job.id)?
        } else {
            dir.join("result.partial")
        };
        if job.state != "COMPLETED" && job.processed == 0 && !output.exists() {
            return Ok(true);
        }
        Ok(sha256_file(&output)? == job.output_sha256)
    }
    pub async fn run_job(
        &self,
        id: &str,
        snapshot: Option<Snapshot>,
        control: RunControl,
    ) -> Result<Job, Error> {
        let job = self.job(id)?;
        if let Some(r) = &self.progress {
            r.job(&job.id);
        }
        let _locks = self.lock_job(&job.id)?;
        let owner = job.id.clone();
        let result = self.run_locked(job, snapshot, control).await;
        if !result.as_ref().is_ok_and(|job| job.state == "PAUSED") {
            self.release_job_filter(&owner)?;
            self.discard_pending()?;
        }
        result
    }
    async fn run_locked(
        &self,
        mut job: Job,
        snapshot: Option<Snapshot>,
        control: RunControl,
    ) -> Result<Job, Error> {
        if let Some(threads) = control.threads {
            super::validate_threads(threads)?;
            job.threads = threads;
        }
        super::validate_threads(job.threads)?;
        if job.state == "COMPLETED" {
            if self.verify(&job.id)? {
                return Ok(job);
            }
            return Err(bad("Completed job checksum mismatch."));
        }
        let dir = self.job_dir(&job.id)?;
        let result_path = self.result_path(&job.id)?;
        if result_path.exists() {
            if job.processed != job.total
                || sha256_file(&dir.join("input.csv"))? != job.input_sha256
                || sha256_file(&result_path)? != job.output_sha256
            {
                return Err(bad("Published result or input checksum mismatch."));
            }
            job.state = "COMPLETED".into();
            self.save_job(&job)?;
            return Ok(job);
        }
        let loaded = if let Some(resident) = self.job_filter(&job.id)? {
            resident
        } else if let Some(pinned) = job.filter.as_deref() {
            if let Some(snapshot) = snapshot.filter(|s| s.bloom.info().sha256 == pinned) {
                snapshot
            } else {
                if let Some(staged) = self.cached(Some(pinned))? {
                    staged
                } else {
                    self.download(control.stop.clone()).await?
                }
            }
        } else if let Some(snapshot) = snapshot {
            snapshot
        } else if let Some(cached) = self.cached(None)? {
            cached
        } else {
            self.download(control.stop.clone()).await?
        };
        let loaded = self.bind_filter(&job.id, loaded)?;
        if job
            .filter
            .as_ref()
            .is_some_and(|pin| *pin != loaded.bloom.info().sha256)
        {
            return Err(bad("Downloaded DAT differs from the job pin; restore the exact snapshot at the source or create a new job."));
        }
        loaded.bloom.compatible(&job.rule)?;
        if loaded.bloom.info().bytes > self.settings()?.max_memory_mib * 1024 * 1024 {
            return Err(bad("DAT exceeds the configured memory limit."));
        }
        if loaded.bloom.info().sha256.len() != 64 {
            return Err(bad("Use a verified DAT snapshot."));
        }
        self.phase(Phase::VerifyingInput, 0, None);
        if sha256_file(&self.job_dir(&job.id)?.join("input.csv"))? != job.input_sha256 {
            return Err(bad("Input checksum mismatch."));
        }
        job.filter = Some(loaded.bloom.info().sha256.clone());
        job.state = "RUNNING".into();
        job.error = None;
        self.save_job(&job)?;
        let owned = self.clone();
        let failed_id = job.id.clone();
        // Coordinator exits on EOF/error/cancellation; its bounded CPU pool always joins on drop.
        let result = tokio::task::spawn_blocking(move || owned.process(job, loaded.bloom, control))
            .await
            .map_err(|_| bad("Matching worker interrupted."))?;
        if let Err(error) = &result {
            let mut failed = self.job(&failed_id)?;
            failed.state = "PREPARE_FAILED".into();
            failed.error =
                Some("Matching failed; repair the input/filter/output and retry.".into());
            self.save_job(&failed)?;
            tracing::error!(
                event = "matching_job_failed",
                reason = "local_io_or_validation",
                "Matching stopped without publishing results"
            );
            let _ = error;
        }
        result
    }
    fn process(&self, mut job: Job, bloom: Arc<Bloom>, control: RunControl) -> Result<Job, Error> {
        let started = std::time::Instant::now();
        let dir = self.job_dir(&job.id)?;
        let result_path = self.result_path(&job.id)?;
        if result_path.exists() {
            if job.processed != job.total {
                return Err(bad("An unexpected result file already exists."));
            }
            if sha256_file(&result_path)? != job.output_sha256 {
                return Err(bad("Output checksum mismatch."));
            }
            job.state = "COMPLETED".into();
            self.save_job(&job)?;
            return Ok(job);
        }
        let mut input = Dataset::open(&dir.join("input.csv"), &job.rule)?;
        if job.processed > 0 {
            input.seek(job.byte, job.line, job.record)?;
        }
        self.phase(Phase::VerifyingCheckpoint, 0, Some(job.output_bytes));
        let (file, mut hash) =
            output_prefix(&dir.join("result.partial"), &job, self.progress.as_ref())?;
        let mut output = BufWriter::new(file);
        let start = job.processed;
        let checkpoint_rows = self.settings()?.checkpoint_rows;
        let pool = Pool::new(job.threads, Arc::clone(&bloom), job.rule.clone())?;
        self.phase(Phase::Matching, job.processed, Some(job.total));
        loop {
            if *control.stop.borrow()
                || control
                    .pause_after
                    .is_some_and(|n| job.processed - start >= n)
            {
                checkpoint(self, &mut job, &mut output, &hash)?;
                job.state = "PAUSED".into();
                self.save_job(&job)?;
                return Ok(job);
            }
            let budget = control.pause_after.map_or(WAVE_ROWS as u64, |n| {
                n.saturating_sub(job.processed - start)
                    .min(WAVE_ROWS as u64)
            }) as usize;
            let mut rows = Vec::with_capacity(budget);
            for _ in 0..budget {
                let Some(key) = input.next_key()? else {
                    break;
                };
                rows.push(Row {
                    key,
                    position: input.position(),
                    matched: false,
                });
            }
            if rows.is_empty() {
                break;
            }
            for chunk in pool.evaluate(rows)? {
                for row in chunk {
                    let line = format!("{}: {}\n", output_key(&row.key), row.matched);
                    output.write_all(line.as_bytes())?;
                    hash.update(line.as_bytes());
                    job.output_bytes += line.len() as u64;
                    job.processed += 1;
                    job.positives += u64::from(row.matched);
                    job.negatives += u64::from(!row.matched);
                    job.byte = row.position.byte();
                    job.line = row.position.line();
                    job.record = row.position.record();
                    if job.processed.is_multiple_of(checkpoint_rows) {
                        checkpoint(self, &mut job, &mut output, &hash)?;
                    }
                }
            }
            self.count(job.processed, Some(job.total));
        }
        if job.processed != job.total {
            return Err(bad("Dataset row count differs from the prepared job."));
        }
        self.phase(Phase::Publishing, job.processed, Some(job.total));
        checkpoint(self, &mut job, &mut output, &hash)?;
        drop(output);
        super::output::publish(&dir.join("result.partial"), &result_path)?;
        job.state = "COMPLETED".into();
        self.save_job(&job)?;
        tracing::info!(
            event = "matching_job_completed",
            rows = job.processed,
            threads = job.threads,
            positives = job.positives,
            negatives = job.negatives,
            elapsed_ms = started.elapsed().as_millis() as u64,
            "Local matching completed"
        );
        Ok(job)
    }
}
fn copy_input(
    source: &Path,
    target: &Path,
    stop: &watch::Receiver<bool>,
    progress: Option<&Reporter>,
) -> Result<String, Error> {
    let mut input = File::open(source)?;
    if !input.metadata()?.is_file() {
        return Err(bad("Dataset must be a regular file."));
    }
    let mut output = File::create(target)?;
    let total = input.metadata()?.len();
    if let Some(r) = progress {
        r.phase(Phase::CopyingInput, 0, Some(total));
    }
    let mut copied = 0;
    crate::error::restrict_file(target)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        if *stop.borrow() {
            return Err(bad("Import cancelled."));
        }
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
        output.write_all(&buffer[..n])?;
        copied += n as u64;
        if let Some(r) = progress {
            r.count(copied, Some(total));
        }
    }
    output.sync_all()?;
    Ok(hex(&hash.finalize()))
}
fn output_prefix(
    path: &Path,
    job: &Job,
    progress: Option<&Reporter>,
) -> Result<(File, Sha256), Error> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    crate::error::restrict_file(path)?;
    if file.metadata()?.len() < job.output_bytes {
        return Err(bad("Output prefix is truncated."));
    }
    let mut hash = Sha256::new();
    let mut remaining = job.output_bytes;
    let mut buffer = [0; 65536];
    while remaining > 0 {
        let n = (remaining as usize).min(buffer.len());
        file.read_exact(&mut buffer[..n])?;
        hash.update(&buffer[..n]);
        remaining -= n as u64;
        if let Some(r) = progress {
            r.count(job.output_bytes - remaining, Some(job.output_bytes));
        }
    }
    if hex(&hash.clone().finalize()) != job.output_sha256 {
        return Err(bad("Committed output prefix checksum mismatch."));
    }
    file.set_len(job.output_bytes)?;
    file.seek(SeekFrom::Start(job.output_bytes))?;
    Ok((file, hash))
}
fn checkpoint(
    service: &Service,
    job: &mut Job,
    output: &mut BufWriter<File>,
    hash: &Sha256,
) -> Result<(), Error> {
    output.flush()?;
    output.get_ref().sync_all()?;
    job.output_sha256 = hex(&hash.clone().finalize());
    service.save_job(job)
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
