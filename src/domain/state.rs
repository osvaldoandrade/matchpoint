//! Explicit state machines for jobs, batches, and records.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordState {
    Pending,
    InFlight,
    RetryWait,
    Succeeded,
    FailedFinal,
    Unknown,
}

impl RecordState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::InFlight => "IN_FLIGHT",
            Self::RetryWait => "RETRY_WAIT",
            Self::Succeeded => "SUCCEEDED",
            Self::FailedFinal => "FAILED_FINAL",
            Self::Unknown => "UNKNOWN",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "PENDING" => Ok(Self::Pending),
            "IN_FLIGHT" => Ok(Self::InFlight),
            "RETRY_WAIT" => Ok(Self::RetryWait),
            "SUCCEEDED" => Ok(Self::Succeeded),
            "FAILED_FINAL" => Ok(Self::FailedFinal),
            "UNKNOWN" => Ok(Self::Unknown),
            _ => Err(DomainError::BadState),
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::FailedFinal | Self::Unknown)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordEvent {
    Claim,
    RejectLocal,
    Succeed,
    FailFinal,
    ScheduleRetry,
    LoseOutcome,
    ReleaseUnsent,
    Requeue,
}

impl RecordEvent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claim => "Claim",
            Self::RejectLocal => "RejectLocal",
            Self::Succeed => "Succeed",
            Self::FailFinal => "FailFinal",
            Self::ScheduleRetry => "ScheduleRetry",
            Self::LoseOutcome => "LoseOutcome",
            Self::ReleaseUnsent => "ReleaseUnsent",
            Self::Requeue => "Requeue",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobState {
    Preparing,
    PrepareFailed,
    Prepared,
    Running,
    Paused,
    Interrupted,
    Completed,
    CompletedWithErrors,
    BlockedUnknown,
}

impl JobState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Preparing => "PREPARING",
            Self::PrepareFailed => "PREPARE_FAILED",
            Self::Prepared => "PREPARED",
            Self::Running => "RUNNING",
            Self::Paused => "PAUSED",
            Self::Interrupted => "INTERRUPTED",
            Self::Completed => "COMPLETED",
            Self::CompletedWithErrors => "COMPLETED_WITH_ERRORS",
            Self::BlockedUnknown => "BLOCKED_UNKNOWN",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "PREPARING" => Ok(Self::Preparing),
            "PREPARE_FAILED" => Ok(Self::PrepareFailed),
            "PREPARED" => Ok(Self::Prepared),
            "RUNNING" => Ok(Self::Running),
            "PAUSED" => Ok(Self::Paused),
            "INTERRUPTED" => Ok(Self::Interrupted),
            "COMPLETED" => Ok(Self::Completed),
            "COMPLETED_WITH_ERRORS" => Ok(Self::CompletedWithErrors),
            "BLOCKED_UNKNOWN" => Ok(Self::BlockedUnknown),
            _ => Err(DomainError::BadState),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobEvent {
    PreparedOk,
    FailPrepare,
    Start,
    Pause,
    Interrupt,
    Resume,
    Complete,
    CompleteWithErrors,
    BlockUnknown,
    RetryFailed,
    RequeueUnknown,
}

impl JobEvent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreparedOk => "PreparedOk",
            Self::FailPrepare => "FailPrepare",
            Self::Start => "Start",
            Self::Pause => "Pause",
            Self::Interrupt => "Interrupt",
            Self::Resume => "Resume",
            Self::Complete => "Complete",
            Self::CompleteWithErrors => "CompleteWithErrors",
            Self::BlockUnknown => "BlockUnknown",
            Self::RetryFailed => "RetryFailed",
            Self::RequeueUnknown => "RequeueUnknown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchState {
    Pending,
    Running,
    Completed,
    CompletedWithErrors,
}

