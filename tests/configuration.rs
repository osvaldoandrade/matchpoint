use std::sync::Arc;

use pgm_data::cli::{execute_collect, parse_line, AuthCmd, Cli, Cmd};
use pgm_data::engine::{App, AppConfig, RunOptions};
use pgm_data::error::{Error, Secret};
use pgm_data::jobcfg::{default_tuning, DEFAULT_BATCH_SIZE};
use pgm_data::secrets::{MemorySecrets, SecretStore, ACCOUNT_REFRESH, ACCOUNT_SECRET};
use pgm_data::timeutil::SystemClock;
use tempfile::TempDir;

fn app() -> (TempDir, App, Arc<MemorySecrets>) {
    let dir = tempfile::tempdir().unwrap();
    let secrets = Arc::new(MemorySecrets::new());
    let app = App::open(
        dir.path().join("data"),
        secrets.clone(),
        Arc::new(SystemClock),
    )
    .unwrap();
    (dir, app, secrets)
}

fn profile() -> AppConfig {
    AppConfig {
        client_id: "test-client".into(),
        token_url: "https://example.test/token".into(),
        process_url: "https://example.test/data".into(),
        remote_enabled: true,
        ..AppConfig::default()
    }
}

fn secret() -> Secret {
    Secret::new("test-only-secret".into()).unwrap()
}

#[tokio::test]
async fn empty_url_defaults_allow_local_preparation_and_refuse_remote_execution() {
    let (dir, app, _) = app();
    app.configure_auth("fixture-client", secret(), None)
        .unwrap();
    let mut profile = app.api_config().unwrap();
    assert!(profile.token_url.is_empty() && profile.process_url.is_empty());
    profile.remote_enabled = true;
    assert!(profile.validate().is_err());
    let input = dir.path().join("input.csv");
    std::fs::write(&input, "52998224725\n").unwrap();
    let id = app.create_job(&input, default_tuning(), None).unwrap();
    assert!(app.run_job(&id, RunOptions::run()).await.is_err());
    assert_eq!(app.status(&id).unwrap().state, "PREPARED");
}

#[test]
fn saves_only_metadata_and_preserves_urls_when_credentials_change() {
    let (_dir, app, secrets) = app();
    let mut config = profile();
    config.process_url = "https://api.example.com/custom/intent".into();
    config.token_url = "https://api.example.com/custom/token".into();
    app.configure_api(
        config.clone(),
        secret(),
        Some(Secret::new("test-only-refresh".into()).unwrap()),
    )
    .unwrap();
    let json = std::fs::read_to_string(app.paths.config_file()).unwrap();
    assert!(!json.contains("test-only-secret"));
    assert!(!json.contains("test-only-refresh"));
    assert_eq!(
        secrets.get(ACCOUNT_SECRET).unwrap().unwrap().expose(),
        "test-only-secret"
    );
    let reopened = App::open(
        app.paths.root.clone(),
        secrets.clone(),
        Arc::new(SystemClock),
    )
    .unwrap();
    let saved = reopened.api_config().unwrap();
    assert!(saved.remote_enabled);
    assert_eq!(saved.process_url, config.process_url);
    app.configure_auth("new-test-client", secret(), None)
        .unwrap();
    let saved = app.api_config().unwrap();
    assert_eq!(saved.process_url, config.process_url);
    assert_eq!(saved.token_url, config.token_url);
    assert_eq!(saved.client_id, "new-test-client");
    assert!(secrets.get(ACCOUNT_REFRESH).unwrap().is_none());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(app.paths.config_file())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[tokio::test]
async fn old_profiles_stay_disabled_and_disable_does_not_replace_secrets() {
    let (_dir, app, secrets) = app();
    app.configure_api(profile(), secret(), None).unwrap();
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(app.paths.config_file()).unwrap()).unwrap();
    json.as_object_mut().unwrap().remove("remote_enabled");
    std::fs::write(app.paths.config_file(), serde_json::to_vec(&json).unwrap()).unwrap();
    assert!(!app.api_config().unwrap().remote_enabled);
    app.configure_api(profile(), secret(), None).unwrap();
    let command = parse_line("auth configure --disable-remote")
        .unwrap()
        .command
        .unwrap();
    assert_eq!(execute_collect(&app, command).await.unwrap().0, 0);
    assert!(!app.api_config().unwrap().remote_enabled);
    assert_eq!(
        secrets.get(ACCOUNT_SECRET).unwrap().unwrap().expose(),
        "test-only-secret"
    );
}

