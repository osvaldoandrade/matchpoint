use arrow_array::{ArrayRef, Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use parquet::{arrow::ArrowWriter, basic::Compression, file::properties::WriterProperties};
use pgm_data::{
    error::Secret,
    matching::{
        dataset::Dataset,
        format::{self, Bloom},
        Job, Rule, RunControl, Service, Settings, Snapshot, Transform,
    },
    secrets::MemorySecrets,
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::watch,
};

fn service(root: &Path) -> Service {
    Service::open(root.join("data"), Arc::new(MemorySecrets::new())).unwrap()
}

#[test]
fn canonical_token_dat_urls_read_but_do_not_write_legacy_refresh_url() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let defaults = service.settings().unwrap();
    assert!(
        defaults.url.is_empty()
            && defaults.token_url.is_empty()
            && defaults.reset_token_url.is_empty()
    );
    let legacy: Settings =
        serde_json::from_str(r#"{"url":"https://data.example.test/list.dat"}"#).unwrap();
    assert!(legacy.token_url.is_empty());
    let cfg = Settings {
        token_url: "https://auth.example.test/issue".into(),
        reset_token_url: "https://reset.example.test/reset".into(),
        ..legacy
    };
    service.save_settings(&cfg).unwrap();
    let json = fs::read_to_string(service.root.join("settings.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(value.get("url").is_none());
    assert_eq!(value["dat_url"], cfg.url);
    assert!(value.get("reset_token_url").is_none());
    let loaded = service.settings().unwrap();
    assert_eq!(
        (loaded.url, loaded.token_url, loaded.reset_token_url),
        (cfg.url, cfg.token_url, String::new())
    );
    assert!(serde_json::from_str::<Settings>(
        r#"{"url":"https://one.test/dat","dat_url":"https://two.test/dat"}"#
    )
    .is_err());
    assert!(pgm_data::engine::AppConfig::default().token_url.is_empty());
    assert!(pgm_data::jobcfg::default_tuning().endpoint.is_empty());
}

#[test]
fn each_api_url_is_validated_without_echoing_unsafe_contents() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    for field in 0..3 {
        for value in [
            "http://example.test/path",
            "https://user:do-not-echo@example.test/path",
            "https://example.test/path?secret=do-not-echo",
            "https://example.test/path#fragment",
            "https://example.test/\npath",
            "file:///tmp/filter.dat",
        ] {
            let mut cfg = Settings::default();
            match field {
                0 => cfg.token_url = value.into(),
                1 => cfg.reset_token_url = value.into(),
                _ => cfg.url = value.into(),
            };
            let error = service.save_settings(&cfg).unwrap_err().to_string();
            assert!(!error.contains("do-not-echo"));
            assert!(!service.root.join("settings.json").exists());
        }
    }
}

#[test]
fn cli_configures_common_token_url_and_preserves_unspecified_fields() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("data");
    let binary = env!("CARGO_BIN_EXE_pgm-data");
    for args in [
        vec![
            "bloom",
            "configure",
            "--token-url",
            "https://auth.example.test/issue",
            "--reset-token-url",
            "https://auth.example.test/issue",
            "--dat-url",
            "https://data.example.test/list.dat",
        ],
        vec![
            "bloom",
            "configure",
            "--url",
            "https://data.example.test/updated.dat",
        ],
    ] {
        let result = std::process::Command::new(binary)
            .env("PGM_DATA_HOME", &state)
            .args(args)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let cfg = service(root.path()).settings().unwrap();
    assert_eq!(cfg.token_url, "https://auth.example.test/issue");
    assert!(cfg.reset_token_url.is_empty());
    assert_eq!(cfg.url, "https://data.example.test/updated.dat");
    let before = fs::read(state.join("matching/settings.json")).unwrap();
    let rejected = std::process::Command::new(binary)
        .env("PGM_DATA_HOME", &state)
        .args([
            "bloom",
            "configure",
            "--reset-token-url",
            "https://other.example.test/refresh",
        ])
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert_eq!(
        fs::read(state.join("matching/settings.json")).unwrap(),
        before
    );
}
fn csv(root: &Path, name: &str, text: &str) -> PathBuf {
    let path = root.join(name);
    fs::write(&path, text).unwrap();
    path
}
fn snapshot(root: &Path, keys: &[&str], rule: &Rule) -> Snapshot {
    let path = root.join(format!("{}.dat", uuid::Uuid::new_v4()));
    let mut bloom = Bloom::new(keys.len() as u64, rule).unwrap();
    for key in keys {
        bloom.insert(key).unwrap();
    }
    bloom.write(&path).unwrap();
    Snapshot {
        bloom: Arc::new(Bloom::read(&path, 1024 * 1024).unwrap()),
    }
}
fn pin(service: &Service, snapshot: &Snapshot) {
    service.retain(snapshot.clone()).unwrap();
}

fn parquet(root: &Path, compression: Compression, keys: Vec<Option<&str>>) -> PathBuf {
    let schema = Arc::new(Schema::new(vec![
        Field::new("ignore", DataType::Int64, false),
        Field::new("identifier", DataType::Utf8, true),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(Int64Array::from(vec![42; keys.len()])) as ArrayRef,
            Arc::new(StringArray::from(keys)),
        ],
    )
    .unwrap();
    let path = root.join(format!("{}.parquet", uuid::Uuid::new_v4()));
    let props = WriterProperties::builder()
        .set_compression(compression)
        .set_max_row_group_row_count(Some(2))
        .build();
    let mut writer =
        ArrowWriter::try_new(fs::File::create(&path).unwrap(), schema, Some(props)).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
    path
}
async fn run(service: &Service, input: &Path, rule: Rule, snapshot: Snapshot) -> Job {
    let job = service.create_job(input, rule).unwrap();
    service
        .run_job(&job.id, Some(snapshot), RunControl::default())
        .await
        .unwrap()
}

#[tokio::test]
async fn csv_matches_any_identifier_preserves_order_zeros_duplicates_and_escapes() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let input = csv(
        root.path(),
        "input.csv",
        "other;identifier\n1;000123\n2;absent\n3;000123\n4;\"a:b\"\n5;\"line\nbreak\"\n",
    );
    let rule = Rule {
        column: "identifier".into(),
        delimiter: b';',
        ..Rule::default()
    };
    let filter = snapshot(root.path(), &["000123", "a:b", "line\nbreak"], &rule);
    let job = run(&service, &input, rule, filter).await;
    assert_eq!(
        fs::read_to_string(service.result_path(&job.id).unwrap()).unwrap(),
        "000123: true\nabsent: false\n000123: true\n\"a:b\": true\n\"line\\nbreak\": true\n"
    );
    assert_eq!((job.total, job.positives, job.negatives), (5, 4, 1));
    assert!(service.verify(&job.id).unwrap());
    let destination = root.path().join("existing.txt");
    fs::write(&destination, "keep").unwrap();
    assert!(pgm_data::matching::cli::copy_result(
        &service.result_path(&job.id).unwrap(),
        &destination
    )
    .is_err());
    assert_eq!(fs::read_to_string(destination).unwrap(), "keep");
    let old = root.path().join("data/jobs/legacy");
    fs::create_dir_all(&old).unwrap();
    fs::write(old.join("keep"), b"historic").unwrap();
    assert_eq!(service.jobs().unwrap().len(), 1);
    assert_eq!(fs::read(old.join("keep")).unwrap(), b"historic");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(service.result_path(&job.id).unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&service.root).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}
#[tokio::test]
async fn parquet_projection_and_resume_across_row_groups_and_codecs() {
    for compression in [
        Compression::UNCOMPRESSED,
        Compression::SNAPPY,
        Compression::GZIP(Default::default()),
        Compression::LZ4_RAW,
        Compression::ZSTD(Default::default()),
        Compression::BROTLI(Default::default()),
    ] {
        let root = tempfile::tempdir().unwrap();
        let service = service(root.path());
        let input = parquet(
            root.path(),
            compression,
            vec![
                Some("0001"),
                Some("not-present"),
                Some("0001"),
                Some("partner-x"),
                Some("0009"),
            ],
        );
        let rule = Rule {
            column: "identifier".into(),
            headers: false,
            transform: Transform::Sha256,
            ..Rule::default()
        };
        let filter = snapshot(root.path(), &["0001", "partner-x"], &rule);
        pin(&service, &filter);
        let job = service.create_job(&input, rule).unwrap();
        let mut control = RunControl::default();
        control.pause_after = Some(3);
        let paused = service
            .run_job(&job.id, Some(filter), control)
            .await
            .unwrap();
        assert_eq!((paused.state.as_str(), paused.processed), ("PAUSED", 3));
        let done = service
            .run_job(&job.id, None, RunControl::default())
            .await
            .unwrap();
        assert_eq!(done.processed, 5);
        assert_eq!(
            fs::read_to_string(service.result_path(&job.id).unwrap()).unwrap(),
            "0001: true\nnot-present: false\n0001: true\npartner-x: true\n0009: false\n"
        );
        assert!(service.verify(&job.id).unwrap());
    }
}

#[tokio::test]
async fn large_parquet_resumes_across_batch_and_row_group_boundaries_in_order() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let keys = (0..20_000).map(|i| format!("{i:011}")).collect::<Vec<_>>();
    let schema = Arc::new(Schema::new(vec![Field::new(
        "identifier",
        DataType::Utf8,
        false,
    )]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![Arc::new(StringArray::from(
            keys.iter().map(String::as_str).collect::<Vec<_>>(),
        ))],
    )
    .unwrap();
    let input = root.path().join("large.parquet");
    let props = WriterProperties::builder()
        .set_compression(Compression::ZSTD(Default::default()))
        .set_max_row_group_row_count(Some(4096))
        .build();
    let mut writer =
        ArrowWriter::try_new(fs::File::create(&input).unwrap(), schema, Some(props)).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
    let rule = Rule::default();
    let filter = snapshot(
        root.path(),
        &keys.iter().map(String::as_str).collect::<Vec<_>>(),
        &rule,
    );
    pin(&service, &filter);
    let job = service.create_job(&input, rule).unwrap();
    let mut control = RunControl::default();
    control.pause_after = Some(8193);
    let paused = service
        .run_job(&job.id, Some(filter), control)
        .await
        .unwrap();
    assert_eq!(paused.processed, 8193);
    service
        .run_job(&job.id, None, RunControl::default())
        .await
        .unwrap();
    let expected = keys
        .iter()
        .map(|key| format!("{key}: true\n"))
        .collect::<String>();
    assert_eq!(
        fs::read_to_string(service.result_path(&job.id).unwrap()).unwrap(),
        expected
    );
}
#[test]
fn parquet_nulls_unsupported_columns_and_bad_csv_never_become_false() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let rule = Rule {
        column: "identifier".into(),
        ..Rule::default()
    };
    let input = parquet(root.path(), Compression::SNAPPY, vec![Some("ok"), None]);
    assert!(service.create_job(&input, rule.clone()).is_err());
    let mut keys = Dataset::open(
        &input,
        &Rule {
            column: "#1".into(),
            ..rule.clone()
        },
    )
    .unwrap();
    assert_eq!(keys.next_key().unwrap().unwrap(), "42");
    for text in [
        "identifier,identifier\na,b\n",
        "identifier\n\"\"\n",
        "identifier,other\nx\n",
    ] {
        let input = csv(root.path(), "bad.csv", text);
        assert!(service.create_job(&input, rule.clone()).is_err());
    }
    let input = csv(root.path(), "noheaders.csv", "0001\n0002\n");
    assert!(Dataset::open(
        &input,
        &Rule {
            headers: false,
            column: "identifier".into(),
            ..Rule::default()
        }
    )
    .is_err());
    assert!(service.jobs().unwrap().is_empty());
}
#[tokio::test]
async fn pause_resume_verifies_pinned_filter_input_and_committed_output_prefix() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let rule = Rule::default();
    let input = csv(root.path(), "input.csv", "key\n0001\n0002\n0001\n0009\n");
    let filter = snapshot(root.path(), &["0001", "0002"], &rule);
    pin(&service, &filter);
    let job = service.create_job(&input, rule).unwrap();
    let mut control = RunControl::default();
    control.pause_after = Some(2);
    let paused = service
        .run_job(&job.id, Some(filter.clone()), control)
        .await
        .unwrap();
    assert_eq!(paused.processed, 2);
    let dir = service.root.join("jobs").join(&job.id);
    let partial = dir.join("result.partial");
    let committed = fs::read(&partial).unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(&partial)
        .unwrap()
        .write_all(b"uncommitted\n")
        .unwrap();
    let lock = pgm_data::disk::acquire(&dir.join("run.lock")).unwrap();
    assert!(service
        .run_job(&job.id, None, RunControl::default())
        .await
        .is_err());
    drop(lock);
    let done = service
        .run_job(&job.id, None, RunControl::default())
        .await
        .unwrap();
    assert_eq!((done.processed, done.positives, done.negatives), (4, 3, 1));
    let result = service.result_path(&job.id).unwrap();
    assert_eq!(
        fs::read_to_string(&result).unwrap(),
        "0001: true\n0002: true\n0001: true\n0009: false\n"
    );
    assert!(fs::read(&result).unwrap().starts_with(&committed));
    // Crash after result publication, before final job metadata commit.
    let metadata = dir.join("job.json");
    let mut crashed = done;
    crashed.state = "RUNNING".into();
    fs::write(&metadata, serde_json::to_vec(&crashed).unwrap()).unwrap();
    fs::hard_link(&result, &partial).unwrap();
    assert_eq!(
        service
            .run_job(&job.id, None, RunControl::default())
            .await
            .unwrap()
            .state,
        "COMPLETED"
    );
    fs::write(&result, b"tampered").unwrap();
    assert!(!service.verify(&job.id).unwrap());
    assert!(service
        .run_job(&job.id, Some(filter), RunControl::default())
        .await
        .is_err());
}
#[tokio::test]
async fn mismatched_rules_missing_pin_mutated_input_and_bad_prefix_abort_publication() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let rule = Rule::default();
    let input = csv(root.path(), "input.csv", "key\n0001\n0002\n");
    let filter = snapshot(root.path(), &["0001"], &rule);
    let wrong = service
        .create_job(
            &input,
            Rule {
                transform: Transform::Sha256,
                ..rule.clone()
            },
        )
        .unwrap();
    assert!(service
        .run_job(&wrong.id, Some(filter.clone()), RunControl::default())
        .await
        .is_err());
    let job = service.create_job(&input, rule.clone()).unwrap();
    let mut control = RunControl::default();
    control.pause_after = Some(1);
    service
        .run_job(&job.id, Some(filter.clone()), control)
        .await
        .unwrap();
    service.clear_filters().unwrap();
    assert!(service
        .run_job(&job.id, None, RunControl::default())
        .await
        .is_err());
    let dir = service.root.join("jobs").join(&job.id);
    fs::write(dir.join("result.partial"), b"broken").unwrap();
    assert!(service
        .run_job(&job.id, Some(filter.clone()), RunControl::default())
        .await
        .is_err());
    assert!(!dir.join("result.txt").exists());
    let job = service.create_job(&input, rule).unwrap();
    fs::write(
        service.root.join("jobs").join(&job.id).join("input.csv"),
        b"changed",
    )
    .unwrap();
    assert!(service
        .run_job(&job.id, Some(filter), RunControl::default())
        .await
        .is_err());
}
#[test]
fn dataset_builder_supports_sha256_raw_trim_and_headerless_csv() {
    let root = tempfile::tempdir().unwrap();
    let input = csv(root.path(), "input.csv", " 0001 \nabc\nabc\n");
    let output = root.path().join("filter.dat");
    let rule = Rule {
        headers: false,
        transform: Transform::Sha256Raw,
        trim: true,
        ..Rule::default()
    };
    let info = format::build_dataset(&input, &output, &rule).unwrap();
    assert_eq!(info.inserted, 3);
    let b = Bloom::read(&output, 1024).unwrap();
    assert!(b.contains("0001").unwrap());
    assert!(b.contains(" abc ").unwrap());
    assert!(format::build_dataset(&input, &output, &rule).is_err());
    let parquet = parquet(
        root.path(),
        Compression::SNAPPY,
        vec![Some("0001"), Some("abc")],
    );
    let output = root.path().join("parquet.dat");
    let info = format::build_dataset(
        &parquet,
        &output,
        &Rule {
            column: "identifier".into(),
            ..rule
        },
    )
    .unwrap();
    assert_eq!(info.inserted, 2);
}

