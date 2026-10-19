use std::io::{self, IsTerminal};

use pgm_data::engine::system_app;
use pgm_data::matching::cli::{execute, parse_args};
use pgm_data::secrets::KeychainSecrets;
use pgm_data::tui::launch;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        if io::stdout().is_terminal() {
            let _ = restore_terminal();
        }
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), pgm_data::error::Error> {
    let cli = parse_args()?;
    if let Some(pgm_data::matching::cli::Cmd::Manual { roff }) = &cli.command {
        use std::io::Write;
        match io::stdout()
            .lock()
            .write_all(pgm_data::matching::manual::text(*roff).as_bytes())
        {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
            Err(error) => return Err(error.into()),
        }
    }
    let secrets = std::sync::Arc::new(KeychainSecrets::new());
    let app = system_app(secrets)?;
    let service = app.matching()?;
    let path = service.root.join("events.log");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    pgm_data::error::restrict_file(&path)?;
    let _ = tracing_subscriber::fmt()
        .with_env_filter("warn,pgm_data=info")
        .with_ansi(false)
        .with_target(false)
        .with_writer(std::sync::Mutex::new(file))
        .try_init();
    match cli.command {
        None => {
            if !io::stdout().is_terminal() {
                eprintln!("pgm-data requires a terminal. Use `pgm-data --help`.");
                std::process::exit(2);
            }
            launch(&app).await
        }
        Some(pgm_data::matching::cli::Cmd::Tui) => launch(&app).await,
        Some(cmd) => {
            let code = execute(&app, cmd).await?;
            if code != 0 {
                std::process::exit(code);
            }
            Ok(())
        }
    }
}

fn restore_terminal() -> io::Result<()> {
    use crossterm::execute;
    use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
    let _ = disable_raw_mode();
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
