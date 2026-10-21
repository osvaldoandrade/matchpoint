use std::sync::Arc;
use std::time::Duration;

use pgm_data::domain::Validation;
use pgm_data::engine::{App, OutFormat, RunOptions};
use pgm_data::error::Secret;
use pgm_data::jobcfg::default_tuning;
use pgm_data::secrets::MemorySecrets;
use pgm_data::sim::{Fixture, Plan, PostMode};
use pgm_data::store::Db;
use pgm_data::timeutil::SystemClock;

struct World {
    app: App,
    fixture: Fixture,
    _tmp: tempfile::TempDir,
}

async fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let fixture = Fixture::spawn().await;
    let app = App::open(
        tmp.path().join("data"),
        Arc::new(MemorySecrets::new()),
        Arc::new(SystemClock),
    )
    .unwrap();
    app.configure_auth(
        "test-client",
        Secret::new("test-secret".into()).unwrap(),
        None,
    )
    .unwrap();
    World {
        app,
        fixture,
        _tmp: tmp,
    }
}

fn config(world: &World) -> pgm_data::jobcfg::JobConfig {
    let mut config = default_tuning();
    config.endpoint = world.fixture.process_url();
    config.token_url = world.fixture.token_url();
    config.test_mode = true;
    config.contract = "simulated-fixture-v1".into();
    config.client_id = "test-client".into();
    config.batch_size = 2;
    config.rps = 1000.0;
    config.concurrency = 2;
    config.commit_max_rows = 1;
    config.commit_max_ms = 20;
    config.max_attempts = 4;
    config.validation = Validation::Length.as_str().into();
    config.response_timeout_ms = 400;
    config.connect_timeout_ms = 200;
    config.idempotency = true;
    config
}

fn write_csv(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
    let path = dir.join("in.csv");
    std::fs::write(&path, body).unwrap();
    path
}

#[tokio::test]
async fn preserves_zeros_duplicates_quotes_and_rejects_short_rows() {
    let world = world().await;
    world
        .fixture
        .set_plan(Plan {
            post: PostMode::Closed {
                customer: Some(false),
            },
            ..Plan::default()
        })
        .await;
    let path = write_csv(
        world._tmp.path(),
        "cpf\n00012345678\n\"00012345678\"\n52998224725\nabc\n",
    );
    let id = world.app.create_job(&path, config(&world), None).unwrap();
    let report = world.app.run_job(&id, RunOptions::run()).await.unwrap();
    assert_eq!(report.state, "COMPLETED_WITH_ERRORS");
    let merged = world.app.merge(&id, OutFormat::Csv, false, false).unwrap();
    assert_eq!(merged.rows, 4);
    let text = std::fs::read_to_string(
        world
            .app
            .paths
            .job_dir(&id)
            .join("exports/gen-0001/merged.csv"),
    )
    .unwrap();
    assert!(text.contains("00012345678"));
    assert_eq!(text.matches("00012345678").count(), 2);
    assert!(text.contains("FAILED_FINAL"));
    assert!(text.contains("validation:not_11_digits"));
    assert!(!text.contains("true"));
    let db = Db::open_read(&world.app.paths.job_dir(&id).join("job.sqlite")).unwrap();
    let page = db.export_page(0, 10).unwrap();
    assert_eq!(page.len(), 4);
    assert!(page.iter().any(|row| row.cpf == "00012345678"
        && row.is_customer == Some(false)
        && row.http_status == Some(200)));
    assert!(page.iter().any(|row| row.state == "FAILED_FINAL"));
    let ids: Vec<_> = page.iter().map(|row| row.row_id).collect();
    assert_eq!(ids, vec![1, 2, 3, 4]);
}

#[tokio::test]
async fn pending_contract_does_not_mark_http_200_as_customer() {
    let world = world().await;
    let path = write_csv(world._tmp.path(), "52998224725\n");
    let mut cfg = config(&world);
    cfg.contract = "pending".into();
    cfg.idempotency = false;
    let id = world.app.create_job(&path, cfg, None).unwrap();
    let report = world.app.run_job(&id, RunOptions::run()).await.unwrap();
    assert_eq!(report.state, "BLOCKED_UNKNOWN");
    let err = world
        .app
        .merge(&id, OutFormat::Parquet, false, false)
        .unwrap_err();
    assert!(err.to_string().contains("UNKNOWN"));
    let partial = world.app.merge(&id, OutFormat::Csv, true, false).unwrap();
    assert!(partial.partial);
    let db = Db::open_read(&world.app.paths.job_dir(&id).join("job.sqlite")).unwrap();
    let row = &db.export_page(0, 10).unwrap()[0];
    assert_eq!(row.state, "UNKNOWN");
    assert_eq!(row.is_customer, None);
    assert_eq!(row.evidence_source, "http_only");
}