impl BatchState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Running => "RUNNING",
            Self::Completed => "COMPLETED",
            Self::CompletedWithErrors => "COMPLETED_WITH_ERRORS",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "PENDING" => Ok(Self::Pending),
            "RUNNING" => Ok(Self::Running),
            "COMPLETED" => Ok(Self::Completed),
            "COMPLETED_WITH_ERRORS" => Ok(Self::CompletedWithErrors),
            _ => Err(DomainError::BadState),
        }
    }

    /// Derive the batch state from its counters. Open work wins over a terminal label.
    pub fn from_counts(
        pending: i64,
        in_flight: i64,
        retry_wait: i64,
        failed: i64,
        unknown: i64,
    ) -> Self {
        if in_flight > 0 {
            return Self::Running;
        }
        if pending > 0 || retry_wait > 0 {
            return Self::Pending;
        }
        if failed > 0 || unknown > 0 {
            Self::CompletedWithErrors
        } else {
            Self::Completed
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainError {
    BadId,
    BadState,
    BadContract,
    IllegalTransition {
        entity: &'static str,
        from: String,
        event: String,
    },
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadId => write!(f, "invalid job id"),
            Self::BadState => write!(f, "unknown state"),
            Self::BadContract => write!(f, "unknown contract mode"),
            Self::IllegalTransition {
                entity,
                from,
                event,
            } => {
                write!(f, "illegal transition for {entity}: {from} + {event}")
            }
        }
    }
}

pub fn transition_record(
    from: RecordState,
    event: RecordEvent,
) -> Result<RecordState, DomainError> {
    use RecordEvent::*;
    use RecordState::*;
    let to = match (from, event) {
        (Pending, Claim) => InFlight,
        (Pending, RejectLocal) => FailedFinal,
        (RetryWait, Claim) => InFlight,
        (InFlight, Succeed) => Succeeded,
        (InFlight, FailFinal) => FailedFinal,
        (InFlight, ScheduleRetry) => RetryWait,
        (InFlight, LoseOutcome) => Unknown,
        (InFlight, ReleaseUnsent) => RetryWait,
        (FailedFinal, Requeue) => Pending,
        (Unknown, Requeue) => Pending,
        _ => {
            return Err(DomainError::IllegalTransition {
                entity: "record",
                from: from.as_str().to_string(),
                event: event.as_str().to_string(),
            });
        }
    };
    Ok(to)
}

