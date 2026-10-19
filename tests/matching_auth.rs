use pgm_data::{
    error::Secret,
    matching::{Rule, RunControl, Service, Settings},
    secrets::MemorySecrets,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
#[derive(Clone, Debug)]
struct Call {
    method: String,
    path: String,
    headers: String,
    body: serde_json::Value,
}
#[derive(Default)]
struct Plan {
    dat_body: Option<Vec<u8>>,
    reject_refresh: bool,
    dat_401: u32,
    token_status: Option<&'static str>,
    token_body: Option<Vec<u8>>,
    delay: Duration,
    expires_in: u64,
}
struct Fixture {
    base: String,
    calls: Arc<Mutex<Vec<Call>>>,
    plan: Arc<Mutex<Plan>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn fixture() -> Fixture {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let calls = Arc::new(Mutex::new(Vec::new()));
    let plan = Arc::new(Mutex::new(Plan::default()));
    let recorded = calls.clone();
    let shared = plan.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            let mut content_start = None;
            let mut length = 0;
            loop {
                let n = socket.read(&mut buffer).await.unwrap();
                if n == 0 {
                    break;
                }
                bytes.extend(&buffer[..n]);
                if content_start.is_none() {
                    if let Some(at) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                        content_start = Some(at + 4);
                        let headers = String::from_utf8_lossy(&bytes[..at]);
                        length = headers
                            .lines()
                            .find_map(|s| {
                                s.to_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|v| v.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                    }
                }
                if content_start.is_some_and(|at| bytes.len() >= at + length) {
                    break;
                }
            }
            let at = content_start.unwrap();
            let headers = String::from_utf8_lossy(&bytes[..at]).into_owned();
            let mut first = headers.lines().next().unwrap().split_whitespace();
            let method = first.next().unwrap().to_string();
            let path = first.next().unwrap().to_string();
            let body =
                serde_json::from_slice(&bytes[at..at + length]).unwrap_or(serde_json::Value::Null);
            recorded.lock().unwrap().push(Call {
                method,
                path: path.clone(),
                headers,
                body: body.clone(),
            });
            let count = recorded.lock().unwrap().len();
            let (status, body, delay) = {
                let mut plan = shared.lock().unwrap();
                if path == "/filter.dat" {
                    if plan.dat_401 > 0 {
                        plan.dat_401 -= 1;
                        ("401 Unauthorized", vec![], Duration::ZERO)
                    } else {
                        (
                            "200 OK",
                            plan.dat_body.clone().unwrap_or_else(|| {
                                include_bytes!("fixtures/bloom/python-v1-0.dat").to_vec()
                            }),
                            Duration::ZERO,
                        )
                    }
                } else if body.get("grant_type").and_then(|v| v.as_str()) == Some("refresh_token")
                    && plan.reject_refresh
                {
                    (
                        "400 Bad Request",
                        br#"{"error":"invalid_grant","detail":"do-not-echo-sensitive-body"}"#
                            .to_vec(),
                        plan.delay,
                    )
                } else if let Some(body) = &plan.token_body {
                    (
                        plan.token_status.unwrap_or("200 OK"),
                        body.clone(),
                        plan.delay,
                    )
                } else {
                    ("200 OK",serde_json::to_vec(&serde_json::json!({"access_token":format!("synthetic-access-{count}"),"refresh_token":format!("synthetic-refresh-{count}"),"token_type":"Bearer","expires_in":if plan.expires_in==0{3600}else{plan.expires_in}})).unwrap(),plan.delay)
                }
            };
            tokio::time::sleep(delay).await;
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = socket.write_all(head.as_bytes()).await;
            let _ = socket.write_all(&body).await;
        }
    });
    Fixture {
        base,
        calls,
        plan,
        task,
    }
}
fn configured(root: &std::path::Path, fixture: &Fixture) -> Service {
    let service = Service::open(root.to_path_buf(), Arc::new(MemorySecrets::new())).unwrap();
    service
        .save_settings(&Settings {
            url: format!("{}/filter.dat", fixture.base),
            token_url: format!("{}/issue", fixture.base),
            reset_token_url: format!("{}/legacy-unused", fixture.base),
            client_id: "synthetic-client".into(),
            scope: "partners.read".into(),
            oauth: true,
            allow_http_loopback: true,
            ..Settings::default()
        })
        .unwrap();
    service
        .save_oauth_credentials(
            Some(&Secret::new("synthetic-client-secret".into()).unwrap()),
            None,
        )
        .unwrap();
    service
}
#[tokio::test]
async fn acquisition_needs_only_token_url_id_and_secret_but_other_operations_need_their_urls() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = Service::open(root.path().into(), Arc::new(MemorySecrets::new())).unwrap();
    service
        .save_settings(&Settings {
            token_url: format!("{}/issue", fixture.base),
            client_id: "synthetic-client".into(),
            oauth: true,
            allow_http_loopback: true,
            ..Default::default()
        })
        .unwrap();
    service
        .save_oauth_credentials(
            Some(&Secret::new("synthetic-client-secret".into()).unwrap()),
            None,
        )
        .unwrap();
    assert!(service.obtain_token().await.unwrap().expires_in > 0);
    service.refresh_token().await.unwrap();
    assert!(service.download(RunControl::default().stop).await.is_err());
    let calls = fixture.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].path, "/issue");
    assert_eq!(calls[1].path, "/issue");
    assert_eq!(calls[0].body["grant_type"], "client_credentials");
    assert_eq!(calls[1].body["grant_type"], "refresh_token");
}
#[tokio::test]
async fn forgetting_access_discards_cached_bearer_without_revocation_or_deleting_credentials() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = configured(root.path(), &fixture);
    service.download(RunControl::default().stop).await.unwrap();
    service.forget_access().unwrap();
    service.download(RunControl::default().stop).await.unwrap();
    let calls = fixture.calls.lock().unwrap();
    assert_eq!(
        calls.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
        vec!["/issue", "/filter.dat", "/issue", "/filter.dat"]
    );
    assert!(calls[1].headers.contains("synthetic-access-1"));
    assert!(calls[3].headers.contains("synthetic-access-3"));
}
#[tokio::test]
async fn failed_fresh_login_does_not_replace_previously_valid_credentials() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = configured(root.path(), &fixture);
    service.obtain_token().await.unwrap();
    fixture.plan.lock().unwrap().token_status = Some("401 Unauthorized");
    fixture.plan.lock().unwrap().token_body = Some(b"{}".to_vec());
    assert!(service
        .sign_in(Secret::new("incorrect-fresh-secret".into()).unwrap())
        .await
        .is_err());
    {
        let mut plan = fixture.plan.lock().unwrap();
        plan.token_status = None;
        plan.token_body = None;
    }
    service.obtain_token().await.unwrap();
    let calls = fixture.calls.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[1].body["client_secret"], "incorrect-fresh-secret");
    assert_eq!(calls[2].body["client_secret"], "synthetic-client-secret");
}
#[tokio::test]
async fn home_session_fresh_jobs_refresh_rotation_and_invalid_grant_never_touch_keychain() {
    struct Forbidden;
    impl pgm_data::secrets::SecretStore for Forbidden {
        fn get(&self, _: &str) -> Result<Option<Secret>, pgm_data::error::Error> {
            panic!("Keychain read")
        }
        fn put(&self, _: &str, _: &Secret) -> Result<(), pgm_data::error::Error> {
            panic!("Keychain write")
        }
        fn delete(&self, _: &str) -> Result<(), pgm_data::error::Error> {
            panic!("Keychain delete")
        }
    }
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let cfg = configured(root.path(), &fixture).settings().unwrap();
    let service = Service::open(root.path().into(), Arc::new(Forbidden)).unwrap();
    service.save_settings(&cfg).unwrap();
    service
        .sign_in(Secret::new("session-only".into()).unwrap())
        .await
        .unwrap();
    fixture.plan.lock().unwrap().dat_401 = 1;
    service.download(RunControl::default().stop).await.unwrap();
    service.clear_filters().unwrap();
    service.refresh_token().await.unwrap();
    fixture.plan.lock().unwrap().reject_refresh = true;
    service.refresh_token().await.unwrap();
    fixture.plan.lock().unwrap().reject_refresh = false;
    service.refresh_token().await.unwrap();
    service.download(RunControl::default().stop).await.unwrap();
    service.clear_filters().unwrap();
    service.forget_access().unwrap();
    fixture.plan.lock().unwrap().token_body = Some(
        serde_json::to_vec(&serde_json::json!({
            "access_token":"session-no-refresh", "token_type":"Bearer", "expires_in":3600
        }))
        .unwrap(),
    );
    service
        .sign_in(Secret::new("session-only".into()).unwrap())
        .await
        .unwrap();
    assert!(service
        .refresh_token()
        .await
        .unwrap_err()
        .to_string()
        .contains("No refresh token"));
    service.obtain_token().await.unwrap();
    service.forget_access().unwrap();
    let calls = fixture.calls.lock().unwrap();
    assert_eq!(calls.iter().filter(|c| c.path == "/filter.dat").count(), 3);
    assert!(calls
        .iter()
        .filter(|c| c.path == "/issue")
        .all(|c| c.body["client_secret"] == "session-only"));
    let grants = calls
        .iter()
        .filter(|c| c.path == "/issue")
        .map(|c| c.body["grant_type"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        grants,
        [
            "client_credentials",
            "client_credentials",
            "refresh_token",
            "refresh_token",
            "refresh_token",
            "client_credentials",
            "refresh_token",
            "client_credentials",
            "client_credentials",
            "client_credentials"
        ]
    );
}
#[tokio::test]
async fn successful_http_without_usable_access_token_never_completes_sign_in() {
    for body in [
        serde_json::json!({}),
        serde_json::json!({"token_type":"Bearer","expires_in":3600}),
        serde_json::json!({"access_token":"","token_type":"Bearer","expires_in":3600}),
        serde_json::json!({"access_token":"   ","token_type":"Bearer","expires_in":3600}),
        serde_json::json!({"access_token":"synthetic","token_type":"Basic","expires_in":3600}),
        serde_json::json!({"access_token":"synthetic","token_type":"Bearer","expires_in":0}),
    ] {
        let root = tempfile::tempdir().unwrap();
        let fixture = fixture().await;
        let service = configured(root.path(), &fixture);
        fixture.plan.lock().unwrap().token_body = Some(serde_json::to_vec(&body).unwrap());
        assert!(service
            .sign_in(Secret::new("fresh-input".into()).unwrap())
            .await
            .is_err());
        assert_eq!(fixture.calls.lock().unwrap().len(), 1);
        assert!(!service.root.join("current.json").exists());
    }
}
#[tokio::test]
async fn historical_refresh_destination_is_ignored_for_routing_and_new_settings() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = Service::open(root.path().into(), Arc::new(MemorySecrets::new())).unwrap();
    let cfg = serde_json::json!({"token_url":format!("{}/issue",fixture.base),"reset_token_url":format!("{}/must-never-be-called",fixture.base),"client_id":"fixture","oauth":true,"allow_http_loopback":true});
    std::fs::write(
        service.root.join("settings.json"),
        serde_json::to_vec(&cfg).unwrap(),
    )
    .unwrap();
    service
        .sign_in(Secret::new("fresh-input".into()).unwrap())
        .await
        .unwrap();
    service.refresh_token().await.unwrap();
    let calls = fixture.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|c| c.path == "/issue"));
    assert_eq!(calls[1].body["grant_type"], "refresh_token");
    drop(calls);
    service.save_settings(&service.settings().unwrap()).unwrap();
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(service.root.join("settings.json")).unwrap())
            .unwrap();
    assert!(saved.get("reset_token_url").is_none());
}
#[tokio::test]
async fn common_token_endpoint_rotates_refresh_and_keeps_matching_local() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = configured(root.path(), &fixture);
    service.obtain_token().await.unwrap();
    let filter = service.download(RunControl::default().stop).await.unwrap();
    service.refresh_token().await.unwrap();
    service.refresh_token().await.unwrap();
    let calls = fixture.calls.lock().unwrap().clone();
    assert_eq!(
        calls
            .iter()
            .map(|c| (c.method.as_str(), c.path.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("POST", "/issue"),
            ("POST", "/issue"),
            ("GET", "/filter.dat"),
            ("POST", "/issue"),
            ("POST", "/issue")
        ]
    );
    assert_eq!(calls[0].body["grant_type"], "client_credentials");
    assert_eq!(calls[0].body["client_id"], "synthetic-client");
    assert_eq!(calls[0].body["scope"], "partners.read");
    assert_eq!(calls[0].body["client_secret"], "synthetic-client-secret");
    assert!(calls[2]
        .headers
        .to_lowercase()
        .contains("authorization: bearer synthetic-access-2"));
    assert_eq!(calls[3].body["grant_type"], "refresh_token");
    assert_eq!(calls[3].body["refresh_token"], "synthetic-refresh-2");
    assert_eq!(calls[4].body["refresh_token"], "synthetic-refresh-4");
    drop(fixture);
    let input = root.path().join("input.csv");
    std::fs::write(&input, "key\n00012345678\nnot-member\n").unwrap();
    let job = service.create_job(&input, Rule::default()).unwrap();
    let done = service
        .run_job(&job.id, Some(filter), RunControl::default())
        .await
        .unwrap();
    assert_eq!(done.processed, 2);
    let json = std::fs::read_to_string(service.root.join("settings.json")).unwrap();
    for secret in [
        "synthetic-client-secret",
        "synthetic-access",
        "synthetic-refresh",
    ] {
        assert!(!json.contains(secret));
    }
}