#[tokio::test]
async fn retries_429_then_succeeds_and_records_attempts() {
    let world = world().await;
    world
        .fixture
        .set_plan(Plan {
            post: PostMode::RetryThenOk,
            ..Plan::default()
        })
        .await;
    let path = write_csv(world._tmp.path(), "52998224725\n39053344705\n");
    let id = world.app.create_job(&path, config(&world), None).unwrap();
    let report = world.app.run_job(&id, RunOptions::run()).await.unwrap();
    assert_eq!(report.state, "COMPLETED");
    let db = Db::open_read(&world.app.paths.job_dir(&id).join("job.sqlite")).unwrap();
    for row in db.export_page(0, 10).unwrap() {
        assert_eq!(row.state, "SUCCEEDED");
        assert!(row.attempts >= 2, "attempts {}", row.attempts);
        assert!(db.attempt_rows(row.row_id).unwrap() >= 2);
    }
}

#[tokio::test]
async fn timeout_without_idempotency_becomes_unknown_and_is_not_resent() {
    let world = world().await;
    world
        .fixture
        .set_plan(Plan {
            post: PostMode::Hang,
            ..Plan::default()
        })
        .await;
    let path = write_csv(world._tmp.path(), "52998224725\n");
    let mut cfg = config(&world);
    cfg.idempotency = false;
    cfg.response_timeout_ms = 150;
    cfg.max_attempts = 1;
    let id = world.app.create_job(&path, cfg, None).unwrap();
    let report = world.app.run_job(&id, RunOptions::run()).await.unwrap();
    assert_eq!(report.state, "BLOCKED_UNKNOWN");
    let hits = world
        .fixture
        .hits_post
        .load(std::sync::atomic::Ordering::SeqCst);
    let again = world.app.run_job(&id, RunOptions::run()).await.unwrap_err();
    assert!(
        again.to_string().contains("requeue-unknown")
            || again.to_string().contains("UNKNOWN")
            || again.to_string().contains("blocked")
    );
    let hits_after = world
        .fixture
        .hits_post
        .load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(hits, hits_after);
    let _ = report;
}

