use super::*;
use crate::secrets::MemorySecrets;
use std::{
    io::Read,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[derive(Clone, Copy, PartialEq)]
enum ServerMode {
    Ranges,
    Ignore,
    NoEtag,
    WeakEtag,
    Changed,
    WrongRange,
    ShortPart,
    HeadersSlow,
    BodySlow,
    Broken,
    Trickle,
    Huge,
}
struct Fixture {
    url: String,
    calls: Arc<Mutex<Vec<String>>>,
    peak: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn fixture(mode: ServerMode) -> Fixture {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/dat", listener.local_addr().unwrap());
    let calls = Arc::new(Mutex::new(Vec::new()));
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let (recorded, concurrent, high) = (calls.clone(), active.clone(), peak.clone());
    let task = tokio::spawn(async move {
        let mut children = tokio::task::JoinSet::new();
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (calls, active, peak) = (recorded.clone(), concurrent.clone(), high.clone());
            children.spawn(async move {
                let mut bytes = Vec::new();
                let mut buf = [0; 8192];
                loop {
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(at) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
                        let h = String::from_utf8_lossy(&bytes[..at]);
                        let len = h
                            .lines()
                            .find_map(|s| {
                                s.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|s| s.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if bytes.len() >= at + 4 + len {
                            break;
                        }
                    }
                }
                let request = String::from_utf8(bytes).unwrap();
                calls.lock().unwrap().push(request.clone());
                let running = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(running, Ordering::SeqCst);
                let total = 2 * 1024 * 1024 + 97;
                let range = request
                    .lines()
                    .find_map(|s| s.strip_prefix("range: bytes="))
                    .map(|s| {
                        let (start, end) = s.split_once('-').unwrap();
                        (
                            start.parse::<usize>().unwrap(),
                            end.parse::<usize>().unwrap(),
                        )
                    });
                let partial = range.is_some() && mode != ServerMode::Ignore;
                let (start, end) = if partial {
                    range.unwrap()
                } else {
                    (0, total - 1)
                };
                if range.is_some_and(|r| r != (0, 0)) {
                    tokio::time::sleep(Duration::from_millis(if start == 0 { 60 } else { 10 }))
                        .await;
                }
                if mode == ServerMode::HeadersSlow {
                    tokio::time::sleep(Duration::from_millis(1400)).await;
                }
                let mut body = (start..=end).map(|i| (i % 251) as u8).collect::<Vec<_>>();
                if mode == ServerMode::ShortPart && range.is_some_and(|r| r != (0, 0)) {
                    body.pop();
                }
                let declared = if mode == ServerMode::Huge {
                    99 * 1024 * 1024
                } else {
                    body.len()
                };
                let mut head = format!(
                    "HTTP/1.1 {}\r\nContent-Length: {declared}\r\nConnection: close\r\n",
                    if partial {
                        "206 Partial Content"
                    } else {
                        "200 OK"
                    }
                );
                if partial {
                    head.push_str(&format!(
                        "Content-Range: bytes {}-{end}/{total}\r\n",
                        if mode == ServerMode::WrongRange && end != 0 {
                            start + 1
                        } else {
                            start
                        }
                    ));
                    if mode != ServerMode::NoEtag {
                        head.push_str(if mode == ServerMode::WeakEtag {
                            "ETag: W/\"stable\"\r\n"
                        } else if mode == ServerMode::Changed && end != 0 {
                            "ETag: \"changed\"\r\n"
                        } else {
                            "ETag: \"stable\"\r\n"
                        });
                    }
                }
                head.push_str("\r\n");
                let _ = socket.write_all(head.as_bytes()).await;
                if mode == ServerMode::BodySlow {
                    tokio::time::sleep(Duration::from_millis(1400)).await;
                }
                if mode == ServerMode::Broken {
                    body.truncate(17);
                }
                if mode == ServerMode::Trickle {
                    for chunk in body.chunks(262144) {
                        let _ = socket.write_all(chunk).await;
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                } else {
                    let _ = socket.write_all(&body).await;
                }
                active.fetch_sub(1, Ordering::SeqCst);
            });
            while children.try_join_next().is_some() {}
        }
    });
    Fixture {
        url,
        calls,
        peak,
        task,
    }
}
fn configured(root: &std::path::Path, f: &Fixture, mode: DownloadMode) -> (Service, Settings) {
    let service = Service::open(root.into(), Arc::new(MemorySecrets::new())).unwrap();
    let cfg = Settings {
        url: f.url.clone(),
        allow_http_loopback: true,
        download_mode: mode,
        download_parallelism: 2,
        download_part_mib: 1,
        max_download_mib: 4,
        timeout_seconds: 1,
        download_timeout_seconds: 3,
        download_idle_seconds: 1,
        ..Default::default()
    };
    service.save_settings(&cfg).unwrap();
    (service, cfg)
}
async fn read(service: &Service, cfg: &Settings) -> Result<Vec<u8>, Error> {
    let mut wire = service.transfer(cfg, &cfg.source_url()?).await?;
    let mut output = Vec::new();
    wire.read_to_end(&mut output)?;
    Ok(output)
}
#[tokio::test]
async fn parallel_parts_are_ordered_version_pinned_and_bounded() {
    let f = fixture(ServerMode::Ranges).await;
    let root = tempfile::tempdir().unwrap();
    let (service, cfg) = configured(root.path(), &f, DownloadMode::Parallel);
    let (service, progress) = service.observed();
    let output = read(&service, &cfg).await.unwrap();
    assert_eq!(progress.borrow().completed, output.len() as u64);
    assert_eq!(progress.borrow().total, Some(output.len() as u64));
    assert_eq!(
        output,
        (0..2 * 1024 * 1024 + 97)
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>()
    );
    let calls = f.calls.lock().unwrap();
    assert_eq!(calls.len(), 4);
    assert!(calls[0].contains("range: bytes=0-0"));
    assert!(calls[1..]
        .iter()
        .all(|c| c.contains("if-match: \"stable\"")));
    assert_eq!(f.peak.load(Ordering::SeqCst), 2);
    assert_eq!(
        std::fs::read_dir(service.root.join("filters"))
            .unwrap()
            .count(),
        0
    );
}

#[tokio::test]
async fn delayed_headers_after_504_remain_visible_and_bytes_advance_during_body() {
    use crate::matching::Phase;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/dat", listener.local_addr().unwrap());
    let ready = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let (notify, gate) = (ready.clone(), release.clone());
    let server = tokio::spawn(async move {
        async fn read_headers(socket: &mut tokio::net::TcpStream) {
            let mut bytes = Vec::new();
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                let mut buffer = [0; 4096];
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
                assert!(bytes.len() <= 8192);
            }
        }
        let (mut socket, _) = listener.accept().await.unwrap();
        read_headers(&mut socket).await;
        socket.write_all(b"HTTP/1.1 504 Gateway Timeout\r\nRetry-After: 0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
        drop(socket);
        let (mut socket, _) = listener.accept().await.unwrap();
        read_headers(&mut socket).await;
        notify.notify_one();
        gate.notified().await;
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\n1234")
            .await
            .unwrap();
        gate.notified().await;
        socket.write_all(b"5678").await.unwrap();
    });
    let root = tempfile::tempdir().unwrap();
    let service = Service::open(root.path().into(), Arc::new(MemorySecrets::new())).unwrap();
    let (service, mut progress) = service.observed();
    let cfg = Settings {
        url,
        allow_http_loopback: true,
        download_timeout_seconds: 5,
        ..Default::default()
    };
    let transfer = tokio::spawn(async move { read(&service, &cfg).await });
    tokio::time::timeout(Duration::from_secs(2), ready.notified())
        .await
        .unwrap();
    let waiting = progress.borrow().clone();
    assert_eq!(waiting.phase, Phase::WaitingForDat);
    assert_eq!(waiting.completed, 0);
    assert_eq!(waiting.last_http, Some(504));
    assert_eq!(waiting.retry, 1);
    assert!(waiting
        .headline(waiting.phase_started + Duration::from_secs(45), true)
        .contains("45s · 2/3"));
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if progress.borrow().completed == 4 {
                break;
            }
            progress.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert!(!transfer.is_finished());
    assert_eq!(progress.borrow().phase, Phase::DownloadingDat);
    assert_eq!(progress.borrow().total, Some(8));
    release.notify_one();
    assert_eq!(transfer.await.unwrap().unwrap(), b"12345678");
    assert_eq!(progress.borrow().completed, 8);
    server.await.unwrap();
}
#[tokio::test]
async fn server_ignoring_range_uses_that_response_without_another_get() {
    let f = fixture(ServerMode::Ignore).await;
    let root = tempfile::tempdir().unwrap();
    let (service, cfg) = configured(root.path(), &f, DownloadMode::Parallel);
    assert_eq!(
        read(&service, &cfg).await.unwrap().len(),
        2 * 1024 * 1024 + 97
    );
    assert_eq!(f.calls.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn missing_etag_changed_versions_wrong_ranges_and_short_parts_fail_closed() {
    for (mode, message) in [
        (ServerMode::NoEtag, "strong ETag"),
        (ServerMode::WeakEtag, "strong ETag"),
        (ServerMode::Changed, "range/version"),
        (ServerMode::WrongRange, "range/version"),
        (ServerMode::ShortPart, "Content-Length"),
    ] {
        let f = fixture(mode).await;
        let root = tempfile::tempdir().unwrap();
        let (service, cfg) = configured(root.path(), &f, DownloadMode::Parallel);
        let error = read(&service, &cfg).await.unwrap_err().to_string();
        assert!(error.contains(message), "{error}");
        assert!(service.cached(None).unwrap().is_none());
    }
}
#[tokio::test]
async fn multipart_post_encodes_bounded_fields_and_keeps_auth_out_of_body() {
    let f = fixture(ServerMode::Ignore).await;
    let root = tempfile::tempdir().unwrap();
    let (service, mut cfg) = configured(root.path(), &f, DownloadMode::Multipart);
    cfg.download_form.insert("format".into(), "binary".into());
    cfg.download_form.insert("scope".into(), "a=b & c".into());
    assert!(read(&service, &cfg).await.is_ok());
    let calls = f.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].starts_with("POST /dat HTTP/1.1"));
    assert!(calls[0].contains("content-type: multipart/form-data; boundary=pgm-"));
    assert!(calls[0].contains("name=\"format\"\r\n\r\nbinary\r\n"));
    assert!(calls[0].contains("a=b & c"));
    assert!(!calls[0].contains("client_secret"));
}
#[tokio::test]
async fn slow_headers_can_exceed_oauth_budget_but_total_and_idle_limits_are_distinct() {
    for (mode, total, expected) in [
        (ServerMode::HeadersSlow, 3, None),
        (ServerMode::HeadersSlow, 1, Some("headers")),
        (ServerMode::BodySlow, 3, Some("idle")),
        (ServerMode::Trickle, 1, Some("deadline")),
        (ServerMode::Broken, 3, Some("interrupted")),
        (ServerMode::Huge, 3, Some("limit")),
    ] {
        let f = fixture(mode).await;
        let root = tempfile::tempdir().unwrap();
        let (service, mut cfg) = configured(root.path(), &f, DownloadMode::Single);
        cfg.download_timeout_seconds = total;
        cfg.download_retries = 0; // isolate each deadline/error from recovery
        match (read(&service, &cfg).await, expected) {
            (Ok(bytes), None) => assert_eq!(bytes.len(), 2 * 1024 * 1024 + 97),
            (Err(error), Some(message)) => assert!(error.to_string().contains(message), "{error}"),
            _ => panic!("Unexpected transport result"),
        }
    }
}
#[tokio::test]
async fn cancellation_drops_transfer_and_releases_source_lock() {
    let f = fixture(ServerMode::HeadersSlow).await;
    let root = tempfile::tempdir().unwrap();
    let (service, _) = configured(root.path(), &f, DownloadMode::Single);
    let (stop, rx) = tokio::sync::watch::channel(false);
    let cloned = service.clone();
    let task = tokio::spawn(async move { cloned.download(rx).await });
    while f.calls.lock().unwrap().is_empty() {
        tokio::task::yield_now().await;
    }
    stop.send(true).unwrap();
    let error = tokio::time::timeout(Duration::from_millis(500), task)
        .await
        .unwrap()
        .unwrap()
        .err()
        .unwrap();
    assert!(error.to_string().contains("cancelled"));
    assert!(crate::disk::acquire(&service.root.join("source.lock")).is_ok());
}
#[test]
fn settings_limits_nested_urls_and_form_injection_are_rejected() {
    for raw in [
        r#"{"download_parallelism":0}"#,
        r#"{"download_parallelism":17}"#,
        r#"{"download_part_mib":0}"#,
        r#"{"download_part_mib":129}"#,
        r#"{"download_timeout_seconds":0}"#,
        r#"{"download_idle_seconds":3601}"#,
        r#"{"download_form":{"bad\"name":"x"}}"#,
        r#"{"download_form":{"a":"header\r\ninjection"}}"#,
        r#"{"dat_url":"https://example.test/https://example.test"}"#,
        r#"{"dat_url":"https://example.test/https%3A%2F%2Fexample.test"}"#,
    ] {
        let cfg: Settings = serde_json::from_str(raw).unwrap();
        assert!(cfg.validate().is_err(), "{raw}");
    }
    let cfg: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(cfg.download_mode, DownloadMode::Single);
    assert_eq!(cfg.download_timeout_seconds, 600);
    let oversized = BTreeMap::from([("a".into(), "x".repeat(2049))]);
    assert!(validate_form(&oversized).is_err());
    for value in ["", "x", "unicode á", "---", "quotes \" ok"] {
        let form = BTreeMap::from([("format".into(), value.into())]);
        assert!(validate_form(&form).is_ok());
        let (content_type, body) = multipart(&form);
        let boundary = content_type.split_once("boundary=").unwrap().1;
        assert!(body.ends_with(&format!("--{boundary}--\r\n")));
    }
}
#[tokio::test]
async fn transient_get_retries_are_bounded_and_post_is_never_repeated() {
    for (mode, failures, retries, expected_calls, success, retry_after) in [
        (DownloadMode::Single, 1, 2, 2, true, None),
        (DownloadMode::Single, 10, 1, 2, false, None),
        (DownloadMode::Single, 1, 0, 1, false, None),
        (DownloadMode::Multipart, 1, 2, 1, false, None),
        (DownloadMode::Single, 1, 2, 1, false, Some("120")),
        (DownloadMode::Single, 1, 2, 1, false, Some("invalid")),
        (DownloadMode::Single, 1, 2, 2, true, Some("0")),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/dat", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let counted = hits.clone();
        let task = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 8192];
                let _ = socket.read(&mut request).await;
                let n = counted.fetch_add(1, Ordering::SeqCst);
                let mut head = if n < failures {
                    "HTTP/1.1 504 Gateway Timeout\r\n".to_string()
                } else {
                    "HTTP/1.1 200 OK\r\n".to_string()
                };
                if let Some(value) = retry_after {
                    head.push_str(&format!("Retry-After: {value}\r\n"));
                }
                head.push_str("Content-Length: 2\r\nConnection: close\r\n\r\nok");
                let _ = socket.write_all(head.as_bytes()).await;
            }
        });
        let root = tempfile::tempdir().unwrap();
        let service = Service::open(root.path().into(), Arc::new(MemorySecrets::new())).unwrap();
        let cfg = Settings {
            url,
            allow_http_loopback: true,
            download_mode: mode,
            download_retries: retries,
            ..Default::default()
        };
        assert_eq!(read(&service, &cfg).await.is_ok(), success);
        assert_eq!(hits.load(Ordering::SeqCst), expected_calls);
        task.abort();
    }
}
#[test]
fn legacy_long_request_deadline_is_preserved_and_new_limit_is_explicit() {
    let root = tempfile::tempdir().unwrap();
    let service = Service::open(root.path().into(), Arc::new(MemorySecrets::new())).unwrap();
    for (json, expected) in [
        (r#"{"timeout_seconds":900}"#, 900),
        (
            r#"{"timeout_seconds":900,"download_timeout_seconds":10}"#,
            10,
        ),
    ] {
        std::fs::write(service.root.join("settings.json"), json).unwrap();
        assert_eq!(
            service.settings().unwrap().download_timeout_seconds,
            expected
        );
    }
    let cfg = Settings {
        download_retries: 6,
        ..Default::default()
    };
    assert!(cfg.validate().is_err());
}
#[test]
fn range_probe_parser_mutation_harness_never_accepts_unknown_ranges_or_overflow() {
    for value in ["bytes 0-0/1", "bytes 0-0/4194304"] {
        assert!(probe_total(
            Some(&header::HeaderValue::from_str(value).unwrap()),
            4194304
        )
        .is_ok());
    }
    for value in [
        "bytes 0-0/0",
        "bytes 0-0/4194305",
        "bytes 0-1/100",
        "bytes 1-1/100",
        "bytes 0-0/+100",
        "bytes 0-0/*",
        "bytes 0-0/18446744073709551616",
        "bytes 0-0/10 extra",
    ] {
        assert!(probe_total(
            Some(&header::HeaderValue::from_str(value).unwrap()),
            4194304
        )
        .is_err());
    }
    assert!(probe_total(None, 4194304).is_err());
    let mut seed = 0xfeed_0123_4567_89abu64;
    for iteration in 0..4096 {
        let mut bytes = b"bytes 0-0/169768248".to_vec();
        for _ in 0..=iteration % 8 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let index = (seed as usize) % bytes.len();
            bytes[index] = (seed >> 32) as u8;
        }
        if let Ok(header) = header::HeaderValue::from_bytes(&bytes) {
            if let Ok(total) = probe_total(Some(&header), 200000000) {
                assert!((1..=200000000).contains(&total));
                assert!(header.to_str().unwrap().starts_with("bytes 0-0/"));
            }
        }
    }
}
