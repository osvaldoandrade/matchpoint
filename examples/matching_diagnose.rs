//! Authorized, interactive provider check. No credentials or DAT files are saved.
use pgm_data::{
    cli::prompt_hidden,
    error::Secret,
    matching::{crypto::DatKey, DownloadMode, Rule, Service, Settings, Transform},
    secrets::MemorySecrets,
};
use std::{path::PathBuf, sync::Arc, time::Instant};
use zeroize::Zeroizing;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("Usage: matching_diagnose SETTINGS_JSON [single|parallel|multipart]")?;
    let mut cfg: Settings = serde_json::from_slice(&std::fs::read(path)?)?;
    if let Some(mode) = arguments.next() {
        cfg.download_mode = match mode.to_str() {
            Some("single") => DownloadMode::Single,
            Some("parallel") => DownloadMode::Parallel,
            Some("multipart") => DownloadMode::Multipart,
            _ => return Err("Unknown download mode".into()),
        };
    }
    tracing_subscriber::fmt()
        .with_env_filter("pgm_data::matching=info")
        .with_ansi(false)
        .init();
    cfg.validate()?;
    let temporary = tempfile::tempdir()?;
    let service = Service::open(temporary.path().to_owned(), Arc::new(MemorySecrets::new()))?;
    service.save_settings(&cfg)?;
    let secret = Secret::new(prompt_hidden("Client secret (hidden): ")?)?;
    let encoded = Zeroizing::new(prompt_hidden("AES key, Base64 (hidden): ")?);
    service.set_dat_key(DatKey::from_base64(&encoded)?)?;
    let started = Instant::now();
    service.sign_in(secret).await?;
    println!("Token issued in {:.3}s", started.elapsed().as_secs_f64());
    let (_stop, receiver) = tokio::sync::watch::channel(false);
    let started = Instant::now();
    println!("Downloading, authenticating and decoding DAT in locked RAM…");
    match service.download(receiver).await {
        Ok(snapshot) => {
            println!(
                "Verified in {:.3}s: {}",
                started.elapsed().as_secs_f64(),
                serde_json::to_string(snapshot.bloom.info())?
            );
            let probes = Zeroizing::new(prompt_hidden(
                "Raw probe keys, space-separated (hidden; Enter skips): ",
            )?);
            for (index, key) in probes.split_whitespace().enumerate() {
                let rule = if snapshot.bloom.info().wire_format == "pgmbloom-provider-v1" {
                    Rule {
                        transform: Transform::Sha256,
                        ..cfg.rule.clone()
                    }
                } else {
                    cfg.rule.clone()
                };
                println!(
                    "Probe {}: {}",
                    index + 1,
                    snapshot.bloom.contains_with_rule(key, &rule)?
                );
            }
            drop(snapshot);
        }
        Err(error) => {
            println!(
                "Failed after {:.3}s: {error}",
                started.elapsed().as_secs_f64()
            );
            service.clear_filters()?;
            service.forget_access()?;
            return Err("Provider check failed; no DAT saved".into());
        }
    }
    service.clear_filters()?;
    service.forget_access()?;
    println!("Session and DAT released. Temporary test directory removed on exit.");
    Ok(())
}
