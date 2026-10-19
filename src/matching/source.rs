use super::progress::Phase;
use super::{bad, format::Bloom, hex, Service, Settings};
use crate::{disk::acquire, error::Error};
use reqwest::Url;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path, sync::Arc};

pub const CANCELLED: &str = "Operation cancelled.";
#[derive(Clone)]
pub struct Snapshot {
    pub bloom: Arc<Bloom>,
}
#[derive(Default)]
pub(super) struct Resident {
    pub(super) dat_key: Option<(String, Arc<super::crypto::DatKey>)>,
    pending: Option<(String, Snapshot)>,
    jobs: HashMap<String, (String, Snapshot)>,
}

pub(super) fn retire_disk_cache(root: &Path) -> Result<(), Error> {
    let marker = root.join("ram-only-v1.json");
    if marker.exists() {
        if !std::fs::symlink_metadata(&marker)?.file_type().is_file() {
            return Err(bad("Invalid DAT memory-policy marker."));
        }
        let version: u8 = super::read_json(&marker)?;
        if version != 1 {
            return Err(bad("Unsupported DAT memory policy."));
        }
        return Ok(());
    }
    let _lock = acquire(&root.join("source.lock"))?;
    for entry in std::fs::read_dir(root.join("filters"))? {
        let entry = entry?;
        let path = entry.path();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let owned = match path.extension().and_then(|s| s.to_str()) {
            Some("dat") => stem.len() == 64 && stem.bytes().all(|b| b.is_ascii_hexdigit()),
            Some("partial") => uuid::Uuid::parse_str(stem).is_ok(),
            _ => false,
        };
        if owned {
            if !entry.file_type()?.is_file() {
                return Err(bad(
                    "Legacy DAT cache contains a link or unexpected directory.",
                ));
            }
            std::fs::remove_file(path)?;
        }
    }
    match std::fs::remove_file(root.join("current.json")) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    super::save_json(&marker, &1u8)
}
impl Settings {
    pub(crate) fn source_url(&self) -> Result<Url, Error> {
        self.api_url(&self.url, "DAT")
    }
    pub(crate) fn api_url(&self, value: &str, label: &str) -> Result<Url, Error> {
        if value.len() > 4096 || value.chars().any(char::is_control) {
            return Err(bad(&format!(
                "{label} URL is invalid; edit Settings > API."
            )));
        }
        let url = Url::parse(value)
            .map_err(|_| bad(&format!("Configure a valid {label} URL in Settings > API.")))?;
        let loopback = matches!(
            url.host_str(),
            Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
        );
        if (url.scheme() != "https"
            && !(self.allow_http_loopback && loopback && url.scheme() == "http"))
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(bad(&format!("{label} URL requires HTTPS, without credentials/query/fragment; HTTP is loopback-test only.")));
        }
        if url.path().contains("://") || url.path().to_ascii_lowercase().contains("%3a%2f%2f") {
            return Err(bad(&format!(
                "{label} URL contains another URL; enter one complete endpoint URL."
            )));
        }
        Ok(url)
    }
}
pub(crate) fn token_account(root: &Path, url: &Url) -> String {
    let mut hash = Sha256::new();
    hash.update(root.to_string_lossy().as_bytes());
    hash.update([0]);
    hash.update(url.as_str());
    format!("bloom-bearer-{}", hex(&hash.finalize()))
}
impl Service {
    /// Return only the next unclaimed RAM download; never read a DAT from disk.
    pub fn cached(&self, id: Option<&str>) -> Result<Option<Snapshot>, Error> {
        let cfg = self.settings()?;
        let source = self.resident_identity(&cfg)?;
        let state = self
            .resident
            .lock()
            .map_err(|_| bad("Resident DAT registry unavailable."))?;
        Ok(state
            .pending
            .as_ref()
            .filter(|(profile, s)| {
                profile == &source && id.is_none_or(|id| s.bloom.info().sha256 == id)
            })
            .map(|(_, s)| s.clone()))
    }
    pub(super) fn resident_identity(&self, cfg: &Settings) -> Result<String, Error> {
        if cfg.url.is_empty() {
            Ok("local".into())
        } else {
            self.source_identity(cfg)
        }
    }
    /// Stage a verified snapshot for one job. Replacing an unused preload releases it.
    pub fn retain(&self, snapshot: Snapshot) -> Result<Snapshot, Error> {
        let cfg = self.settings()?;
        let source = self.resident_identity(&cfg)?;
        let id = &snapshot.bloom.info().sha256;
        if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(bad("Use a verified DAT snapshot."));
        }
        let mut state = self
            .resident
            .lock()
            .map_err(|_| bad("Resident DAT registry unavailable."))?;
        let bytes = state
            .jobs
            .values()
            .map(|(_, s)| s.bloom.info().bytes)
            .sum::<u64>();
        if bytes + snapshot.bloom.info().bytes > cfg.max_memory_mib * 1024 * 1024 {
            return Err(bad("Resident DAT budget is full. Finish or delete paused jobs before loading another DAT."));
        }
        state.pending = Some((source, snapshot.clone()));
        Ok(snapshot)
    }
    pub(super) fn job_filter(&self, id: &str) -> Result<Option<Snapshot>, Error> {
        let state = self
            .resident
            .lock()
            .map_err(|_| bad("Resident DAT registry unavailable."))?;
        Ok(state.jobs.get(id).map(|(_, s)| s.clone()))
    }
    pub(super) fn bind_filter(&self, id: &str, snapshot: Snapshot) -> Result<Snapshot, Error> {
        let cfg = self.settings()?;
        let source = self.resident_identity(&cfg)?;
        let mut state = self
            .resident
            .lock()
            .map_err(|_| bad("Resident DAT registry unavailable."))?;
        if state
            .jobs
            .iter()
            .any(|(owner, (_, s))| owner != id && Arc::ptr_eq(&s.bloom, &snapshot.bloom))
        {
            return Err(bad(
                "DAT belongs to another job; download a fresh DAT for this job.",
            ));
        }
        let bytes = state
            .jobs
            .iter()
            .filter(|(owner, _)| *owner != id)
            .map(|(_, (_, s))| s.bloom.info().bytes)
            .sum::<u64>();
        if bytes + snapshot.bloom.info().bytes > cfg.max_memory_mib * 1024 * 1024 {
            return Err(bad("Resident DAT budget exceeded."));
        }
        if state
            .pending
            .as_ref()
            .is_some_and(|(_, s)| Arc::ptr_eq(&s.bloom, &snapshot.bloom))
        {
            state.pending = None;
        }
        state.jobs.insert(id.into(), (source, snapshot.clone()));
        Ok(snapshot)
    }
    pub(super) fn release_job_filter(&self, id: &str) -> Result<(), Error> {
        self.resident
            .lock()
            .map_err(|_| bad("Resident DAT registry unavailable."))?
            .jobs
            .remove(id);
        Ok(())
    }
    pub(super) fn discard_pending(&self) -> Result<(), Error> {
        self.resident
            .lock()
            .map_err(|_| bad("Resident DAT registry unavailable."))?
            .pending = None;
        Ok(())
    }
    /// End a session after joining active workers. Each final mapping is wiped on drop.
    pub fn clear_filters(&self) -> Result<(), Error> {
        let mut state = self
            .resident
            .lock()
            .map_err(|_| bad("Resident DAT registry unavailable."))?;
        state.jobs.clear();
        state.pending = None;
        Ok(())
    }
    pub async fn download(
        &self,
        stop: tokio::sync::watch::Receiver<bool>,
    ) -> Result<Snapshot, Error> {
        let cfg = self.settings()?;
        cfg.validate()?;
        let url = cfg.source_url()?;
        let key = self.dat_key()?;
        let encrypted = key.is_some();
        let _lock = acquire(&self.root.join("source.lock"))?;
        let wire = tokio::select! {
            result=async { if cfg.oauth { self.phase(Phase::RequestingToken,0,None); self.fresh_job_token(&cfg).await?; } self.transfer(&cfg,&url).await }=>result?,
            _=cancelled(stop.clone())=>return Err(bad(CANCELLED)),
        };
        let memory = cfg.max_memory_mib * 1024 * 1024;
        // Decoder is always joined, including cancellation. Wire and bits never have a file backing.
        let progress = self.progress.clone();
        let bloom = tokio::task::spawn_blocking(move || {
            match key {
                Some(key) => {
                    if let Some(r) = &progress {
                        r.phase(Phase::DecryptingDat, 0, None);
                    }
                    let plain = key.decrypt(wire)?;
                    if let Some(r) = &progress {
                        r.phase(Phase::VerifyingBloom, 0, None);
                    }
                    Bloom::decode(plain, memory)
                }
                None => {
                    // Explicit legacy library/plaintext mode.
                    if let Some(r) = &progress {
                        r.phase(Phase::VerifyingBloom, 0, None);
                    }
                    Bloom::decode(wire, memory)
                }
            }
        })
        .await
        .map_err(|_| bad("DAT decoder interrupted."))??;
        if *stop.borrow() {
            return Err(bad(CANCELLED));
        }
        tracing::info!(
            event = "bloom_loaded",
            encrypted,
            bytes = bloom.info().bytes,
            rows = bloom.info().inserted,
            fpp = bloom.info().target_fpp,
            "DAT verified and loaded"
        );
        self.retain(Snapshot {
            bloom: Arc::new(bloom),
        })
    }
}
pub(crate) async fn cancelled(mut stop: tokio::sync::watch::Receiver<bool>) {
    if *stop.borrow() {
        return;
    }
    let _ = stop.wait_for(|value| *value).await;
}
