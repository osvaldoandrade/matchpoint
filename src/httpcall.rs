use crate::domain::{ContractMode, Evidence, RecordEvent};

/// What the worker should do with one HTTP result. `Unauthorized` is not a record
/// transition; the worker invalidates that token generation and tries once more.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Disposition {
    Unauthorized,
    Retry {
        after_ms: Option<i64>,
        code: String,
    },
    FinalFail {
        code: String,
    },
    Unknown {
        code: String,
        request_id: Option<String>,
    },
    Succeeded {
        is_customer: Option<bool>,
        request_id: Option<String>,
        evidence: Evidence,
    },
}

impl Disposition {
    pub fn event(&self) -> Option<RecordEvent> {
        match self {
            Self::Unauthorized => None,
            Self::Retry { .. } => Some(RecordEvent::ScheduleRetry),
            Self::FinalFail { .. } => Some(RecordEvent::FailFinal),
            Self::Unknown { .. } => Some(RecordEvent::LoseOutcome),
            Self::Succeeded { .. } => Some(RecordEvent::Succeed),
        }
    }

    pub fn error_code(&self) -> Option<String> {
        match self {
            Self::Unauthorized => Some("unauthorized".into()),
            Self::Retry { code, .. } | Self::FinalFail { code } | Self::Unknown { code, .. } => {
                Some(code.clone())
            }
            Self::Succeeded { .. } => None,
        }
    }

    pub fn request_id(&self) -> Option<String> {
        match self {
            Self::Unknown { request_id, .. } => request_id.clone(),
            Self::Succeeded { request_id, .. } => request_id.clone(),
            _ => None,
        }
    }

    pub fn is_customer(&self) -> Option<bool> {
        match self {
            Self::Succeeded { is_customer, .. } => *is_customer,
            _ => None,
        }
    }

    pub fn evidence(&self) -> Evidence {
        match self {
            Self::Succeeded { evidence, .. } => *evidence,
            Self::Unknown { .. } => Evidence::HttpOnly,
            _ => Evidence::None,
        }
    }
}

pub fn classify_status(
    mode: ContractMode,
    status: u16,
    body: &[u8],
    retry_after_ms: Option<i64>,
) -> Disposition {
    if (300..400).contains(&status) {
        return Disposition::Unknown {
            code: "redirect_refused".into(),
            request_id: None,
        };
    }
    if status == 401 {
        return Disposition::Unauthorized;
    }
    if status == 429 {
        return Disposition::Retry {
            after_ms: retry_after_ms,
            code: "http_429".into(),
        };
    }
    if status == 403 {
        return Disposition::FinalFail {
            code: "forbidden".into(),
        };
    }
    if (500..600).contains(&status) {
        if mode == ContractMode::Pending {
            return Disposition::Unknown {
                code: format!("http_{status}"),
                request_id: request_id_of(body),
            };
        }
        return Disposition::Retry {
            after_ms: retry_after_ms,
            code: format!("http_{status}"),
        };
    }
    if (400..500).contains(&status) {
        return Disposition::FinalFail {
            code: format!("http_{status}"),
        };
    }
    if (200..300).contains(&status) {
        return classify_success(mode, body);
    }
    Disposition::Unknown {
        code: format!("http_{status}"),
        request_id: request_id_of(body),
    }
}

fn classify_success(mode: ContractMode, body: &[u8]) -> Disposition {
    match mode {
        ContractMode::Pending => Disposition::Unknown {
            code: "contract_pending".into(),
            request_id: request_id_of(body),
        },
        ContractMode::SimulatedFixtureV1 => match parse_fixture(body) {
            Ok(fixture) => Disposition::Succeeded {
                is_customer: fixture.is_customer,
                request_id: Some(fixture.request_id),
                evidence: Evidence::SimulatedFixture,
            },
            Err(code) => Disposition::Unknown {
                code,
                request_id: request_id_of(body),
            },
        },
    }
}

struct Fixture {
    request_id: String,
    is_customer: Option<bool>,
}

