use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Head,
    Post,
    Put,
    Patch,
    Delete,
    Options,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentedStatus {
    Documented,
    Undocumented,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointSpec {
    pub id: String,
    pub method: HttpMethod,
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub body_allowed: bool,
    pub documented_status: DocumentedStatus,
    pub readonly_review: bool,
    pub response_schema: String,
    pub reviewed_at: String,
}

#[derive(Debug, Clone)]
pub struct ConnectorRequest {
    pub method: HttpMethod,
    pub url: Url,
    pub has_body: bool,
    pub endpoint_id: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SafetyError {
    #[error("endpoint path is denied by read-only policy")]
    ConsumePathDenied,
    #[error("authenticated connector method is not read-only")]
    NonReadonlyMethod,
    #[error("authenticated connector request body is denied")]
    RequestBodyDenied,
    #[error("endpoint is not registered")]
    UnregisteredEndpoint,
    #[error("endpoint response schema is missing")]
    MissingResponseSchema,
    #[error("endpoint host is not allowed")]
    UnsafeHost,
    #[error("endpoint transport scheme is not allowed")]
    UnsafeScheme,
    #[error("endpoint readonly review is missing")]
    MissingReadonlyReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyAuditEvent {
    pub endpoint_id: String,
    pub allowed: bool,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct EndpointRegistry {
    endpoints: BTreeMap<String, EndpointSpec>,
}

impl EndpointRegistry {
    pub fn default_readonly() -> Self {
        let mut endpoints = BTreeMap::new();
        for endpoint in [
            EndpointSpec {
                id: "known-reset-credit".to_string(),
                method: HttpMethod::Get,
                scheme: "https".to_string(),
                host: "chatgpt.com".to_string(),
                path: "/wham/rate-limit-reset-credits".to_string(),
                body_allowed: false,
                documented_status: DocumentedStatus::Undocumented,
                readonly_review: true,
                response_schema: "reset_credit_batches_v1".to_string(),
                reviewed_at: "2026-07-02".to_string(),
            },
            EndpointSpec {
                id: "undocumented-rate-limits".to_string(),
                method: HttpMethod::Get,
                scheme: "https".to_string(),
                host: "chatgpt.com".to_string(),
                path: "/backend-api/rate_limits".to_string(),
                body_allowed: false,
                documented_status: DocumentedStatus::Undocumented,
                readonly_review: true,
                response_schema: "rate_limit_windows_v1".to_string(),
                reviewed_at: "2026-07-02".to_string(),
            },
        ] {
            endpoints.insert(endpoint.id.clone(), endpoint);
        }
        Self { endpoints }
    }

    #[allow(dead_code)]
    pub fn with_endpoint(endpoint: EndpointSpec) -> Self {
        let mut endpoints = BTreeMap::new();
        endpoints.insert(endpoint.id.clone(), endpoint);
        Self { endpoints }
    }

    pub fn get(&self, id: &str) -> Option<&EndpointSpec> {
        self.endpoints.get(id)
    }
}

#[derive(Debug, Clone)]
pub struct SafetyGuard {
    registry: EndpointRegistry,
}

impl SafetyGuard {
    pub fn new(registry: EndpointRegistry) -> Self {
        Self { registry }
    }

    pub fn validate(&self, request: &ConnectorRequest) -> Result<SafetyAuditEvent, SafetyError> {
        if normalized_path_contains_consume(request.url.path()) {
            return Err(SafetyError::ConsumePathDenied);
        }

        if !matches!(request.method, HttpMethod::Get | HttpMethod::Head) {
            return Err(SafetyError::NonReadonlyMethod);
        }

        if request.has_body {
            return Err(SafetyError::RequestBodyDenied);
        }

        let endpoint = self
            .registry
            .get(&request.endpoint_id)
            .ok_or(SafetyError::UnregisteredEndpoint)?;

        if endpoint.method != request.method {
            return Err(SafetyError::NonReadonlyMethod);
        }

        if endpoint.body_allowed {
            return Err(SafetyError::RequestBodyDenied);
        }

        let Some(host) = request.url.host_str() else {
            return Err(SafetyError::UnsafeHost);
        };
        if host != endpoint.host {
            return Err(SafetyError::UnsafeHost);
        }

        if request.url.scheme() != endpoint.scheme {
            return Err(SafetyError::UnsafeScheme);
        }

        if endpoint.response_schema.trim().is_empty() {
            return Err(SafetyError::MissingResponseSchema);
        }

        if !endpoint.readonly_review {
            return Err(SafetyError::MissingReadonlyReview);
        }

        if normalized_path_contains_consume(&endpoint.path) {
            return Err(SafetyError::ConsumePathDenied);
        }

        if request.url.path() != endpoint.path {
            return Err(SafetyError::UnregisteredEndpoint);
        }

        Ok(SafetyAuditEvent {
            endpoint_id: endpoint.id.clone(),
            allowed: true,
            reason: "registered read-only endpoint passed guard".to_string(),
        })
    }
}

pub fn normalized_path_contains_consume(path: &str) -> bool {
    let decoded = percent_decode_str(path).decode_utf8_lossy();
    decoded.to_ascii_lowercase().contains("/consume")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(path: &str, method: HttpMethod) -> ConnectorRequest {
        ConnectorRequest {
            method,
            url: Url::parse(&format!("https://chatgpt.com{path}")).unwrap(),
            has_body: false,
            endpoint_id: "known-reset-credit".to_string(),
        }
    }

    #[test]
    fn rejects_any_path_containing_consume() {
        let guard = SafetyGuard::new(EndpointRegistry::default_readonly());
        for path in [
            "/consume",
            "/v1/consume",
            "/wham/consume/reset",
            "/wham/%63onsume/reset",
        ] {
            let error = guard.validate(&request(path, HttpMethod::Get)).unwrap_err();
            assert_eq!(error, SafetyError::ConsumePathDenied);
        }
    }

    #[test]
    fn rejects_non_readonly_methods() {
        let guard = SafetyGuard::new(EndpointRegistry::default_readonly());
        for method in [
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Patch,
            HttpMethod::Delete,
            HttpMethod::Options,
        ] {
            let error = guard
                .validate(&request("/wham/rate-limit-reset-credits", method))
                .unwrap_err();
            assert_eq!(error, SafetyError::NonReadonlyMethod);
        }
    }

    #[test]
    fn rejects_request_body_for_authenticated_connectors() {
        let guard = SafetyGuard::new(EndpointRegistry::default_readonly());
        let mut req = request("/wham/rate-limit-reset-credits", HttpMethod::Get);
        req.has_body = true;
        assert_eq!(
            guard.validate(&req).unwrap_err(),
            SafetyError::RequestBodyDenied
        );
    }

    #[test]
    fn allows_registered_get_reset_credit_endpoint() {
        let guard = SafetyGuard::new(EndpointRegistry::default_readonly());
        let event = guard
            .validate(&request("/wham/rate-limit-reset-credits", HttpMethod::Get))
            .unwrap();
        assert!(event.allowed);
    }

    #[test]
    fn rejects_unregistered_undocumented_endpoint() {
        let guard = SafetyGuard::new(EndpointRegistry::default_readonly());
        assert_eq!(
            guard
                .validate(&request("/backend-api/not-reviewed", HttpMethod::Get))
                .unwrap_err(),
            SafetyError::UnregisteredEndpoint
        );
    }

    #[test]
    fn rejects_missing_response_schema() {
        let endpoint = EndpointSpec {
            id: "known-reset-credit".to_string(),
            method: HttpMethod::Get,
            scheme: "https".to_string(),
            host: "chatgpt.com".to_string(),
            path: "/wham/rate-limit-reset-credits".to_string(),
            body_allowed: false,
            documented_status: DocumentedStatus::Undocumented,
            readonly_review: true,
            response_schema: String::new(),
            reviewed_at: "2026-07-02".to_string(),
        };
        let guard = SafetyGuard::new(EndpointRegistry::with_endpoint(endpoint));
        assert_eq!(
            guard
                .validate(&request("/wham/rate-limit-reset-credits", HttpMethod::Get))
                .unwrap_err(),
            SafetyError::MissingResponseSchema
        );
    }

    #[test]
    fn rejects_plaintext_transport_for_registered_auth_endpoint() {
        let guard = SafetyGuard::new(EndpointRegistry::default_readonly());
        let req = ConnectorRequest {
            method: HttpMethod::Get,
            url: Url::parse("http://chatgpt.com/wham/rate-limit-reset-credits").unwrap(),
            has_body: false,
            endpoint_id: "known-reset-credit".to_string(),
        };
        assert_eq!(guard.validate(&req).unwrap_err(), SafetyError::UnsafeScheme);
    }
}
