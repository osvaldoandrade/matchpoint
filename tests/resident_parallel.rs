use pgm_data::{
    matching::{format::Bloom, Rule, RunControl, Service, Snapshot, Transform},
    secrets::MemorySecrets,
};
use std::{fs, path::Path, sync::Arc};

fn service(path: &Path) -> Service {
    Service::open(path.into(), Arc::new(MemorySecrets::new())).unwrap()
}
fn dat(path: &Path, rule: &Rule) -> std::path::PathBuf {
    let mut b = Bloom::new(4, rule).unwrap();
    for key in ["000123", "a:b", "quoted\nline", " 000123 "] {
        b.insert(key).unwrap();
    }
    let path = path.join(format!("{}.dat", uuid::Uuid::new_v4()));
    b.write(&path).unwrap();
    path
}
fn snapshot(path: &Path) -> Snapshot {
    Snapshot {
        bloom: Arc::new(Bloom::read(path, 1024 * 1024).unwrap()),
    }
}

#[tokio::test]
async fn n_workers_preserve_csv_records_order_and_wipe_each_completed_job() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    for transform in [Transform::None, Transform::Sha256, Transform::Sha256Raw] {
        let rule = Rule {
            column: "identifier".into(),
            delimiter: b';',
            transform,
            trim: true,
            ..Rule::default()
        };
        let filter = dat(root.path(), &rule);
        let input = root.path().join("quoted.csv");
        let mut writer = csv::WriterBuilder::new()
            .delimiter(b';')
            .from_path(&input)
            .unwrap();
        writer.write_record(["identifier", "ignored"]).unwrap();
        for i in 0..20000 {
            writer
                .write_record([
                    ["000123", "a:b", "quoted\nline", " 000123 ", "absent"][i % 5],
                    "ignored",
                ])
                .unwrap();
        }
        writer.flush().unwrap();
        let mut expected = None;
        for threads in [1, 2, 7, 32, 256] {
            let loaded = snapshot(&filter);
            let weak = Arc::downgrade(&loaded.bloom);
            let job = service.create_job(&input, rule.clone()).unwrap();
            let mut control = RunControl::default();
            control.threads = Some(threads);
            let done = service
                .run_job(&job.id, Some(loaded), control)
                .await
                .unwrap();
            assert_eq!(done.processed, 20000);
            assert_eq!(done.threads, threads);
            assert_eq!(done.positives, 16000);
            let output = fs::read(service.result_path(&job.id).unwrap()).unwrap();
            if let Some(expected) = &expected {
                assert_eq!(&output, expected);
            } else {
                assert!(String::from_utf8_lossy(&output).starts_with("000123: true\n\"a:b\": true\n\"quoted\\nline\": true\n\" 000123 \": true\nabsent: false\n"));
                expected = Some(output);
            }
            assert!(
                weak.upgrade().is_none(),
                "job retains filter after completion"
            );
            assert!(service.cached(None).unwrap().is_none());
            assert!(service.verify(&job.id).unwrap());
        }
    }
    assert!(!service.root.join("current.json").exists());
    assert!(fs::read_dir(service.root.join("filters"))
        .unwrap()
        .next()
        .is_none());
}

#[tokio::test]
async fn pause_keeps_same_mapping_resume_changes_only_scheduling_and_error_releases() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let input = root.path().join("keys.csv");
    fs::write(&input, "identifier\n000123\nabsent\n000123\n").unwrap();
    let rule = Rule::default();
    let path = dat(root.path(), &rule);
    let loaded = snapshot(&path);
    let weak = Arc::downgrade(&loaded.bloom);
    let job = service.create_job(&input, rule.clone()).unwrap();
    let mut control = RunControl::default();
    control.pause_after = Some(1);
    control.threads = Some(3);
    let paused = service
        .run_job(&job.id, Some(loaded), control)
        .await
        .unwrap();
    assert_eq!(paused.processed, 1);
    assert!(weak.upgrade().is_some());
    service.set_job_threads(&job.id, 8).unwrap();
    let done = service
        .run_job(&job.id, None, RunControl::default())
        .await
        .unwrap();
    assert_eq!(done.threads, 8);
    assert!(weak.upgrade().is_none());
    let loaded = snapshot(&path);
    let weak = Arc::downgrade(&loaded.bloom);
    let wrong = service
        .create_job(
            &input,
            Rule {
                transform: Transform::Sha256,
                ..rule
            },
        )
        .unwrap();
    assert!(service
        .run_job(&wrong.id, Some(loaded), RunControl::default())
        .await
        .is_err());
    assert!(weak.upgrade().is_none());
    assert!(!service.result_path(&wrong.id).unwrap().exists());
}

