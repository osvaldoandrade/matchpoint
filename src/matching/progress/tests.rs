use super::*;
use crate::{
    matching::{format::Bloom, Rule, RunControl, Snapshot},
    secrets::MemorySecrets,
};
use std::{io::Write, sync::Arc};

fn observer() -> (Reporter, watch::Receiver<Progress>) {
    let (tx, rx) = watch::channel(Progress::new(Phase::Starting));
    (Reporter(tx), rx)
}

#[test]
fn quiet_header_wait_has_a_live_clock_and_preserves_last_http_failure() {
    let (reporter, rx) = observer();
    reporter.download_start(2);
    let p = rx.borrow().clone();
    assert!(p
        .headline(p.phase_started + Duration::from_secs(43), true)
        .contains("43s · 1/3"));
    assert_eq!(p.ratio(), 0.0);
    reporter.retry(1, Some(504), Duration::from_secs(2));
    let p = rx.borrow().clone();
    assert!(p.headline(p.updated, true).contains("HTTP 504 · retry 1/2"));
    assert!(p.detail(p.updated).contains("retry in"));
    reporter.waiting(false);
    let p = rx.borrow().clone();
    assert_eq!(p.phase, Phase::WaitingForDat);
    assert!(p
        .headline(p.phase_started + Duration::from_secs(61), true)
        .contains("61s · 2/3"));
    assert!(p
        .detail(p.started + Duration::from_secs(120))
        .contains("Last HTTP 504 · elapsed 120s"));
    assert!(p.wait_until.is_none());
    reporter.retry(2, None, Duration::ZERO);
    assert!(rx
        .borrow()
        .headline(Instant::now(), true)
        .contains("DAT interrupted"));
    reporter.retry(1, None, Duration::ZERO);
    assert_eq!(rx.borrow().retry, 2);
}

#[test]
fn logical_bytes_exclude_probe_and_discarded_prefix_with_parallel_updates() {
    let (reporter, rx) = observer();
    reporter.download_start(2);
    reporter.total(Some(100));
    let mut probe = ByteCounter::new(None);
    probe.add(1);
    assert_eq!(rx.borrow().completed, 0);
    let mut first = ByteCounter::new(Some(reporter.clone()));
    let mut second = ByteCounter::new(Some(reporter.clone()));
    first.add(30);
    second.add(20);
    reporter.waiting(true);
    assert_eq!(rx.borrow().phase, Phase::DownloadingDat);
    assert_eq!(rx.borrow().completed, 50);
    first.rewind();
    assert_eq!(rx.borrow().completed, 20);
    first.add(80);
    let p = rx.borrow().clone();
    assert_eq!(p.completed, 100);
    assert_eq!(p.ratio(), 1.0);
    assert!(p
        .detail(p.updated + Duration::from_secs(3))
        .contains("no data 3s"));
    drop((first, second));
    assert_eq!(rx.borrow().completed, 100);
}

#[test]
fn watch_retains_only_latest_value_and_separate_operations_are_isolated() {
    let root = tempfile::tempdir().unwrap();
    let original = Service::open(root.path().into(), Arc::new(MemorySecrets::new())).unwrap();
    let (one, mut rx) = original.observed();
    let (two, other) = original.observed();
    one.phase(Phase::ScanningInput, 0, None);
    for n in 1..100_000 {
        one.count(n, None);
    }
    assert_eq!(rx.borrow_and_update().completed, 99_999);
    assert!(!rx.has_changed().unwrap());
    assert_eq!(other.borrow().phase, Phase::Starting);
    two.phase(Phase::Matching, 10, Some(10));
    assert_eq!(rx.borrow().phase, Phase::ScanningInput);
    drop(one);
    assert!(rx.has_changed().is_err());
    assert!(original.progress.is_none());
}

#[test]
fn all_phases_use_correct_units_and_unknown_total_never_means_complete() {
    for phase in [
        Phase::Starting,
        Phase::CopyingInput,
        Phase::ScanningInput,
        Phase::RequestingToken,
        Phase::WaitingForDat,
        Phase::DownloadingDat,
        Phase::RetryWait,
        Phase::DecryptingDat,
        Phase::VerifyingBloom,
        Phase::VerifyingInput,
        Phase::VerifyingCheckpoint,
        Phase::Matching,
        Phase::Publishing,
    ] {
        let mut p = Progress::new(phase);
        p.completed = 1_048_576;
        let now = p.phase_started + Duration::from_secs(2);
        for compact in [true, false] {
            assert!(!p.headline(now, compact).is_empty());
        }
        assert_eq!(p.ratio(), 0.0);
        p.total = Some(0);
        assert_eq!(p.ratio(), 0.0);
        p.total = Some(524_288);
        assert_eq!(p.ratio(), 1.0);
        if phase.bytes() {
            assert!(p.detail(now).contains("0.5 MiB/s"));
        } else if matches!(phase, Phase::Matching | Phase::ScanningInput) {
            assert!(p.detail(now).contains("524288 rows/s"));
        } else {
            assert!(p.detail(now).contains("elapsed 2s"));
        }
    }
}

#[tokio::test]
async fn preparation_and_matching_report_live_rows_without_changing_order_or_checkpoints() {
    let root = tempfile::tempdir().unwrap();
    let service =
        Service::open(root.path().join("matching"), Arc::new(MemorySecrets::new())).unwrap();
    let input = root.path().join("source.csv");
    let mut file = std::fs::File::create(&input).unwrap();
    writeln!(file, "identifier").unwrap();
    for i in 0..20_000 {
        writeln!(file, "{i:011}").unwrap();
    }
    drop(file);
    let (observed, mut rx) = service.observed();
    let worker = observed.clone();
    let import = tokio::task::spawn_blocking(move || worker.create_job(&input, Rule::default()));
    let job = import.await.unwrap().unwrap();
    let p = rx.borrow_and_update().clone();
    assert_eq!(p.phase, Phase::ScanningInput);
    assert_eq!(p.completed, 20_000);
    assert_eq!(p.job, Some(uuid::Uuid::parse_str(&job.id).unwrap()));
    let mut bloom = Bloom::new(1, &Rule::default()).unwrap();
    bloom.insert("00000000000").unwrap();
    let dat = root.path().join("fixture.dat");
    bloom.write(&dat).unwrap();
    let filter = Snapshot {
        bloom: Arc::new(Bloom::read(&dat, 4096).unwrap()),
    };
    let mut control = RunControl::default();
    control.pause_after = Some(8192);
    let paused = observed
        .run_job(&job.id, Some(filter.clone()), control)
        .await
        .unwrap();
    let p = rx.borrow().clone();
    assert_eq!(paused.state, "PAUSED");
    assert_eq!(paused.processed, 8192);
    assert_eq!(p.phase, Phase::Matching);
    assert_eq!(p.completed, 8192);
    assert_eq!(p.total, Some(20_000));
    let done = observed
        .run_job(&job.id, Some(filter), RunControl::default())
        .await
        .unwrap();
    assert_eq!(done.state, "COMPLETED");
    assert_eq!(rx.borrow().phase, Phase::Publishing);
    assert_eq!(rx.borrow().completed, 20_000);
    assert!(service.verify(&job.id).unwrap());
    let output = std::fs::read_to_string(service.result_path(&job.id).unwrap()).unwrap();
    assert!(output.starts_with("00000000000: true\n00000000001: "));
    assert_eq!(output.lines().count(), 20_000);
}