struct Server {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn server(body: Vec<u8>, status: &'static str, extra: &'static str) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/filter.dat", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded = requests.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buf = [0; 1024];
            loop {
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                bytes.extend(&buf[..n]);
                if bytes.windows(4).any(|v| v == b"\r\n\r\n") {
                    break;
                }
            }
            recorded
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&bytes).into_owned());
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.write_all(&body).await;
        }
    });
    Server {
        url,
        requests,
        task,
    }
}
fn configure(service: &Service, url: &str) {
    service
        .save_settings(&Settings {
            url: url.into(),
            allow_http_loopback: true,
            ..Settings::default()
        })
        .unwrap();
}
#[tokio::test]
async fn downloads_once_get_only_then_matches_and_resumes_offline() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let filter = snapshot(root.path(), &["0001"], &Rule::default());
    let path = root.path().join("source.dat");
    filter.bloom.write(&path).unwrap();
    let server = server(fs::read(&path).unwrap(), "200 OK", "").await;
    configure(&service, &server.url);
    service
        .save_token(&Secret::new("synthetic-test-bearer".into()).unwrap())
        .unwrap();
    let lock = pgm_data::disk::acquire(&service.root.join("source.lock")).unwrap();
    assert!(service
        .save_token(&Secret::new("should-not-replace-active-token".into()).unwrap())
        .is_err());
    drop(lock);
    let snapshot = service.download(RunControl::default().stop).await.unwrap();
    let id = snapshot.bloom.info().sha256.clone();
    assert_eq!(server.requests.lock().unwrap().len(), 1);
    let request = server.requests.lock().unwrap()[0].clone();
    assert!(request.starts_with("GET /filter.dat HTTP/1.1"));
    assert!(request
        .to_lowercase()
        .contains("authorization: bearer synthetic-test-bearer"));
    assert!(!request.contains("0001"));
    drop(server);
    let input = csv(root.path(), "input.csv", "key\n0001\nabsent\n0001\n");
    for _ in 0..2 {
        let job = run(&service, &input, Rule::default(), snapshot.clone()).await;
        assert_eq!((job.positives, job.negatives), (2, 1));
    }
    assert_eq!(snapshot.bloom.info().sha256, id);
    let cfg = service.settings().unwrap();
    assert!(!serde_json::to_string(&cfg)
        .unwrap()
        .contains("synthetic-test-bearer"));
}
#[tokio::test]
async fn source_policy_auth_binding_redirect_limits_and_corruption_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    for url in [
        "http://example.com/f.dat",
        "https://user:pass@example.com/f.dat",
        "https://example.com/f.dat?token=x",
        "https://example.com/f.dat#x",
        "file:///tmp/filter",
    ] {
        assert!(service
            .save_settings(&Settings {
                url: url.into(),
                ..Settings::default()
            })
            .is_err());
    }
    for (body, status, extra) in [
        (
            vec![],
            "302 Found",
            "Location: http://127.0.0.1:1/other\r\n",
        ),
        (vec![1, 2, 3], "200 OK", ""),
        (vec![], "503 Unavailable", ""),
    ] {
        let server = server(body, status, extra).await;
        configure(&service, &server.url);
        assert!(service.download(RunControl::default().stop).await.is_err());
        assert_eq!(
            server.requests.lock().unwrap().len(),
            if status.starts_with("503") { 3 } else { 1 }
        );
        assert!(!service.root.join("current.json").exists());
    }
    let server = server(vec![0; 1024 * 1024 + 1], "200 OK", "").await;
    configure(&service, &server.url);
    let mut cfg = service.settings().unwrap();
    cfg.max_download_mib = 1;
    service.save_settings(&cfg).unwrap();
    assert!(service.download(RunControl::default().stop).await.is_err());
    service
        .save_token(&Secret::new("source-bound-test-token".into()).unwrap())
        .unwrap();
    let other = server.url.replace("/filter.dat", "/other.dat");
    cfg = service.settings().unwrap();
    cfg.url = other;
    service.save_settings(&cfg).unwrap();
    let before = server.requests.lock().unwrap().len();
    assert!(service.download(RunControl::default().stop).await.is_err());
    assert_eq!(server.requests.lock().unwrap().len(), before);
    assert!(fs::read_dir(service.root.join("filters"))
        .unwrap()
        .next()
        .is_none());
}
#[tokio::test]
async fn cancellation_before_import_removes_own_incomplete_directory() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let input = csv(root.path(), "in.csv", "key\n0001\n");
    let (_tx, rx) = watch::channel(true);
    assert!(service
        .create_job_cancellable(&input, Rule::default(), &rx)
        .is_err());
    assert!(service.jobs().unwrap().is_empty());
    assert!(fs::read_dir(service.root.join("jobs"))
        .unwrap()
        .next()
        .is_none());
}

