//! Primary results use the job's pinned launch directory, independently of state storage.
use pgm_data::{
    matching::{format::Bloom, Rule, RunControl, Service, Snapshot},
    secrets::MemorySecrets,
};
use std::{fs, path::Path, process::Command, sync::Arc};

fn service(root: &Path) -> Service {
    Service::open(root.to_path_buf(), Arc::new(MemorySecrets::new())).unwrap()
}
fn snapshot(root: &Path) -> Snapshot {
    let mut bloom = Bloom::new(32, &Rule::default()).unwrap();
    bloom.insert("0001").unwrap();
    let path = root.join("fixture.dat");
    bloom.write(&path).unwrap();
    Snapshot {
        bloom: Arc::new(Bloom::read(&path, 4096).unwrap()),
    }
}
fn cli(state: &Path, cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pgm-data"))
        .env("PGM_DATA_HOME", state)
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}
const EXPECTED: &str = "0001: true\nabsent: false\n0001: true\n";

#[tokio::test]
async fn cli_job_pins_cwd_and_publishes_there_after_pause_restart_and_other_cwd_resume() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    let cwd = root.path().join("launch with spaces");
    let resume_cwd = root.path().join("resume elsewhere");
    fs::create_dir(&cwd).unwrap();
    fs::create_dir(&resume_cwd).unwrap();
    let cwd = cwd.canonicalize().unwrap();
    let input = root.path().join("keys.csv");
    fs::write(&input, "key\n0001\nabsent\n0001\n").unwrap();
    let created = cli(
        &state,
        &cwd,
        &["job", "create", "--input", input.to_str().unwrap()],
    );
    assert!(created.status.success(), "{:?}", created);
    let id = String::from_utf8(created.stdout).unwrap().trim().to_owned();
    let owned = service(&state);
    let target = cwd.join(format!("result-{id}.txt"));
    assert_eq!(owned.job(&id).unwrap().output_path.as_ref(), Some(&target));
    assert_eq!(owned.result_path(&id).unwrap(), target);
    let filter = snapshot(root.path());
    let mut control = RunControl::default();
    control.pause_after = Some(1);
    assert_eq!(
        owned
            .run_job(&id, Some(filter.clone()), control)
            .await
            .unwrap()
            .state,
        "PAUSED"
    );
    assert!(!target.exists());
    drop(owned);
    let owned = service(&state);
    assert_eq!(
        owned
            .run_job(&id, Some(filter), RunControl::default())
            .await
            .unwrap()
            .state,
        "COMPLETED"
    );
    assert_eq!(fs::read_to_string(&target).unwrap(), EXPECTED);
    assert!(owned.verify(&id).unwrap());
    let internal = owned.root.join("jobs").join(&id);
    assert!(!internal.join("result.txt").exists());
    assert!(!internal.join("result.partial").exists());
    let resumed = cli(&state, &resume_cwd, &["job", "resume", &id]);
    assert!(resumed.status.success(), "{:?}", resumed);
    assert!(String::from_utf8(resumed.stdout)
        .unwrap()
        .contains(&format!("Output: {}", target.display())));
    assert!(!resume_cwd.join(target.file_name().unwrap()).exists());
    let exported = cli(&state, &cwd, &["job", "export", &id]);
    assert!(exported.status.success(), "{:?}", exported);
    assert_eq!(fs::read_to_string(&target).unwrap(), EXPECTED);
    owned.delete_job(&id).unwrap();
    assert_eq!(fs::read_to_string(target).unwrap(), EXPECTED);
}

