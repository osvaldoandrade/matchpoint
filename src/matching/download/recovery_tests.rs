use super::*;
use crate::{
    matching::{crypto::DatKey, Rule, RunControl, Transform},
    secrets::MemorySecrets,
};
use base64::Engine;
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

#[derive(Clone, Copy, Default, PartialEq)]
enum Reply {
    #[default]
    Valid,
    ChangedTag,
    WrongRange,
    WrongTotal,
    Full,
}
struct Plan {
    bytes: Vec<u8>,
    breaks: usize,
    tag: Option<&'static str>,
    reply: Reply,
    second_status: Option<(u16, &'static str)>,
    idle_first: bool,
}
impl Default for Plan {
    fn default() -> Self {
        Self {
            bytes: (0..2 * 1024 * 1024 + 97).map(|i| (i % 251) as u8).collect(),
            breaks: 1,
            tag: Some("\"version-1\""),
            reply: Reply::Valid,
            second_status: None,
            idle_first: false,
        }
    }
}
struct Fixture {
    url: String,
    calls: Arc<Mutex<Vec<String>>>,
    cut: Arc<tokio::sync::Notify>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn fixture(plan: Plan) -> Fixture {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/dat", listener.local_addr().unwrap());
    let calls = Arc::new(Mutex::new(Vec::new()));
    let recorded = calls.clone();
    let cut = Arc::new(tokio::sync::Notify::new());
    let notify = cut.clone();
    let task = tokio::spawn(async move {
        let plan = Arc::new(plan);
        let remaining = Arc::new(AtomicUsize::new(plan.breaks));
        let mut children = tokio::task::JoinSet::new();
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (plan, remaining, calls, cut) = (
                plan.clone(),
                remaining.clone(),
                recorded.clone(),
                notify.clone(),
            );
            children.spawn(async move {
                let mut bytes=Vec::new();
                loop {
                    let mut buffer=[0;8192];
                    let n=socket.read(&mut buffer).await.unwrap_or(0);
                    if n==0 {return;}
                    bytes.extend_from_slice(&buffer[..n]);
                    if let Some(at)=bytes.windows(4).position(|w|w==b"\r\n\r\n") {
                        let headers=String::from_utf8_lossy(&bytes[..at]);
                        let len=headers.lines().find_map(|s|s.to_ascii_lowercase().strip_prefix("content-length:").and_then(|s|s.trim().parse::<usize>().ok())).unwrap_or(0);
                        if bytes.len()>=at+4+len {break;}
                    }
                }
                let request=String::from_utf8(bytes).unwrap();
                let number={let mut calls=calls.lock().unwrap();calls.push(request.clone());calls.len()};
                if number==2 {
                    if let Some((status,retry_after))=plan.second_status {
                        let response=format!("HTTP/1.1 {status} Unavailable\r\nRetry-After: {retry_after}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                        let _=socket.write_all(response.as_bytes()).await;
                        return;
                    }
                }
                let range=request.lines().find_map(|s|s.strip_prefix("range: bytes=")).map(|v| {
                    let (a,b)=v.split_once('-').unwrap();(a.parse::<usize>().unwrap(),b.parse::<usize>().unwrap())
                });
                let total=plan.bytes.len();
                let partial=range.is_some() && plan.reply!=Reply::Full;
                let (start,end)=if partial {range.unwrap()}else{(0,total-1)};
                let mut head=format!("HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\nAccept-Ranges: bytes\r\n",if partial {"206 Partial Content"}else{"200 OK"},end-start+1);
                if partial {
                    head.push_str(&format!("Content-Range: bytes {}-{end}/{}\r\n",if plan.reply==Reply::WrongRange {start+1}else{start},if plan.reply==Reply::WrongTotal {total+1}else{total}));
                }
                if let Some(tag)=plan.tag {
                    head.push_str(&format!("ETag: {}\r\n",if number>1 && plan.reply==Reply::ChangedTag {"\"version-2\""}else{tag}));
                }
                head.push_str("\r\n");
                let _=socket.write_all(head.as_bytes()).await;
                let broken=(start!=end) && remaining.fetch_update(Ordering::SeqCst,Ordering::SeqCst,|n|n.checked_sub(1)).is_ok();
                let count=if broken {(end-start+1).min(33013).min(total-1)}else{end-start+1};
                let _=socket.write_all(&plan.bytes[start..start+count]).await;
                if broken {
                    cut.notify_one();
                    if plan.idle_first {tokio::time::sleep(Duration::from_secs(2)).await;}
                }
                // Closing before declared Content-Length reproduces the real body error.
            });
            while children.try_join_next().is_some() {}
        }
    });
    Fixture {
        url,
        calls,
        cut,
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
        download_timeout_seconds: 5,
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
async fn interrupted_get_resumes_from_exact_unaligned_offset_instead_of_restarting() {
    let plan = Plan::default();
    let expected = plan.bytes.clone();
    let f = fixture(plan).await;
    let root = tempfile::tempdir().unwrap();
    let (service, cfg) = configured(root.path(), &f, DownloadMode::Single);
    let (service, progress) = service.observed();
    assert_eq!(read(&service, &cfg).await.unwrap(), expected);
    assert_eq!(progress.borrow().completed, expected.len() as u64);
    assert_eq!(progress.borrow().total, Some(expected.len() as u64));
    let calls = f.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert!(calls[1].contains("range: bytes=33013-2097248"));
    assert!(calls[1].contains("if-match: \"version-1\""));
}
#[tokio::test]
async fn missing_or_weak_validator_restarts_full_get_without_mixing_prefixes() {
    for tag in [None, Some("W/\"weak\"")] {
        let plan = Plan {
            tag,
            ..Default::default()
        };
        let expected = plan.bytes.clone();
        let f = fixture(plan).await;
        let root = tempfile::tempdir().unwrap();
        let (service, cfg) = configured(root.path(), &f, DownloadMode::Single);
        let (service, progress) = service.observed();
        assert_eq!(read(&service, &cfg).await.unwrap(), expected);
        assert_eq!(progress.borrow().completed, expected.len() as u64);
        let calls = f.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert!(!calls[1].contains("range:"));
    }
}
#[tokio::test]
async fn changed_tag_range_or_total_never_reaches_matching() {
    for reply in [Reply::ChangedTag, Reply::WrongRange, Reply::WrongTotal] {
        let f = fixture(Plan {
            reply,
            ..Default::default()
        })
        .await;
        let root = tempfile::tempdir().unwrap();
        let (service, cfg) = configured(root.path(), &f, DownloadMode::Single);
        assert!(read(&service, &cfg)
            .await
            .unwrap_err()
            .to_string()
            .contains("range/version"));
        assert_eq!(f.calls.lock().unwrap().len(), 2);
        assert!(service.cached(None).unwrap().is_none());
    }
}
#[tokio::test]
async fn ignored_recovery_range_replaces_prefix_with_one_full_response() {
    let plan = Plan {
        reply: Reply::Full,
        ..Default::default()
    };
    let expected = plan.bytes.clone();
    let f = fixture(plan).await;
    let root = tempfile::tempdir().unwrap();
    let (service, cfg) = configured(root.path(), &f, DownloadMode::Single);
    let (service, progress) = service.observed();
    assert_eq!(read(&service, &cfg).await.unwrap(), expected);
    assert_eq!(progress.borrow().completed, expected.len() as u64);
    assert_eq!(f.calls.lock().unwrap().len(), 2);
}
#[tokio::test]
async fn repeated_interruptions_and_http_failures_share_one_retry_budget() {
    for (breaks, status, success, hits) in [
        (2, None, true, 3),
        (3, None, false, 3),
        (1, Some((503, "0")), true, 3),
        (1, Some((429, "0")), true, 3),
    ] {
        let plan = Plan {
            breaks,
            second_status: status,
            ..Default::default()
        };
        let expected = plan.bytes.clone();
        let f = fixture(plan).await;
        let root = tempfile::tempdir().unwrap();
        let (service, cfg) = configured(root.path(), &f, DownloadMode::Single);
        let result = read(&service, &cfg).await;
        if success {
            assert_eq!(result.unwrap(), expected);
        } else {
            assert!(result.unwrap_err().to_string().contains("interrupted"));
        }
        assert_eq!(f.calls.lock().unwrap().len(), hits);
    }
}
#[tokio::test]
async fn interrupted_multipart_is_not_replayed() {
    let f = fixture(Plan::default()).await;
    let root = tempfile::tempdir().unwrap();
    let (service, cfg) = configured(root.path(), &f, DownloadMode::Multipart);
    assert!(read(&service, &cfg).await.is_err());
    assert_eq!(f.calls.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn idle_body_is_recovered_within_parent_deadline_and_cancellation_is_prompt() {
    let plan = Plan {
        idle_first: true,
        ..Default::default()
    };
    let expected = plan.bytes.clone();
    let f = fixture(plan).await;
    let root = tempfile::tempdir().unwrap();
    let (service, cfg) = configured(root.path(), &f, DownloadMode::Single);
    assert_eq!(read(&service, &cfg).await.unwrap(), expected);
    let f = fixture(Plan {
        idle_first: true,
        ..Default::default()
    })
    .await;
    let root = tempfile::tempdir().unwrap();
    let (service, _) = configured(root.path(), &f, DownloadMode::Single);
    let (stop, rx) = tokio::sync::watch::channel(false);
    let cloned = service.clone();
    let task = tokio::spawn(async move { cloned.download(rx).await });
    f.cut.notified().await;
    stop.send(true).unwrap();
    assert!(tokio::time::timeout(Duration::from_millis(500), task)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    assert!(crate::disk::acquire(&service.root.join("source.lock")).is_ok());
}
#[tokio::test]
async fn encrypted_recovered_dat_matches_locally_and_releases_without_disk_staging() {
    let f = fixture(Plan {
        bytes: include_bytes!("../../../tests/fixtures/bloom/provider-raw-gcm.enc").to_vec(),
        ..Default::default()
    })
    .await;
    let root = tempfile::tempdir().unwrap();
    let (service, _) = configured(root.path(), &f, DownloadMode::Single);
    let key = base64::engine::general_purpose::STANDARD.encode([0x42; 32]);
    service
        .set_dat_key(DatKey::from_base64(&key).unwrap())
        .unwrap();
    let input = root.path().join("keys.csv");
    std::fs::write(&input, "key\n00012345678\nabsent\n00012345678\n").unwrap();
    let job = service
        .create_job(
            &input,
            Rule {
                transform: Transform::Sha256,
                ..Default::default()
            },
        )
        .unwrap();
    let done = service
        .run_job(&job.id, None, RunControl::default())
        .await
        .unwrap();
    assert_eq!((done.positives, done.negatives), (2, 1));
    assert_eq!(
        std::fs::read_to_string(service.result_path(&job.id).unwrap()).unwrap(),
        "00012345678: true\nabsent: false\n00012345678: true\n"
    );
    assert_eq!(service.jobs().unwrap().len(), 1);
    assert!(service.cached(None).unwrap().is_none());
    assert_eq!(
        std::fs::read_dir(service.root.join("filters"))
            .unwrap()
            .count(),
        0
    );
}
#[tokio::test]
async fn parallel_parts_resume_and_share_a_global_attempt_limit() {
    let plan = Plan::default();
    let expected = plan.bytes.clone();
    let f = fixture(plan).await;
    let root = tempfile::tempdir().unwrap();
    let (service, cfg) = configured(root.path(), &f, DownloadMode::Parallel);
    assert_eq!(read(&service, &cfg).await.unwrap(), expected);
    assert_eq!(f.calls.lock().unwrap().len(), 5); // probe + 3 parts + 1 recovery
    for _ in 0..3 {
        let f = fixture(Plan {
            breaks: 100,
            ..Default::default()
        })
        .await;
        let root = tempfile::tempdir().unwrap();
        let (service, cfg) = configured(root.path(), &f, DownloadMode::Parallel);
        let error = tokio::time::timeout(Duration::from_secs(3), read(&service, &cfg))
            .await
            .unwrap()
            .unwrap_err();
        assert!(error.to_string().contains("interrupted"));
        assert!(
            f.calls.lock().unwrap().len() <= 5,
            "retry limit multiplied by concurrency"
        );
    }
}
#[tokio::test]
async fn recovery_cannot_extend_total_deadline_or_publish_corrupt_encrypted_data() {
    let f = fixture(Plan {
        idle_first: true,
        ..Default::default()
    })
    .await;
    let root = tempfile::tempdir().unwrap();
    let (service, mut cfg) = configured(root.path(), &f, DownloadMode::Single);
    cfg.download_timeout_seconds = 1;
    assert!(read(&service, &cfg)
        .await
        .unwrap_err()
        .to_string()
        .contains("deadline"));
    assert_eq!(f.calls.lock().unwrap().len(), 1);
    let mut bytes = include_bytes!("../../../tests/fixtures/bloom/provider-raw-gcm.enc").to_vec();
    *bytes.last_mut().unwrap() ^= 1;
    let f = fixture(Plan {
        bytes,
        ..Default::default()
    })
    .await;
    let root = tempfile::tempdir().unwrap();
    let (service, _) = configured(root.path(), &f, DownloadMode::Single);
    let encoded = base64::engine::general_purpose::STANDARD.encode([0x42; 32]);
    service
        .set_dat_key(DatKey::from_base64(&encoded).unwrap())
        .unwrap();
    let input = root.path().join("keys.csv");
    std::fs::write(&input, "key\n00012345678\n").unwrap();
    let job = service
        .create_job(
            &input,
            Rule {
                transform: Transform::Sha256,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(service
        .run_job(&job.id, None, RunControl::default())
        .await
        .unwrap_err()
        .to_string()
        .contains("checksum"));
    assert!(!service
        .root
        .join("jobs")
        .join(job.id)
        .join("result.txt")
        .exists());
    assert!(service.cached(None).unwrap().is_none());
}
