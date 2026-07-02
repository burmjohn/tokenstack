use crate::auth::AuthHandle;
use crate::safety::{ConnectorRequest, EndpointRegistry, HttpMethod, SafetyError, SafetyGuard};
use crate::telemetry::{public_error, PublicError};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;

const CONNECTOR_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const CONNECTOR_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetCreditBatch {
    pub credit_count: i64,
    pub expires_at_utc: DateTime<Utc>,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitWindowSnapshot {
    pub window_key: String,
    pub limit_tokens: i64,
    pub used_tokens: i64,
    pub remaining_tokens: i64,
    pub resets_at_utc: DateTime<Utc>,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorRunResult {
    pub connector_id: String,
    pub status: String,
    pub batches: Vec<ResetCreditBatch>,
    pub rate_limit_windows: Vec<RateLimitWindowSnapshot>,
    pub redacted_error: Option<PublicError>,
}

pub struct KnownResetCreditsConnector {
    base_url: Url,
    guard: SafetyGuard,
    client: Client,
}

impl KnownResetCreditsConnector {
    pub fn new(base_url: Url) -> Self {
        Self::with_registry(base_url, EndpointRegistry::default_readonly())
    }

    pub fn with_registry(base_url: Url, registry: EndpointRegistry) -> Self {
        Self::with_client(base_url, registry, bounded_http_client())
    }

    fn with_client(base_url: Url, registry: EndpointRegistry, client: Client) -> Self {
        Self {
            base_url,
            guard: SafetyGuard::new(registry),
            client,
        }
    }

    pub fn fetch(&self, auth: &AuthHandle) -> ConnectorRunResult {
        match self.try_fetch(auth) {
            Ok(batches) => ConnectorRunResult {
                connector_id: "known-reset-credit".to_string(),
                status: "complete".to_string(),
                batches,
                rate_limit_windows: Vec::new(),
                redacted_error: None,
            },
            Err(error) => ConnectorRunResult {
                connector_id: "known-reset-credit".to_string(),
                status: "failed".to_string(),
                batches: Vec::new(),
                rate_limit_windows: Vec::new(),
                redacted_error: Some(public_error("connector_failed", &error.to_string())),
            },
        }
    }

    fn try_fetch(&self, auth: &AuthHandle) -> anyhow::Result<Vec<ResetCreditBatch>> {
        let url = self.base_url.join("/wham/rate-limit-reset-credits")?;
        self.guard.validate(&ConnectorRequest {
            method: HttpMethod::Get,
            url: url.clone(),
            has_body: false,
            endpoint_id: "known-reset-credit".to_string(),
        })?;

        let response = self
            .client
            .get(url)
            .header("Authorization", auth.bearer_header())
            .send()?
            .error_for_status()?;
        parse_reset_credit_response(&response.text()?)
    }
}

pub struct UndocumentedRateLimitsConnector {
    base_url: Url,
    guard: SafetyGuard,
    client: Client,
}

impl UndocumentedRateLimitsConnector {
    pub fn new(base_url: Url) -> Self {
        Self::with_registry(base_url, EndpointRegistry::default_readonly())
    }

    pub fn with_registry(base_url: Url, registry: EndpointRegistry) -> Self {
        Self::with_client(base_url, registry, bounded_http_client())
    }

    fn with_client(base_url: Url, registry: EndpointRegistry, client: Client) -> Self {
        Self {
            base_url,
            guard: SafetyGuard::new(registry),
            client,
        }
    }

    pub fn fetch(&self, auth: &AuthHandle) -> ConnectorRunResult {
        match self.try_fetch(auth) {
            Ok(rate_limit_windows) => ConnectorRunResult {
                connector_id: "undocumented-rate-limits".to_string(),
                status: "complete".to_string(),
                batches: Vec::new(),
                rate_limit_windows,
                redacted_error: None,
            },
            Err(error) => ConnectorRunResult {
                connector_id: "undocumented-rate-limits".to_string(),
                status: "failed".to_string(),
                batches: Vec::new(),
                rate_limit_windows: Vec::new(),
                redacted_error: Some(public_error("connector_failed", &error.to_string())),
            },
        }
    }

    fn try_fetch(&self, auth: &AuthHandle) -> anyhow::Result<Vec<RateLimitWindowSnapshot>> {
        let url = self.base_url.join("/backend-api/rate_limits")?;
        self.guard.validate(&ConnectorRequest {
            method: HttpMethod::Get,
            url: url.clone(),
            has_body: false,
            endpoint_id: "undocumented-rate-limits".to_string(),
        })?;

        let response = self
            .client
            .get(url)
            .header("Authorization", auth.bearer_header())
            .send()?
            .error_for_status()?;
        parse_rate_limit_response(&response.text()?)
    }
}

fn bounded_http_client() -> Client {
    Client::builder()
        .connect_timeout(CONNECTOR_CONNECT_TIMEOUT)
        .timeout(CONNECTOR_REQUEST_TIMEOUT)
        .build()
        .expect("bounded connector HTTP client should build")
}

#[allow(dead_code)]
pub fn validate_registered_request(url: Url) -> Result<(), SafetyError> {
    SafetyGuard::new(EndpointRegistry::default_readonly()).validate(&ConnectorRequest {
        method: HttpMethod::Get,
        url,
        has_body: false,
        endpoint_id: "known-reset-credit".to_string(),
    })?;
    Ok(())
}

pub fn parse_reset_credit_response(text: &str) -> anyhow::Result<Vec<ResetCreditBatch>> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let batches = value
        .get("reset_credits")
        .or_else(|| value.get("credits"))
        .and_then(|value| value.as_array())
        .ok_or_else(|| anyhow::anyhow!("missing reset credit array"))?;

    batches
        .iter()
        .map(|item| {
            let credit_count = item
                .get("credit_count")
                .or_else(|| item.get("count"))
                .and_then(|value| value.as_i64())
                .ok_or_else(|| anyhow::anyhow!("missing credit count"))?;
            let expires = item
                .get("expires_at")
                .or_else(|| item.get("expires_at_utc"))
                .and_then(|value| value.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing expiration"))?;
            Ok(ResetCreditBatch {
                credit_count,
                expires_at_utc: DateTime::parse_from_rfc3339(expires)?.with_timezone(&Utc),
                confidence: "high".to_string(),
            })
        })
        .collect()
}

pub fn parse_rate_limit_response(text: &str) -> anyhow::Result<Vec<RateLimitWindowSnapshot>> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let windows = value
        .get("rate_limit_windows")
        .or_else(|| value.get("windows"))
        .and_then(|value| value.as_array())
        .ok_or_else(|| anyhow::anyhow!("missing rate-limit window array"))?;

    windows
        .iter()
        .map(|item| {
            let window_key = item
                .get("window_key")
                .or_else(|| item.get("window"))
                .and_then(|value| value.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing window key"))?
                .to_string();
            let limit_tokens = item
                .get("limit_tokens")
                .or_else(|| item.get("limit"))
                .and_then(|value| value.as_i64())
                .ok_or_else(|| anyhow::anyhow!("missing limit tokens"))?;
            let used_tokens = item
                .get("used_tokens")
                .or_else(|| item.get("used"))
                .and_then(|value| value.as_i64())
                .ok_or_else(|| anyhow::anyhow!("missing used tokens"))?;
            let remaining_tokens = item
                .get("remaining_tokens")
                .or_else(|| item.get("remaining"))
                .and_then(|value| value.as_i64())
                .ok_or_else(|| anyhow::anyhow!("missing remaining tokens"))?;
            let resets_at = item
                .get("resets_at")
                .or_else(|| item.get("resets_at_utc"))
                .and_then(|value| value.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing reset timestamp"))?;
            Ok(RateLimitWindowSnapshot {
                window_key,
                limit_tokens,
                used_tokens,
                remaining_tokens,
                resets_at_utc: DateTime::parse_from_rfc3339(resets_at)?.with_timezone(&Utc),
                confidence: "medium".to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthHandle;
    use crate::safety::{DocumentedStatus, EndpointSpec};
    use secrecy::SecretString;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::thread;
    use std::time::Duration;
    use tiny_http::{Response, Server};

    #[test]
    fn known_reset_credit_schema_accepts_expected_shape() {
        let batches = parse_reset_credit_response(
            r#"{"reset_credits":[{"credit_count":4,"expires_at":"2026-07-28T18:14:00Z"}]}"#,
        )
        .unwrap();
        assert_eq!(batches[0].credit_count, 4);
    }

    #[test]
    fn known_reset_credit_schema_rejects_missing_expiration() {
        assert!(parse_reset_credit_response(r#"{"reset_credits":[{"credit_count":4}]}"#).is_err());
    }

    #[test]
    fn allowed_endpoint_request_reaches_server_only_after_guard_approval() {
        let server = Server::http("127.0.0.1:0").unwrap();
        let addr = format!("http://{}", server.server_addr());
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_for_thread = Arc::clone(&hits);
        let handle = thread::spawn(move || {
            if let Ok(request) = server.recv() {
                hits_for_thread.fetch_add(1, Ordering::SeqCst);
                request
                    .respond(Response::from_string(
                        r#"{"reset_credits":[{"credit_count":4,"expires_at":"2026-07-28T18:14:00Z"}]}"#,
                    ))
                    .unwrap();
            }
        });

        let host = Url::parse(&addr).unwrap().host_str().unwrap().to_string();
        let connector = KnownResetCreditsConnector::with_registry(
            Url::parse(&addr).unwrap(),
            EndpointRegistry::with_endpoint(EndpointSpec {
                id: "known-reset-credit".to_string(),
                method: HttpMethod::Get,
                scheme: "http".to_string(),
                host,
                path: "/wham/rate-limit-reset-credits".to_string(),
                body_allowed: false,
                documented_status: DocumentedStatus::Undocumented,
                readonly_review: true,
                response_schema: "reset_credit_batches_v1".to_string(),
                reviewed_at: "2026-07-02".to_string(),
            }),
        );
        let result = connector.fetch(&AuthHandle::from_token(
            SecretString::from("synthetic-token".to_string()),
            None,
        ));
        handle.join().unwrap();

        assert_eq!(result.status, "complete");
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn undocumented_rate_limit_schema_accepts_expected_shape() {
        let windows = parse_rate_limit_response(
            r#"{"rate_limit_windows":[{"window_key":"gpt-5","limit_tokens":1000,"used_tokens":250,"remaining_tokens":750,"resets_at":"2026-07-03T18:14:00Z"}]}"#,
        )
        .unwrap();

        assert_eq!(windows[0].window_key, "gpt-5");
        assert_eq!(windows[0].remaining_tokens, 750);
    }

    #[test]
    fn undocumented_rate_limit_schema_rejects_missing_reset_timestamp() {
        assert!(parse_rate_limit_response(
            r#"{"rate_limit_windows":[{"window_key":"gpt-5","limit_tokens":1000,"used_tokens":250,"remaining_tokens":750}]}"#,
        )
        .is_err());
    }

    #[test]
    fn undocumented_rate_limit_request_reaches_server_only_after_guard_approval() {
        let server = Server::http("127.0.0.1:0").unwrap();
        let addr = format!("http://{}", server.server_addr());
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_for_thread = Arc::clone(&hits);
        let handle = thread::spawn(move || {
            if let Ok(request) = server.recv() {
                hits_for_thread.fetch_add(1, Ordering::SeqCst);
                request
                    .respond(Response::from_string(
                        r#"{"rate_limit_windows":[{"window_key":"gpt-5","limit_tokens":1000,"used_tokens":250,"remaining_tokens":750,"resets_at":"2026-07-03T18:14:00Z"}]}"#,
                    ))
                    .unwrap();
            }
        });

        let host = Url::parse(&addr).unwrap().host_str().unwrap().to_string();
        let connector = UndocumentedRateLimitsConnector::with_registry(
            Url::parse(&addr).unwrap(),
            EndpointRegistry::with_endpoint(EndpointSpec {
                id: "undocumented-rate-limits".to_string(),
                method: HttpMethod::Get,
                scheme: "http".to_string(),
                host,
                path: "/backend-api/rate_limits".to_string(),
                body_allowed: false,
                documented_status: DocumentedStatus::Undocumented,
                readonly_review: true,
                response_schema: "rate_limit_windows_v1".to_string(),
                reviewed_at: "2026-07-02".to_string(),
            }),
        );
        let result = connector.fetch(&AuthHandle::from_token(
            SecretString::from("synthetic-token".to_string()),
            None,
        ));
        handle.join().unwrap();

        assert_eq!(result.status, "complete");
        assert_eq!(result.rate_limit_windows[0].remaining_tokens, 750);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn known_reset_credit_timeout_returns_failed_result() {
        let server = Server::http("127.0.0.1:0").unwrap();
        let addr = format!("http://{}", server.server_addr());
        let handle = thread::spawn(move || {
            if let Ok(request) = server.recv() {
                thread::sleep(Duration::from_millis(150));
                let _ = request.respond(Response::from_string(
                    r#"{"reset_credits":[{"credit_count":4,"expires_at":"2026-07-28T18:14:00Z"}]}"#,
                ));
            }
        });

        let host = Url::parse(&addr).unwrap().host_str().unwrap().to_string();
        let client = Client::builder()
            .connect_timeout(Duration::from_millis(25))
            .timeout(Duration::from_millis(25))
            .build()
            .unwrap();
        let connector = KnownResetCreditsConnector::with_client(
            Url::parse(&addr).unwrap(),
            EndpointRegistry::with_endpoint(EndpointSpec {
                id: "known-reset-credit".to_string(),
                method: HttpMethod::Get,
                scheme: "http".to_string(),
                host,
                path: "/wham/rate-limit-reset-credits".to_string(),
                body_allowed: false,
                documented_status: DocumentedStatus::Undocumented,
                readonly_review: true,
                response_schema: "reset_credit_batches_v1".to_string(),
                reviewed_at: "2026-07-02".to_string(),
            }),
            client,
        );

        let result = connector.fetch(&AuthHandle::from_token(
            SecretString::from("synthetic-token".to_string()),
            None,
        ));
        handle.join().unwrap();

        assert_eq!(result.status, "failed");
        assert_eq!(
            result
                .redacted_error
                .as_ref()
                .map(|error| error.code.as_str()),
            Some("connector_failed")
        );
    }

    #[test]
    fn consume_request_attempt_never_reaches_server() {
        let server = Server::http("127.0.0.1:0").unwrap();
        let addr = format!("http://{}", server.server_addr());
        let url = Url::parse(&format!("{addr}/consume")).unwrap();
        let error = validate_registered_request(url).unwrap_err();
        assert_eq!(error, SafetyError::ConsumePathDenied);
        drop(server);
    }

    #[test]
    fn connector_failure_does_not_expose_auth_values() {
        let token_like = format!("{}{}", "sk-", "syntheticSecretValue1234567890");
        let result = parse_reset_credit_response(&format!(r#"{{"error":"Bearer {token_like}"}}"#));
        let public = public_error("connector_failed", &format!("{result:?}"));
        assert!(!public.message.contains(&token_like));
    }
}
