use flate2::{write::GzEncoder, Compression};
use pgm_data::{
    error::Error,
    matching::{format::Bloom, Rule, RunControl, Service, Snapshot, Transform},
    secrets::MemorySecrets,
};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/bloom")
        .join(name)
}
fn vectors() -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(fixture("provider-v1-vectors.json")).unwrap()).unwrap()
}
fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut gz = GzEncoder::new(Vec::new(), Compression::fast());
    gz.write_all(bytes).unwrap();
    gz.finish().unwrap()
}
fn reseal(bytes: &mut [u8]) {
    let n = bytes.len() - 32;
    let hash = Sha256::digest(&bytes[..n]);
    bytes[n..].copy_from_slice(&hash);
}

#[test]
fn independently_generated_raw_and_gzip_vectors_query_digest_without_rehashing() {
    let vectors = vectors();
    for name in ["provider-v1.dat", "provider-v1.dat.gz"] {
        let b = Bloom::read(&fixture(name), 4096).unwrap();
        assert_eq!(b.info().wire_format, "pgmbloom-provider-v1");
        assert_eq!(b.info().bits, 19176);
        assert_ne!(b.info().bits % 64, 0);
        assert_eq!(b.info().hashes, 13);
        for item in vectors["vectors"].as_array().unwrap() {
            let key = item["sha256"].as_str().unwrap();
            let raw = item["raw"].as_str().unwrap();
            let expected = item["expected"].as_bool().unwrap();
            assert_eq!(b.contains(key).unwrap(), expected);
            assert_eq!(b.contains(&key.to_ascii_uppercase()).unwrap(), expected);
            for transform in [Transform::Sha256, Transform::Sha256Raw] {
                assert_eq!(
                    b.contains_with_rule(
                        raw,
                        &Rule {
                            transform,
                            ..Default::default()
                        }
                    )
                    .unwrap(),
                    expected
                );
            }
            assert_eq!(
                b.contains_with_rule(
                    &format!(" {key} "),
                    &Rule {
                        trim: true,
                        ..Default::default()
                    }
                )
                .unwrap(),
                expected
            );
        }
        for invalid in [
            "",
            "52998224725",
            "not-a-digest",
            &"g".repeat(64),
            &"0".repeat(63),
            &"0".repeat(65),
        ] {
            assert!(b.contains(invalid).is_err());
        }
    }
}

#[test]
fn provider_header_payload_checksum_memory_and_trailing_fail_closed() {
    let raw = std::fs::read(fixture("provider-v1.dat")).unwrap();
    for cut in [0, 1, 9, 12, 63, 64, raw.len() - 33, raw.len() - 1] {
        assert!(Bloom::decode(&raw[..cut], 4096).is_err());
    }
    for at in [0, 8, 12, 16, 20, 24, 32, 40, 48, 56, 64, raw.len() - 1] {
        let mut corrupt = raw.clone();
        corrupt[at] ^= 0xff;
        assert!(Bloom::decode(corrupt.as_slice(), 4096).is_err());
    }
    let cases: [(usize, &[u8]); 12] = [
        (8, &2u32.to_be_bytes()),
        (12, &2u32.to_be_bytes()),
        (16, &0u32.to_be_bytes()),
        (16, &33u32.to_be_bytes()),
        (20, &1u32.to_be_bytes()),
        (24, &0u64.to_be_bytes()),
        (24, &19175u64.to_be_bytes()),
        (32, &0u64.to_be_bytes()),
        (40, &1001u64.to_be_bytes()),
        (48, &f64::NAN.to_be_bytes()),
        (48, &(-0.1f64).to_be_bytes()),
        (56, &1u64.to_be_bytes()),
    ];
    for (at, value) in cases {
        let mut corrupt = raw.clone();
        corrupt[at..at + value.len()].copy_from_slice(value);
        reseal(&mut corrupt);
        assert!(
            Bloom::decode(corrupt.as_slice(), 4096).is_err(),
            "offset {at}"
        );
    }
    for p in [0.0f64, 0.0011, 0.000000001] {
        let mut corrupt = raw.clone();
        corrupt[48..56].copy_from_slice(&p.to_be_bytes());
        reseal(&mut corrupt);
        assert!(Bloom::decode(corrupt.as_slice(), 4096).is_err());
    }
    assert!(Bloom::decode(raw.as_slice(), 2396).is_err());
    let mut extra = raw.clone();
    extra.push(0);
    assert!(Bloom::decode(extra.as_slice(), 4096).is_err());
    let good = gzip(&raw);
    let mut crc = good.clone();
    let at = crc.len() - 8;
    crc[at] ^= 1;
    assert!(Bloom::decode(crc.as_slice(), 4096).is_err());
    let mut members = good.clone();
    members.extend(&good);
    assert!(Bloom::decode(members.as_slice(), 4096).is_err());
    assert!(Bloom::decode(gzip(&extra).as_slice(), 4096).is_err());
    let dir = tempfile::tempdir().unwrap();
    let mut b = Bloom::read(&fixture("provider-v1.dat"), 4096).unwrap();
    assert!(b.insert("x").is_err());
    assert!(b.write(&dir.path().join("native.dat")).is_err());
}