#[tokio::test]
async fn explicit_primary_destinations_refuse_collisions_and_verify_actual_output() {
    let root = tempfile::tempdir().unwrap();
    let owned = service(&root.path().join("state"));
    let input = root.path().join("keys.csv");
    fs::write(&input, "key\n0001\nabsent\n0001\n").unwrap();
    let filter = snapshot(root.path());
    let directory = root.path().join("exports");
    fs::create_dir(&directory).unwrap();
    for requested in [root.path().join("custom output.txt"), directory.clone()] {
        let job = owned
            .create_job_cancellable_to(
                &input,
                Rule::default(),
                &RunControl::default().stop,
                &requested,
            )
            .unwrap();
        let target = owned.result_path(&job.id).unwrap();
        let expected = if requested == directory {
            directory.join(format!("result-{}.txt", job.id))
        } else {
            requested
        };
        assert_eq!(target, expected);
        fs::write(&target, "preserve existing file").unwrap();
        assert!(owned
            .run_job(&job.id, Some(filter.clone()), RunControl::default())
            .await
            .is_err());
        assert_eq!(
            fs::read_to_string(&target).unwrap(),
            "preserve existing file"
        );
        fs::remove_file(&target).unwrap();
        owned
            .run_job(&job.id, Some(filter.clone()), RunControl::default())
            .await
            .unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), EXPECTED);
        fs::write(&target, "tampered").unwrap();
        assert!(!owned.verify(&job.id).unwrap());
        assert!(owned
            .run_job(&job.id, None, RunControl::default())
            .await
            .is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "tampered");
    }
    let existing = root.path().join("already exists.txt");
    fs::write(&existing, "keep").unwrap();
    let count = owned.jobs().unwrap().len();
    assert!(owned
        .create_job_cancellable_to(
            &input,
            Rule::default(),
            &RunControl::default().stop,
            &existing
        )
        .unwrap_err()
        .to_string()
        .contains("already exists"));
    assert_eq!(owned.jobs().unwrap().len(), count);
    assert_eq!(fs::read_to_string(existing).unwrap(), "keep");
}

#[tokio::test]
async fn failed_publication_keeps_checkpoint_and_retries_to_the_same_pinned_destination() {
    let root = tempfile::tempdir().unwrap();
    let owned = service(&root.path().join("state"));
    let input = root.path().join("keys.csv");
    fs::write(&input, "key\n0001\nabsent\n0001\n").unwrap();
    let target = root.path().join("missing directory").join("result.txt");
    let job = owned
        .create_job_cancellable_to(
            &input,
            Rule::default(),
            &RunControl::default().stop,
            &target,
        )
        .unwrap();
    let filter = snapshot(root.path());
    assert!(owned
        .run_job(&job.id, Some(filter.clone()), RunControl::default())
        .await
        .is_err());
    let internal = owned.root.join("jobs").join(&job.id);
    assert!(internal.join("result.partial").exists());
    assert!(!internal.join("result.txt").exists());
    assert!(!target.exists());
    assert_ne!(owned.job(&job.id).unwrap().state, "COMPLETED");
    fs::create_dir(target.parent().unwrap()).unwrap();
    owned
        .run_job(&job.id, Some(filter), RunControl::default())
        .await
        .unwrap();
    assert_eq!(fs::read_to_string(target).unwrap(), EXPECTED);
    assert!(owned.verify(&job.id).unwrap());
}

#[tokio::test]
async fn legacy_metadata_keeps_internal_results_and_empty_completed_outputs_must_exist() {
    let root = tempfile::tempdir().unwrap();
    let owned = service(&root.path().join("state"));
    let input = root.path().join("keys.csv");
    fs::write(&input, "key\n").unwrap();
    let job = owned.create_job(&input, Rule::default()).unwrap();
    let metadata = owned.root.join("jobs").join(&job.id).join("job.json");
    assert!(!fs::read_to_string(metadata)
        .unwrap()
        .contains("output_path"));
    assert!(owned.job(&job.id).unwrap().output_path.is_none());
    let path = owned.result_path(&job.id).unwrap();
    assert_eq!(
        path,
        std::path::absolute(owned.root.join("jobs").join(&job.id).join("result.txt")).unwrap()
    );
    owned
        .run_job(&job.id, Some(snapshot(root.path())), RunControl::default())
        .await
        .unwrap();
    assert!(owned.verify(&job.id).unwrap());
    assert_eq!(fs::metadata(&path).unwrap().len(), 0);
    fs::remove_file(path).unwrap();
    assert!(owned.verify(&job.id).is_err());
}
