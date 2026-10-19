use std::sync::Arc;

use pgm_data::disk::acquire;
use pgm_data::engine::{App, RunOptions};
use pgm_data::error::Secret;
use pgm_data::jobcfg::default_tuning;
use pgm_data::secrets::MemorySecrets;
use pgm_data::sim::Fixture;
use pgm_data::timeutil::SystemClock;

#[tokio::test]
async fn two_locks_on_one_job_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let fixture = Fixture::spawn().await;
    let app = App::open(
        tmp.path().join("data"),
        Arc::new(MemorySecrets::new()),
        Arc::new(SystemClock),
    )
    .unwrap();
    app.configure_auth("c", Secret::new("s".into()).unwrap(), None)
        .unwrap();
    let input = tmp.path().join("in.csv");
    std::fs::write(&input, "52998224725\n").unwrap();
    let mut config = default_tuning();
    config.endpoint = fixture.process_url();
    config.token_url = fixture.token_url();
    config.test_mode = true;
    config.contract = "simulated-fixture-v1".into();
    config.client_id = "c".into();
    config.idempotency = true;
    let id = app.create_job(&input, config, None).unwrap();
    let first = acquire(&app.paths.job_dir(&id).join("run.lock")).unwrap();
    let second = acquire(&app.paths.job_dir(&id).join("run.lock"));
    assert!(second.is_err());
    drop(first);
    assert!(acquire(&app.paths.job_dir(&id).join("run.lock")).is_ok());
    let _ = RunOptions::run();
}

#[test]
fn low_space_error_is_distinct_from_a_normal_result() {
    let err = pgm_data::error::Error::LowSpace {
        free_mib: 1,
        need_mib: 256,
    };
    assert!(err.to_string().contains("1"));
    let full = pgm_data::error::Error::DiskFull;
    assert!(full.to_string().contains("disk") || full.to_string().contains("write"));
}
