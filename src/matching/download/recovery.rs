//! Recovery never exposes incomplete bytes to crypto or matching.
use super::super::progress::ByteCounter;
use super::*;
use std::{error::Error as StdError, sync::atomic::Ordering};
use tokio::time::timeout;

#[derive(Clone)]
pub(super) struct Segment {
    pub start: u64,
    pub end: u64,
    pub total: u64,
    pub etag: header::HeaderValue,
}
impl Segment {
    fn full(response: &Response) -> Option<Self> {
        let total = response.content_length().filter(|&n| n > 0)?;
        Some(Self {
            start: 0,
            end: total - 1,
            total,
            etag: strong_etag(response)?,
        })
    }
    fn len(&self) -> u64 {
        self.end - self.start + 1
    }
    pub(super) fn validate(&self, response: &Response, start: u64) -> Result<(), Error> {
        let expected = format!("bytes {start}-{}/{}", self.end, self.total);
        if response.status() != StatusCode::PARTIAL_CONTENT
            || response
                .headers()
                .get(header::CONTENT_RANGE)
                .is_none_or(|v| v != expected.as_str())
            || response.headers().get(header::ETAG) != Some(&self.etag)
        {
            return Err(bad(
                "DAT part range/version changed; no partial data will be used.",
            ));
        }
        Ok(())
    }
}
pub(super) fn strong_etag(response: &Response) -> Option<header::HeaderValue> {
    response
        .headers()
        .get(header::ETAG)
        .filter(|v| {
            v.to_str().is_ok_and(|s| {
                s.len() >= 2
                    && s.starts_with('"')
                    && s.ends_with('"')
                    && !s[1..s.len() - 1].contains('"')
            })
        })
        .cloned()
}
#[derive(Clone, Copy)]
pub(super) enum RetryDelay {
    Default,
    Seconds(u64),
    Refuse,
}
impl RetryDelay {
    pub(super) fn from_response(response: &Response) -> Self {
        match response.headers().get(header::RETRY_AFTER) {
            None => Self::Default,
            Some(value) => match value.to_str().ok().and_then(|v| v.parse::<u64>().ok()) {
                Some(seconds) if seconds <= 30 => Self::Seconds(seconds),
                _ => Self::Refuse,
            },
        }
    }
}
enum ReadFailure {
    Transient(&'static str),
    Fatal(Error),
}
impl From<Error> for ReadFailure {
    fn from(error: Error) -> Self {
        Self::Fatal(error)
    }
}

impl Transport<'_> {
    pub(super) async fn retry(
        &self,
        reason: &'static str,
        status: Option<StatusCode>,
        delay: RetryDelay,
    ) -> Result<bool, Error> {
        if matches!(delay, RetryDelay::Refuse) || self.cfg.download_mode == DownloadMode::Multipart
        {
            return Ok(false);
        }
        // A single CAS budget bounds amplification across concurrent part failures.
        let Ok(used) = self
            .retries
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                if used < self.cfg.download_retries {
                    Some(used + 1)
                } else {
                    None
                }
            })
        else {
            return Ok(false);
        };
        let random = uuid::Uuid::new_v4().as_u128();
        let wait = match delay {
            RetryDelay::Seconds(seconds) => {
                Duration::from_secs(seconds) + Duration::from_millis((random % 251) as u64)
            }
            _ => Duration::from_millis((random % (250u128 << used)) as u64),
        };
        tracing::warn!(
            event = "dat_retry",
            reason,
            status = status.map(|s| s.as_u16()),
            retry = used + 1,
            delay_ms = wait.as_millis() as u64,
            "Retrying DAT transport within shared budget"
        );
        if let Some(r) = &self.progress {
            r.retry(used + 1, status.map(|s| s.as_u16()), wait);
        }
        timeout_at(self.deadline, tokio::time::sleep(wait))
            .await
            .map_err(|_| bad("DAT total deadline exceeded during retry backoff."))?;
        Ok(true)
    }
    pub(super) async fn recovery_request(
        &self,
        access: Option<&Access>,
        range: Option<(u64, u64)>,
        etag: Option<&header::HeaderValue>,
    ) -> Result<Response, Error> {
        loop {
            let response = self.send(access, range, etag).await?;
            if matches!(response.status().as_u16(), 429 | 502 | 503 | 504) {
                let status = response.status();
                let delay = RetryDelay::from_response(&response);
                drop(response);
                if self.retry("http_status", Some(status), delay).await? {
                    continue;
                }
                return Err(status_error(status));
            }
            check_status(&response)?;
            return Ok(response);
        }
    }
    pub(super) async fn body(
        &self,
        mut response: Response,
        limit: u64,
        range: Option<Segment>,
        access: Option<&Access>,
        visible: bool,
    ) -> Result<Download, Error> {
        let strict = range.is_some();
        let mut segment = range.or_else(|| Segment::full(&response));
        let capacity = usize::try_from(limit).map_err(|_| bad("DAT cannot fit this platform."))?;
        let mut wire = Download::new(capacity);
        let mut counter = ByteCounter::new(self.progress.clone().filter(|_| visible));
        loop {
            let expected = segment
                .as_ref()
                .map(|s| s.len() - wire.len() as u64)
                .or(response.content_length());
            let total_expected = expected.map(|n| n + wire.len() as u64);
            match self
                .receive(response, &mut wire, limit, expected, &mut counter)
                .await
            {
                Ok(()) => return Ok(wire),
                Err(ReadFailure::Fatal(error)) => return Err(error),
                Err(ReadFailure::Transient(reason)) => {
                    let expected = total_expected;
                    tracing::warn!(
                        event = "dat_body_interrupted",
                        reason,
                        bytes = wire.len(),
                        expected,
                        "DAT stream interrupted; partial bytes remain in locked RAM"
                    );
                    if !self.retry(reason, None, RetryDelay::Default).await? {
                        let note = if self.cfg.download_mode == DownloadMode::Multipart {
                            "POST replay disabled"
                        } else {
                            "retry limit reached"
                        };
                        return Err(bad(&format!(
                            "DAT body interrupted: {}/{} bytes ({reason}); {note}.",
                            wire.len(),
                            expected.map_or_else(|| "unknown".into(), |n| n.to_string())
                        )));
                    }
                }
            }
            if segment
                .as_ref()
                .is_some_and(|s| wire.len() as u64 >= s.len())
            {
                drop(wire);
                counter.rewind();
                wire = Download::new(capacity);
            }
            let suffix = segment
                .as_ref()
                .filter(|s| (wire.len() as u64) < s.len())
                .map(|s| (s.start + wire.len() as u64, s.end));
            response = self
                .recovery_request(
                    access,
                    suffix,
                    segment
                        .as_ref()
                        .filter(|_| suffix.is_some())
                        .map(|s| &s.etag),
                )
                .await?;
            if let (Some(segment), Some((start, _))) = (&segment, suffix) {
                if response.status() == StatusCode::PARTIAL_CONTENT {
                    segment.validate(&response, start)?;
                    tracing::info!(
                        event = "dat_body_resumed",
                        offset = start,
                        total = segment.total,
                        "Resuming exact DAT representation"
                    );
                    continue;
                }
            }
            if strict || response.status() != StatusCode::OK {
                return Err(bad(
                    "DAT part range/version changed; no partial data will be used.",
                ));
            }
            // No usable validator, a complete-but-malformed stream, or ignored Range:
            // wipe the prefix before receiving this new full representation.
            drop(wire);
            counter.rewind();
            wire = Download::new(capacity);
            segment = Segment::full(&response);
            if let Some(r) = &self.progress {
                r.total(response.content_length());
            }
            tracing::info!(
                event = "dat_body_restarted",
                "Discarded interrupted prefix before full GET restart"
            );
        }
    }
    async fn receive(
        &self,
        response: Response,
        wire: &mut Download,
        limit: u64,
        expected: Option<u64>,
        counter: &mut ByteCounter,
    ) -> Result<(), ReadFailure> {
        if response
            .headers()
            .get(header::CONTENT_ENCODING)
            .is_some_and(|v| v != "identity")
        {
            return Err(bad(
                "DAT HTTP Content-Encoding must be identity; serve GZip as binary content.",
            )
            .into());
        }
        let declared = response.content_length();
        if declared.is_some_and(|n| {
            n > limit.saturating_sub(wire.len() as u64) || expected.is_some_and(|e| e != n)
        }) {
            return Err(
                bad("DAT Content-Length exceeds limit or differs from requested part.").into(),
            );
        }
        let expected = expected.or(declared);
        let before = wire.len();
        // Headers arrived, including on suffix recovery; body-idle time starts here.
        counter.add(0);
        let mut stream = response.bytes_stream();
        loop {
            let next = timeout_at(
                self.deadline,
                timeout(
                    Duration::from_secs(self.cfg.download_idle_seconds),
                    stream.next(),
                ),
            )
            .await
            .map_err(|_| {
                ReadFailure::Fatal(bad("DAT total deadline exceeded while reading body."))
            })?
            .map_err(|_| ReadFailure::Transient("body_idle_timeout"))?;
            let Some(chunk) = next else {
                break;
            };
            let chunk = chunk.map_err(|error| {
                if error.is_timeout() {
                    ReadFailure::Fatal(bad("DAT body exceeded total deadline."))
                } else {
                    ReadFailure::Transient(classify(&error))
                }
            })?;
            wire.append(&chunk)?;
            counter.add(chunk.len());
        }
        if expected.is_some_and(|n| (wire.len() - before) as u64 != n) {
            return Err(ReadFailure::Transient("unexpected_eof"));
        }
        Ok(())
    }
}
fn classify(error: &dyn StdError) -> &'static str {
    let mut current = Some(error);
    for _ in 0..12 {
        let Some(cause) = current else {
            break;
        };
        // Strings are inspected only to emit a fixed category, never logged/returned.
        let text = cause.to_string().to_ascii_lowercase();
        if text.contains("connection reset") {
            return "connection_reset";
        }
        if text.contains("http2") || text.contains("http/2") || text.contains("stream error") {
            return "http2_stream_error";
        }
        if text.contains("unexpected eof")
            || text.contains("end of file")
            || text.contains("incomplete message")
        {
            return "unexpected_eof";
        }
        if text.contains("broken pipe") {
            return "broken_pipe";
        }
        current = cause.source();
    }
    "body_transport_error"
}
