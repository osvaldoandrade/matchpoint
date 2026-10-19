//! Executable commands for local matching; no per-identifier HTTP action exists here.
use super::{bad, format, Rule, RunControl, Service, Transform};
use crate::{
    engine::App,
    error::{Error, Secret},
};
use clap::{Args, Parser, Subcommand};
use std::{io::IsTerminal, path::PathBuf};

#[derive(Parser)]
#[command(
    name = "pgm-data",
    version,
    about = "MATCHPOINT — Ultra-fast dataset matching",
    long_about = "MATCHPOINT — Ultra-fast, safe dataset matching.\n\nMatch customer, device, product or other identifiers in CSV/Parquet against very large reference collections using local Filtering. Stream input in bounded batches and share one immutable reference across workers. Identifiers stay local; true indicates possible presence and false indicates absence under the same key rules. Integrity or input errors stop the job.",
    after_long_help = "EXAMPLES:\n  pgm-data tui\n  pgm-data match --input keys.csv --key identifier --transform sha256\n  pgm-data job resume JOB_ID\n\nMANUAL:\n  pgm-data manual | less\n  pgm-data manual --roff > /tmp/pgm-data.1\n  man /tmp/pgm-data.1\n\nUse COMMAND --help for command flags. F1 opens the searchable TUI manual."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Cmd>,
}
#[derive(Subcommand)]
pub enum Cmd {
    /// Print the complete manual without opening data storage or Keychain.
    Manual {
        /// Emit Unix man (roff) source instead of plain text.
        #[arg(long)]
        roff: bool,
    },
    /// Open Home, sign in and use the interactive workspace.
    Tui,
    /// Report local source/RAM/job health; does not contact the API.
    Doctor,
    /// Configure hidden OAuth credentials, acquire or renew access.
    Auth {
        #[command(subcommand)]
        action: AuthCmd,
    },
    /// Configure, download, inspect or build Filtering data.
    Bloom {
        #[command(subcommand)]
        action: BloomCmd,
    },
    /// Snapshot an input and run a durable local matching job.
    Match {
        /// Source CSV/Parquet; the job snapshots it before matching.
        #[arg(long)]
        input: PathBuf,
        /// Primary result file/directory (default: ./result-JOB_ID.txt); never overwrite.
        #[arg(long)]
        output: Option<PathBuf>,
        #[command(flatten)]
        rule: RuleArgs,
        /// CPU matching workers sharing one Filtering snapshot (1-256; default from Settings).
        #[arg(long, value_parser=clap::value_parser!(u16).range(1..=256))]
        threads: Option<u16>,
    },
    /// Prepare, inspect, resume, verify or export durable jobs.
    Job {
        #[command(subcommand)]
        action: JobCmd,
    },
}
#[derive(Subcommand)]
pub enum AuthCmd {
    /// Obtain an access token using client_credentials.
    GetToken,
    /// Renew access at the same token URL using refresh_token.
    RefreshToken,
    /// Enter client secret and an optional refresh token in hidden prompts.
    Credentials,
}
#[derive(Subcommand)]
pub enum BloomCmd {
    /// Build a native GZip DAT with a 0.1% target false-positive rate.
    Build {
        /// CSV/Parquet containing the identifiers to include in the filter.
        #[arg(long)]
        input: PathBuf,
        /// Destination for the native GZip DAT.
        #[arg(long)]
        output: PathBuf,
        #[command(flatten)]
        rule: RuleArgs,
    },
    /// Verify a local raw/GZip DAT and print its parameters as JSON.
    Inspect {
        /// Local raw/GZip DAT; validates header, bits and checksum.
        file: PathBuf,
        /// Maximum decoded bit-array size in MiB (1-4096).
        #[arg(long, default_value_t = 512)]
        max_memory_mib: u64,
    },
    /// Save external source URLs and OAuth profile (no secret arguments).
    Configure {
        /// DAT download URL (HTTPS, no credentials/query/fragment/redirect).
        #[arg(long = "dat-url", alias = "url")]
        url: Option<String>,
        /// Common OAuth acquisition and refresh URL; initially unset.
        #[arg(long)]
        token_url: Option<String>,
        #[arg(long, hide = true)]
        reset_token_url: Option<String>,
        /// OAuth client identity; enter its secret with auth credentials.
        #[arg(long)]
        client_id: Option<String>,
        /// Optional OAuth scope; omitted flags keep their saved values.
        #[arg(long)]
        scope: Option<String>,
        /// Use OAuth for DAT requests; a bare flag means true.
        #[arg(long,num_args=0..=1,default_missing_value="true",action=clap::ArgAction::Set)]
        oauth: Option<bool>,
        /// Permit HTTP only on loopback fixture hosts; default false.
        #[arg(long,num_args=0..=1,default_missing_value="true",action=clap::ArgAction::Set)]
        allow_http_loopback: Option<bool>,
    },
    /// Download and verify in locked RAM; this command releases it on exit.
    Download,
    /// Enter an optional bearer token in a hidden prompt; never as a flag.
    Token,
}
#[derive(Subcommand)]
pub enum JobCmd {
    /// Snapshot input and pin rules without running the job.
    Create {
        /// Source CSV/Parquet to snapshot; prints the new job ID.
        #[arg(long)]
        input: PathBuf,
        #[command(flatten)]
        rule: RuleArgs,
        #[arg(long, value_parser=clap::value_parser!(u16).range(1..=256))]
        threads: Option<u16>,
    },
    /// List job IDs, state, progress and result counts.
    List,
    /// Print the selected job's complete metadata as JSON.
    Status { id: String },
    /// Run a prepared or interrupted job against its pinned snapshot.
    Run {
        id: String,
        #[arg(long, value_parser=clap::value_parser!(u16).range(1..=256))]
        threads: Option<u16>,
    },
    /// Resume from the last committed checkpoint (same behavior as run).
    Resume {
        id: String,
        #[arg(long, value_parser=clap::value_parser!(u16).range(1..=256))]
        threads: Option<u16>,
    },
    /// Change a prepared/paused job's CPU worker count.
    Threads {
        id: String,
        #[arg(long, value_parser=clap::value_parser!(u16).range(1..=256))]
        threads: u16,
    },
    /// Delete this stored job, input snapshot and results; keep external files.
    Delete {
        id: String,
        /// Confirm irreversible deletion of the selected job.
        #[arg(long)]
        yes: bool,
    },
    /// Verify stored input and committed output integrity.
    Checksum { id: String },
    /// Copy a complete result; refuse an existing destination.
    Export {
        /// Completed job UUID; partial results cannot be exported.
        id: String,
        /// Destination file/directory (default: ./result-JOB_ID.txt); never overwrite.
        #[arg(long, default_value = ".")]
        output: PathBuf,
    },
}
#[derive(Args, Default)]
pub struct RuleArgs {
    /// Header name or one-based #index, e.g. customer_id or #1.
    #[arg(long, alias = "key-column")]
    pub key: Option<String>,
    /// Whether CSV has a header row; headerless input requires --key '#N'.
    #[arg(long,action=clap::ArgAction::Set)]
    pub headers: Option<bool>,
    /// Single ASCII separator, or "tab".
    #[arg(long)]
    pub delimiter: Option<String>,
    /// Lookup encoding; provider DAT: sha256 for raw keys, none for hashes.
    #[arg(long, value_enum)]
    pub transform: Option<Transform>,
    /// Trim surrounding whitespace for lookup; retain original output keys.
    #[arg(long,action=clap::ArgAction::Set)]
    pub trim: Option<bool>,
}
impl RuleArgs {
    pub fn apply(self, mut rule: Rule) -> Result<Rule, Error> {
        if let Some(value) = self.key {
            rule.column = value;
        }
        if let Some(value) = self.headers {
            rule.headers = value;
        }
        if let Some(value) = self.delimiter {
            rule.delimiter = parse_delimiter(&value)?;
        }
        if let Some(value) = self.transform {
            rule.transform = value;
        }
        if let Some(value) = self.trim {
            rule.trim = value;
        }
        rule.validate()?;
        Ok(rule)
    }
}
pub fn parse_delimiter(value: &str) -> Result<u8, Error> {
    if value == "tab" || value == "\\t" {
        return Ok(b'\t');
    }
    if value.len() != 1 {
        return Err(bad("CSV delimiter must be one ASCII character or tab."));
    }
    Ok(value.as_bytes()[0])
}
pub fn parse_args() -> Result<Cli, Error> {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args.iter().skip(1).any(|s| {
        let s = s.to_string_lossy().to_lowercase();
        [
            "--token",
            "--bearer",
            "--client-secret",
            "--secret",
            "--refresh-token",
            "--access-token",
            "--aes-key",
            "--dat-key",
            "--encryption-key",
        ]
        .iter()
        .any(|flag| s == *flag || s.starts_with(&format!("{flag}=")))
    }) {
        return Err(bad(
            "Enter credentials through Home sign-in, auth credentials or bloom token.",
        ));
    }
    Ok(Cli::parse_from(args))
}
pub async fn execute(app: &App, command: Cmd) -> Result<i32, Error> {
    let service = app.matching()?;
    match command {
        Cmd::Manual { roff } => print!("{}", super::manual::text(roff)),
        Cmd::Tui => super::ui::launch(app).await?,
        Cmd::Auth { action } => match action {
            AuthCmd::Credentials => {
                if !std::io::stdin().is_terminal() {
                    return Err(Error::NotATty);
                }
                let secret = Secret::new(crate::cli::prompt_hidden("Client secret: ")?)?;
                let refresh =
                    crate::cli::prompt_hidden("Refresh token (Enter to keep existing): ")?;
                let refresh = if refresh.is_empty() {
                    None
                } else {
                    Some(Secret::new(refresh)?)
                };
                service.save_oauth_credentials(Some(&secret), refresh.as_ref())?;
                println!("OAuth credentials stored in Keychain.");
            }
            AuthCmd::GetToken | AuthCmd::RefreshToken => {
                let refresh = matches!(action, AuthCmd::RefreshToken);
                let task = async {
                    if refresh {
                        service.refresh_token().await
                    } else {
                        service.obtain_token().await
                    }
                };
                let status = tokio::select! {result=task=>result?,_=tokio::signal::ctrl_c()=>return Err(bad("Authentication cancelled."))};
                println!(
                    "Token {} · expires in {} seconds.",
                    if refresh { "renewed" } else { "obtained" },
                    status.expires_in
                );
            }
        },
        Cmd::Doctor => {
            let cfg = service.settings()?;
            println!(
                "DAT source: {}",
                if cfg.url.is_empty() {
                    "not configured"
                } else {
                    "configured"
                }
            );
            println!("Jobs: {}", service.jobs()?.len());
            if let Some(snapshot) = service.cached(None)? {
                println!(
                    "DAT verified: {} · {:.1} MiB · FPP {:.3}%",
                    snapshot.bloom.info().sha256,
                    snapshot.bloom.info().bytes as f64 / 1048576.0,
                    snapshot.bloom.info().target_fpp * 100.0
                );
            }
        }
        Cmd::Bloom { action } => bloom(&service, action).await?,
        Cmd::Match {
            input,
            output,
            rule,
            threads,
        } => {
            prompt_dat_key(&service)?;
            let rule = rule.apply(service.settings()?.rule)?;
            let job = prepare(&service, input, rule, output).await?;
            run(&service, &job.id, threads).await?;
        }
        Cmd::Job { action } => job(&service, action).await?,
    }
    Ok(0)
}
async fn bloom(service: &Service, action: BloomCmd) -> Result<(), Error> {
    match action {
        BloomCmd::Build {
            input,
            output,
            rule,
        } => {
            let rule = rule.apply(Rule::default())?;
            let info = format::build_dataset(&input, &output, &rule)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&info).map_err(|_| bad("Cannot encode DAT info."))?
            );
        }
        BloomCmd::Inspect {
            file,
            max_memory_mib,
        } => {
            if !(1..=4096).contains(&max_memory_mib) {
                return Err(bad("Memory limit must be 1–4096 MiB."));
            }
            let filter = format::Bloom::read(&file, max_memory_mib * 1024 * 1024)?;
            println!(
                "{}",
                serde_json::to_string_pretty(filter.info())
                    .map_err(|_| bad("Cannot encode DAT info."))?
            );
        }
        BloomCmd::Configure {
            url,
            token_url,
            reset_token_url,
            client_id,
            scope,
            oauth,
            allow_http_loopback,
        } => {
            if url.is_none()
                && token_url.is_none()
                && reset_token_url.is_none()
                && client_id.is_none()
                && scope.is_none()
                && oauth.is_none()
                && allow_http_loopback.is_none()
            {
                return Err(bad("Specify --token-url or --dat-url."));
            }
            let mut cfg = service.settings()?;
            if let Some(url) = url {
                cfg.url = url;
            }
            if let Some(url) = token_url {
                cfg.token_url = url;
            }
            if let Some(url) = reset_token_url {
                if cfg.token_url.is_empty()
                    || cfg.api_url(&url, "Token")? != cfg.api_url(&cfg.token_url, "Token")?
                {
                    return Err(bad(
                        "Refresh uses --token-url. A separate refresh URL is not supported.",
                    ));
                }
            }
            cfg.reset_token_url.clear();
            if let Some(value) = client_id {
                cfg.client_id = value;
            }
            if let Some(value) = scope {
                cfg.scope = value;
            }
            if let Some(value) = oauth {
                cfg.oauth = value;
            }
            if let Some(value) = allow_http_loopback {
                cfg.allow_http_loopback = value;
            }
            service.save_settings(&cfg)?;
            println!("API URLs saved. Use bloom token if DAT requires a bearer credential.");
        }
        BloomCmd::Download => {
            prompt_dat_key(service)?;
            let control = RunControl::default();
            let snapshot = service.download(control.stop.clone()).await?;
            println!("DAT ready: {}", snapshot.bloom.info().sha256);
        }
        BloomCmd::Token => {
            if !std::io::stdin().is_terminal() {
                return Err(Error::NotATty);
            }
            let secret = Secret::new(crate::cli::prompt_hidden("DAT bearer token: ")?)?;
            service.save_token(&secret)?;
            println!("DAT token stored in Keychain.");
        }
    }
    Ok(())
}
async fn job(service: &Service, action: JobCmd) -> Result<(), Error> {
    match action {
        JobCmd::Create {
            input,
            rule,
            threads,
        } => {
            let job = prepare(service, input, rule.apply(service.settings()?.rule)?, None).await?;
            if let Some(threads) = threads {
                service.set_job_threads(&job.id, threads)?;
            }
            println!("{}", job.id);
        }
        JobCmd::List => {
            for job in service.jobs()? {
                println!(
                    "{} {} {}/{} matched={} not_matched={}",
                    job.id, job.state, job.processed, job.total, job.positives, job.negatives
                );
            }
        }
        JobCmd::Status { id } => println!(
            "{}",
            serde_json::to_string_pretty(&service.job(&id)?)
                .map_err(|_| bad("Cannot encode job status."))?
        ),
        JobCmd::Run { id, threads } | JobCmd::Resume { id, threads } => {
            run(service, &id, threads).await?;
        }
        JobCmd::Threads { id, threads } => {
            service.set_job_threads(&id, threads)?;
            println!("Matching threads: {threads}");
        }
        JobCmd::Delete { id, yes } => {
            if !yes {
                return Err(bad("Deletion is irreversible. Pass --yes to delete this job's stored input and results."));
            }
            service.delete_job(&id)?;
            println!("Job deleted.");
        }
        JobCmd::Checksum { id } => {
            if !service.verify(&id)? {
                return Err(bad("Job checksum mismatch."));
            }
            println!("SHA-256 MATCH");
        }
        JobCmd::Export { id, output } => {
            if service.job(&id)?.state != "COMPLETED" || !service.verify(&id)? {
                return Err(bad("Only complete, verified results can be exported."));
            }
            let target = export_target(&id, Some(&output))?;
            copy_result(&service.result_path(&id)?, &target)?;
            println!("Output: {}", target.display());
        }
    }
    Ok(())
}
async fn prepare(
    service: &Service,
    input: PathBuf,
    rule: Rule,
    output: Option<PathBuf>,
) -> Result<super::Job, Error> {
    let owned = service.clone();
    let requested = output
        .as_deref()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let output = std::path::absolute(requested)?;
    let (stop, rx) = tokio::sync::watch::channel(false);
    let mut task = tokio::task::spawn_blocking(move || {
        owned.create_job_cancellable_to(&input, rule, &rx, &output)
    });
    tokio::select! {result=&mut task=>result.map_err(|_|bad("Import worker interrupted."))?,
    _=tokio::signal::ctrl_c()=>{let _=stop.send(true);task.await.map_err(|_|bad("Import worker interrupted."))?}}
}
async fn run(service: &Service, id: &str, threads: Option<u16>) -> Result<super::Job, Error> {
    let job = service.job(id)?;
    if job.state != "COMPLETED" && !service.result_path(id)?.exists() {
        prompt_dat_key(service)?;
    }
    let (tx, rx) = tokio::sync::watch::channel(false);
    let mut control = RunControl::with_stop(rx);
    control.threads = threads;
    let work = service.run_job(id, None, control);
    tokio::pin!(work);
    let job = tokio::select! {result=&mut work=>result?,_=tokio::signal::ctrl_c()=> {let _=tx.send(true);work.await?}};
    println!(
        "{} {} {}/{} matched={} not_matched={}",
        job.id, job.state, job.processed, job.total, job.positives, job.negatives
    );
    if job.state == "COMPLETED" {
        println!("Output: {}", service.result_path(&job.id)?.display());
    }
    Ok(job)
}
fn prompt_dat_key(service: &Service) -> Result<(), Error> {
    if service.dat_key()?.is_some() {
        return Ok(());
    }
    service.settings()?.source_url()?;
    if !std::io::stdin().is_terminal() {
        return Err(bad("DAT decryption needs a hidden AES-256 key prompt. Use interactive match/job run or Home sign-in; key arguments/environment are not supported."));
    }
    let encoded = zeroize::Zeroizing::new(crate::cli::prompt_hidden(
        "DAT AES-256 key (Base64, hidden): ",
    )?);
    service.set_dat_key(super::crypto::DatKey::from_base64(&encoded)?)
}
pub(crate) fn export_target(
    id: &str,
    requested: Option<&std::path::Path>,
) -> Result<PathBuf, Error> {
    super::output::target(id, requested)
}
pub fn copy_result(source: &std::path::Path, target: &std::path::Path) -> Result<(), Error> {
    super::output::copy(source, target)
}
