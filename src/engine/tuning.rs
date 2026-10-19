use serde::{Deserialize, Serialize};

use super::{load_config, App};
use crate::disk::{acquire, publish_file};
use crate::domain::JobState;
use crate::error::Error;
use crate::store::Db;

pub const MAX_THREADS: u32 = 256;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreadDefaults {
    pub threads: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ProcessingDefaults {
    pub threads: u32,
    pub rps: f64,
    pub batch_size: u32,
    pub validation: String,
}
impl Default for ProcessingDefaults {
    fn default() -> Self {
        Self {
            threads: 8,
            rps: 10.0,
            batch_size: 1000,
            validation: "checksum".into(),
        }
    }
}
impl ProcessingDefaults {
    pub fn validate(&self) -> Result<(), Error> {
        validate_threads(self.threads)?;
        if !self.rps.is_finite() || self.rps <= 0.0 || self.rps > 100_000.0 {
            return Err(Error::Msg(
                "RPS must be greater than 0 and at most 100000.".into(),
            ));
        }
        if !(1..=100_000).contains(&self.batch_size) {
            return Err(Error::Msg(
                "Batch size must be an integer from 1 to 100000.".into(),
            ));
        }
        crate::domain::Validation::parse(&self.validation)
            .ok_or_else(|| Error::Msg("Choose a supported CPF validation mode.".into()))?;
        Ok(())
    }
}

impl Default for ThreadDefaults {
    fn default() -> Self {
        Self { threads: 8 }
    }
}

pub fn validate_threads(threads: u32) -> Result<(), Error> {
    if !(1..=MAX_THREADS).contains(&threads) {
        return Err(Error::Msg(format!(
            "Threads must be an integer from 1 to {MAX_THREADS}."
        )));
    }
    Ok(())
}

impl App {
    pub fn thread_defaults(&self) -> Result<ThreadDefaults, Error> {
        Ok(ThreadDefaults {
            threads: self.processing_defaults()?.threads,
        })
    }

    pub fn processing_defaults(&self) -> Result<ProcessingDefaults, Error> {
        let path = self.paths.root.join("processing.json");
        if !path.exists() {
            return Ok(ProcessingDefaults::default());
        }
        let settings: ProcessingDefaults =
            serde_json::from_slice(&std::fs::read(path)?).map_err(|_| {
                Error::Msg(
                    "Invalid processing defaults. Open Settings > Processing to repair them."
                        .into(),
                )
            })?;
        settings.validate()?;
        Ok(settings)
    }

    pub fn set_default_threads(&self, threads: u32) -> Result<(), Error> {
        validate_threads(threads)?;
        let mut settings = self.processing_defaults().unwrap_or_default();
        settings.threads = threads;
        self.set_processing_defaults(settings)
    }

    pub fn set_processing_defaults(&self, settings: ProcessingDefaults) -> Result<(), Error> {
        settings.validate()?;
        let temporary = self
            .paths
            .root
            .join(format!("processing-{}.tmp", uuid::Uuid::new_v4()));
        std::fs::write(
            &temporary,
            serde_json::to_vec_pretty(&settings).map_err(|e| Error::Msg(e.to_string()))?,
        )?;
        crate::error::restrict_file(&temporary)?;
        publish_file(&temporary, &self.paths.root.join("processing.json"))
    }

    /// The same lock as the runner prevents a second process from changing active workers.
    pub fn configure_threads(&self, id: &str, threads: u32) -> Result<(), Error> {
        validate_threads(threads)?;
        let id = self.resolve(id)?;
        let dir = self.paths.job_dir(&id);
        let _lock = acquire(&dir.join("run.lock"))?;
        let db = Db::open(&dir.join("job.sqlite"))?;
        if db.state()? == JobState::Running {
            return Err(Error::ThreadsBusy);
        }
        let mut config = load_config(&db)?;
        config.concurrency = threads;
        db.meta_set(
            "config",
            &serde_json::to_string(&config).map_err(|e| Error::Msg(e.to_string()))?,
        )?;
        db.push_event(&self.now(), "threads_saved", &format!("threads={threads}"))?;
        Ok(())
    }
}
