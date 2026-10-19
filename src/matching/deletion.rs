//! User-selected deletion. Stable locks outlive job directories; links are refused.
use super::{bad, read_json, Job, Service};
use crate::{
    disk::{acquire, JobLock},
    error::Error,
};
use std::path::PathBuf;

pub(super) struct Guards {
    _legacy: JobLock,
    _stable: JobLock,
}

impl Service {
    pub(super) fn finish_deletions(&self) -> Result<(), Error> {
        let trash = self.root.join("deleting");
        if !trash.exists() {
            return Ok(());
        }
        let trash = self.owned_directory("deleting")?;
        self.owned_directory("locks")?;
        for entry in std::fs::read_dir(trash)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.is_ascii()
                || name.len() != 73
                || &name[36..37] != "-"
                || uuid::Uuid::parse_str(&name[..36]).is_err()
                || uuid::Uuid::parse_str(&name[37..]).is_err()
            {
                continue;
            }
            if !entry.file_type()?.is_dir() {
                return Err(bad("Deletion stage must not be a symlink."));
            }
            match acquire(
                &self
                    .root
                    .join("locks")
                    .join(format!("{}.lock", &name[..36])),
            ) {
                Ok(_lock) => std::fs::remove_dir_all(entry.path())?,
                Err(Error::Locked) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    pub(super) fn lock_job(&self, id: &str) -> Result<Guards, Error> {
        self.job_dir(id)?;
        self.owned_directory("locks")?;
        // Lock order: stable UUID lock -> legacy run.lock. No registry mutex across I/O.
        let stable = acquire(&self.root.join("locks").join(format!("{id}.lock")))?;
        let dir = self.owned_job_dir(id)?;
        let legacy = acquire(&dir.join("run.lock"))?;
        Ok(Guards {
            _legacy: legacy,
            _stable: stable,
        })
    }
    fn owned_directory(&self, name: &str) -> Result<PathBuf, Error> {
        let path = self.root.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_dir() => {}
            Ok(_) => {
                return Err(bad(
                    "Application staging/lock directories must not be symlinks.",
                ))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => std::fs::create_dir(&path)?,
            Err(e) => return Err(e.into()),
        }
        super::private_dir(&path)?;
        Ok(path)
    }
    fn owned_job_dir(&self, id: &str) -> Result<PathBuf, Error> {
        let dir = self.job_dir(id)?;
        let jobs = self.root.join("jobs");
        for path in [&jobs, &dir] {
            if !std::fs::symlink_metadata(path)?.file_type().is_dir() {
                return Err(bad(
                    "Job directory must be owned and must not be a symlink.",
                ));
            }
        }
        if jobs.canonicalize()?.parent() != Some(self.root.canonicalize()?.as_path()) {
            return Err(bad("Job directory escapes application storage."));
        }
        let job: Job = read_json(&dir.join("job.json"))?;
        if job.id != id {
            return Err(bad("Job metadata identity mismatch."));
        }
        Ok(dir)
    }
    /// Remove only the selected stored job. Original input and external exports are untouched.
    /// Errors include unknown/ambiguous ID, a running job, symlinks and filesystem failures.
    pub fn delete_job(&self, id: &str) -> Result<(), Error> {
        let job = self.job(id)?;
        let _locks = self.lock_job(&job.id)?;
        let dir = self.owned_job_dir(&job.id)?;
        let trash = self.owned_directory("deleting")?;
        let stage = trash.join(format!("{}-{}", job.id, uuid::Uuid::new_v4()));
        std::fs::rename(&dir, &stage)?;
        self.release_job_filter(&job.id)?;
        std::fs::remove_dir_all(stage)?;
        tracing::info!(event = "matching_job_deleted", "Selected job data removed");
        Ok(())
    }
    /// Change scheduling for a prepared/paused job without changing keys, rule or filter pin.
    pub fn set_job_threads(&self, id: &str, threads: u16) -> Result<Job, Error> {
        super::validate_threads(threads)?;
        let job = self.job(id)?;
        let _locks = self.lock_job(&job.id)?;
        let mut job = self.job(&job.id)?;
        if job.state == "COMPLETED" {
            return Err(bad("Completed jobs cannot change threads."));
        }
        job.threads = threads;
        self.save_job(&job)?;
        Ok(job)
    }
}
