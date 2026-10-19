use std::fmt;
use std::path::PathBuf;

use thiserror::Error;
use zeroize::Zeroize;

use crate::domain::DomainError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid job id")]
    BadId,
    #[error("unknown contract mode")]
    BadContract,
    #[error("illegal transition for {entity}: {from} + {event}")]
    IllegalTransition {
        entity: &'static str,
        from: String,
        event: String,
    },
    #[error("job not found: {0}")]
    NotFound(String),
    #[error("job id prefix is ambiguous: {0}")]
    Ambiguous(String),
    #[error("this job is already running in another process")]
    Locked,
    #[error("Job is running. Pause and wait before changing threads.")]
    ThreadsBusy,
    #[error("Wait for the local operation before starting another.")]
    LocalBusy,
    #[error("Job is {state}; there is no run to pause.")]
    JobNotRunning { state: String },
    #[error("remote processing is disabled while the provider contract is pending")]
    ContractPending,
    #[error("plain HTTP is allowed only for a loopback host in explicit test mode")]
    InsecureUrl,
    #[error("the URL must not carry a username, password, or secret query")]
    UrlSecret,
    #[error("idempotency keys are sent only in test mode against loopback")]
    IdempotencyGate,
    #[error("input checksum does not match the job")]
    ChecksumMismatch,
    #[error("schema version {found} is not supported (this binary understands {supported})")]
    Schema { found: i32, supported: i32 },
    #[error("refusing WAL on filesystem type {kind}")]
    NetworkFs { kind: String },
    #[error("disk write failed; new sends are stopped and the last commit is kept")]
    DiskFull,
    #[error("free space is {free_mib} MiB; need at least {need_mib} MiB")]
    LowSpace { free_mib: u64, need_mib: u64 },
    #[error("{unknown} records are UNKNOWN; pass --allow-unknown to export a partial file")]
    UnknownBlocks { unknown: i64 },
    #[error("{open} records are still open; merge waits until every row is terminal")]
    NotTerminal { open: i64 },
    #[error("row_id sequence is not contiguous at {row_id}")]
    RowGap { row_id: i64 },
    #[error("export generation {generation} is already published")]
    GenerationPublished { generation: u32 },
    #[error("pass --yes to delete this job directory")]
    ConfirmRequired,
    #[error("authentication is not configured")]
    AuthMissing,
    #[error("oauth token was rejected: {0}")]
    TokenInvalid(&'static str),
    #[error("authentication failed; new sends are paused")]
    AuthPaused,
    #[error("the terminal is not interactive")]
    NotATty,
    #[error("unclosed quote in command")]
    UnclosedQuote,
    #[error("{0}")]
    Command(String),
    #[error("csv: {0}")]
    Csv(String),
    #[error("sqlite: {0}")]
    Sqlite(String),
    #[error("http: {0}")]
    Http(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}

impl From<DomainError> for Error {
    fn from(value: DomainError) -> Self {
        match value {
            DomainError::BadId => Self::BadId,
            DomainError::BadContract => Self::BadContract,
            DomainError::BadState => Self::Msg("unknown state".into()),
            DomainError::IllegalTransition {
                entity,
                from,
                event,
            } => Self::IllegalTransition {
                entity,
                from,
                event,
            },
        }
    }
}

pub fn map_sql(err: rusqlite::Error) -> Error {
    let text = err.to_string();
    if text.contains("disk is full") || text.contains("SQLITE_FULL") {
        Error::DiskFull
    } else {
        Error::Sqlite(text)
    }
}

/// Secret bytes. `Debug` never prints them. Drop zeroes the buffer.
pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Result<Self, Error> {
        if value.is_empty() || value.len() > 8192 || value.chars().any(|c| c.is_control()) {
            return Err(Error::TokenInvalid("secret rejected"));
        }
        Ok(Self(value))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl Clone for Secret {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([redacted])")
    }
}

#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn default_root() -> Result<PathBuf, Error> {
        if let Ok(override_root) = std::env::var("PGM_DATA_HOME") {
            if !override_root.is_empty() {
                return Ok(PathBuf::from(override_root));
            }
        }
        let home = std::env::var("HOME").map_err(|_| Error::Msg("HOME is not set".into()))?;
        if cfg!(target_os = "macos") {
            Ok(PathBuf::from(home).join("Library/Application Support/pgm-data"))
        } else {
            let base = std::env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(home).join(".local/share"));
            Ok(base.join("pgm-data"))
        }
    }

    pub fn ensure(&self) -> Result<(), Error> {
        std::fs::create_dir_all(self.jobs())?;
        std::fs::create_dir_all(self.root.join("logs"))?;
        Ok(())
    }

    pub fn jobs(&self) -> PathBuf {
        self.root.join("jobs")
    }

    pub fn job_dir(&self, id: &str) -> PathBuf {
        self.jobs().join(id)
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.json")
    }

    pub fn session_file(&self) -> PathBuf {
        self.root.join("session.json")
    }

    pub fn history_file(&self) -> PathBuf {
        self.root.join("repl-history.txt")
    }

    pub fn log_file(&self) -> PathBuf {
        self.root.join("logs").join("pgm-data.jsonl")
    }
}

pub fn restrict_file(path: &std::path::Path) -> Result<(), Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}
