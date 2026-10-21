//! Processing domain. No I/O and no third-party crates.

mod cpf;
mod state;

pub use cpf::{validate_cpf, Validation};
pub use state::{
    transition_job, transition_record, BatchState, DomainError, JobEvent, JobState, RecordEvent,
    RecordState,
};

/// Stable job identifier. Callers generate the value; this type only checks shape.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct JobId(String);

impl JobId {
    pub fn from_string(value: String) -> Result<Self, DomainError> {
        if !is_uuid(&value) {
            return Err(DomainError::BadId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn is_uuid(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 36 {
        return false;
    }
    let groups = [8, 4, 4, 4, 12];
    let mut i = 0;
    for (g, size) in groups.into_iter().enumerate() {
        if g > 0 {
            if b[i] != b'-' {
                return false;
            }
            i += 1;
        }
        for _ in 0..size {
            if !b[i].is_ascii_hexdigit() {
                return false;
            }
            i += 1;
        }
    }
    i == 36
}

/// How a business field was filled. `HttpOnly` means the body was kept as evidence
/// and was not turned into `is_customer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evidence {
    None,
    HttpOnly,
    SimulatedFixture,
}

impl Evidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::HttpOnly => "http_only",
            Self::SimulatedFixture => "simulated_fixture",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "http_only" => Self::HttpOnly,
            "simulated_fixture" => Self::SimulatedFixture,
            _ => Self::None,
        }
    }
}

/// Which response rules are allowed to set a business result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractMode {
    /// Production provider semantics are not documented. No business result is inferred.
    Pending,
    /// Local fixture documented in the project's Fixtures-e-contratos-de-teste Wiki page.
    SimulatedFixtureV1,
}

impl ContractMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::SimulatedFixtureV1 => "simulated-fixture-v1",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "pending" => Ok(Self::Pending),
            "simulated-fixture-v1" => Ok(Self::SimulatedFixtureV1),
            _ => Err(DomainError::BadContract),
        }
    }
}

/// Counters that must match a `GROUP BY state` over records.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub pending: i64,
    pub in_flight: i64,
    pub retry_wait: i64,
    pub succeeded: i64,
    pub failed_final: i64,
    pub unknown: i64,
}

impl Counts {
    pub fn total(self) -> i64 {
        self.pending
            + self.in_flight
            + self.retry_wait
            + self.succeeded
            + self.failed_final
            + self.unknown
    }

    pub fn open(self) -> i64 {
        self.pending + self.in_flight + self.retry_wait
    }

    pub fn add(&mut self, state: RecordState, delta: i64) {
        let slot = match state {
            RecordState::Pending => &mut self.pending,
            RecordState::InFlight => &mut self.in_flight,
            RecordState::RetryWait => &mut self.retry_wait,
            RecordState::Succeeded => &mut self.succeeded,
            RecordState::FailedFinal => &mut self.failed_final,
            RecordState::Unknown => &mut self.unknown,
        };
        *slot += delta;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_uuid_shape() {
        let id = JobId::from_string("550e8400-e29b-41d4-a716-446655440000".to_string()).unwrap();
        assert_eq!(id.as_str().len(), 36);
        assert!(JobId::from_string("nope".to_string()).is_err());
    }
}
