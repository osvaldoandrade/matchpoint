use std::io::{self, IsTerminal};

use clap::{Parser, Subcommand, ValueEnum};

use crate::domain::ContractMode;
use crate::engine::{App, OutFormat, RunOptions};
use crate::error::{Error, Secret};
use crate::jobcfg::{default_tuning, JobConfig};
use crate::repl::tokenize;

mod doctor;
mod prompt;
use prompt::prompt_default;
pub use prompt::prompt_hidden;

fn styles() -> clap::builder::Styles {
    use clap::builder::styling::AnsiColor;
    clap::builder::Styles::styled()
        .header(AnsiColor::Green.on_default().bold())
        .usage(AnsiColor::Green.on_default().bold())
        .literal(AnsiColor::Cyan.on_default().bold())
        .placeholder(AnsiColor::Yellow.on_default())
        .error(AnsiColor::Red.on_default().bold())
}

#[derive(Parser, Debug)]
#[command(
    name = "pgm-data",
    version,
    about = "Parter Get Parter — dataset processing tools",
    styles = styles(),
    after_help = concat!(
        "Start with: pgm-data auth configure --allow-remote\n",
        "Open the interface: pgm-data tui (S opens Settings)"
    )
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Cmd>,
}

/// Parse OS arguments while refusing secret-bearing flags before Clap prints errors.
pub fn parse_args() -> Result<Cli, Error> {
    let args: Vec<_> = std::env::args_os().collect();
    if args
        .iter()
        .skip(1)
        .any(|arg| crate::repl::sensitive(&arg.to_string_lossy()))
    {
        return Err(Error::Command(
            "Enter secrets only in Settings > Secrets or the auth configure prompt.".into(),
        ));
    }
    Ok(Cli::parse_from(args))
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Open the interactive terminal workspace.
    Tui,
    /// Check local data and API configuration.
    Doctor,
    Job {
        #[command(subcommand)]
        action: JobCmd,
    },
    Auth {
        #[command(subcommand)]
        action: AuthCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum JobCmd {
    Create {
        #[arg(long)]
        input: std::path::PathBuf,
        #[arg(long)]
        endpoint: Option<String>,
        #[arg(long)]
        token_url: Option<String>,
        #[arg(long)]
        test_mode: bool,
        #[arg(long, value_enum)]
        contract: Option<ContractArg>,
        /// Rows per batch; saved default or 1000. Retries belong to the same record.
        #[arg(long)]
        batch_size: Option<u32>,
        #[arg(long)]
        rps: Option<f64>,
        #[arg(long)]
        concurrency: Option<u32>,
        #[arg(long, value_enum)]
        validation: Option<ValidationArg>,
        #[arg(long)]
        idempotency: bool,
        #[arg(long)]
        client_id: Option<String>,
    },
    List,
    Status {
        id: String,
    },
    /// Check SHA-256 of stored input and published exports.
    Checksum {
        id: String,
    },
    /// Change threads for a stopped job; --default applies to new jobs.
    Threads {
        id: Option<String>,
        #[arg(long)]
        count: u32,
        #[arg(long, conflicts_with = "id")]
        default: bool,
    },
    Prepare {
        id: String,
    },
    Run {
        id: String,
    },
    Pause {
        id: String,
    },
    Resume {
        id: String,
    },
    RetryFailed {
        id: String,
    },
    RequeueUnknown {
        id: String,
    },
    Merge {
        id: String,
        #[arg(long, value_enum)]
        format: FormatArg,
        #[arg(long)]
        allow_unknown: bool,
    },
    Export {
        id: String,
        #[arg(long, value_enum)]
        format: FormatArg,
    },
    Drop {
        id: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum AuthCmd {
    /// Configure URLs and client ID; enter secrets in the hidden prompt.
    Configure {
        #[arg(long)]
        client_id: Option<String>,
        #[arg(long)]
        endpoint: Option<String>,
        #[arg(long)]
        token_url: Option<String>,
        /// Enable remote requests only for the saved URLs.
        #[arg(long, conflicts_with = "disable_remote")]
        allow_remote: bool,
        /// Disable future remote requests.
        #[arg(long)]
        disable_remote: bool,
    },
    /// Show URLs, client ID and remote access status.
    Status,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ContractArg {
    Pending,
    Simulated,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ValidationArg {
    None,
    Length,
    Checksum,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum FormatArg {
    Parquet,
    Csv,
}

pub fn parse_line(line: &str) -> Result<Cli, Error> {
    if crate::repl::sensitive(line) {
        return Err(Error::Command(
            "Enter secrets only in Settings > Secrets or the auth configure prompt.".into(),
        ));
    }
    let args = tokenize(line)?;
    Cli::try_parse_from(std::iter::once("pgm-data".to_string()).chain(args))
        .map_err(|err| Error::Command(err.to_string()))
}

pub async fn execute(app: &App, cmd: Cmd) -> Result<i32, Error> {
    let (code, lines) = execute_collect(app, cmd).await?;
    for line in lines {
        println!("{line}");
    }
    Ok(code)
}

/// Execute a command without writing over the alternate-screen interface.
pub async fn execute_collect(app: &App, cmd: Cmd) -> Result<(i32, Vec<String>), Error> {
    let mut lines = Vec::new();
    let code = match cmd {
        Cmd::Tui => {
            crate::tui::launch(app).await?;
            Ok(0)
        }
        Cmd::Doctor => {
            let report = app.doctor();
            lines.extend(doctor::lines(&report));
            Ok(if report.ok() { 0 } else { 1 })
        }
        Cmd::Auth { action } => match action {
            AuthCmd::Configure {
                client_id,
                endpoint,
                token_url,
                allow_remote,
                disable_remote,
            } => {
                if disable_remote
                    && client_id.is_none()
                    && endpoint.is_none()
                    && token_url.is_none()
                {
                    app.disable_remote()?;
                    lines
                        .push("Envio remoto disabled; credenciais preservadas no Keychain.".into());
                    return Ok((0, lines));
                }
                if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
                    return Err(Error::NotATty);
                }
                let mut config = app.api_config().unwrap_or_default();
                config.client_id = match client_id {
                    Some(id) => id,
                    None => prompt_default("client-id", &config.client_id)?,
                };
                config.process_url = match endpoint {
                    Some(url) => url,
                    None => prompt_default("API URL", &config.process_url)?,
                };
                config.token_url = match token_url {
                    Some(url) => url,
                    None => prompt_default("URL OAuth", &config.token_url)?,
                };
                if allow_remote {
                    config.remote_enabled = true;
                }
                if disable_remote {
                    config.remote_enabled = false;
                }
                config.validate()?;
                let secret = Secret::new(prompt_hidden("client secret: ")?)?;
                let refresh = prompt_hidden("refresh token (empty to skip): ")?;
                let refresh = if refresh.is_empty() {
                    None
                } else {
                    Some(Secret::new(refresh)?)
                };
                app.configure_api(config, secret, refresh)?;
                lines.push("Settings saved; client secret protected in Keychain.".into());
                Ok(0)
            }
            AuthCmd::Status => {
                let config = app.api_config()?;
                lines.push(format!(
                    "client-id: {}",
                    if config.client_id.is_empty() {
                        "not configured"
                    } else {
                        &config.client_id
                    }
                ));
                lines.push(format!("Account-opening URL: {}", config.process_url));
                lines.push(format!("OAuth token URL: {}", config.token_url));
                lines.push(format!(
                    "Remote access: {}",
                    if config.remote_enabled {
                        "enabled for saved URLs"
                    } else {
                        "disabled"
                    }
                ));
                lines.push("Client secret: hidden (Keychain); API credentials not verified".into());
                Ok(0)
            }
        },
        Cmd::Job { action } => execute_job(app, action, &mut lines).await,
    }?;
    Ok((code, lines))
}

async fn execute_job(app: &App, action: JobCmd, lines: &mut Vec<String>) -> Result<i32, Error> {
    match action {
        JobCmd::Create {
            input,
            endpoint,
            token_url,
            test_mode,
            contract,
            batch_size,
            rps,
            concurrency,
            validation,
            idempotency,
            client_id,
        } => {
            let config = config_from_flags(
                endpoint,
                token_url,
                test_mode,
                contract,
                batch_size,
                rps,
                concurrency,
                validation,
                idempotency,
                client_id,
                app,
            )?;
            let id = app.create_job(&input, config, None)?;
            lines.push(id);
            Ok(0)
        }
        JobCmd::List => {
            for job in app.list_jobs()? {
                lines.push(format!(
                    "{} {} pending={} succeeded={} failed={} unknown={}",
                    job.id,
                    job.state,
                    job.counts.pending,
                    job.counts.succeeded,
                    job.counts.failed_final,
                    job.counts.unknown
                ));
            }
            Ok(0)
        }
        JobCmd::Status { id } => {
            let job = app.status(&id)?;
            let config = app.job_config(&id)?;
            lines.push(format!(
                "{} {} desired={} total={} threads={} RPS={} file={}",
                job.id,
                job.state,
                job.desired,
                job.counts.total(),
                config.concurrency,
                config.rps,
                config.input_name
            ));
            Ok(0)
        }
        JobCmd::Threads { id, count, default } => {
            if default {
                app.set_default_threads(count)?;
                lines.push(format!("Default saved: {count} threads for new jobs."));
            } else {
                let id =
                    id.ok_or_else(|| Error::Command("Specify a job or use --default.".into()))?;
                app.configure_threads(&id, count)?;
                lines.push(format!("Job {id}: {count} threads saved. RPS preserved."));
            }
            Ok(0)
        }
        JobCmd::Checksum { id } => {
            let report = app.verify_checksum(&id)?;
            lines.push(format!(
                "SHA-256 · {} · {}",
                report.job_id, report.checked_at
            ));
            for artifact in &report.artifacts {
                lines.push(format!(
                    "{}: {}",
                    artifact.name,
                    if artifact.matches {
                        "MATCH"
                    } else {
                        "MISMATCH"
                    }
                ));
                lines.push(format!("Expected: {}", artifact.expected));
                lines.push(format!("Actual:   {}", artifact.actual));
            }
            Ok(if report.matches() { 0 } else { 1 })
        }
        JobCmd::Prepare { id } => {
            app.prepare_job(&id, None)?;
            lines.push("Preparation completed".into());
            Ok(0)
        }
        JobCmd::Run { id } | JobCmd::Resume { id } => {
            let (tx, rx) = tokio::sync::watch::channel(false);
            let tx2 = tx.clone();
            tokio::spawn(async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    let _ = tx2.send(true);
                }
            });
            let mut opts = RunOptions::with_stop(rx);
            opts.grace = std::time::Duration::from_secs(15);
            let report = app.run_job(&id, opts).await?;
            let _ = tx.send(true);
            lines.push(report.state);
            Ok(0)
        }
        JobCmd::Pause { id } => {
            app.pause_job(&id)?;
            lines.push("Pause requested".into());
            Ok(0)
        }
        JobCmd::RetryFailed { id } => {
            let n = app.retry_failed(&id)?;
            lines.push(format!("{n} records requeued"));
            Ok(0)
        }
        JobCmd::RequeueUnknown { id } => {
            let n = app.requeue_unknown(&id)?;
            lines.push(format!("{n} records requeued"));
            Ok(0)
        }
        JobCmd::Merge {
            id,
            format,
            allow_unknown,
        } => {
            let report = app.merge(&id, map_format(format), allow_unknown, false)?;
            lines.push(format!(
                "SHA-256 {} · rows={} · partial={}",
                report.sha256, report.rows, report.partial
            ));
            lines.push(format!(
                "File: {}",
                app.paths
                    .job_dir(&report.job_id)
                    .join("exports")
                    .join(format!(
                        "gen-{:04}/merged.{}",
                        report.generation, report.format
                    ))
                    .display()
            ));
            lines.push("Check integrity with K or job checksum <id>.".into());
            Ok(0)
        }
        JobCmd::Export { id, format } => {
            let report = app.merge(&id, map_format(format), false, false)?;
            lines.push(format!("{} rows={}", report.sha256, report.rows));
            Ok(0)
        }
        JobCmd::Drop { id, yes } => {
            app.drop_job(&id, yes)?;
            lines.push("Job removed".into());
            Ok(0)
        }
    }
}

fn map_format(format: FormatArg) -> OutFormat {
    match format {
        FormatArg::Parquet => OutFormat::Parquet,
        FormatArg::Csv => OutFormat::Csv,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn config_from_flags(
    endpoint: Option<String>,
    token_url: Option<String>,
    test_mode: bool,
    contract: Option<ContractArg>,
    batch_size: Option<u32>,
    rps: Option<f64>,
    concurrency: Option<u32>,
    validation: Option<ValidationArg>,
    idempotency: bool,
    client_id: Option<String>,
    app: &App,
) -> Result<JobConfig, Error> {
    let mut config = default_tuning();
    let defaults = app.processing_defaults()?;
    config.concurrency = defaults.threads;
    config.rps = defaults.rps;
    config.batch_size = defaults.batch_size;
    config.validation = defaults.validation;
    let saved = app.api_config()?;
    config.endpoint = endpoint.unwrap_or(saved.process_url);
    config.token_url = token_url.unwrap_or(saved.token_url);
    config.test_mode = test_mode;
    config.contract = match contract {
        Some(ContractArg::Pending) => ContractMode::Pending.as_str().into(),
        Some(ContractArg::Simulated) => ContractMode::SimulatedFixtureV1.as_str().into(),
        None if test_mode => ContractMode::SimulatedFixtureV1.as_str().into(),
        None => ContractMode::Pending.as_str().into(),
    };
    if let Some(batch_size) = batch_size {
        config.batch_size = batch_size;
    }
    if let Some(rps) = rps {
        config.rps = rps;
    }
    if let Some(concurrency) = concurrency {
        config.concurrency = concurrency;
    }
    if let Some(validation) = validation {
        config.validation = match validation {
            ValidationArg::None => "none",
            ValidationArg::Length => "length",
            ValidationArg::Checksum => "checksum",
        }
        .into();
    }
    config.idempotency = idempotency;
    config.client_id = client_id.or(app.client_id()?).unwrap_or_default();
    Ok(config)
}
