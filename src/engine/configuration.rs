use uuid::Uuid;

use super::App;
use crate::error::{Error, Secret};
use crate::httpcall::endpoint_allowed;
use crate::jobcfg::JobConfig;
use crate::secrets::{ACCOUNT_REFRESH, ACCOUNT_SECRET};

impl App {
    /// Update the remote toggle while preserving credentials for the exact saved profile.
    /// A changed identity or destination requires freshly supplied credentials.
    pub fn configure_api_metadata(&self, mut config: AppConfig) -> Result<(), Error> {
        config.validate()?;
        let previous = self.api_config()?;
        if previous.client_id != config.client_id
            || previous.process_url != config.process_url
            || previous.token_url != config.token_url
            || !previous
                .secrets_stored
                .iter()
                .any(|name| name == ACCOUNT_SECRET)
        {
            return Err(Error::Msg(
                "Enter a client secret in Settings > Secrets for this profile.".into(),
            ));
        }
        config.secrets_stored = previous.secrets_stored;
        self.save_api_config(&config)
    }
    pub fn configure_auth(
        &self,
        client_id: &str,
        secret: Secret,
        refresh: Option<Secret>,
    ) -> Result<(), Error> {
        let mut config = self.api_config()?;
        config.client_id = client_id.to_string();
        self.configure_api(config, secret, refresh)
    }

    /// Configure the saved API profile; secrets are stored only in Keychain.
    pub fn configure_api(
        &self,
        mut config: AppConfig,
        secret: Secret,
        refresh: Option<Secret>,
    ) -> Result<(), Error> {
        config.validate()?;
        let previous_secret = self.secrets.get(ACCOUNT_SECRET)?;
        let previous_refresh = self.secrets.get(ACCOUNT_REFRESH)?;
        let result = self.store_credentials(&mut config, secret, refresh);
        if result.is_err() {
            for (account, previous) in [
                (ACCOUNT_SECRET, previous_secret),
                (ACCOUNT_REFRESH, previous_refresh),
            ] {
                match previous {
                    Some(value) => self.secrets.put(account, &value)?,
                    None => self.secrets.delete(account)?,
                }
            }
        }
        result
    }

    fn store_credentials(
        &self,
        config: &mut AppConfig,
        secret: Secret,
        refresh: Option<Secret>,
    ) -> Result<(), Error> {
        config.secrets_stored = vec![ACCOUNT_SECRET.into()];
        self.secrets.put(ACCOUNT_SECRET, &secret)?;
        if let Some(refresh) = refresh {
            self.secrets.put(ACCOUNT_REFRESH, &refresh)?;
            config.secrets_stored.push(ACCOUNT_REFRESH.into());
        } else {
            self.secrets.delete(ACCOUNT_REFRESH)?;
        }
        self.save_api_config(config)
    }

    /// Disable future remote runs without reading or replacing stored secrets.
    pub fn disable_remote(&self) -> Result<(), Error> {
        let mut config = self.api_config()?;
        config.remote_enabled = false;
        self.save_api_config(&config)
    }

    fn save_api_config(&self, config: &AppConfig) -> Result<(), Error> {
        let temporary = self
            .paths
            .root
            .join(format!("config-{}.tmp", Uuid::new_v4()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let result = (|| {
            use std::io::Write;
            let mut file = options.open(&temporary)?;
            file.write_all(
                &serde_json::to_vec_pretty(&config).map_err(|e| Error::Msg(e.to_string()))?,
            )?;
            file.sync_all()?;
            std::fs::rename(&temporary, self.paths.config_file())?;
            Ok::<_, Error>(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result?;
        Ok(())
    }

    pub fn client_id(&self) -> Result<Option<String>, Error> {
        let id = self.api_config()?.client_id;
        Ok((!id.is_empty()).then_some(id))
    }

    /// Read non-secret API settings; old profiles keep remote access disabled.
    pub fn api_config(&self) -> Result<AppConfig, Error> {
        let path = self.paths.config_file();
        if !path.exists() {
            return Ok(AppConfig::default());
        }
        let config: AppConfig = serde_json::from_slice(&std::fs::read(path)?).map_err(|_| {
            Error::Msg("invalid API configuration; use auth configure to repair it".into())
        })?;
        for url in [&config.process_url, &config.token_url] {
            if !url.is_empty() {
                endpoint_allowed(url, false, false)?;
            }
        }
        if !config.client_id.is_empty() {
            config.validate()?;
        }
        Ok(config)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    pub client_id: String,
    pub token_url: String,
    pub process_url: String,
    #[serde(default)]
    pub remote_enabled: bool,
    #[serde(default)]
    pub secrets_stored: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            token_url: crate::auth::DEFAULT_TOKEN_URL.into(),
            process_url: crate::auth::DEFAULT_PROCESS_URL.into(),
            remote_enabled: false,
            secrets_stored: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), Error> {
        if self.client_id.trim().is_empty()
            || self.client_id.trim() != self.client_id
            || self.client_id.len() > 256
            || self.client_id.chars().any(|c| c.is_control())
        {
            return Err(Error::TokenInvalid(
                "client-id must have 1 to 256 characters, without control characters",
            ));
        }
        // Disabled profiles may be saved as drafts; remote authorization still requires both URLs.
        for url in [&self.process_url, &self.token_url] {
            if !url.is_empty() {
                endpoint_allowed(url, false, false)?;
            }
        }
        if self.remote_enabled && (self.process_url.is_empty() || self.token_url.is_empty()) {
            return Err(Error::Msg(
                "Configure the token and data URLs before enabling remote access.".into(),
            ));
        }
        Ok(())
    }

    /// Authorize a remote job only for this explicitly enabled profile.
    pub fn authorize_job(&self, job: &JobConfig) -> Result<(), Error> {
        if !self.remote_enabled {
            return Err(Error::ContractPending);
        }
        self.validate()?;
        if job.test_mode || job.contract_mode()? != crate::domain::ContractMode::Pending {
            return Err(Error::Msg(
                "remote jobs must use the pending contract without test mode".into(),
            ));
        }
        if endpoint_allowed(&job.endpoint, false, false)?
            != endpoint_allowed(&self.process_url, false, false)?
            || endpoint_allowed(&job.token_url, false, false)?
                != endpoint_allowed(&self.token_url, false, false)?
            || job.client_id != self.client_id
        {
            return Err(Error::Msg(
                "job URLs and client-id must match the enabled API configuration; create a new job"
                    .into(),
            ));
        }
        Ok(())
    }
}