#[tokio::test]
async fn service_clones_obtain_fresh_download_tokens_and_omit_empty_scope() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = configured(root.path(), &fixture);
    let mut cfg = service.settings().unwrap();
    cfg.scope.clear();
    service.save_settings(&cfg).unwrap();
    service
        .save_oauth_credentials(
            Some(&Secret::new("synthetic-client-secret".into()).unwrap()),
            None,
        )
        .unwrap();
    service.download(RunControl::default().stop).await.unwrap();
    service
        .clone()
        .download(RunControl::default().stop)
        .await
        .unwrap();
    let calls = fixture.calls.lock().unwrap();
    assert_eq!(
        calls.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
        vec!["/issue", "/filter.dat", "/issue", "/filter.dat"]
    );
    assert!(calls[0].body.get("scope").is_none());
}

#[tokio::test]
async fn expired_access_renews_using_the_same_token_endpoint() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    fixture.plan.lock().unwrap().expires_in = 1;
    let service = configured(root.path(), &fixture);
    service.download(RunControl::default().stop).await.unwrap();
    tokio::time::sleep(Duration::from_millis(1100)).await;
    service.download(RunControl::default().stop).await.unwrap();
    let calls = fixture.calls.lock().unwrap();
    assert_eq!(
        calls.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
        vec!["/issue", "/filter.dat", "/issue", "/filter.dat"]
    );
}

