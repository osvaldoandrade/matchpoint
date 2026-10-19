use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, Notify};

use crate::error::{Error, Secret};
use crate::secrets::{SecretStore, ACCOUNT_REFRESH, ACCOUNT_SECRET};
use crate::timeutil::WallClock;

pub const SCOPE: &str = "partner.account.onboarding";
// Compatibility names only: destinations must come from external configuration.
pub const DEFAULT_TOKEN_URL: &str = "";
pub const DEFAULT_PROCESS_URL: &str = "";

#[derive(Clone)]
pub struct Issued {
    pub access: Secret,
    pub generation: u64,
    pub expires_at_ms: i64,
    pub refreshed_at_ms: i64,
}

impl std::fmt::Debug for Issued {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Issued")
            .field("access", &self.access)
            .field("generation", &self.generation)
            .field("expires_at_ms", &self.expires_at_ms)
            .finish()
    }
}

#[derive(Debug)]
struct GateInner {
    token: Option<Issued>,
    refreshing: bool,
    paused_until_ms: i64,
    last_refresh_rfc3339: Option<String>,
}

/// One shared token. A single refresh runs at a time; other callers wait.
///
/// Termination: `access` returns when the in-flight refresh finishes or the
/// caller is cancelled by dropping the future. There is no background task.
pub struct TokenGate {
    inner: Mutex<GateInner>,
    notify: Notify,
    generation: AtomicU64,
    client: reqwest::Client,
    token_url: String,
    client_id: String,
    secrets: Arc<dyn SecretStore>,
    clock: Arc<dyn WallClock>,
    skew_ms: i64,
    http_hits: AtomicU64,
}

impl TokenGate {
    pub fn new(
        client: reqwest::Client,
        token_url: String,
        client_id: String,
        secrets: Arc<dyn SecretStore>,
        clock: Arc<dyn WallClock>,
        skew_ms: i64,
    ) -> Self {
        Self {
            inner: Mutex::new(GateInner {
                token: None,
                refreshing: false,
                paused_until_ms: 0,
                last_refresh_rfc3339: None,
            }),
            notify: Notify::new(),
            generation: AtomicU64::new(0),
            client,
            token_url,
            client_id,
            secrets,
            clock,
            skew_ms,
            http_hits: AtomicU64::new(0),
        }
    }

    pub fn http_hits(&self) -> u64 {
        self.http_hits.load(Ordering::SeqCst)
    }

    pub async fn view(&self) -> AuthView {
        let guard = self.inner.lock().await;
        let now = self.clock.now_ms();
        AuthView {
            configured: true,
            token_present: guard.token.is_some(),
            expires_in_secs: guard.token.as_ref().map(|t| (t.expires_at_ms - now) / 1000),
            last_refresh: guard.last_refresh_rfc3339.clone(),
            generation: guard.token.as_ref().map(|t| t.generation),
            paused: now < guard.paused_until_ms,
        }
    }

    pub async fn paused_until_ms(&self) -> i64 {
        self.inner.lock().await.paused_until_ms
    }

    /// Drops the token only when `generation` is still the current one.
    pub async fn invalidate(&self, generation: u64) {
        let mut guard = self.inner.lock().await;
        if guard.token.as_ref().map(|t| t.generation) == Some(generation) {
            guard.token = None;
        }
    }

    pub async fn note_auth_failure(&self, pause_ms: i64) {
        let mut guard = self.inner.lock().await;
        let until = self.clock.now_ms().saturating_add(pause_ms);
        if until > guard.paused_until_ms {
            guard.paused_until_ms = until;
        }
    }

    pub async fn access(&self) -> Result<Issued, Error> {
        loop {
            let now = self.clock.now_ms();
            {
                let mut guard = self.inner.lock().await;
                if now < guard.paused_until_ms && guard.token.is_none() {
                    return Err(Error::AuthPaused);
                }
                if let Some(token) = guard.token.clone() {
                    if usable(&token, now, self.skew_ms) {
                        return Ok(token);
                    }
                }
                if guard.refreshing {
                    let notified = self.notify.notified();
                    drop(guard);
                    notified.await;
                    continue;
                }
                guard.refreshing = true;
            }
            let fetched = self.fetch().await;
            let mut guard = self.inner.lock().await;
            guard.refreshing = false;
            match fetched {
                Ok(issued) => {
                    guard.token = Some(issued.clone());
                    guard.last_refresh_rfc3339 =
                        Some(crate::timeutil::format_rfc3339(issued.refreshed_at_ms));
                    guard.paused_until_ms = 0;
                    drop(guard);
                    self.notify.notify_waiters();
                    return Ok(issued);
                }
                Err(err) => {
                    guard.paused_until_ms = self.clock.now_ms().saturating_add(1_000);
                    drop(guard);
                    self.notify.notify_waiters();
                    return Err(err);
                }
            }
        }
    }

