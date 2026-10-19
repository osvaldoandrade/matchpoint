use serde::{Deserialize, Serialize};

use crate::domain::{ContractMode, Validation};

pub const APP_SCHEMA: u32 = 1;
pub const DEFAULT_BATCH_SIZE: u32 = 1_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobConfig {
    pub job_id: String,
    pub endpoint: String,
    pub token_url: String,
    pub test_mode: bool,
    pub contract: String,
    pub batch_size: u32,
    pub rps: f64,
    pub concurrency: u32,
    pub max_attempts: u32,
    pub commit_max_rows: u32,
    pub commit_max_ms: u64,
    pub response_timeout_ms: u64,
    pub connect_timeout_ms: u64,
    pub token_skew_secs: u64,
    pub circuit_threshold: u32,
    pub circuit_open_ms: u64,
    pub backoff_base_ms: u64,
    pub backoff_max_ms: u64,
    pub validation: String,
    pub idempotency: bool,
    pub max_response_bytes: usize,
    pub input_sha256: String,
    pub input_name: String,
    pub client_id: String,
}

impl JobConfig {
    pub fn contract_mode(&self) -> Result<ContractMode, crate::domain::DomainError> {
        ContractMode::parse(&self.contract)
    }

    pub fn validation_mode(&self) -> Validation {
        Validation::parse(&self.validation).unwrap_or(Validation::Length)
    }
}

pub fn default_tuning() -> JobConfig {
    JobConfig {
        job_id: String::new(),
        endpoint: crate::auth::DEFAULT_PROCESS_URL.into(),
        token_url: crate::auth::DEFAULT_TOKEN_URL.into(),
        test_mode: false,
        contract: ContractMode::Pending.as_str().into(),
        batch_size: DEFAULT_BATCH_SIZE,
        rps: 10.0,
        concurrency: 8,
        max_attempts: 5,
        commit_max_rows: 500,
        commit_max_ms: 250,
        response_timeout_ms: 30_000,
        connect_timeout_ms: 5_000,
        token_skew_secs: 60,
        circuit_threshold: 20,
        circuit_open_ms: 5_000,
        backoff_base_ms: 200,
        backoff_max_ms: 30_000,
        validation: Validation::Length.as_str().into(),
        idempotency: false,
        max_response_bytes: 65_536,
        input_sha256: String::new(),
        input_name: String::new(),
        client_id: String::new(),
    }
}
