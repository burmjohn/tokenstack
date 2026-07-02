use crate::telemetry::redact_sensitive;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthError {
    #[error("auth path is not allowlisted")]
    PathDenied,
    #[error("auth document is malformed")]
    Malformed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthMetadata {
    pub available: bool,
    pub account_label: Option<String>,
    pub connector_status: String,
}

#[derive(Debug, Clone)]
pub struct AuthHandle {
    token: SecretString,
    metadata: AuthMetadata,
}

impl AuthHandle {
    pub fn from_token(token: SecretString, account_label: Option<String>) -> Self {
        Self {
            token,
            metadata: AuthMetadata {
                available: true,
                account_label,
                connector_status: "available".to_string(),
            },
        }
    }

    pub fn metadata(&self) -> &AuthMetadata {
        &self.metadata
    }

    pub fn bearer_header(&self) -> String {
        format!("Bearer {}", self.token.expose_secret())
    }
}

#[derive(Debug, Clone)]
pub struct AuthLocator {
    home: PathBuf,
}

impl AuthLocator {
    pub fn new(home: PathBuf) -> Self {
        Self { home }
    }

    pub fn allowed_path(&self, candidate: &Path) -> Result<PathBuf, AuthError> {
        let allowed = [
            self.home.join(".codex").join("auth.json"),
            self.home.join(".config").join("codex").join("auth.json"),
        ];
        let normalized = candidate.components().collect::<PathBuf>();
        if allowed.iter().any(|path| path == &normalized) {
            Ok(normalized)
        } else {
            Err(AuthError::PathDenied)
        }
    }
}

pub fn parse_auth_document(input: &str) -> Result<AuthHandle, AuthError> {
    let value: serde_json::Value = serde_json::from_str(input).map_err(|_| AuthError::Malformed)?;
    let token = value
        .pointer("/tokens/access_token")
        .or_else(|| value.pointer("/access_token"))
        .and_then(|value| value.as_str())
        .ok_or(AuthError::Malformed)?;
    let account_label = value
        .pointer("/account/label")
        .or_else(|| value.pointer("/user/email"))
        .and_then(|value| value.as_str())
        .map(redact_account_label);

    Ok(AuthHandle::from_token(
        SecretString::from(token.to_string()),
        account_label,
    ))
}

fn redact_account_label(label: &str) -> String {
    if let Some((name, domain)) = label.split_once('@') {
        let first = name.chars().next().unwrap_or('*');
        let safe_domain = domain.split('.').next().unwrap_or("account");
        format!("{first}***@{safe_domain}.*")
    } else {
        redact_sensitive(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_locator_reads_only_allowed_paths() {
        let home = PathBuf::from("/home/tester");
        let locator = AuthLocator::new(home.clone());

        assert!(locator.allowed_path(&home.join(".codex/auth.json")).is_ok());
        assert!(locator
            .allowed_path(&home.join(".config/codex/auth.json"))
            .is_ok());
        assert_eq!(
            locator.allowed_path(&home.join(".ssh/id_rsa")).unwrap_err(),
            AuthError::PathDenied
        );
    }

    #[test]
    fn auth_parser_extracts_minimum_required_fields() {
        let handle = parse_auth_document(
            r#"{"tokens":{"access_token":"fixture"},"user":{"email":"tester@example.invalid"}}"#,
        )
        .unwrap();

        assert_eq!(handle.metadata().available, true);
        assert_eq!(
            handle.metadata().account_label.as_deref(),
            Some("t***@example.*")
        );
        assert!(handle.bearer_header().contains("fixture"));
    }

    #[test]
    fn auth_handle_never_serializes_secret() {
        let handle = parse_auth_document(r#"{"access_token":"fixture"}"#).unwrap();
        let payload = serde_json::to_string(handle.metadata()).unwrap();

        assert!(payload.contains("available"));
        assert!(!payload.contains("fixture"));
    }
}