#[test]
fn provider_bounded_mutation_fuzz_harness_never_panics_or_exceeds_budget() {
    let raw = std::fs::read(fixture("provider-v1.dat")).unwrap();
    let mut seed = 17u64;
    for i in 0..4000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut candidate = raw.clone();
        let at = (seed as usize) % candidate.len();
        candidate[at] ^= (i % 255 + 1) as u8;
        if i % 3 == 0 {
            reseal(&mut candidate);
        }
        if i % 5 == 0 {
            candidate.truncate(seed as usize % candidate.len());
        }
        let _ = Bloom::decode(candidate.as_slice(), 4096);
        let _ = Bloom::decode(gzip(&candidate).as_slice(), 4096);
        let _ = Bloom::decode(seed.to_ne_bytes().as_slice(), 4096);
    }
}

#[tokio::test]
async fn provider_matching_preserves_original_keys_rules_order_and_resume() {
    for transform in [Transform::None, Transform::Sha256, Transform::Sha256Raw] {
        let root = tempfile::tempdir().unwrap();
        let service = Service::open(root.path().into(), Arc::new(MemorySecrets::new())).unwrap();
        let values = vectors();
        let rows = values["vectors"].as_array().unwrap();
        let order = [0, 4, 1, 0, 2, 3];
        let column = if transform == Transform::None {
            "sha256"
        } else {
            "raw"
        };
        let input = root.path().join("input.csv");
        let mut csv = csv::Writer::from_path(&input).unwrap();
        csv.write_record(["key"]).unwrap();
        for i in order {
            csv.write_record([rows[i][column].as_str().unwrap()])
                .unwrap();
        }
        csv.flush().unwrap();
        let rule = Rule {
            column: "key".into(),
            transform,
            ..Default::default()
        };
        let job = service.create_job(&input, rule).unwrap();
        let b = Bloom::read(&fixture("provider-v1.dat"), 4096).unwrap();
        let snapshot = Snapshot { bloom: Arc::new(b) };
        service.retain(snapshot.clone()).unwrap();
        let mut control = RunControl::default();
        control.pause_after = Some(2);
        let paused = service
            .run_job(&job.id, Some(snapshot), control)
            .await
            .unwrap();
        assert_eq!(paused.state, "PAUSED");
        let done = service
            .run_job(&job.id, None, RunControl::default())
            .await
            .unwrap();
        assert_eq!(done.positives, 5);
        assert_eq!(done.negatives, 1);
        assert!(service.verify(&job.id).unwrap());
        let result = std::fs::read_to_string(service.result_path(&job.id).unwrap()).unwrap();
        let expected = order
            .into_iter()
            .map(|i| {
                let key = rows[i][column].as_str().unwrap();
                let key = if key.contains(':') {
                    serde_json::to_string(key).unwrap()
                } else {
                    key.into()
                };
                format!("{key}: {}\n", rows[i]["expected"])
            })
            .collect::<String>();
        assert_eq!(result, expected);
    }
}

#[tokio::test]
async fn raw_or_gzip_provider_downloads_are_verified_cached_and_corruption_is_not_published() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let raw = std::fs::read(fixture("provider-v1.dat")).unwrap();
    let mut corrupt = raw.clone();
    corrupt[70] ^= 1;
    for (bytes, valid) in [(raw.clone(), true), (gzip(&raw), true), (corrupt, false)] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0; 1024];
            while !request.windows(4).any(|b| b == b"\r\n\r\n") {
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&chunk[..n]);
            }
            assert!(request.starts_with(b"GET /filter.dat HTTP/1.1"));
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            socket.write_all(&bytes).await.unwrap();
        });
        let root = tempfile::tempdir().unwrap();
        let service = Service::open(root.path().into(), Arc::new(MemorySecrets::new())).unwrap();
        service
            .save_settings(&pgm_data::matching::Settings {
                url: format!("http://{address}/filter.dat"),
                allow_http_loopback: true,
                ..Default::default()
            })
            .unwrap();
        let loaded = service.download(RunControl::default().stop).await;
        if valid {
            let loaded = loaded.unwrap();
            let cached = service.cached(None).unwrap().unwrap();
            assert_eq!(loaded.bloom.info().sha256, cached.bloom.info().sha256);
            assert_eq!(cached.bloom.info().wire_format, "pgmbloom-provider-v1");
            assert!(cached
                .bloom
                .contains(vectors()["vectors"][0]["sha256"].as_str().unwrap())
                .unwrap());
        } else {
            assert!(loaded.is_err());
            assert!(!service.root.join("current.json").exists());
        }
        server.await.unwrap();
    }
}

#[tokio::test]
async fn malformed_digest_stops_job_without_publishing_false_results() {
    let root = tempfile::tempdir().unwrap();
    let service = Service::open(root.path().into(), Arc::new(MemorySecrets::new())).unwrap();
    let input = root.path().join("bad.csv");
    std::fs::write(&input, "key\nnot-a-sha256-digest\n").unwrap();
    let job = service
        .create_job(
            &input,
            Rule {
                column: "key".into(),
                ..Default::default()
            },
        )
        .unwrap();
    let snapshot = Snapshot {
        bloom: Arc::new(Bloom::read(&fixture("provider-v1.dat"), 4096).unwrap()),
    };
    let result: Result<_, Error> = service
        .run_job(&job.id, Some(snapshot), RunControl::default())
        .await;
    assert!(result.is_err());
    assert_ne!(service.job(&job.id).unwrap().state, "COMPLETED");
    assert!(!service.result_path(&job.id).unwrap().exists());
}
