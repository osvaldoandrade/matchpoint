//! Local fixture server. No production API is contacted.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{watch, Mutex};

#[derive(Clone, Debug)]
pub enum PostMode {
    Closed { customer: Option<bool> },
    Status(u16),
    Malformed,
    Hang,
    Oversized,
    Redirect,
    RetryThenOk,
    Always401,
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub expires_in: u64,
    pub rotate_refresh: bool,
    pub reject_refresh: bool,
    pub token_status: u16,
    pub post: PostMode,
    pub token_delay: Duration,
}

impl Default for Plan {
    fn default() -> Self {
        Self {
            expires_in: 3600,
            rotate_refresh: true,
            reject_refresh: false,
            token_status: 200,
            post: PostMode::Closed { customer: None },
            token_delay: Duration::ZERO,
        }
    }
}

#[derive(Default)]
struct Seen {
    idempotency: Vec<String>,
    bodies: Vec<String>,
    content_types: Vec<String>,
    cpf_in_key: bool,
    post_attempts: std::collections::HashMap<String, u32>,
    refresh_tokens: Vec<String>,
}

pub struct Fixture {
    pub base: String,
    pub hits_token: Arc<AtomicU64>,
    pub hits_post: Arc<AtomicU64>,
    pub hits_other: Arc<AtomicU64>,
    plan: Arc<Mutex<Plan>>,
    seen: Arc<Mutex<Seen>>,
    stop: watch::Sender<bool>,
}

impl Fixture {
    pub async fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind fixture");
        let port = listener.local_addr().expect("addr").port();
        let plan = Arc::new(Mutex::new(Plan::default()));
        let seen = Arc::new(Mutex::new(Seen::default()));
        let hits_token = Arc::new(AtomicU64::new(0));
        let hits_post = Arc::new(AtomicU64::new(0));
        let hits_other = Arc::new(AtomicU64::new(0));
        let (stop, stop_rx) = watch::channel(false);
        let state = Shared {
            plan: Arc::clone(&plan),
            seen: Arc::clone(&seen),
            hits_token: Arc::clone(&hits_token),
            hits_post: Arc::clone(&hits_post),
            hits_other: Arc::clone(&hits_other),
            stop: stop_rx,
        };
        tokio::spawn(async move {
            // Terminates when `stop` is set or the listener is dropped.
            loop {
                let mut stop = state.stop.clone();
                tokio::select! {
                    _ = stop.changed() => {
                        if *stop.borrow() { break; }
                    }
                    accepted = listener.accept() => {
                        let Ok((socket, _)) = accepted else { break; };
                        let state = state.clone();
                        tokio::spawn(async move { let _ = serve(socket, state).await; });
                    }
                }
            }
        });
        Self {
            base: format!("http://127.0.0.1:{port}"),
            hits_token,
            hits_post,
            hits_other,
            plan,
            seen,
            stop,
        }
    }

    pub async fn set_plan(&self, plan: Plan) {
        *self.plan.lock().await = plan;
    }

    pub async fn idempotency_keys(&self) -> Vec<String> {
        self.seen.lock().await.idempotency.clone()
    }

    pub async fn cpf_leaked_into_key(&self) -> bool {
        self.seen.lock().await.cpf_in_key
    }

    pub async fn content_types(&self) -> Vec<String> {
        self.seen.lock().await.content_types.clone()
    }

    pub fn token_url(&self) -> String {
        format!("{}/oauth/token", self.base)
    }

    pub fn process_url(&self) -> String {
        format!("{}/v1/intent", self.base)
    }

    pub fn stop(&self) {
        let _ = self.stop.send(true);
    }
}

#[derive(Clone)]
struct Shared {
    plan: Arc<Mutex<Plan>>,
    seen: Arc<Mutex<Seen>>,
    hits_token: Arc<AtomicU64>,
    hits_post: Arc<AtomicU64>,
    hits_other: Arc<AtomicU64>,
    stop: watch::Receiver<bool>,
}

async fn serve(socket: tokio::net::TcpStream, state: Shared) -> std::io::Result<()> {
    let (reader, mut writer) = socket.into_split();
    let mut reader = tokio::io::BufReader::new(reader);
    loop {
        if *state.stop.borrow() {
            break;
        }
        let Some(req) = read_http(&mut reader).await? else {
            break;
        };
        if req.path == "/oauth/token" {
            state.hits_token.fetch_add(1, Ordering::SeqCst);
        } else if req.path == "/v1/intent" {
            state.hits_post.fetch_add(1, Ordering::SeqCst);
        } else {
            state.hits_other.fetch_add(1, Ordering::SeqCst);
        }
        let response = respond(&req, &state).await;
        if response.is_empty() {
            break;
        }
        writer.write_all(&response).await?;
        writer.flush().await?;
    }
    Ok(())
}