#[test]
fn new_binary_cli_builds_and_rejects_remote_legacy_commands_and_secret_flags() {
    let root = tempfile::tempdir().unwrap();
    let input = csv(root.path(), "input.csv", "key\n0001\nabc\n");
    let output = root.path().join("filter.dat");
    let binary = env!("CARGO_BIN_EXE_pgm-data");
    let result = std::process::Command::new(binary)
        .env("PGM_DATA_HOME", root.path().join("data"))
        .args(["bloom", "build", "--input"])
        .arg(input)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(Bloom::read(&output, 1024).unwrap().info().inserted, 2);
    for args in [
        vec!["auth", "configure"],
        vec!["--secret=do-not-echo-test-value"],
    ] {
        let result = std::process::Command::new(binary)
            .env("PGM_DATA_HOME", root.path().join("data"))
            .args(args)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("do-not-echo-test-value"));
    }
}

#[tokio::test]
async fn export_defaults_to_cwd_accepts_directories_and_reports_absolute_paths_without_overwriting()
{
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    let service = Service::open(state.clone(), Arc::new(MemorySecrets::new())).unwrap();
    let input = csv(root.path(), "keys.csv", "key\n0001\nabsent\n");
    let rule = Rule::default();
    let filter = snapshot(root.path(), &["0001"], &rule);
    let job = run(&service, &input, rule, filter).await;
    let expected = fs::read(service.result_path(&job.id).unwrap()).unwrap();
    for (case, requested) in [
        ("default", None),
        ("dot", Some("./")),
        ("file", Some("custom output.txt")),
        ("directory", Some("exports")),
    ] {
        let cwd = root.path().join(case);
        fs::create_dir(&cwd).unwrap();
        if requested == Some("exports") {
            fs::create_dir(cwd.join("exports")).unwrap();
        }
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_pgm-data"));
        command
            .env("PGM_DATA_HOME", &state)
            .current_dir(&cwd)
            .args(["job", "export", &job.id]);
        if let Some(path) = requested {
            command.args(["--output", path]);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let message = String::from_utf8(output.stdout).unwrap();
        let actual = PathBuf::from(message.trim().strip_prefix("Output: ").unwrap());
        assert!(actual.is_absolute());
        let directory = if requested == Some("exports") {
            cwd.join("exports")
        } else {
            cwd.clone()
        };
        let name = if requested == Some("custom output.txt") {
            "custom output.txt".into()
        } else {
            format!("result-{}.txt", job.id)
        };
        assert_eq!(actual, directory.canonicalize().unwrap().join(name));
        assert_eq!(fs::read(&actual).unwrap(), expected);
        let again = command.output().unwrap();
        assert!(!again.status.success());
        assert!(String::from_utf8_lossy(&again.stderr).contains("already exists"));
        assert_eq!(fs::read(&actual).unwrap(), expected);
        assert!(service.verify(&job.id).unwrap());
    }
}

#[test]
fn result_path_is_absolute_for_a_relative_data_root_without_changing_root_identity() {
    let cwd = std::env::current_dir().unwrap();
    let root = tempfile::Builder::new()
        .prefix("output-path-")
        .tempdir_in(&cwd)
        .unwrap();
    let relative = root.path().strip_prefix(&cwd).unwrap().join("data");
    assert!(!relative.is_absolute());
    let service = Service::open(relative.clone(), Arc::new(MemorySecrets::new())).unwrap();
    let input = csv(root.path(), "keys.csv", "key\n0001\n");
    let job = service.create_job(&input, Rule::default()).unwrap();
    assert_eq!(service.root, relative.join("matching"));
    assert_eq!(
        service.result_path(&job.id).unwrap(),
        cwd.join(relative)
            .join("matching")
            .join("jobs")
            .join(job.id)
            .join("result.txt")
    );
}
