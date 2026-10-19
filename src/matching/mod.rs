//! Versioned, local Bloom matching. Identifiers never leave this context over HTTP.
use crate::{
    error::{Error, Secret},
    secrets::SecretStore,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

mod auth;
pub mod cli;
pub mod crypto;
pub mod dataset;
mod deletion;
mod download;
pub mod format;
mod jobs;
pub mod manual;
mod memory;
mod output;
mod parallel;
mod progress;
pub mod rules;
mod source;
pub mod ui;
pub use auth::TokenStatus;
pub use download::DownloadMode;
pub use jobs::{Job, RunControl};
pub use progress::{Phase, Progress};
pub use rules::{CsvKeys, Rule, Transform};
pub use source::Snapshot;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    #[serde(rename = "dat_url", alias = "url")]
    pub url: String,
    pub token_url: String,
    /// Deprecated read-only metadata; both OAuth grants use token_url.
    #[serde(skip_serializing)]
    pub reset_token_url: String,
    pub client_id: String,
    pub scope: String,
    pub oauth: bool,
    pub allow_http_loopback: bool,
    pub bearer: bool,
    pub max_download_mib: u64,
    pub max_memory_mib: u64,
    pub timeout_seconds: u64,
    pub download_timeout_seconds: u64,
    pub download_idle_seconds: u64,
    pub download_mode: DownloadMode,
    pub download_part_mib: u64,
    pub download_parallelism: usize,
    pub download_retries: u8,
    pub download_form: std::collections::BTreeMap<String, String>,
    pub checkpoint_rows: u64,
    pub threads: u16,
    pub rule: Rule,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            url: String::new(),
            token_url: String::new(),
            reset_token_url: String::new(),
            client_id: String::new(),
            scope: String::new(),
            oauth: false,
            allow_http_loopback: false,
            bearer: false,
            max_download_mib: 256,
            max_memory_mib: 512,
            timeout_seconds: 60,
            download_timeout_seconds: 600,
            download_idle_seconds: 60,
            download_mode: DownloadMode::Single,
            download_part_mib: 16,
            download_parallelism: 4,
            download_retries: 2,
            download_form: Default::default(),
            checkpoint_rows: 100_000,
            threads: default_threads(),
            rule: Rule::default(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), Error> {
        self.rule.validate()?;
        if self.client_id.len() > 256
            || self.client_id.trim() != self.client_id
            || self.client_id.chars().any(char::is_control)
            || self.scope.len() > 2048
            || self.scope.chars().any(char::is_control)
        {
            return Err(bad(
                "Check client ID (up to 256 characters) and scope (up to 2048 characters).",
            ));
        }
        if !self.url.is_empty() {
            self.source_url()?;
        }
        for (value, label) in [
            (&self.token_url, "Get token"),
            (&self.reset_token_url, "Reset token"),
        ] {
            if !value.is_empty() {
                self.api_url(value, label)?;
            }
        }
        validate_threads(self.threads)?;
        if !(1..=4096).contains(&self.max_download_mib)
            || !(1..=4096).contains(&self.max_memory_mib)
            || !(1..=3600).contains(&self.timeout_seconds)
            || !(1..=3600).contains(&self.download_timeout_seconds)
            || !(1..=3600).contains(&self.download_idle_seconds)
            || !(1..=128).contains(&self.download_part_mib)
            || !(1..=16).contains(&self.download_parallelism)
            || self.download_retries > 5
            || !(1..=1_000_000).contains(&self.checkpoint_rows)
        {
            return Err(bad(
                "Limits: DAT 1–4096 MiB; deadlines 1–3600 s; parts 1–128 MiB; parallelism 1–16; checkpoint 1–1000000 rows.",
            ));
        }
        download::validate_form(&self.download_form)
    }
}