#[test]
fn delete_is_scoped_locked_and_preserves_originals_and_external_exports() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let input = root.path().join("original.csv");
    fs::write(&input, "key\n000123\n").unwrap();
    let export = root.path().join("external.txt");
    fs::write(&export, "keep").unwrap();
    let job = service.create_job(&input, Rule::default()).unwrap();
    let stable =
        pgm_data::disk::acquire(&service.root.join("locks").join(format!("{}.lock", job.id)))
            .unwrap();
    assert!(service.delete_job(&job.id).is_err());
    assert!(service.set_job_threads(&job.id, 2).is_err());
    drop(stable);
    let legacy =
        pgm_data::disk::acquire(&service.root.join("jobs").join(&job.id).join("run.lock")).unwrap();
    assert!(service.delete_job(&job.id).is_err());
    drop(legacy);
    for id in ["", "../original.csv", "not-a-uuid"] {
        assert!(service.delete_job(id).is_err());
    }
    service.delete_job(&job.id[..8]).unwrap();
    assert!(service.jobs().unwrap().is_empty());
    assert!(input.exists());
    assert_eq!(fs::read_to_string(&export).unwrap(), "keep");
    assert!(service.delete_job(&job.id).is_err());
}

#[cfg(unix)]
#[test]
fn deletion_refuses_directory_symlinks_and_does_not_follow_child_links() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let input = root.path().join("original.csv");
    fs::write(&input, "key\n000123\n").unwrap();
    let foreign = root.path().join("foreign");
    fs::create_dir(&foreign).unwrap();
    fs::write(foreign.join("keep"), "safe").unwrap();
    let job = service.create_job(&input, Rule::default()).unwrap();
    for name in ["locks", "deleting"] {
        let path = service.root.join(name);
        symlink(&foreign, &path).unwrap();
        assert!(service.delete_job(&job.id).is_err());
        fs::remove_file(path).unwrap();
    }
    symlink(
        &foreign,
        service
            .root
            .join("jobs")
            .join(uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    symlink(
        &foreign,
        service.root.join("jobs").join(&job.id).join("child-link"),
    )
    .unwrap();
    service.delete_job(&job.id).unwrap();
    assert_eq!(fs::read_to_string(foreign.join("keep")).unwrap(), "safe");
}

#[test]
fn legacy_metadata_defaults_to_one_thread_and_only_owned_cache_names_are_retired() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let input = root.path().join("original.csv");
    fs::write(&input, "key\n000123\n").unwrap();
    let job = service.create_job(&input, Rule::default()).unwrap();
    let meta = service.root.join("jobs").join(&job.id).join("job.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&meta).unwrap()).unwrap();
    value.as_object_mut().unwrap().remove("threads");
    fs::write(&meta, serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(service.job(&job.id).unwrap().threads, 1);
    fs::remove_file(service.root.join("ram-only-v1.json")).unwrap();
    let legacy = service
        .root
        .join("filters")
        .join(format!("{}.dat", "a".repeat(64)));
    fs::write(&legacy, "old private data").unwrap();
    let keep = service.root.join("filters").join("unrelated.txt");
    fs::write(&keep, "keep").unwrap();
    fs::write(service.root.join("current.json"), "{}").unwrap();
    drop(service);
    let reopened = Service::open(root.path().into(), Arc::new(MemorySecrets::new())).unwrap();
    assert!(!legacy.exists());
    assert!(keep.exists());
    assert!(!reopened.root.join("current.json").exists());
    assert_eq!(reopened.jobs().unwrap().len(), 1);
}

#[tokio::test]
async fn deleting_a_paused_job_releases_its_only_resident_filter() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let input = root.path().join("keys.csv");
    fs::write(&input, "identifier\n000123\nabsent\n").unwrap();
    let loaded = snapshot(&dat(root.path(), &Rule::default()));
    let weak = Arc::downgrade(&loaded.bloom);
    let job = service.create_job(&input, Rule::default()).unwrap();
    let mut control = RunControl::default();
    control.pause_after = Some(1);
    service
        .run_job(&job.id, Some(loaded), control)
        .await
        .unwrap();
    assert!(weak.upgrade().is_some());
    service.delete_job(&job.id).unwrap();
    assert!(weak.upgrade().is_none());
    assert!(input.exists());
}

#[test]
fn startup_retries_only_recognized_unlocked_deletion_stages() {
    let root = tempfile::tempdir().unwrap();
    let svc = service(root.path());
    let trash = svc.root.join("deleting");
    fs::create_dir(&trash).unwrap();
    fs::create_dir(svc.root.join("locks")).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let stage = trash.join(format!("{id}-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&stage).unwrap();
    fs::write(stage.join("result.txt"), "discard").unwrap();
    // An arbitrary UTF-8 filename must never be sliced at a non-character boundary.
    let unrelated = trash.join(format!("{}é{}", "x".repeat(35), "x".repeat(36)));
    fs::create_dir(&unrelated).unwrap();
    let lock = pgm_data::disk::acquire(&svc.root.join("locks").join(format!("{id}.lock"))).unwrap();
    drop(svc);
    drop(service(root.path()));
    assert!(stage.exists());
    drop(lock);
    drop(service(root.path()));
    assert!(!stage.exists());
    assert!(unrelated.exists());
}

#[test]
fn cli_requires_delete_confirmation_and_valid_worker_count() {
    use std::process::Command;
    let root = tempfile::tempdir().unwrap();
    let svc = service(root.path());
    let input = root.path().join("original.csv");
    fs::write(&input, "key\n000123\n").unwrap();
    let job = svc.create_job(&input, Rule::default()).unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_pgm-data"))
            .env("PGM_DATA_HOME", root.path())
            .args(args)
            .output()
            .unwrap()
    };
    assert!(!run(&["job", "delete", &job.id]).status.success());
    assert!(svc.job(&job.id).is_ok());
    for n in ["0", "257", "-1"] {
        assert!(!run(&["job", "threads", &job.id, "--threads", n])
            .status
            .success());
    }
    assert!(run(&["job", "threads", &job.id, "--threads", "7"])
        .status
        .success());
    assert_eq!(svc.job(&job.id).unwrap().threads, 7);
    assert!(run(&["job", "delete", &job.id, "--yes"]).status.success());
    assert!(svc.jobs().unwrap().is_empty());
    assert!(input.exists());
}

#[cfg(target_os = "macos")]
#[test]
fn denied_mlock_fails_closed_without_creating_a_dat_cache() {
    let root = tempfile::tempdir().unwrap();
    let path = dat(root.path(), &Rule::default());
    let output=std::process::Command::new("/usr/bin/python3")
        .args(["-c","import os,resource,sys; resource.setrlimit(resource.RLIMIT_MEMLOCK,(0,0)); os.execv(sys.argv[1],sys.argv[1:])",env!("CARGO_BIN_EXE_pgm-data"),"bloom","inspect"])
        .arg(&path).env("PGM_DATA_HOME",root.path()).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Cannot lock DAT pages in RAM"));
    assert!(fs::read_dir(root.path().join("matching/filters"))
        .unwrap()
        .next()
        .is_none());
}
