//! OAuth JSON grants for the configured token and refresh URLs. No per-key calls.
use super::{bad, hex, Service, Settings};
use crate::{
    disk::acquire,
    error::{Error, Secret},
};
use reqwest::{header::HeaderValue, Url};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    time::{Duration, Instant},
};

#[derive(Default)]
pub(super) struct State {
    profile: String,
    token: Option<Cached>,
    generation: u64,
    retry_after: Option<Instant>,
    session: Option<Session>,
}
struct Session {
    secret: Secret,
    refresh: Option<Secret>,
}
struct Cached {
    access: Secret,
    generation: u64,
    expires: Instant,
    usable_until: Instant,
}
pub(crate) struct Access {
    pub secret: Secret,
    pub generation: u64,
}
#[derive(Clone, Debug)]
pub struct TokenStatus {
    pub expires_in: u64,
}
enum Grant {
    Auto,
    FreshJob,
    Issue,
    Refresh,
    Login(Secret),
}
struct Issued {
    access: Secret,
    refresh: Option<Secret>,
    expires_in: u64,
}
struct Profile {
    key: String,
    token: Url,
}
impl Settings {
    fn oauth_profile(&self, root: &Path) -> Result<Profile, Error> {
        self.validate()?;
        if self.client_id.is_empty() {
            return Err(bad(
                "Enter the client ID on Home, or configure CLI credentials.",
            ));
        }
        let token = self.api_url(&self.token_url, "Get token")?;
        let source = (!self.url.is_empty())
            .then(|| self.source_url())
            .transpose()?;
        let mut hash = Sha256::new();
        for item in [
            root.to_string_lossy().as_ref(),
            source.as_ref().map_or("", Url::as_str),
            token.as_str(),
            token.as_str(),
            &self.client_id,
            &self.scope,
        ] {
            hash.update(item.as_bytes());
            hash.update([0]);
        }
        if self.download_mode != super::DownloadMode::Single || !self.download_form.is_empty() {
            hash.update(
                serde_json::to_vec(&(self.download_mode, &self.download_form))
                    .map_err(|_| bad("Invalid download settings."))?,
            );
        }
        Ok(Profile {
            key: format!("bloom-oauth-{}", hex(&hash.finalize())),
            token,
        })
    }
}
impl Service {
    /// Clear the RAM session without revoking tokens or modifying saved CLI credentials.
    pub fn forget_access(&self) -> Result<(), Error> {
        let mut state = self.auth.try_lock().map_err(|_| Error::Locked)?;
        state.token = None;
        state.session = None;
        state.profile.clear();
        state.retry_after = None;
        // Keep generations monotonic: an older in-flight 401 must not clear a new token.
        drop(state);
        self.clear_dat_key()
    }
    pub(crate) fn source_identity(&self, cfg: &Settings) -> Result<String, Error> {
        if cfg.oauth {
            Ok(cfg.oauth_profile(&self.root)?.key)
        } else {
            let mut hash = Sha256::new();
            hash.update(cfg.source_url()?.as_str().as_bytes());
            if cfg.download_mode != super::DownloadMode::Single || !cfg.download_form.is_empty() {
                hash.update(
                    serde_json::to_vec(&(cfg.download_mode, &cfg.download_form))
                        .map_err(|_| bad("Invalid download settings."))?,
                );
            }
            Ok(hex(&hash.finalize()))
        }
    }
    /// Explicit CLI persistence; omitted credentials preserve existing entries.
    pub fn save_oauth_credentials(
        &self,
        secret: Option<&Secret>,
        refresh: Option<&Secret>,
    ) -> Result<(), Error> {
        let cfg = self.settings()?;
        let profile = cfg.oauth_profile(&self.root)?;
        let mut state = self.auth.try_lock().map_err(|_| Error::Locked)?;
        let _lock = acquire(&self.root.join("auth.lock"))?;
        let changes = [("client-secret", secret), ("refresh-token", refresh)]
            .into_iter()
            .filter_map(|(suffix, value)| {
                value.map(|value| (format!("{}-{suffix}", profile.key), value))
            })
            .collect::<Vec<_>>();
        let previous = changes
            .iter()
            .map(|(account, _)| {
                self.secrets
                    .get(account)
                    .map(|value| (account.clone(), value))
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (account, value) in &changes {
            if let Err(error) = self.secrets.put(account, value) {
                for (account, old) in previous {
                    match old {
                        Some(value) => self.secrets.put(&account, &value)?,
                        None => self.secrets.delete(&account)?,
                    }
                }
                return Err(error);
            }
        }
        state.token = None;
        state.session = None;
        state.retry_after = None;
        Ok(())
    }
    pub async fn obtain_token(&self) -> Result<TokenStatus, Error> {
        self.oauth_access(&self.settings()?, Grant::Issue).await?;
        self.token_status().await
    }
    /// A new DAT/job requires fresh acquisition, while retaining the automatic failure cooldown.
    pub(crate) async fn fresh_job_token(&self, cfg: &Settings) -> Result<(), Error> {
        self.oauth_access(cfg, Grant::FreshJob).await.map(|_| ())
    }
    /// Verify fresh input and retain credentials in this RAM session only.
    pub async fn sign_in(&self, secret: Secret) -> Result<TokenStatus, Error> {
        self.oauth_access(&self.settings()?, Grant::Login(secret))
            .await?;
        self.token_status().await
    }
    pub async fn refresh_token(&self) -> Result<TokenStatus, Error> {
        self.oauth_access(&self.settings()?, Grant::Refresh).await?;
        self.token_status().await
    }
    async fn token_status(&self) -> Result<TokenStatus, Error> {
        let state = self.auth.lock().await;
        let token = state
            .token
            .as_ref()
            .ok_or_else(|| bad("Token is unavailable."))?;
        Ok(TokenStatus {
            expires_in: token
                .expires
                .saturating_duration_since(Instant::now())
                .as_secs(),
        })
    }
    pub(crate) async fn authorization(
        &self,
        cfg: &Settings,
        url: &Url,
    ) -> Result<Option<Access>, Error> {
        if cfg.oauth {
            return self.oauth_access(cfg, Grant::Auto).await.map(Some);
        }
        if !cfg.bearer {
            return Ok(None);
        }
        let secret = self
            .secrets
            .get(&super::source::token_account(&self.root, url))?
            .ok_or_else(|| bad("Enter the DAT bearer token in Settings > Secrets."))?;
        Ok(Some(Access {
            secret,
            generation: 0,
        }))
    }
    pub(crate) async fn invalidate_access(&self, generation: u64) {
        let mut state = self.auth.lock().await;
        if state
            .token
            .as_ref()
            .is_some_and(|token| token.generation == generation)
        {
            state.token = None;
        }
    }
    async fn oauth_access(&self, cfg: &Settings, grant: Grant) -> Result<Access, Error> {
        if !cfg.oauth {
            return Err(bad("Choose OAuth2 in Settings > Secrets and save first."));
        }
        let profile = cfg.oauth_profile(&self.root)?;
        // Source lock -> auth mutex -> auth file lock; auth never acquires a source/job lock.
        // No refreshing flag or spawned task: cancellation drops both guards immediately.
        let mut state = self.auth.lock().await;
        if state.profile != profile.key {
            state.profile = profile.key.clone();
            state.token = None;
            state.session = None;
            state.retry_after = None;
        }
        if matches!(grant, Grant::FreshJob)
            && state
                .retry_after
                .is_some_and(|until| Instant::now() < until)
        {
            return Err(bad("Authentication cooling down; retry shortly."));
        }
        if matches!(grant, Grant::Auto) {
            if let Some(token) = &state.token {
                if Instant::now() < token.usable_until {
                    return Ok(Access {
                        secret: token.access.clone(),
                        generation: token.generation,
                    });
                }
            }
            if state
                .retry_after
                .is_some_and(|until| Instant::now() < until)
            {
                return Err(bad("Authentication cooling down; retry shortly."));
            }
        }
        let _lock = acquire(&self.root.join("auth.lock"))?;
        let session = matches!(grant, Grant::Login(_)) || state.session.is_some();
        let secret = match &grant {
            Grant::Login(secret) => secret.clone(),
            _ if state.session.is_some() => state.session.as_ref().unwrap().secret.clone(),
            _ => self
                .secrets
                .get(&format!("{}-client-secret", profile.key))?
                .ok_or_else(|| bad("Sign in on Home, or enter CLI credentials."))?,
        };
        let refresh = if matches!(grant, Grant::Login(_)) {
            None
        } else if let Some(session) = &state.session {
            session.refresh.clone()
        } else {
            self.secrets
                .get(&format!("{}-refresh-token", profile.key))?
        };
        let client = client(cfg)?;
        let use_refresh =
            !matches!(grant, Grant::Issue | Grant::FreshJob | Grant::Login(_)) && refresh.is_some();
        if matches!(grant, Grant::Refresh) && refresh.is_none() {
            return Err(bad("No refresh token is available. Obtain a token first."));
        }
        let fetched = if use_refresh {
            match request(&client, &profile.token, cfg, &secret, refresh.as_ref()).await {
                Err(GrantError::InvalidGrant) => {
                    if let Some(session) = &mut state.session {
                        session.refresh = None;
                    } else {
                        self.secrets
                            .delete(&format!("{}-refresh-token", profile.key))?;
                    }
                    request(&client, &profile.token, cfg, &secret, None).await
                }
                result => result,
            }
        } else {
            request(&client, &profile.token, cfg, &secret, None).await
        };
        let issued = match fetched {
            Ok(issued) => issued,
            Err(error) => {
                state.retry_after = Some(Instant::now() + Duration::from_secs(1));
                tracing::warn!(
                    event = "oauth_failed",
                    reason = "request_or_response",
                    "Authentication failed"
                );
                return Err(error.into_error());
            }
        };
        if matches!(grant, Grant::Login(_)) {
            state.session = Some(Session {
                secret,
                refresh: issued.refresh.clone(),
            });
        }
        if let Some(refresh) = &issued.refresh {
            if session {
                state.session.as_mut().unwrap().refresh = Some(refresh.clone());
            } else {
                self.secrets
                    .put(&format!("{}-refresh-token", profile.key), refresh)?;
            }
        }
        let now = Instant::now();
        state.generation += 1;
        let generation = state.generation;
        state.token = Some(Cached {
            access: issued.access.clone(),
            generation,
            expires: now + Duration::from_secs(issued.expires_in),
            usable_until: now
                + Duration::from_secs(issued.expires_in - (issued.expires_in / 10).min(60)),
        });
        state.retry_after = None;
        tracing::info!(
            event = "oauth_ready",
            refresh = use_refresh,
            expires_in = issued.expires_in,
            "Access token ready in memory"
        );
        Ok(Access {
            secret: issued.access,
            generation,
        })
    }
}
pub(super) fn client(cfg: &Settings) -> Result<reqwest::Client, Error> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(cfg.timeout_seconds))
        .build()
        .map_err(|_| bad("Cannot initialize API client."))
}
pub(crate) fn bearer(token: &Secret) -> Result<HeaderValue, Error> {
    let mut header = HeaderValue::from_str(&format!("Bearer {}", token.expose()))
        .map_err(|_| bad("Invalid bearer token header."))?;
    header.set_sensitive(true);
    Ok(header)
}

