//! Bounded HTTP transport. All assembled DAT pages stay locked in RAM.
use super::{auth::Access, bad, memory::Download, Service, Settings};
use crate::error::Error;
use futures_util::{stream, StreamExt};
use reqwest::{header, Client, Response, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::atomic::AtomicU8, time::Duration};
use tokio::time::{timeout_at, Instant};

mod recovery;
use super::progress::Reporter;
use recovery::{RetryDelay, Segment};

#[cfg(test)]
mod recovery_tests;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Default, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadMode {
    #[default]
    Single,
    Parallel,
    Multipart,
}
impl DownloadMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Single => "Single GET",
            Self::Parallel => "Parallel GET · HTTP Range",
            Self::Multipart => "Multipart POST · form-data",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Single => Self::Parallel,
            Self::Parallel => Self::Multipart,
            Self::Multipart => Self::Single,
        }
    }
}
pub(super) fn validate_form(form: &BTreeMap<String, String>) -> Result<(), Error> {
    if form.len() > 16
        || form.iter().any(|(name, value)| {
            name.is_empty()
                || name.len() > 64
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
                || value.len() > 2048
                || value.chars().any(char::is_control)
        })
        || form.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>() > 4096
    {
        return Err(bad("Multipart: up to 16 non-secret text fields, 4096 bytes total; names use letters/digits/_.-, values up to 2048 bytes without control characters."));
    }
    Ok(())
}
fn multipart(form: &BTreeMap<String, String>) -> (String, String) {
    let boundary = format!("pgm-{}", uuid::Uuid::new_v4().simple());
    let mut body = String::new();
    for (name, value) in form {
        body.push_str(&format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
        ));
    }
    body.push_str(&format!("--{boundary}--\r\n"));
    (format!("multipart/form-data; boundary={boundary}"), body)
}
impl Service {
    pub(super) async fn transfer(&self, cfg: &Settings, url: &Url) -> Result<Download, Error> {
        let started = Instant::now();
        if let Some(r) = &self.progress {
            r.download_start(cfg.download_retries);
        }
        let transport = Transport {
            cfg,
            url,
            client: dat_client(cfg)?,
            deadline: started + Duration::from_secs(cfg.download_timeout_seconds),
            retries: AtomicU8::new(0),
            progress: self.progress.clone(),
        };
        let probe = (cfg.download_mode == DownloadMode::Parallel).then_some((0, 0));
        let mut attempt = 0;
        let (response, access) = loop {
            let access = timeout_at(transport.deadline, self.authorization(cfg, url))
                .await
                .map_err(|_| bad("DAT total deadline exceeded during authentication."))??;
            let response = transport.send(access.as_ref(), probe, None).await?;
            if response.status() == StatusCode::UNAUTHORIZED && cfg.oauth && attempt == 0 {
                if let Some(access) = access {
                    self.invalidate_access(access.generation).await;
                }
                attempt += 1;
                continue;
            }
            if cfg.download_mode != DownloadMode::Multipart
                && matches!(response.status().as_u16(), 429 | 502 | 503 | 504)
            {
                let status = response.status();
                let delay = RetryDelay::from_response(&response);
                drop(response);
                if transport.retry("http_status", Some(status), delay).await? {
                    continue;
                }
                return Err(status_error(status));
            }
            check_status(&response)?;
            break (response, access);
        };
        tracing::info!(
            event = "dat_headers",
            elapsed_ms = started.elapsed().as_millis() as u64,
            status = response.status().as_u16(),
            protocol = ?response.version(),
            mode = cfg.download_mode.label(),
            range_support = response
                .headers()
                .get(header::ACCEPT_RANGES)
                .is_some_and(|v| v == "bytes"),
            etag_present = response.headers().contains_key(header::ETAG),
            "DAT response headers received"
        );
        let limit = cfg.max_download_mib * 1024 * 1024;
        let wire = if probe.is_some() && response.status() == StatusCode::PARTIAL_CONTENT {
            transport.parts(response, access.as_ref(), limit).await?
        } else {
            if response.status() != StatusCode::OK {
                return Err(bad("DAT full download requires HTTP 200."));
            }
            if probe.is_some() {
                tracing::info!(
                    event = "dat_range_ignored",
                    "Server ignores Range; using this single response"
                );
            }
            if let Some(r) = &self.progress {
                r.total(response.content_length());
            }
            transport
                .body(response, limit, None, access.as_ref(), true)
                .await?
        };
        tracing::info!(
            event = "dat_received",
            bytes = wire.len(),
            elapsed_ms = started.elapsed().as_millis() as u64,
            "DAT received in locked memory"
        );
        Ok(wire)
    }
}
fn dat_client(cfg: &Settings) -> Result<Client, Error> {
    // Large provider bodies repeatedly reset HTTP/2 streams. Keep OAuth's client
    // separate and preserve TLS verification, redirect refusal and all bounds.
    Client::builder()
        .http1_only()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(cfg.timeout_seconds))
        .build()
        .map_err(|_| bad("Cannot initialize DAT HTTP client."))
}
struct Transport<'a> {
    cfg: &'a Settings,
    url: &'a Url,
    client: Client,
    deadline: Instant,
    // Existing concurrent part futures share this bound; no lock/task is added.
    retries: AtomicU8,
    progress: Option<Reporter>,
}
impl Transport<'_> {
    async fn send(
        &self,
        access: Option<&Access>,
        range: Option<(u64, u64)>,
        etag: Option<&header::HeaderValue>,
    ) -> Result<Response, Error> {
        if let Some(r) = &self.progress {
            r.waiting(self.cfg.download_mode == DownloadMode::Parallel);
        }
        let mut request = if self.cfg.download_mode == DownloadMode::Multipart {
            let (content_type, body) = multipart(&self.cfg.download_form);
            self.client
                .post(self.url.clone())
                .header(header::CONTENT_TYPE, content_type)
                .body(body)
        } else {
            self.client.get(self.url.clone())
        };
        request = request
            .header(header::ACCEPT, "application/octet-stream")
            .header(header::ACCEPT_ENCODING, "identity")
            .timeout(self.deadline.saturating_duration_since(Instant::now()));
        if let Some(access) = access {
            request = request.header(header::AUTHORIZATION, super::auth::bearer(&access.secret)?);
        }
        if let Some((start, end)) = range {
            request = request.header(header::RANGE, format!("bytes={start}-{end}"));
        }
        if let Some(etag) = etag {
            request = request.header(header::IF_MATCH, etag.clone());
        }
        timeout_at(self.deadline, request.send()).await
            .map_err(|_| bad("DAT total deadline exceeded while waiting for response headers; increase Settings > API > DAT deadline."))?
            .map_err(|e| {
                if e.is_timeout() { bad("DAT timed out waiting for response headers.") }
                else if e.is_connect() { bad("DAT connection or TLS handshake failed.") }
                else { bad("DAT request failed before response headers.") }
            })
    }
    async fn parts(
        &self,
        response: Response,
        access: Option<&Access>,
        limit: u64,
    ) -> Result<Download, Error> {
        let total = probe_total(response.headers().get(header::CONTENT_RANGE), limit)?;
        if let Some(r) = &self.progress {
            r.total(Some(total));
        }
        let etag = recovery::strong_etag(&response).ok_or_else(|| {
            bad("Parallel DAT requires a strong ETag; choose Single GET for this server.")
        })?;
        self.body(
            response,
            1,
            Some(Segment {
                start: 0,
                end: 0,
                total,
                etag: etag.clone(),
            }),
            access,
            false,
        )
        .await?;
        let size = self.cfg.download_part_mib * 1024 * 1024;
        let count = total.div_ceil(size);
        let parts = stream::iter(0..count)
            .map(|i| {
                let etag = &etag;
                async move {
                    let start = i * size;
                    let end = (start + size).min(total) - 1;
                    let segment = Segment {
                        start,
                        end,
                        total,
                        etag: etag.clone(),
                    };
                    let response = self
                        .recovery_request(access, Some((start, end)), Some(etag))
                        .await?;
                    segment.validate(&response, start)?;
                    self.body(response, end - start + 1, Some(segment), access, true)
                        .await
                }
            })
            .buffered(self.cfg.download_parallelism);
        futures_util::pin_mut!(parts);
        let mut wire = Download::new(
            usize::try_from(total).map_err(|_| bad("DAT cannot fit this platform."))?,
        );
        while let Some(part) = parts.next().await {
            wire.join(part?)?;
        }
        if wire.len() as u64 != total {
            return Err(bad("DAT parts are incomplete."));
        }
        Ok(wire)
    }
}
fn check_status(response: &Response) -> Result<(), Error> {
    if response.status().is_success() {
        Ok(())
    } else {
        Err(status_error(response.status()))
    }
}
fn status_error(status: StatusCode) -> Error {
    bad(&format!("DAT endpoint returned HTTP {}.", status.as_u16()))
}
fn probe_total(value: Option<&header::HeaderValue>, limit: u64) -> Result<u64, Error> {
    value
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("bytes 0-0/"))
        .filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|n| n.parse::<u64>().ok())
        .filter(|&n| n > 0 && n <= limit)
        .ok_or_else(|| bad("Invalid DAT range probe or total exceeds download limit."))
}