#[tokio::test]
async fn concurrent_refreshes_use_each_newly_rotated_refresh_token() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = configured(root.path(), &fixture);
    service.obtain_token().await.unwrap();
    let a = service.clone();
    let b = service.clone();
    let (a, b) = tokio::join!(a.refresh_token(), b.refresh_token());
    a.unwrap();
    b.unwrap();
    let calls = fixture.calls.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[1].body["refresh_token"], "synthetic-refresh-1");
    assert_eq!(calls[2].body["refresh_token"], "synthetic-refresh-2");
}

#[tokio::test]
async fn refresh_response_without_replacement_preserves_the_existing_refresh_token() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = configured(root.path(), &fixture);
    service.obtain_token().await.unwrap();
    fixture.plan.lock().unwrap().token_body = Some(
        br#"{"access_token":"synthetic-new-access","token_type":"Bearer","expires_in":3600}"#
            .to_vec(),
    );
    service.refresh_token().await.unwrap();
    service.refresh_token().await.unwrap();
    let calls = fixture.calls.lock().unwrap();
    assert_eq!(calls[1].body["refresh_token"], "synthetic-refresh-1");
    assert_eq!(calls[2].body["refresh_token"], "synthetic-refresh-1");
}
#[tokio::test]
async fn dat_401_renews_once_and_second_401_stops_without_loop() {
    for unauthorized in [1, 10] {
        let root = tempfile::tempdir().unwrap();
        let fixture = fixture().await;
        let service = configured(root.path(), &fixture);
        fixture.plan.lock().unwrap().dat_401 = unauthorized;
        let result = service.download(RunControl::default().stop).await;
        assert_eq!(result.is_ok(), unauthorized == 1);
        let calls = fixture.calls.lock().unwrap();
        assert_eq!(
            calls.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
            vec!["/issue", "/filter.dat", "/issue", "/filter.dat"]
        );
        assert_eq!(calls[2].body["refresh_token"], "synthetic-refresh-1");
        assert!(!service.root.join("current.json").exists());
        assert_eq!(service.cached(None).unwrap().is_some(), unauthorized == 1);
    }
}
#[tokio::test]
async fn invalid_refresh_falls_back_once_to_acquisition_and_no_refresh_is_not_revocation() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = configured(root.path(), &fixture);
    assert!(service.refresh_token().await.is_err());
    assert!(fixture.calls.lock().unwrap().is_empty());
    service.obtain_token().await.unwrap();
    fixture.plan.lock().unwrap().reject_refresh = true;
    service.refresh_token().await.unwrap();
    let calls = fixture.calls.lock().unwrap();
    assert_eq!(
        calls.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
        vec!["/issue", "/issue", "/issue"]
    );
    assert_eq!(calls[2].body["grant_type"], "client_credentials");
}
#[tokio::test]
async fn malformed_oversized_rejected_token_responses_do_not_publish_or_echo() {
    for (status,body) in [("200 OK",b"not json".to_vec()),("200 OK",br#"{"access_token":"do-not-echo-sensitive-body","token_type":"Basic","expires_in":60}"#.to_vec()),("200 OK",br#"{"access_token":"a","token_type":"Bearer","expires_in":0}"#.to_vec()),("200 OK",vec![0;65537]),("503 Unavailable",br#"{"secret":"do-not-echo-sensitive-body"}"#.to_vec()),("302 Found",b"{}".to_vec())] {
        let root=tempfile::tempdir().unwrap();let fixture=fixture().await;let service=configured(root.path(),&fixture);{let mut plan=fixture.plan.lock().unwrap();plan.token_status=Some(status);plan.token_body=Some(body);}
        let error=service.download(RunControl::default().stop).await.err().unwrap().to_string();assert!(!error.contains("do-not-echo-sensitive-body"));assert!(!service.root.join("current.json").exists());
        assert!(service.download(RunControl::default().stop).await.is_err());assert_eq!(fixture.calls.lock().unwrap().len(),1);
    }
}
#[tokio::test]
async fn changed_endpoint_or_client_never_reuses_old_credentials() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = configured(root.path(), &fixture);
    service.obtain_token().await.unwrap();
    service.download(RunControl::default().stop).await.unwrap();
    assert!(service.cached(None).unwrap().is_some());
    let original = service.settings().unwrap();
    for field in 0..4 {
        let mut cfg = original.clone();
        match field {
            0 => cfg.token_url.push_str("-changed"),
            1 => cfg.url.push_str("-changed"),
            2 => cfg.client_id.push_str("-changed"),
            _ => cfg.scope.push_str("-changed"),
        };
        service.save_settings(&cfg).unwrap();
        assert!(service.cached(None).unwrap().is_none());
        assert!(service.download(RunControl::default().stop).await.is_err());
    }
    assert_eq!(fixture.calls.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn every_job_gets_a_new_token_and_dat_pause_reuses_only_its_own_memory() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = configured(root.path(), &fixture);
    let input = root.path().join("keys.csv");
    std::fs::write(&input, "key\n000123\nabsent\n000123\n").unwrap();
    let loaded = service.download(RunControl::default().stop).await.unwrap();
    let weak = Arc::downgrade(&loaded.bloom);
    let job = service.create_job(&input, Rule::default()).unwrap();
    let mut control = RunControl::default();
    control.pause_after = Some(1);
    control.threads = Some(4);
    service
        .run_job(&job.id, Some(loaded), control)
        .await
        .unwrap();
    assert!(weak.upgrade().is_some());
    assert_eq!(fixture.calls.lock().unwrap().len(), 2);
    service
        .run_job(&job.id, None, RunControl::default())
        .await
        .unwrap();
    assert!(weak.upgrade().is_none());
    assert_eq!(fixture.calls.lock().unwrap().len(), 2);
    let next = service.create_job(&input, Rule::default()).unwrap();
    service
        .run_job(&next.id, None, RunControl::default())
        .await
        .unwrap();
    let calls = fixture.calls.lock().unwrap();
    assert_eq!(
        calls.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
        vec!["/issue", "/filter.dat", "/issue", "/filter.dat"]
    );
    assert_eq!(calls[0].body["grant_type"], "client_credentials");
    assert_eq!(calls[2].body["grant_type"], "client_credentials");
    assert!(!service.root.join("current.json").exists());
    assert!(std::fs::read_dir(service.root.join("filters"))
        .unwrap()
        .next()
        .is_none());
    assert!(service.cached(None).unwrap().is_none());
}

#[tokio::test]
async fn restart_redownloads_and_refuses_changed_dat_without_mixing_results() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = configured(root.path(), &fixture);
    let input = root.path().join("keys.csv");
    std::fs::write(&input, "key\n000123\nabsent\n000123\n").unwrap();
    let loaded = service.download(RunControl::default().stop).await.unwrap();
    let weak = Arc::downgrade(&loaded.bloom);
    let job = service.create_job(&input, Rule::default()).unwrap();
    let mut control = RunControl::default();
    control.pause_after = Some(1);
    service
        .run_job(&job.id, Some(loaded), control)
        .await
        .unwrap();
    let prefix = std::fs::read(
        service
            .root
            .join("jobs")
            .join(&job.id)
            .join("result.partial"),
    )
    .unwrap();
    drop(service);
    assert!(weak.upgrade().is_none());
    fixture.plan.lock().unwrap().dat_body =
        Some(include_bytes!("fixtures/bloom/python-v1-1.dat").to_vec());
    let service = configured(root.path(), &fixture);
    let error = service
        .run_job(&job.id, None, RunControl::default())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("differs from the job pin"));
    assert!(!service.result_path(&job.id).unwrap().exists());
    assert_eq!(
        std::fs::read(
            service
                .root
                .join("jobs")
                .join(&job.id)
                .join("result.partial")
        )
        .unwrap(),
        prefix
    );
    assert!(service.cached(None).unwrap().is_none());
    fixture.plan.lock().unwrap().dat_body = None;
    let done = service
        .run_job(&job.id, None, RunControl::default())
        .await
        .unwrap();
    assert_eq!(done.state, "COMPLETED");
    assert_eq!(done.processed, 3);
    assert_eq!(fixture.calls.lock().unwrap().len(), 6);
}