struct Incoming {
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

async fn read_http(
    reader: &mut tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> std::io::Result<Option<Incoming>> {
    let mut header = Vec::new();
    let mut buf = [0_u8; 1];
    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            return Ok(None);
        }
        header.push(buf[0]);
        if header.ends_with(b"\r\n\r\n") || header.len() > 64 * 1024 {
            break;
        }
    }
    let text = String::from_utf8_lossy(&header).to_string();
    let mut lines = text.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();
    let mut headers = Vec::new();
    let mut length = 0_usize;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            if name == "content-length" {
                length = value.parse().unwrap_or(0);
            }
            headers.push((name, value));
        }
    }
    let mut body = vec![0_u8; length.min(1_048_576)];
    if !body.is_empty() {
        reader.read_exact(&mut body).await?;
    }
    Ok(Some(Incoming {
        path,
        headers,
        body: String::from_utf8_lossy(&body).to_string(),
    }))
}

async fn respond(req: &Incoming, state: &Shared) -> Vec<u8> {
    let plan = state.plan.lock().await.clone();
    if req.path == "/oauth/token" {
        if !plan.token_delay.is_zero() {
            tokio::time::sleep(plan.token_delay).await;
        }
        {
            let mut seen = state.seen.lock().await;
            if let Some(ctype) = header(req, "content-type") {
                seen.content_types.push(ctype);
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&req.body) {
                if let Some(token) = json.get("refresh_token").and_then(|v| v.as_str()) {
                    seen.refresh_tokens.push(token.to_string());
                }
                if plan.reject_refresh
                    && json.get("grant_type").and_then(|v| v.as_str()) == Some("refresh_token")
                {
                    return json_response(400, br#"{"error":"invalid_grant"}"#);
                }
            }
        }
        if plan.token_status != 200 {
            return json_response(plan.token_status, br#"{"error":"invalid_client"}"#);
        }
        let n = state.hits_token.load(Ordering::SeqCst);
        let refresh = format!("refresh-{n}");
        let body = format!(
            "{{\"access_token\":\"sim-token-{n}\",\"token_type\":\"Bearer\",\"expires_in\":{},\"refresh_token\":\"{refresh}\"}}",
            plan.expires_in
        );
        let _ = plan.rotate_refresh;
        return json_response(200, body.as_bytes());
    }
    if req.path == "/v1/intent" {
        let cpf = serde_json::from_str::<serde_json::Value>(&req.body)
            .ok()
            .and_then(|v| v.get("cpf").and_then(|c| c.as_str()).map(|s| s.to_string()))
            .unwrap_or_default();
        let key = header(req, "idempotency-key").unwrap_or_default();
        {
            let mut seen = state.seen.lock().await;
            if !key.is_empty() {
                if key.contains(&cpf) && !cpf.is_empty() {
                    seen.cpf_in_key = true;
                }
                seen.idempotency.push(key);
            }
            seen.bodies.push(req.body.clone());
            if let Some(ctype) = header(req, "content-type") {
                seen.content_types.push(ctype);
            }
        }
        match plan.post {
            PostMode::Hang => {
                let mut stop = state.stop.clone();
                let _ = stop.changed().await;
                return Vec::new();
            }
            PostMode::Oversized => {
                let body = "x".repeat(80_000);
                return json_response(200, body.as_bytes());
            }
            PostMode::Redirect => {
                return b"HTTP/1.1 302 Found\r\nLocation: /evil\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec();
            }
            PostMode::Malformed => return json_response(200, b"not-json"),
            PostMode::Status(code) => return json_response(code, br#"{"error":"status"}"#),
            PostMode::Always401 => return json_response(401, br#"{"error":"unauthorized"}"#),
            PostMode::RetryThenOk => {
                let mut seen = state.seen.lock().await;
                let n = seen.post_attempts.entry(cpf.clone()).or_insert(0);
                *n += 1;
                let attempt = *n;
                drop(seen);
                if attempt == 1 {
                    return b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 0\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: keep-alive\r\n\r\n{}".to_vec();
                }
            }
            PostMode::Closed { .. } => {}
        }
        let customer = match plan.post {
            PostMode::Closed {
                customer: Some(true),
            } => "\"customer\"",
            PostMode::Closed {
                customer: Some(false),
            } => "\"not_customer\"",
            _ => "null",
        };
        let n = state.hits_post.load(Ordering::SeqCst);
        let body = format!(
            "{{\"fixture_version\":1,\"disposition\":\"closed\",\"request_id\":\"sim-{n}\",\"customer_evidence\":{customer}}}"
        );
        return json_response(200, body.as_bytes());
    }
    json_response(404, br#"{"error":"not_found"}"#)
}

fn header(req: &Incoming, name: &str) -> Option<String> {
    req.headers
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.clone())
}

fn json_response(status: u16, body: &[u8]) -> Vec<u8> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Error",
        _ => "Result",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n",
        body.len()
    );
    let mut out = head.into_bytes();
    out.extend_from_slice(body);
    out
}
