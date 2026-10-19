use crate::error::Error;

#[derive(Clone, Debug)]
pub struct Feedback {
    pub title: String,
    pub detail: String,
    pub recovery: String,
}

pub fn safe(text: &str) -> String {
    let lower = text.to_ascii_lowercase().replace('-', "_");
    if [
        "client_secret=",
        "access_token=",
        "refresh_token=",
        "bearer ",
        "\"client_secret\"",
        "\"access_token\"",
        "\"refresh_token\"",
    ]
    .iter()
    .any(|pattern| lower.contains(pattern))
    {
        return "[sensitive content hidden; use Settings]".into();
    }
    text.chars()
        .filter(|ch| !ch.is_control() || *ch == '\n')
        .collect()
}

impl Feedback {
    pub fn from_error(err: &Error) -> Self {
        let (title, detail, recovery) = match err {
            Error::Msg(text) if text.starts_with("Threads must") => (
                "Invalid threads",
                "Use an integer from 1 to 256.".into(),
                "Fix the value and save.",
            ),
            Error::Msg(text) if text.starts_with("RPS must") => {
                ("Invalid request rate", safe(text), "Fix RPS; Ctrl+S saves.")
            }
            Error::Msg(text) if text.starts_with("Batch size must") => (
                "Invalid batch size",
                safe(text),
                "Fix rows/batch; Ctrl+S saves.",
            ),
            Error::Msg(text) if text.starts_with("Enter the full CSV") => (
                "CSV missing",
                "Enter the complete file path.".into(),
                "Type the path; Enter creates.",
            ),
            Error::Msg(text) if text.starts_with("Enter a client secret") => (
                "Secret required",
                "This profile needs a client secret.".into(),
                "Settings > Secrets: enter and save.",
            ),
            Error::Msg(text) if text.starts_with("Isolated preview:") => (
                "Isolated preview",
                "Font saved; preview cannot open Ghostty.".into(),
                "Use run-ansi.command in the app.",
            ),
            Error::ThreadsBusy => (
                "Threads locked",
                "Job is running.".into(),
                "P: pause; wait, then T: threads.",
            ),
            Error::LocalBusy => (
                "Local operation active",
                "Data is still being written.".into(),
                "Wait; navigation remains available.",
            ),
            Error::Csv(_) => (
                "Invalid CSV",
                "CSV import did not finish.".into(),
                "N: fix file/path and try again.",
            ),
            Error::JobNotRunning { state } => (
                "Job stopped",
                format!("State: {}.", super::view::state_label(state)),
                "R: run · H: home.",
            ),
            Error::Locked => (
                "Job busy",
                "Another run holds this job's lock.".into(),
                "P: pause; wait for Paused.",
            ),
            Error::AuthMissing => (
                "Credentials missing",
                "Client ID or secret is missing.".into(),
                "S: Settings · R: resume.",
            ),
            Error::AuthPaused => (
                "Access rejected",
                "Requests stopped after an access error.".into(),
                "S: fix credentials · R: resume.",
            ),
            Error::TokenInvalid(reason) if reason.contains("client-id") => (
                "Invalid client ID",
                "Use 1–256 characters, no controls.".into(),
                "Settings > API: fix client ID.",
            ),
            Error::TokenInvalid(_) => (
                "Token rejected",
                "API did not return a valid token.".into(),
                "S: check credentials · R: resume.",
            ),
            Error::ContractPending => (
                "Remote disabled",
                "This profile does not allow requests.".into(),
                "S: Settings > API: enable remote.",
            ),
            Error::ChecksumMismatch => (
                "Checksum mismatch",
                "input.csv changed; requests blocked.".into(),
                "K: inspect · N: create new job.",
            ),
            Error::UnknownBlocks { unknown } => (
                "Export blocked",
                format!("{unknown} uncertain results (UNKNOWN)."),
                "E: enable PARTIAL to export.",
            ),
            Error::NotTerminal { open } => (
                "Job incomplete",
                format!("{open} records remain open."),
                "R: resume · E: export when done.",
            ),
            Error::LowSpace { free_mib, need_mib } => (
                "Low disk space",
                format!("Free: {free_mib} MiB; need: {need_mib} MiB."),
                "Free local disk space; R: resume.",
            ),
            Error::DiskFull => (
                "Write failed",
                "Requests stopped; last commit preserved.".into(),
                "Free space; /doctor, then R: resume.",
            ),
            Error::NotFound(_) | Error::BadId | Error::Ambiguous(_) => (
                "Job not found",
                "ID is missing or ambiguous.".into(),
                "H: home · J: select a job.",
            ),
            Error::UnclosedQuote => (
                "Unclosed quote",
                "A quote or escape is incomplete.".into(),
                "Close quotes; N imports directly.",
            ),
            Error::InsecureUrl | Error::UrlSecret => (
                "URL rejected",
                "HTTPS required; no secrets in URLs.".into(),
                "Settings > API: fix URLs.",
            ),
            Error::IdempotencyGate => (
                "Idempotency unavailable",
                "The selected contract does not permit it.".into(),
                "Check the contract before enabling.",
            ),
            Error::Io(e) if e.kind() == std::io::ErrorKind::NotFound => (
                "File not found",
                "The file path does not exist.".into(),
                "N: check the CSV path and filename.",
            ),
            Error::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied => (
                "File access denied",
                "Cannot read or write this path.".into(),
                "Check permissions; /doctor for data.",
            ),
            Error::Command(text) => (
                "Invalid command",
                safe(text),
                "Fix the input; / opens suggestions.",
            ),
            _ => (
                "Operation failed",
                safe(&err.to_string()),
                "Check Events, fix the cause and retry.",
            ),
        };
        Self {
            title: title.into(),
            detail,
            recovery: recovery.into(),
        }
    }
}
