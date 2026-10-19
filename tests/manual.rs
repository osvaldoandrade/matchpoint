use std::process::Command;

#[test]
fn manual_and_all_help_paths_work_without_application_storage() {
    let root = tempfile::tempdir().unwrap();
    let unavailable = root.path().join("blocked");
    std::fs::write(&unavailable, b"not a directory").unwrap();
    for args in [
        "manual",
        "manual --roff",
        "--help",
        "tui --help",
        "doctor --help",
        "auth --help",
        "auth credentials --help",
        "auth get-token --help",
        "auth refresh-token --help",
        "bloom --help",
        "bloom configure --help",
        "bloom token --help",
        "bloom download --help",
        "bloom build --help",
        "bloom inspect --help",
        "match --help",
        "job --help",
        "job create --help",
        "job list --help",
        "job status --help",
        "job run --help",
        "job resume --help",
        "job checksum --help",
        "job export --help",
        "job threads --help",
        "job delete --help",
        "manual --help",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_pgm-data"))
            .args(args.split_whitespace())
            .env("PGM_DATA_HOME", &unavailable)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.stdout.is_empty());
        if args == "manual" {
            let manual = String::from_utf8(output.stdout).unwrap();
            for topic in [
                "TRANSFORMATIONS",
                "FILES AND RECOVERY",
                "matching/jobs/JOB_ID/result.txt",
                "refresh_token",
                "0.1%",
                "PGM_DATA_HOME",
            ] {
                assert!(manual.contains(topic));
            }
        }
    }
}

#[test]
fn aes_key_arguments_are_rejected_without_echo_or_storage() {
    let root = tempfile::tempdir().unwrap();
    for flag in ["--aes-key", "--dat-key", "--encryption-key"] {
        for inline in [false, true] {
            let value = "synthetic-sensitive-value-never-echo";
            let args = if inline {
                vec![format!("{flag}={value}")]
            } else {
                vec![flag.into(), value.into()]
            };
            let output = Command::new(env!("CARGO_BIN_EXE_pgm-data"))
                .args(args)
                .env("PGM_DATA_HOME", root.path())
                .output()
                .unwrap();
            assert!(!output.status.success());
            assert!(!String::from_utf8_lossy(&output.stderr).contains(value));
            assert!(!String::from_utf8_lossy(&output.stdout).contains(value));
        }
    }
    assert!(std::fs::read_dir(root.path()).unwrap().next().is_none());
}