fn parse_fixture(body: &[u8]) -> Result<Fixture, String> {
    let value: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| "malformed_body".to_string())?;
    let version = value.get("fixture_version").and_then(|v| v.as_i64());
    if version != Some(1) {
        return Err("malformed_body".into());
    }
    if value.get("disposition").and_then(|v| v.as_str()) != Some("closed") {
        return Err("malformed_body".into());
    }
    let request_id = value
        .get("request_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && s.len() <= 200)
        .ok_or_else(|| "malformed_body".to_string())?
        .to_string();
    let is_customer = match value.get("customer_evidence") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(text)) if text == "customer" => Some(true),
        Some(serde_json::Value::String(text)) if text == "not_customer" => Some(false),
        _ => return Err("malformed_body".into()),
    };
    Ok(Fixture {
        request_id,
        is_customer,
    })
}

fn request_id_of(body: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    let text = value.get("request_id")?.as_str()?;
    if text.is_empty() || text.len() > 200 {
        None
    } else {
        Some(text.to_string())
    }
}

pub fn classify_transport(connect: bool, timeout: bool, idempotent: bool) -> Disposition {
    if connect {
        return Disposition::Retry {
            after_ms: None,
            code: "connect".into(),
        };
    }
    if timeout && idempotent {
        return Disposition::Retry {
            after_ms: None,
            code: "timeout".into(),
        };
    }
    if timeout {
        return Disposition::Unknown {
            code: "timeout".into(),
            request_id: None,
        };
    }
    if idempotent {
        Disposition::Retry {
            after_ms: None,
            code: "transport".into(),
        }
    } else {
        Disposition::Unknown {
            code: "transport".into(),
            request_id: None,
        }
    }
}

pub fn retry_after_ms(headers: &reqwest::header::HeaderMap) -> Option<i64> {
    let value = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    let secs: u64 = value.parse().ok()?;
    Some(i64::try_from(secs.saturating_mul(1000)).unwrap_or(i64::MAX))
}

pub fn backoff_ms(
    attempt: i32,
    base_ms: u64,
    max_ms: u64,
    row_id: i64,
    retry_after: Option<i64>,
) -> i64 {
    let shift = u32::try_from(attempt.clamp(0, 8)).unwrap_or(8);
    let exp = base_ms.saturating_mul(1_u64 << shift).min(max_ms);
    let mix = (row_id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let unit = (mix % 1000) as f64 / 1000.0;
    let factor = 0.8 + 0.4 * unit;
    let mut delay = (exp as f64 * factor) as i64;
    if let Some(after) = retry_after {
        delay = delay.max(after);
    }
    delay.max(0)
}

pub async fn post_cpf(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    cpf: &str,
    idempotency_key: Option<&str>,
    max_body: usize,
) -> Result<HttpReply, Transport> {
    let body =
        serde_json::to_vec(&serde_json::json!({ "cpf": cpf })).map_err(|_| Transport::Other)?;
    let mut req = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .body(body);
    if let Some(key) = idempotency_key {
        req = req.header("Idempotency-Key", key);
    }
    let response = match req.send().await {
        Ok(response) => response,
        Err(err) => {
            return Err(Transport::Reqwest {
                connect: err.is_connect(),
                timeout: err.is_timeout(),
            });
        }
    };
    let status = response.status().as_u16();
    let retry_after = retry_after_ms(response.headers());
    let header_request = response
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    match crate::auth::read_limited(response, max_body).await {
        Ok(bytes) => Ok(HttpReply {
            status,
            bytes,
            retry_after,
            header_request,
        }),
        Err(_) => Err(Transport::TooLarge { status }),
    }
}

pub struct HttpReply {
    pub status: u16,
    pub bytes: Vec<u8>,
    pub retry_after: Option<i64>,
    pub header_request: Option<String>,
}

pub enum Transport {
    Reqwest { connect: bool, timeout: bool },
    TooLarge { status: u16 },
    Other,
}

pub fn endpoint_allowed(
    raw: &str,
    test_mode: bool,
    for_run: bool,
) -> Result<reqwest::Url, crate::error::Error> {
    if raw.len() > 2_048 || raw.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
        return Err(crate::error::Error::InsecureUrl);
    }
    let url = reqwest::Url::parse(raw).map_err(|_| crate::error::Error::InsecureUrl)?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(crate::error::Error::UrlSecret);
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(crate::error::Error::UrlSecret);
    }
    let host_loopback = is_loopback(&url);
    match url.scheme() {
        "http" if host_loopback && test_mode => Ok(url),
        "https" if !for_run || host_loopback => Ok(url),
        "https" => Err(crate::error::Error::ContractPending),
        _ => Err(crate::error::Error::InsecureUrl),
    }
}