#[test]
fn invalid_configuration_does_not_overwrite_saved_credentials() {
    let (_dir, app, secrets) = app();
    app.configure_api(profile(), secret(), None).unwrap();
    let before = std::fs::read(app.paths.config_file()).unwrap();
    for id in ["", " ", "client\n", " client", &"x".repeat(257)] {
        let mut config = profile();
        config.client_id = id.into();
        assert!(
            app.configure_api(
                config,
                Secret::new("test-only-replacement".into()).unwrap(),
                None
            )
            .is_err(),
            "id {id:?}"
        );
    }
    for url in [
        "http://api.example.com/path",
        "https://user:password@api.example.com/path",
        "https://api.example.com/path?x=abc",
        "https://api.example.com/path#token",
        "https://api.example.com/\npath",
        "file:///tmp/config",
    ] {
        for field in ["process", "token"] {
            let mut config = profile();
            if field == "process" {
                config.process_url = url.into();
            } else {
                config.token_url = url.into();
            }
            assert!(
                app.configure_api(config, secret(), None).is_err(),
                "{field}: {url}"
            );
        }
    }
    assert_eq!(before, std::fs::read(app.paths.config_file()).unwrap());
    assert_eq!(
        secrets.get(ACCOUNT_SECRET).unwrap().unwrap().expose(),
        "test-only-secret"
    );
}

#[test]
fn failed_atomic_save_restores_both_credentials() {
    let (_dir, app, secrets) = app();
    app.configure_api(
        profile(),
        secret(),
        Some(Secret::new("test-only-refresh".into()).unwrap()),
    )
    .unwrap();
    std::fs::remove_file(app.paths.config_file()).unwrap();
    std::fs::create_dir(app.paths.config_file()).unwrap();
    assert!(app
        .configure_api(
            profile(),
            Secret::new("test-only-new-secret".into()).unwrap(),
            None
        )
        .is_err());
    assert_eq!(
        secrets.get(ACCOUNT_SECRET).unwrap().unwrap().expose(),
        "test-only-secret"
    );
    assert_eq!(
        secrets.get(ACCOUNT_REFRESH).unwrap().unwrap().expose(),
        "test-only-refresh"
    );
    assert!(!std::fs::read_dir(&app.paths.root).unwrap().any(|item| item
        .unwrap()
        .path()
        .extension()
        .is_some_and(|ext| ext == "tmp")));
}

#[test]
fn authorization_requires_enabled_profile_and_exact_destinations_and_identity() {
    let config = profile();
    let job = pgm_data::jobcfg::JobConfig {
        client_id: config.client_id.clone(),
        endpoint: config.process_url.clone(),
        token_url: config.token_url.clone(),
        ..default_tuning()
    };
    assert!(config.authorize_job(&job).is_ok());
    let mut disabled = config.clone();
    disabled.remote_enabled = false;
    assert!(matches!(
        disabled.authorize_job(&job),
        Err(Error::ContractPending)
    ));
    for field in [
        "process",
        "token",
        "identity",
        "test",
        "contract",
        "invalid-contract",
        "http",
    ] {
        let mut changed = job.clone();
        match field {
            "process" => changed.endpoint = "https://other.invalid/intent".into(),
            "token" => changed.token_url = "https://other.invalid/token".into(),
            "identity" => changed.client_id = "other-client".into(),
            "test" => changed.test_mode = true,
            "contract" => changed.contract = "simulated-fixture-v1".into(),
            "invalid-contract" => changed.contract = "other".into(),
            "http" => changed.endpoint = "http://api.example.com/intent".into(),
            _ => unreachable!(),
        }
        assert!(config.authorize_job(&changed).is_err(), "{field}");
    }
}