#[derive(Clone)]
pub struct Service {
    pub root: PathBuf,
    secrets: Arc<dyn SecretStore>,
    auth: Arc<tokio::sync::Mutex<auth::State>>,
    resident: Arc<Mutex<source::Resident>>,
    progress: Option<progress::Reporter>,
}
impl Service {
    pub fn open(root: PathBuf, secrets: Arc<dyn SecretStore>) -> Result<Self, Error> {
        let root = root.join("matching");
        for directory in [&root, &root.join("jobs"), &root.join("filters")] {
            std::fs::create_dir_all(directory)?;
            if !std::fs::symlink_metadata(directory)?.file_type().is_dir() {
                return Err(bad("Matching storage directories must not be symlinks."));
            }
            private_dir(directory)?;
        }
        source::retire_disk_cache(&root)?;
        let service = Self {
            root,
            secrets,
            auth: Arc::new(tokio::sync::Mutex::new(auth::State::default())),
            resident: Arc::new(Mutex::new(source::Resident::default())),
            progress: None,
        };
        service.finish_deletions()?;
        Ok(service)
    }
    pub fn settings(&self) -> Result<Settings, Error> {
        let path = self.root.join("settings.json");
        if !path.exists() {
            return Ok(Settings::default());
        }
        let raw: serde_json::Value = read_json(&path)?;
        let mut value: Settings =
            serde_json::from_value(raw.clone()).map_err(|_| bad("Invalid matching settings."))?;
        if raw.get("download_timeout_seconds").is_none() {
            value.download_timeout_seconds =
                value.download_timeout_seconds.max(value.timeout_seconds);
        }
        value.validate()?;
        Ok(value)
    }
    pub fn save_settings(&self, value: &Settings) -> Result<(), Error> {
        value.validate()?;
        save_json(&self.root.join("settings.json"), value)
    }
    pub fn save_token(&self, token: &Secret) -> Result<(), Error> {
        let _lock = crate::disk::acquire(&self.root.join("source.lock"))?;
        let url = self.settings()?.source_url()?;
        self.secrets
            .put(&source::token_account(&self.root, &url), token)?;
        let mut cfg = self.settings()?;
        cfg.bearer = true;
        self.save_settings(&cfg)?;
        if !cfg.oauth {
            match std::fs::remove_file(self.root.join("current.json")) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(Error::Io(error)),
            }
        }
        Ok(())
    }
    pub fn token_present(&self) -> Result<bool, Error> {
        let cfg = self.settings()?;
        if cfg.url.is_empty() || !cfg.bearer {
            return Ok(false);
        }
        Ok(self
            .secrets
            .get(&source::token_account(&self.root, &cfg.source_url()?))?
            .is_some())
    }
}

pub(crate) fn default_threads() -> u16 {
    std::thread::available_parallelism().map_or(1, |n| n.get().min(32) as u16)
}
pub(crate) fn validate_threads(threads: u16) -> Result<(), Error> {
    if (1..=256).contains(&threads) {
        Ok(())
    } else {
        Err(bad("Matching threads must be 1-256."))
    }
}

pub(crate) fn private_dir(path: &Path) -> Result<(), Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Publish a newly requested artifact without replacing a destination created concurrently.
pub(crate) fn publish_new(stage: &Path, target: &Path) -> Result<(), Error> {
    File::open(stage)?.sync_all()?;
    std::fs::hard_link(stage, target)?;
    std::fs::remove_file(stage)?;
    if let Some(parent) = target.parent().filter(|p| !p.as_os_str().is_empty()) {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}

pub(crate) fn bad(message: &str) -> Error {
    Error::Msg(message.into())
}
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub(crate) fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Error> {
    if std::fs::metadata(path)?.len() > 1024 * 1024 {
        return Err(bad("Metadata exceeds 1 MiB."));
    }
    serde_json::from_reader(File::open(path)?).map_err(|_| bad("Invalid matching metadata."))
}
pub(crate) fn save_json(path: &Path, value: &impl Serialize) -> Result<(), Error> {
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|_| bad("Cannot encode matching metadata."))?;
    let tmp = path.with_extension(format!("{}.partial", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = File::create(&tmp)?;
        crate::error::restrict_file(&tmp)?;
        file.write_all(&bytes)?;
        crate::disk::publish_file(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(tmp);
    }
    result
}