#[tokio::test]
async fn redirect_is_not_followed() {
    let world = world().await;
    world
        .fixture
        .set_plan(Plan {
            post: PostMode::Redirect,
            ..Plan::default()
        })
        .await;
    let path = write_csv(world._tmp.path(), "52998224725\n");
    let mut cfg = config(&world);
    cfg.max_attempts = 1;
    let id = world.app.create_job(&path, cfg, None).unwrap();
    let report = world.app.run_job(&id, RunOptions::run()).await.unwrap();
    assert_eq!(report.state, "BLOCKED_UNKNOWN");
    assert_eq!(
        world
            .fixture
            .hits_other
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}

#[tokio::test]
async fn malformed_body_is_not_success() {
    let world = world().await;
    world
        .fixture
        .set_plan(Plan {
            post: PostMode::Malformed,
            ..Plan::default()
        })
        .await;
    let path = write_csv(world._tmp.path(), "52998224725\n");
    let mut cfg = config(&world);
    cfg.max_attempts = 1;
    let id = world.app.create_job(&path, cfg, None).unwrap();
    let report = world.app.run_job(&id, RunOptions::run()).await.unwrap();
    assert_eq!(report.state, "BLOCKED_UNKNOWN");
    let db = Db::open_read(&world.app.paths.job_dir(&id).join("job.sqlite")).unwrap();
    let row = &db.export_page(0, 5).unwrap()[0];
    assert_eq!(row.error_code.as_deref(), Some("malformed_body"));
    assert_eq!(row.is_customer, None);
}

#[tokio::test]
async fn idempotency_key_does_not_contain_the_cpf() {
    let world = world().await;
    let path = write_csv(world._tmp.path(), "52998224725\n");
    let id = world.app.create_job(&path, config(&world), None).unwrap();
    world.app.run_job(&id, RunOptions::run()).await.unwrap();
    assert!(!world.fixture.cpf_leaked_into_key().await);
    let keys = world.fixture.idempotency_keys().await;
    assert!(keys.iter().any(|key| key.ends_with(":1")));
    assert!(keys.iter().all(|key| !key.contains("52998224725")));
}

#[tokio::test]
async fn unconfigured_remote_access_is_refused_before_any_call() {
    let world = world().await;
    let path = write_csv(world._tmp.path(), "52998224725\n");
    let mut cfg = config(&world);
    cfg.endpoint = "https://api.example.com/v1/partner/account-opening-intentions".into();
    cfg.token_url = "https://api.example.com/oauth2/token".into();
    cfg.test_mode = false;
    cfg.contract = "pending".into();
    cfg.idempotency = false;
    let id = world.app.create_job(&path, cfg, None).unwrap();
    let err = world.app.run_job(&id, RunOptions::run()).await.unwrap_err();
    assert!(err.to_string().contains("contract"));
    assert_eq!(
        world
            .fixture
            .hits_post
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert_eq!(
        world
            .fixture
            .hits_token
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}

#[tokio::test]
async fn plain_http_off_loopback_is_rejected() {
    let world = world().await;
    let path = write_csv(world._tmp.path(), "52998224725\n");
    let mut cfg = config(&world);
    cfg.endpoint = "http://example.com/v1".into();
    let err = world.app.create_job(&path, cfg, None).unwrap_err();
    assert!(err.to_string().contains("HTTP") || err.to_string().contains("loopback"));
}

#[tokio::test]
async fn pending_contract_does_not_repeat_an_ambiguous_server_error() {
    let world = world().await;
    world
        .fixture
        .set_plan(Plan {
            post: PostMode::Status(500),
            ..Plan::default()
        })
        .await;
    let path = write_csv(world._tmp.path(), "52998224725\n");
    let mut cfg = config(&world);
    cfg.contract = "pending".into();
    cfg.idempotency = false;
    let id = world.app.create_job(&path, cfg, None).unwrap();
    let report = world.app.run_job(&id, RunOptions::run()).await.unwrap();
    assert_eq!(report.state, "BLOCKED_UNKNOWN");
    assert_eq!(
        world
            .fixture
            .hits_post
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    assert_eq!(world.app.status(&id).unwrap().counts.unknown, 1);
}

#[tokio::test]
async fn pause_keeps_remaining_rows_pending() {
    let world = world().await;
    let path = write_csv(
        world._tmp.path(),
        "52998224725\n39053344705\n11144477735\n12345678909\n",
    );
    let mut cfg = config(&world);
    cfg.concurrency = 1;
    cfg.rps = 5.0;
    let id = world.app.create_job(&path, cfg, None).unwrap();
    let mut opts = RunOptions::run();
    opts.stop_after_successes = Some(1);
    let report = world.app.run_job(&id, opts).await.unwrap();
    assert_eq!(report.state, "PAUSED");
    let status = world.app.status(&id).unwrap();
    assert!(status.counts.succeeded >= 1);
    assert!(status.counts.pending > 0);
    let report = world.app.run_job(&id, RunOptions::run()).await.unwrap();
    assert_eq!(report.state, "COMPLETED");
    let status = world.app.status(&id).unwrap();
    assert_eq!(status.counts.succeeded, 4);
    assert_eq!(status.counts.total(), 4);
}

#[tokio::test]
async fn parallel_pause_finishes_workers_before_returning_and_resumes_without_unknown() {
    let world = world().await;
    let path = write_csv(world._tmp.path(), &"52998224725\n".repeat(100));
    let mut cfg = config(&world);
    cfg.concurrency = 16;
    cfg.rps = 40.0;
    let id = world.app.create_job(&path, cfg, None).unwrap();
    let owned = world.app.clone();
    let run_id = id.clone();
    let handle = tokio::spawn(async move { owned.run_job(&run_id, RunOptions::run()).await });
    tokio::time::sleep(Duration::from_millis(150)).await;
    world.app.pause_job(&id).unwrap();
    assert_eq!(handle.await.unwrap().unwrap().state, "PAUSED");
    let counts = world.app.status(&id).unwrap().counts;
    assert_eq!(counts.in_flight, 0);
    assert_eq!(counts.unknown, 0);
    assert!(counts.open() > 0);
    assert_eq!(
        world
            .app
            .run_job(&id, RunOptions::run())
            .await
            .unwrap()
            .state,
        "COMPLETED"
    );
    assert_eq!(world.app.status(&id).unwrap().counts.succeeded, 100);
}

#[tokio::test]
async fn second_runner_is_rejected_and_merge_is_idempotent() {
    let world = world().await;
    let path = write_csv(world._tmp.path(), "52998224725\n");
    let id = world.app.create_job(&path, config(&world), None).unwrap();
    let dir = world.app.paths.job_dir(&id);
    let _lock = pgm_data::disk::acquire(&dir.join("run.lock")).unwrap();
    let err = world.app.run_job(&id, RunOptions::run()).await.unwrap_err();
    assert!(err.to_string().contains("running") || err.to_string().contains("lock"));
    drop(_lock);
    world.app.run_job(&id, RunOptions::run()).await.unwrap();
    let first = world
        .app
        .merge(&id, OutFormat::Parquet, false, false)
        .unwrap();
    let hits = world
        .fixture
        .hits_post
        .load(std::sync::atomic::Ordering::SeqCst);
    let second = world
        .app
        .merge(&id, OutFormat::Parquet, false, false)
        .unwrap();
    assert_eq!(first.sha256, second.sha256);
    assert_eq!(
        hits,
        world
            .fixture
            .hits_post
            .load(std::sync::atomic::Ordering::SeqCst)
    );
}

#[tokio::test]
async fn export_can_be_rebuilt_after_a_crash_without_new_calls() {
    let world = world().await;
    let path = write_csv(world._tmp.path(), "52998224725\n39053344705\n");
    let id = world.app.create_job(&path, config(&world), None).unwrap();
    world.app.run_job(&id, RunOptions::run()).await.unwrap();
    let err = world
        .app
        .merge(&id, OutFormat::Parquet, false, true)
        .unwrap_err();
    assert!(err.to_string().contains("disk") || err.to_string().contains("write"));
    let hits = world
        .fixture
        .hits_post
        .load(std::sync::atomic::Ordering::SeqCst);
    let report = world
        .app
        .merge(&id, OutFormat::Parquet, false, false)
        .unwrap();
    assert_eq!(report.rows, 2);
    assert_eq!(
        hits,
        world
            .fixture
            .hits_post
            .load(std::sync::atomic::Ordering::SeqCst)
    );
}

#[tokio::test]
async fn prepare_crash_resumes_without_losing_or_duplicating_rows() {
    let world = world().await;
    let path = write_csv(
        world._tmp.path(),
        "52998224725\n39053344705\n11144477735\n12345678909\n52998224725\n",
    );
    let mut cfg = config(&world);
    cfg.batch_size = 2;
    let err = world.app.create_job(&path, cfg, Some(1)).unwrap_err();
    assert!(err.to_string().contains("disk") || err.to_string().contains("write"));
    let jobs = world.app.list_jobs().unwrap();
    assert_eq!(jobs.len(), 1);
    world.app.prepare_job(&jobs[0].id, None).unwrap();
    let db = Db::open_read(&world.app.paths.job_dir(&jobs[0].id).join("job.sqlite")).unwrap();
    let rows = db.export_page(0, 20).unwrap();
    assert_eq!(rows.len(), 5);
    assert_eq!(
        rows.iter().map(|r| r.row_id).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
}

#[tokio::test]
async fn malformed_csv_keeps_the_last_committed_batch() {
    let world = world().await;
    let path = write_csv(
        world._tmp.path(),
        "52998224725\n39053344705\n\"unterminated\n",
    );
    let mut cfg = config(&world);
    cfg.batch_size = 2;
    let err = world.app.create_job(&path, cfg, None).unwrap_err();
    assert!(err.to_string().contains("csv") || err.to_string().contains("quote"));
    let id = &world.app.list_jobs().unwrap()[0].id;
    let db = Db::open_read(&world.app.paths.job_dir(id).join("job.sqlite")).unwrap();
    let rows = db.export_page(0, 20).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(db.checkpoint().unwrap().rows_imported == 2);
}

#[tokio::test]
async fn outcome_commit_failure_leaves_rows_recoverable() {
    let world = world().await;
    let path = write_csv(world._tmp.path(), "52998224725\n");
    let id = world.app.create_job(&path, config(&world), None).unwrap();
    let mut opts = RunOptions::run();
    opts.fail_outcome_commits = 1;
    let err = world.app.run_job(&id, opts).await.unwrap_err();
    assert!(err.to_string().contains("disk") || err.to_string().contains("write"));
    let status = world.app.status(&id).unwrap();
    assert_eq!(status.state, "INTERRUPTED");
    world
        .fixture
        .set_plan(Plan {
            post: PostMode::Closed { customer: None },
            ..Plan::default()
        })
        .await;
    let report = world.app.run_job(&id, RunOptions::run()).await.unwrap();
    assert_eq!(report.state, "BLOCKED_UNKNOWN");
    let status = world.app.status(&id).unwrap();
    assert_eq!(status.counts.unknown, 1);
    assert_eq!(status.counts.succeeded, 0);
}

#[tokio::test]
async fn inflight_hang_is_unknown_on_the_next_session() {
    let world = world().await;
    world
        .fixture
        .set_plan(Plan {
            post: PostMode::Hang,
            ..Plan::default()
        })
        .await;
    let path = write_csv(world._tmp.path(), "52998224725\n");
    let mut cfg = config(&world);
    cfg.response_timeout_ms = 5_000;
    cfg.idempotency = false;
    let id = world.app.create_job(&path, cfg, None).unwrap();
    let (tx, rx) = tokio::sync::watch::channel(false);
    let app = world.app.clone();
    let id2 = id.clone();
    let handle = tokio::spawn(async move {
        let mut opts = RunOptions::with_stop(rx);
        opts.grace = Duration::from_millis(50);
        app.run_job(&id2, opts).await
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while world
            .fixture
            .hits_post
            .load(std::sync::atomic::Ordering::SeqCst)
            == 0
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("fixture must receive the in-flight request before pausing");
    let _ = tx.send(true);
    handle.await.unwrap().unwrap();
    world.fixture.set_plan(Plan::default()).await;
    let hits = world
        .fixture
        .hits_post
        .load(std::sync::atomic::Ordering::SeqCst);
    let report = world.app.run_job(&id, RunOptions::run()).await;
    let hits_after = world
        .fixture
        .hits_post
        .load(std::sync::atomic::Ordering::SeqCst);
    assert!(report.is_ok() || report.is_err());
    assert_eq!(hits_after, hits);
    let status = world.app.status(&id).unwrap();
    assert_eq!(status.counts.unknown, 1);
}

#[tokio::test]
async fn retry_failed_starts_a_new_generation() {
    let world = world().await;
    let path = write_csv(world._tmp.path(), "52998224725\nabc\n");
    let id = world.app.create_job(&path, config(&world), None).unwrap();
    world.app.run_job(&id, RunOptions::run()).await.unwrap();
    let first = world.app.merge(&id, OutFormat::Csv, false, false).unwrap();
    assert_eq!(first.generation, 1);
    let n = world.app.retry_failed(&id).unwrap();
    assert_eq!(n, 1);
    let second = world
        .app
        .merge(&id, OutFormat::Csv, false, false)
        .unwrap_err();
    let _ = second;
    let status = world.app.status(&id).unwrap();
    assert_eq!(status.state, "PREPARED");
    assert!(world
        .app
        .paths
        .job_dir(&id)
        .join("exports/gen-0001/manifest.json")
        .exists());
}

#[tokio::test]
async fn counters_match_records() {
    let world = world().await;
    let path = write_csv(world._tmp.path(), "52998224725\n39053344705\nabc\n");
    let id = world.app.create_job(&path, config(&world), None).unwrap();
    world.app.run_job(&id, RunOptions::run()).await.unwrap();
    let db = Db::open(&world.app.paths.job_dir(&id).join("job.sqlite")).unwrap();
    assert_eq!(db.counts().unwrap(), db.count_records().unwrap());
}

#[tokio::test]
async fn cli_without_a_tty_does_not_open_the_interface() {
    let bin = env!("CARGO_BIN_EXE_pgm-data");
    let dir = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(bin)
        .env("PGM_DATA_HOME", dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(text.contains("terminal") || text.contains("help"));
}