    async fn fetch(&self) -> Result<Issued, Error> {
        let refresh = self.secrets.get(ACCOUNT_REFRESH)?;
        let result = if let Some(refresh) = refresh {
            match request_token(
                &self.client,
                &self.token_url,
                &self.client_id,
                self.secrets.as_ref(),
                Some(refresh.expose()),
                &self.http_hits,
            )
            .await
            {
                Ok(body) => Ok(body),
                Err(Error::TokenInvalid("invalid_grant")) => {
                    request_token(
                        &self.client,
                        &self.token_url,
                        &self.client_id,
                        self.secrets.as_ref(),
                        None,
                        &self.http_hits,
                    )
                    .await
                }
                Err(err) => Err(err),
            }
        } else {
            request_token(
                &self.client,
                &self.token_url,
                &self.client_id,
                self.secrets.as_ref(),
                None,
                &self.http_hits,
            )
            .await
        }?;
        if let Some(refresh) = result.refresh {
            self.secrets.put(ACCOUNT_REFRESH, &refresh)?;
        }
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let now = self.clock.now_ms();
        Ok(Issued {
            access: result.access,
            generation,
            expires_at_ms: now.saturating_add(result.expires_in_secs * 1000),
            refreshed_at_ms: now,
        })
    }
}

fn usable(token: &Issued, now: i64, skew_ms: i64) -> bool {
    if now >= token.expires_at_ms {
        return false;
    }
    let inside_skew = now >= token.expires_at_ms.saturating_sub(skew_ms);
    if !inside_skew {
        return true;
    }
    // A token that is born inside the skew window must still be used until it expires.
    now < token.refreshed_at_ms.saturating_add(1_000)
}

#[derive(Clone, Debug)]
pub struct AuthView {
    pub configured: bool,
    pub token_present: bool,
    pub expires_in_secs: Option<i64>,
    pub last_refresh: Option<String>,
    pub generation: Option<u64>,
    pub paused: bool,
}

struct TokenBody {
    access: Secret,
    refresh: Option<Secret>,
    expires_in_secs: i64,
}

async fn request_token(
    client: &reqwest::Client,
    url: &str,
    client_id: &str,
    secrets: &dyn SecretStore,
    refresh: Option<&str>,
    hits: &AtomicU64,
) -> Result<TokenBody, Error> {
    hits.fetch_add(1, Ordering::SeqCst);
    let secret = secrets.get(ACCOUNT_SECRET)?.ok_or(Error::AuthMissing)?;
    let payload = if let Some(refresh) = refresh {
        serde_json::json!({
            "grant_type": "refresh_token",
            "scope": SCOPE,
            "client_id": client_id,
            "client_secret": secret.expose(),
            "refresh_token": refresh,
        })
    } else {
        serde_json::json!({
            "grant_type": "client_credentials",
            "scope": SCOPE,
            "client_id": client_id,
            "client_secret": secret.expose(),
        })
    };
    let response = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::ACCEPT, "application/json")
        .body(serde_json::to_vec(&payload).map_err(|e| Error::Msg(e.to_string()))?)
        .send()
        .await
        .map_err(|e| Error::Http(e.to_string()))?;
    let status = response.status();
    let bytes = read_limited(response, 65_536).await?;
    if status == reqwest::StatusCode::BAD_REQUEST {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if value.get("error").and_then(|v| v.as_str()) == Some("invalid_grant") {
                return Err(Error::TokenInvalid("invalid_grant"));
            }
        }
    }
    if !status.is_success() {
        return Err(Error::TokenInvalid("token endpoint rejected the request"));
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| Error::TokenInvalid("token body is not json"))?;
    let access = value
        .get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let token_type = value
        .get("token_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let expires_in = value
        .get("expires_in")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if !token_type.eq_ignore_ascii_case("bearer") {
        return Err(Error::TokenInvalid("token_type must be Bearer"));
    }
    if !(1..=604_800).contains(&expires_in) {
        return Err(Error::TokenInvalid(
            "expires_in is outside 1..=604800 seconds",
        ));
    }
    let access = Secret::new(access.to_string())?;
    let refresh = match value.get("refresh_token") {
        Some(serde_json::Value::Null) | None => None,
        Some(serde_json::Value::String(text)) if text.is_empty() => None,
        Some(serde_json::Value::String(text)) => Some(Secret::new(text.clone())?),
        Some(_) => return Err(Error::TokenInvalid("refresh_token must be a string")),
    };
    Ok(TokenBody {
        access,
        refresh,
        expires_in_secs: expires_in,
    })
}

pub async fn read_limited(response: reqwest::Response, max: usize) -> Result<Vec<u8>, Error> {
    use futures_util::StreamExt;
    if response.content_length().unwrap_or(0) > max as u64 {
        return Err(Error::Http("response too large".into()));
    }
    let mut buf = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| Error::Http(e.to_string()))?;
        if buf.len().saturating_add(chunk.len()) > max {
            return Err(Error::Http("response too large".into()));
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf)
}

pub fn build_client(
    connect: Duration,
    response: Duration,
    pool: usize,
) -> Result<reqwest::Client, Error> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(connect)
        .timeout(response)
        .pool_idle_timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(pool)
        .tcp_nodelay(true)
        .http2_adaptive_window(true)
        .http2_keep_alive_interval(Some(Duration::from_secs(15)))
        .min_tls_version(reqwest::tls::Version::TLS_1_2)
        .build()
        .map_err(|e| Error::Http(e.to_string()))
}