pub fn transition_job(from: JobState, event: JobEvent) -> Result<JobState, DomainError> {
    use JobEvent::*;
    use JobState::*;
    let to = match (from, event) {
        (Preparing, PreparedOk) => Prepared,
        (Preparing, FailPrepare) => PrepareFailed,
        (Preparing, Interrupt) => Preparing,
        (PrepareFailed, Resume) => Preparing,
        (Prepared, Start) => Running,
        (Paused, Resume) => Running,
        (Interrupted, Resume) => Running,
        (BlockedUnknown, RequeueUnknown) => Prepared,
        (CompletedWithErrors, RetryFailed) => Prepared,
        (Running, Pause) => Paused,
        (Running, Interrupt) => Interrupted,
        (Running, Complete) => Completed,
        (Running, CompleteWithErrors) => CompletedWithErrors,
        (Running, BlockUnknown) => BlockedUnknown,
        _ => {
            return Err(DomainError::IllegalTransition {
                entity: "job",
                from: from.as_str().to_string(),
                event: event.as_str().to_string(),
            });
        }
    };
    Ok(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_record_transitions() {
        let cases = [
            (
                RecordState::Pending,
                RecordEvent::Claim,
                RecordState::InFlight,
            ),
            (
                RecordState::Pending,
                RecordEvent::RejectLocal,
                RecordState::FailedFinal,
            ),
            (
                RecordState::RetryWait,
                RecordEvent::Claim,
                RecordState::InFlight,
            ),
            (
                RecordState::InFlight,
                RecordEvent::Succeed,
                RecordState::Succeeded,
            ),
            (
                RecordState::InFlight,
                RecordEvent::FailFinal,
                RecordState::FailedFinal,
            ),
            (
                RecordState::InFlight,
                RecordEvent::ScheduleRetry,
                RecordState::RetryWait,
            ),
            (
                RecordState::InFlight,
                RecordEvent::LoseOutcome,
                RecordState::Unknown,
            ),
            (
                RecordState::InFlight,
                RecordEvent::ReleaseUnsent,
                RecordState::RetryWait,
            ),
            (
                RecordState::FailedFinal,
                RecordEvent::Requeue,
                RecordState::Pending,
            ),
            (
                RecordState::Unknown,
                RecordEvent::Requeue,
                RecordState::Pending,
            ),
        ];
        for (from, event, to) in cases {
            assert_eq!(transition_record(from, event).unwrap(), to);
        }
    }

    #[test]
    fn illegal_record_transitions() {
        let bad = [
            (RecordState::Succeeded, RecordEvent::Claim),
            (RecordState::Succeeded, RecordEvent::Requeue),
            (RecordState::FailedFinal, RecordEvent::Succeed),
            (RecordState::Unknown, RecordEvent::Succeed),
            (RecordState::Pending, RecordEvent::Succeed),
            (RecordState::Pending, RecordEvent::LoseOutcome),
            (RecordState::RetryWait, RecordEvent::Succeed),
        ];
        for (from, event) in bad {
            assert!(transition_record(from, event).is_err());
        }
    }

    #[test]
    fn legal_job_transitions() {
        let cases = [
            (
                JobState::Preparing,
                JobEvent::PreparedOk,
                JobState::Prepared,
            ),
            (
                JobState::Preparing,
                JobEvent::FailPrepare,
                JobState::PrepareFailed,
            ),
            (
                JobState::Preparing,
                JobEvent::Interrupt,
                JobState::Preparing,
            ),
            (
                JobState::PrepareFailed,
                JobEvent::Resume,
                JobState::Preparing,
            ),
            (JobState::Prepared, JobEvent::Start, JobState::Running),
            (JobState::Running, JobEvent::Pause, JobState::Paused),
            (
                JobState::Running,
                JobEvent::Interrupt,
                JobState::Interrupted,
            ),
            (JobState::Running, JobEvent::Complete, JobState::Completed),
            (
                JobState::Running,
                JobEvent::CompleteWithErrors,
                JobState::CompletedWithErrors,
            ),
            (
                JobState::Running,
                JobEvent::BlockUnknown,
                JobState::BlockedUnknown,
            ),
            (JobState::Paused, JobEvent::Resume, JobState::Running),
            (JobState::Interrupted, JobEvent::Resume, JobState::Running),
            (
                JobState::BlockedUnknown,
                JobEvent::RequeueUnknown,
                JobState::Prepared,
            ),
            (
                JobState::CompletedWithErrors,
                JobEvent::RetryFailed,
                JobState::Prepared,
            ),
        ];
        for (from, event, to) in cases {
            assert_eq!(transition_job(from, event).unwrap(), to);
        }
    }

    #[test]
    fn completed_job_has_no_outbound_transition() {
        for event in [
            JobEvent::Start,
            JobEvent::Pause,
            JobEvent::Resume,
            JobEvent::RetryFailed,
            JobEvent::Complete,
        ] {
            assert!(transition_job(JobState::Completed, event).is_err());
        }
    }

    #[test]
    fn random_walk_stays_inside_the_table() {
        let mut state = RecordState::Pending;
        let mut seed = 0x1234_5678_u64;
        for _ in 0..200 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let event = match seed % 8 {
                0 => RecordEvent::Claim,
                1 => RecordEvent::RejectLocal,
                2 => RecordEvent::Succeed,
                3 => RecordEvent::FailFinal,
                4 => RecordEvent::ScheduleRetry,
                5 => RecordEvent::LoseOutcome,
                6 => RecordEvent::ReleaseUnsent,
                _ => RecordEvent::Requeue,
            };
            match transition_record(state, event) {
                Ok(next) => state = next,
                Err(DomainError::IllegalTransition { .. }) => {}
                Err(other) => panic!("unexpected {other}"),
            }
        }
    }

    #[test]
    fn batch_state_follows_counters() {
        assert_eq!(BatchState::from_counts(1, 0, 0, 0, 0), BatchState::Pending);
        assert_eq!(BatchState::from_counts(0, 1, 0, 0, 0), BatchState::Running);
        assert_eq!(
            BatchState::from_counts(0, 0, 0, 0, 0),
            BatchState::Completed
        );
        assert_eq!(
            BatchState::from_counts(0, 0, 0, 2, 0),
            BatchState::CompletedWithErrors
        );
        assert_eq!(
            BatchState::from_counts(0, 0, 0, 0, 1),
            BatchState::CompletedWithErrors
        );
    }
}