#[tokio::test]
async fn encrypted_downloads_authenticate_raw_gzip_native_provider_and_keep_keys_local() {
    use base64::Engine;
    use pgm_data::matching::{crypto::DatKey, format::Bloom, Transform};
    for (encrypted, plain, transform) in [
        (
            include_bytes!("fixtures/bloom/native-gcm.enc").as_slice(),
            "python-v1-0.dat",
            Transform::None,
        ),
        (
            include_bytes!("fixtures/bloom/provider-raw-gcm.enc").as_slice(),
            "provider-v1.dat",
            Transform::Sha256,
        ),
        (
            include_bytes!("fixtures/bloom/provider-gzip-gcm.enc").as_slice(),
            "provider-v1.dat.gz",
            Transform::Sha256,
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let fixture = fixture().await;
        fixture.plan.lock().unwrap().dat_body = Some(encrypted.to_vec());
        let service = configured(root.path(), &fixture);
        let encoded = base64::engine::general_purpose::STANDARD.encode([0x42; 32]);
        service
            .set_dat_key(DatKey::from_base64(&encoded).unwrap())
            .unwrap();
        let input = root.path().join("keys.csv");
        let keys = ["00012345678", "partner-x", "absent"];
        std::fs::write(&input, format!("key\n{}\n", keys.join("\n"))).unwrap();
        let rule = Rule {
            transform,
            ..Default::default()
        };
        let expected = Bloom::read(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/bloom")
                .join(plain),
            1024 * 1024,
        )
        .unwrap();
        let job = service.create_job(&input, rule.clone()).unwrap();
        let done = service
            .run_job(&job.id, None, RunControl::default())
            .await
            .unwrap();
        assert_eq!(done.state, "COMPLETED");
        let result = std::fs::read_to_string(service.result_path(&job.id).unwrap()).unwrap();
        assert_eq!(
            result,
            keys.iter()
                .map(|k| format!("{k}: {}\n", expected.contains_with_rule(k, &rule).unwrap()))
                .collect::<String>()
        );
        assert!(service.cached(None).unwrap().is_none());
        for call in fixture.calls.lock().unwrap().iter() {
            assert!(!call.headers.contains(&encoded));
            assert!(!call.body.to_string().contains(&encoded));
            assert!(!call.body.to_string().contains("aes"));
        }
        assert!(!std::fs::read_to_string(service.root.join("settings.json"))
            .unwrap()
            .contains(&encoded));
        assert!(std::fs::read_dir(service.root.join("filters"))
            .unwrap()
            .next()
            .is_none());
    }
}

#[tokio::test]
async fn encrypted_resume_pins_plaintext_across_nonces_and_wrong_key_never_publishes() {
    use base64::Engine;
    use pgm_data::matching::crypto::DatKey;
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = configured(root.path(), &fixture);
    let encoded = base64::engine::general_purpose::STANDARD.encode([0x42; 32]);
    service
        .set_dat_key(DatKey::from_base64(&encoded).unwrap())
        .unwrap();
    fixture.plan.lock().unwrap().dat_body =
        Some(include_bytes!("fixtures/bloom/native-gcm.enc").to_vec());
    let input = root.path().join("keys.csv");
    std::fs::write(&input, "key\n000123\nabsent\n000123\n").unwrap();
    let job = service.create_job(&input, Rule::default()).unwrap();
    let mut control = RunControl::default();
    control.pause_after = Some(1);
    let paused = service.run_job(&job.id, None, control).await.unwrap();
    service.clear_filters().unwrap(); // Lost job memory; the session key stays until logout.
    fixture.plan.lock().unwrap().dat_body =
        Some(include_bytes!("fixtures/bloom/native-gcm-fresh-nonce.enc").to_vec());
    let done = service
        .run_job(&job.id, None, RunControl::default())
        .await
        .unwrap();
    assert_eq!(done.filter, paused.filter);
    assert_eq!(done.state, "COMPLETED");
    assert_eq!(fixture.calls.lock().unwrap().len(), 4);
    let wrong = base64::engine::general_purpose::STANDARD.encode([0x43; 32]);
    service
        .set_dat_key(DatKey::from_base64(&wrong).unwrap())
        .unwrap();
    let next = service.create_job(&input, Rule::default()).unwrap();
    let err = service
        .run_job(&next.id, None, RunControl::default())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("DAT authentication failed"));
    assert!(service.cached(None).unwrap().is_none());
    assert!(!service.result_path(&next.id).unwrap().exists());
    assert!(!service
        .root
        .join("jobs")
        .join(&next.id)
        .join("result.partial")
        .exists());
    assert_eq!(fixture.calls.lock().unwrap().len(), 6);
}
#[tokio::test]
async fn pgmenc01_checksum_and_header_failures_publish_no_snapshot_or_job_output() {
    use base64::Engine;
    use pgm_data::matching::crypto::DatKey;
    use sha2::{Digest, Sha256};
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = configured(root.path(), &fixture);
    service
        .set_dat_key(
            DatKey::from_base64(&base64::engine::general_purpose::STANDARD.encode([0x42; 32]))
                .unwrap(),
        )
        .unwrap();
    let input = root.path().join("keys.csv");
    std::fs::write(&input, "key\n000123\n").unwrap();
    for kind in 0..3 {
        let mut body = include_bytes!("fixtures/bloom/native-gcm.enc").to_vec();
        match kind {
            0 => body[44] ^= 1,
            1 => body[8..12].copy_from_slice(&1u32.to_le_bytes()),
            _ => {
                body[43] ^= 1;
                let end = body.len() - 32;
                let hash = Sha256::digest(&body[..end]);
                body[end..].copy_from_slice(&hash);
            }
        }
        fixture.plan.lock().unwrap().dat_body = Some(body);
        let job = service.create_job(&input, Rule::default()).unwrap();
        assert!(service
            .run_job(&job.id, None, RunControl::default())
            .await
            .is_err());
        assert!(service.cached(None).unwrap().is_none());
        assert!(!service.result_path(&job.id).unwrap().exists());
        assert!(!service
            .root
            .join("jobs")
            .join(&job.id)
            .join("result.partial")
            .exists());
        assert_eq!(service.job(&job.id).unwrap().processed, 0);
    }
    assert_eq!(fixture.calls.lock().unwrap().len(), 6);
    assert!(std::fs::read_dir(service.root.join("filters"))
        .unwrap()
        .next()
        .is_none());
}
#[tokio::test]
async fn cancellation_releases_auth_state_and_process_lock_for_retry() {
    let root = tempfile::tempdir().unwrap();
    let fixture = fixture().await;
    let service = configured(root.path(), &fixture);
    fixture.plan.lock().unwrap().delay = Duration::from_millis(300);
    let (stop, rx) = tokio::sync::watch::channel(false);
    let owned = service.clone();
    let task = tokio::spawn(async move { owned.download(rx).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while fixture.calls.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    stop.send(true).unwrap();
    assert!(task.await.unwrap().is_err());
    fixture.plan.lock().unwrap().delay = Duration::ZERO;
    tokio::time::timeout(Duration::from_secs(2), service.obtain_token())
        .await
        .unwrap()
        .unwrap();
}
