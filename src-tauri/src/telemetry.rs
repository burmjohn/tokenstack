#[cfg(test)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(test)]
pub struct PublicError {
    pub code: String,
    pub message: String,
}

pub fn redact_sensitive(input: &str) -> String {
    let mut redact_next = false;
    input
        .split_whitespace()
        .map(|part| {
            let lower = part.to_ascii_lowercase();
            let is_marker = ["access_token", "refresh_token", "authorization", "bearer"]
                .iter()
                .any(|marker| lower.contains(marker));
            if redact_next || is_marker || looks_like_secret(part) {
                redact_next = is_marker;
                "[REDACTED]".to_string()
            } else {
                redact_next = false;
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn looks_like_secret(part: &str) -> bool {
    let trimmed =
        part.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.');
    trimmed.starts_with("sk-")
        || trimmed.len() >= 32
            && trimmed.chars().any(|c| c.is_ascii_digit())
            && trimmed.chars().any(|c| c.is_ascii_uppercase())
}

#[cfg(test)]
pub fn public_error(code: &str, message: &str) -> PublicError {
    PublicError {
        code: code.to_string(),
        message: redact_sensitive(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_auth_values_in_errors() {
        let token_like = format!("{}{}", "sk-", "syntheticSecretValue1234567890");
        let error = public_error(
            "connector_failed",
            &format!("authorization Bearer {token_like} access_token synthetic-access-value"),
        );
        assert!(!error.message.contains(&token_like));
        assert!(!error.message.contains("synthetic-access-value"));
        assert!(error.message.contains("[REDACTED]"));
    }
}