pub(crate) fn is_loopback(url: &reqwest::Url) -> bool {
    match url.host_str() {
        Some(host) if host.eq_ignore_ascii_case("localhost") => true,
        Some(host) => host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_contract_does_not_invent_a_customer_flag() {
        let body = br#"{"request_id":"abc","customer_evidence":"customer","fixture_version":1,"disposition":"closed"}"#;
        let got = classify_status(ContractMode::Pending, 200, body, None);
        match got {
            Disposition::Unknown { code, request_id } => {
                assert_eq!(code, "contract_pending");
                assert_eq!(request_id.as_deref(), Some("abc"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn simulated_fixture_sets_customer_only_from_evidence() {
        let body = br#"{"fixture_version":1,"disposition":"closed","request_id":"r","customer_evidence":"not_customer"}"#;
        match classify_status(ContractMode::SimulatedFixtureV1, 200, body, None) {
            Disposition::Succeeded {
                is_customer,
                evidence,
                ..
            } => {
                assert_eq!(is_customer, Some(false));
                assert_eq!(evidence, Evidence::SimulatedFixture);
            }
            other => panic!("{other:?}"),
        }
        let bare = br#"{"fixture_version":1,"disposition":"closed","request_id":"r"}"#;
        match classify_status(ContractMode::SimulatedFixtureV1, 200, bare, None) {
            Disposition::Succeeded { is_customer, .. } => assert_eq!(is_customer, None),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn malformed_success_is_not_success() {
        let got = classify_status(ContractMode::SimulatedFixtureV1, 200, b"nope", None);
        assert!(matches!(got, Disposition::Unknown { code, .. } if code == "malformed_body"));
    }

    #[test]
    fn status_table() {
        assert!(matches!(
            classify_status(ContractMode::Pending, 401, b"", None),
            Disposition::Unauthorized
        ));
        assert!(matches!(
            classify_status(ContractMode::Pending, 403, b"", None),
            Disposition::FinalFail { .. }
        ));
        assert!(matches!(
            classify_status(ContractMode::Pending, 429, b"", Some(0)),
            Disposition::Retry { .. }
        ));
        assert!(matches!(
            classify_status(ContractMode::Pending, 500, b"", None),
            Disposition::Unknown { .. }
        ));
        assert!(matches!(
            classify_status(ContractMode::Pending, 422, b"", None),
            Disposition::FinalFail { .. }
        ));
        assert!(matches!(
            classify_status(ContractMode::Pending, 302, b"", None),
            Disposition::Unknown { .. }
        ));
    }

    #[test]
    fn timeout_without_idempotency_is_unknown() {
        assert!(matches!(
            classify_transport(false, true, false),
            Disposition::Unknown { code, .. } if code == "timeout"
        ));
        assert!(matches!(
            classify_transport(true, false, false),
            Disposition::Retry { .. }
        ));
    }

    #[test]
    fn loopback_http_requires_test_mode() {
        assert!(endpoint_allowed("http://127.0.0.1:9/v1", true, true).is_ok());
        assert!(endpoint_allowed("http://127.0.0.1:9/v1", false, true).is_err());
        assert!(endpoint_allowed("http://example.com/v1", true, true).is_err());
        assert!(matches!(
            endpoint_allowed("https://api.example.test/v1", true, true),
            Err(crate::error::Error::ContractPending)
        ));
        assert!(endpoint_allowed("https://user:pass@127.0.0.1/v1", true, true).is_err());
    }

    #[test]
    fn backoff_respects_retry_after_and_stays_positive() {
        let delay = backoff_ms(0, 100, 10_000, 7, Some(5_000));
        assert!(delay >= 5_000);
        let jittered = backoff_ms(2, 100, 10_000, 7, None);
        assert!(jittered > 0);
    }
}