enum GrantError {
    InvalidGrant,
    Other(Error),
}
impl GrantError {
    fn into_error(self) -> Error {
        match self {
            Self::InvalidGrant => bad("Authentication grant rejected."),
            Self::Other(error) => error,
        }
    }
}
impl From<Error> for GrantError {
    fn from(error: Error) -> Self {
        Self::Other(error)
    }
}
async fn request(
    client: &reqwest::Client,
    url: &Url,
    cfg: &Settings,
    secret: &Secret,
    refresh: Option<&Secret>,
) -> Result<Issued, GrantError> {
    let mut payload = serde_json::json!({"grant_type":if refresh.is_some(){"refresh_token"}else{"client_credentials"},"client_id":cfg.client_id,"client_secret":secret.expose()});
    if !cfg.scope.is_empty() {
        payload["scope"] = cfg.scope.clone().into();
    }
    if let Some(refresh) = refresh {
        payload["refresh_token"] = refresh.expose().into();
    }
    let response = client
        .post(url.clone())
        .json(&payload)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|_| bad("Token request failed or timed out."))?;
    let status = response.status();
    let body = crate::auth::read_limited(response, 65_536)
        .await
        .map_err(|_| bad("Token response interrupted or exceeds 64 KiB."))?;
    let value = serde_json::from_slice::<serde_json::Value>(&body)
        .map_err(|_| bad("Token endpoint returned invalid JSON."))?;
    if status == reqwest::StatusCode::BAD_REQUEST
        && value.get("error").and_then(|v| v.as_str()) == Some("invalid_grant")
    {
        return Err(GrantError::InvalidGrant);
    }
    if !status.is_success() {
        return Err(bad(&format!(
            "Token endpoint returned HTTP {}.",
            status.as_u16()
        ))
        .into());
    }
    if !value
        .get("token_type")
        .and_then(|v| v.as_str())
        .is_some_and(|kind| kind.eq_ignore_ascii_case("bearer"))
    {
        return Err(bad("Token response requires Bearer token_type.").into());
    }
    let expires_in = value
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .filter(|n| (1..=604_800).contains(n))
        .ok_or_else(|| bad("Token expires_in must be 1–604800 seconds."))?;
    let raw_access = value
        .get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if raw_access.trim().is_empty() || raw_access.trim() != raw_access {
        return Err(bad("Token response requires a nonempty access_token.").into());
    }
    let access = Secret::new(raw_access.into())?;
    bearer(&access)?;
    let refresh = match value.get("refresh_token") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) if s.is_empty() => None,
        Some(serde_json::Value::String(s)) => Some(Secret::new(s.clone())?),
        _ => return Err(bad("Invalid refresh_token response type.").into()),
    };
    Ok(Issued {
        access,
        refresh,
        expires_in,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sign_out_keeps_generation_monotonic_and_clears_runtime_state() {
        let tmp = tempfile::tempdir().unwrap();
        let service = Service::open(
            tmp.path().into(),
            std::sync::Arc::new(crate::secrets::MemorySecrets::new()),
        )
        .unwrap();
        {
            let mut state = service.auth.try_lock().unwrap();
            state.generation = 42;
            state.profile = "synthetic-profile".into();
            state.retry_after = Some(Instant::now() + Duration::from_secs(1));
            state.token = Some(Cached {
                access: Secret::new("synthetic-access".into()).unwrap(),
                generation: 42,
                expires: Instant::now() + Duration::from_secs(60),
                usable_until: Instant::now() + Duration::from_secs(50),
            });
        }
        service.forget_access().unwrap();
        let state = service.auth.try_lock().unwrap();
        assert_eq!(state.generation, 42);
        assert!(state.profile.is_empty());
        assert!(state.token.is_none());
        assert!(state.retry_after.is_none());
        assert!(service.forget_access().is_err());
    }
}