#[tokio::test]
async fn cli_jobs_inherit_profile_and_batch_boundaries_are_exact() {
    let (dir, app, _secrets) = app();
    let mut config = profile();
    config.process_url = "https://api.example.com/custom/intent".into();
    config.token_url = "https://api.example.com/custom/token".into();
    app.configure_api(config.clone(), secret(), None).unwrap();
    assert_eq!(DEFAULT_BATCH_SIZE, 1_000);
    for (rows, sizes) in [
        (999, vec![999]),
        (1000, vec![1000]),
        (1001, vec![1000, 1]),
        (2001, vec![1000, 1000, 1]),
    ] {
        let input = dir.path().join(format!("clients {rows}.csv"));
        std::fs::write(&input, "52998224725\n".repeat(rows)).unwrap();
        let command = parse_line(&format!("job create --input \"{}\"", input.display()))
            .unwrap()
            .command
            .unwrap();
        let (_, output) = execute_collect(&app, command).await.unwrap();
        let id = &output[0];
        let batches = app.list_batches(id, "all", 10, 0).unwrap();
        assert_eq!(
            batches
                .iter()
                .map(|batch| batch.row_to - batch.row_from + 1)
                .collect::<Vec<_>>(),
            sizes
        );
        assert_eq!(batches.first().unwrap().row_from, 1);
        assert_eq!(batches.last().unwrap().row_to, rows as i64);
        let db = pgm_data::store::Db::open_read(&app.paths.job_dir(id).join("job.sqlite")).unwrap();
        let saved: pgm_data::jobcfg::JobConfig =
            serde_json::from_str(&db.meta_get("config").unwrap().unwrap()).unwrap();
        assert_eq!(saved.endpoint, config.process_url);
        assert_eq!(saved.token_url, config.token_url);
        assert_eq!(saved.client_id, config.client_id);
    }
}

#[tokio::test]
async fn mismatched_destination_is_rejected_by_engine_before_network_io() {
    let (dir, app, _secrets) = app();
    app.configure_api(profile(), secret(), None).unwrap();
    let input = dir.path().join("input.csv");
    std::fs::write(&input, "52998224725\n").unwrap();
    let mut job = default_tuning();
    job.endpoint = "https://other.invalid/intent".into();
    job.token_url = profile().token_url;
    job.client_id = "test-client".into();
    let id = app.create_job(&input, job, None).unwrap();
    let error = app.run_job(&id, RunOptions::run()).await.unwrap_err();
    assert!(error.to_string().contains("must match"));
    assert_eq!(app.status(&id).unwrap().state, "PREPARED");
    assert_eq!(app.status(&id).unwrap().counts.pending, 1);
}

#[test]
fn clap_exposes_configuration_and_rejects_secret_arguments_without_echo() {
    use clap::{CommandFactory, Parser};
    Cli::command().debug_assert();
    let parsed = parse_line("auth configure --client-id test-client --allow-remote").unwrap();
    assert!(matches!(
        parsed.command,
        Some(Cmd::Auth {
            action: AuthCmd::Configure {
                allow_remote: true,
                ..
            }
        })
    ));
    assert!(Cli::try_parse_from([
        "pgm-data",
        "auth",
        "configure",
        "--allow-remote",
        "--disable-remote"
    ])
    .is_err());
    for line in [
        "auth configure --client-secret=test-only-secret",
        "auth configure client_secret=test-only-secret",
        "auth configure --refresh-token test-only-secret",
    ] {
        let error = parse_line(line).unwrap_err().to_string();
        assert!(!error.contains("test-only-secret"));
        assert!(error.contains("Settings"));
    }
}

#[test]
fn real_cli_rejects_secret_flags_without_printing_the_supplied_value() {
    let dir = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_pgm-data"))
        .env("PGM_DATA_HOME", dir.path())
        .args(["auth", "configure", "--client-secret=test-only-secret"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("test-only-secret"));
}

#[test]
fn malformed_or_secret_bearing_saved_settings_fail_without_echo() {
    let (_dir, app, _secrets) = app();
    for contents in [
        "not-json",
        r#"{"client_id":"test","process_url":"https://user:test-only-secret@api.example.com/intent","token_url":"https://api.example.com/oauth2/token"}"#,
    ] {
        std::fs::write(app.paths.config_file(), contents).unwrap();
        let error = app.api_config().unwrap_err().to_string();
        assert!(!error.contains("test-only-secret"));
    }
}
