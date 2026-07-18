use crate::codex_app_server::{
    AccountIdentitySnapshot, AccountLaunchDiagnostics, AccountMethodSnapshot,
    AccountRateLimitBucket, AccountRateLimitWindow, AccountRefreshDiagnostics,
    AccountRefreshStatus, AccountResetCreditDetail, AccountResetCreditsSnapshot, AccountSnapshot,
    AccountUsageSnapshot, CodexLaunchMode, MethodStatus,
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration as StdDuration;

const OAUTH_SCHEMA_FINGERPRINT: &str = "codex-oauth-wham-v1";
const AUTH_FILE_LIMIT_BYTES: u64 = 1024 * 1024;
const REFRESH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

#[derive(Debug, Clone)]
pub struct CodexOAuthConfig {
    pub auth_home: PathBuf,
    endpoints: OAuthEndpoints,
}

impl CodexOAuthConfig {
    pub fn production(auth_home: PathBuf) -> Self {
        Self {
            auth_home,
            endpoints: OAuthEndpoints::production(),
        }
    }

    #[cfg(test)]
    fn for_test(auth_home: PathBuf, base_url: &str) -> Self {
        let base = base_url.trim_end_matches('/');
        Self {
            auth_home,
            endpoints: OAuthEndpoints {
                usage: format!("{base}/usage"),
                reset_credits: format!("{base}/credits"),
                refresh: format!("{base}/token"),
            },
        }
    }
}

#[derive(Debug, Clone)]
struct OAuthEndpoints {
    usage: String,
    reset_credits: String,
    refresh: String,
}

impl OAuthEndpoints {
    fn production() -> Self {
        Self {
            usage: "https://chatgpt.com/backend-api/wham/usage".to_string(),
            reset_credits: "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits"
                .to_string(),
            refresh: "https://auth.openai.com/oauth/token".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthErrorKind {
    NotConfigured,
    InvalidCredentials,
    Unauthorized,
    Network,
    InvalidResponse,
    CredentialWrite,
}

#[derive(Debug, Clone)]
pub struct OAuthError {
    pub kind: OAuthErrorKind,
    pub public_message: String,
    pub http_status: Option<i64>,
}

impl OAuthError {
    fn new(kind: OAuthErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            public_message: message.into(),
            http_status: None,
        }
    }

    fn status(kind: OAuthErrorKind, status: u16, message: impl Into<String>) -> Self {
        Self {
            kind,
            public_message: message.into(),
            http_status: Some(status.into()),
        }
    }

    pub fn code(&self) -> &'static str {
        match self.kind {
            OAuthErrorKind::NotConfigured => "not_configured",
            OAuthErrorKind::InvalidCredentials => "invalid_credentials",
            OAuthErrorKind::Unauthorized => "unauthorized",
            OAuthErrorKind::Network => "network_error",
            OAuthErrorKind::InvalidResponse => "invalid_response",
            OAuthErrorKind::CredentialWrite => "credential_write_failed",
        }
    }
}

#[derive(Debug, Clone)]
struct LoadedCredentials {
    access_token: String,
    refresh_token: String,
    id_token: Option<String>,
    account_id: Option<String>,
    last_refresh: Option<DateTime<Utc>>,
    document: Value,
    original_bytes: Vec<u8>,
    path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct AuthDocument {
    tokens: Option<AuthTokens>,
    last_refresh: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthTokens {
    #[serde(alias = "accessToken")]
    access_token: Option<String>,
    #[serde(alias = "refreshToken")]
    refresh_token: Option<String>,
    #[serde(alias = "idToken")]
    id_token: Option<String>,
    #[serde(alias = "accountId")]
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    plan_type: Option<String>,
    rate_limit: Option<RateLimitDetails>,
}

#[derive(Debug, Deserialize)]
struct RateLimitDetails {
    primary_window: Option<WindowSnapshot>,
    secondary_window: Option<WindowSnapshot>,
}

#[derive(Debug, Deserialize)]
struct WindowSnapshot {
    used_percent: f64,
    reset_at: i64,
    limit_window_seconds: i64,
}

#[derive(Debug, Deserialize)]
struct ResetCreditsResponse {
    #[serde(default)]
    credits: Vec<ResetCreditResponse>,
    available_count: i64,
}

#[derive(Debug, Deserialize)]
struct ResetCreditResponse {
    id: String,
    reset_type: String,
    status: String,
    granted_at: String,
    expires_at: Option<String>,
    title: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct RefreshRequest<'a> {
    client_id: &'a str,
    grant_type: &'a str,
    refresh_token: &'a str,
    scope: &'a str,
}

pub fn refresh_oauth_snapshot(config: &CodexOAuthConfig) -> Result<AccountSnapshot, OAuthError> {
    let started = Utc::now();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(StdDuration::from_secs(30)))
        .build()
        .into();
    let mut credentials = load_credentials(&config.auth_home)?;
    let mut refreshed = false;

    if credentials
        .last_refresh
        .is_none_or(|last| Utc::now() - last > Duration::days(8))
        && !credentials.refresh_token.is_empty()
    {
        if let Ok(updated) = refresh_credentials(&agent, &config.endpoints, &credentials) {
            credentials = publish_or_reload(updated)?;
            refreshed = true;
        }
    }

    let usage = match fetch_usage(&agent, &config.endpoints, &credentials) {
        Err(error)
            if error.kind == OAuthErrorKind::Unauthorized
                && !refreshed
                && !credentials.refresh_token.is_empty() =>
        {
            credentials = publish_or_reload(refresh_credentials(
                &agent,
                &config.endpoints,
                &credentials,
            )?)?;
            fetch_usage(&agent, &config.endpoints, &credentials)?
        }
        result => result?,
    };
    let (reset_credits, reset_error) =
        match fetch_reset_credits(&agent, &config.endpoints, &credentials) {
            Ok(reset) => (Some(reset), None),
            Err(error) => (None, Some(error)),
        };
    let mut snapshot = normalize_snapshot(started, usage, reset_credits)?;
    if let Some(error) = reset_error {
        snapshot.status = AccountRefreshStatus::Degraded;
        snapshot.diagnostics.first_failing_stage = Some("oauth/reset-credits".to_string());
        snapshot.diagnostics.redacted_error_code = Some(error.code().to_string());
        snapshot.diagnostics.redacted_error_message = error.public_message;
    }
    Ok(snapshot)
}

fn auth_path(auth_home: &Path) -> PathBuf {
    auth_home.join("auth.json")
}

fn load_credentials(auth_home: &Path) -> Result<LoadedCredentials, OAuthError> {
    let path = auth_path(auth_home);
    let metadata = std::fs::metadata(&path).map_err(|_| {
        OAuthError::new(
            OAuthErrorKind::NotConfigured,
            "Codex OAuth credentials are unavailable; run Codex login.",
        )
    })?;
    if !metadata.is_file() || metadata.len() > AUTH_FILE_LIMIT_BYTES {
        return Err(OAuthError::new(
            OAuthErrorKind::InvalidCredentials,
            "Codex OAuth credentials are invalid.",
        ));
    }
    let original_bytes = std::fs::read(&path).map_err(|_| {
        OAuthError::new(
            OAuthErrorKind::InvalidCredentials,
            "Codex OAuth credentials could not be read.",
        )
    })?;
    let parsed: AuthDocument = serde_json::from_slice(&original_bytes).map_err(|_| {
        OAuthError::new(
            OAuthErrorKind::InvalidCredentials,
            "Codex OAuth credentials contain invalid JSON.",
        )
    })?;
    let document: Value = serde_json::from_slice(&original_bytes).map_err(|_| {
        OAuthError::new(
            OAuthErrorKind::InvalidCredentials,
            "Codex OAuth credentials contain invalid JSON.",
        )
    })?;
    let tokens = parsed.tokens.ok_or_else(|| {
        OAuthError::new(
            OAuthErrorKind::NotConfigured,
            "Codex OAuth tokens are unavailable; run Codex login.",
        )
    })?;
    let access_token = tokens
        .access_token
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            OAuthError::new(
                OAuthErrorKind::NotConfigured,
                "Codex OAuth access is unavailable; run Codex login.",
            )
        })?;
    Ok(LoadedCredentials {
        access_token,
        refresh_token: tokens.refresh_token.unwrap_or_default(),
        id_token: tokens.id_token,
        account_id: tokens.account_id,
        last_refresh: parsed
            .last_refresh
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc)),
        document,
        original_bytes,
        path,
    })
}

fn fetch_usage(
    agent: &ureq::Agent,
    endpoints: &OAuthEndpoints,
    credentials: &LoadedCredentials,
) -> Result<UsageResponse, OAuthError> {
    let mut request = agent
        .get(&endpoints.usage)
        .header(
            "Authorization",
            &format!("Bearer {}", credentials.access_token),
        )
        .header("User-Agent", "TokenStack/0.1")
        .header("Accept", "application/json");
    if let Some(account_id) = credentials.account_id.as_deref() {
        request = request.header("ChatGPT-Account-Id", account_id);
    }
    let mut response = request.call().map_err(map_http_error)?;
    response.body_mut().read_json().map_err(|_| {
        OAuthError::new(
            OAuthErrorKind::InvalidResponse,
            "Codex OAuth usage returned an invalid response.",
        )
    })
}

fn fetch_reset_credits(
    agent: &ureq::Agent,
    endpoints: &OAuthEndpoints,
    credentials: &LoadedCredentials,
) -> Result<ResetCreditsResponse, OAuthError> {
    let mut request = agent
        .get(&endpoints.reset_credits)
        .header(
            "Authorization",
            &format!("Bearer {}", credentials.access_token),
        )
        .header("User-Agent", "TokenStack/0.1")
        .header("Accept", "application/json")
        .header("OpenAI-Beta", "codex-1")
        .header("originator", "Codex Desktop");
    if let Some(account_id) = credentials.account_id.as_deref() {
        request = request.header("ChatGPT-Account-ID", account_id);
    }
    let mut response = request.call().map_err(map_http_error)?;
    let parsed: ResetCreditsResponse = response.body_mut().read_json().map_err(|_| {
        OAuthError::new(
            OAuthErrorKind::InvalidResponse,
            "Codex OAuth reset credits returned an invalid response.",
        )
    })?;
    if parsed.available_count < 0 {
        return Err(OAuthError::new(
            OAuthErrorKind::InvalidResponse,
            "Codex OAuth reset credits returned an invalid count.",
        ));
    }
    Ok(parsed)
}

fn refresh_credentials(
    agent: &ureq::Agent,
    endpoints: &OAuthEndpoints,
    credentials: &LoadedCredentials,
) -> Result<LoadedCredentials, OAuthError> {
    let body = RefreshRequest {
        client_id: REFRESH_CLIENT_ID,
        grant_type: "refresh_token",
        refresh_token: &credentials.refresh_token,
        scope: "openid profile email",
    };
    let mut response = agent
        .post(&endpoints.refresh)
        .header("Content-Type", "application/json")
        .header("User-Agent", "TokenStack/0.1")
        .send_json(&body)
        .map_err(map_refresh_error)?;
    let parsed: RefreshResponse = response.body_mut().read_json().map_err(|_| {
        OAuthError::new(
            OAuthErrorKind::InvalidResponse,
            "Codex OAuth token refresh returned an invalid response.",
        )
    })?;
    let access_token = parsed
        .access_token
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            OAuthError::new(
                OAuthErrorKind::InvalidResponse,
                "Codex OAuth token refresh omitted the access token.",
            )
        })?;
    let mut updated = credentials.clone();
    updated.access_token = access_token;
    updated.refresh_token = parsed
        .refresh_token
        .unwrap_or_else(|| updated.refresh_token.clone());
    updated.id_token = parsed.id_token.or(updated.id_token);
    updated.last_refresh = Some(Utc::now());
    Ok(updated)
}

fn map_http_error(error: ureq::Error) -> OAuthError {
    match error {
        ureq::Error::StatusCode(status @ (401 | 403)) => OAuthError::status(
            OAuthErrorKind::Unauthorized,
            status,
            "Codex OAuth access expired; run Codex login.",
        ),
        ureq::Error::StatusCode(status) => OAuthError::status(
            OAuthErrorKind::InvalidResponse,
            status,
            format!("Codex OAuth usage request failed with status {status}."),
        ),
        _ => OAuthError::new(
            OAuthErrorKind::Network,
            "Codex OAuth usage could not reach the account service.",
        ),
    }
}

fn map_refresh_error(error: ureq::Error) -> OAuthError {
    match error {
        ureq::Error::StatusCode(status @ (400 | 401 | 403)) => OAuthError::status(
            OAuthErrorKind::Unauthorized,
            status,
            "Codex OAuth refresh was rejected; run Codex login.",
        ),
        ureq::Error::StatusCode(status) => OAuthError::status(
            OAuthErrorKind::InvalidResponse,
            status,
            format!("Codex OAuth refresh failed with status {status}."),
        ),
        _ => OAuthError::new(
            OAuthErrorKind::Network,
            "Codex OAuth refresh could not reach the authentication service.",
        ),
    }
}

fn publish_or_reload(mut credentials: LoadedCredentials) -> Result<LoadedCredentials, OAuthError> {
    let current = std::fs::read(&credentials.path).map_err(|_| {
        OAuthError::new(
            OAuthErrorKind::CredentialWrite,
            "Codex OAuth credentials changed during refresh.",
        )
    })?;
    if current != credentials.original_bytes {
        let auth_home = credentials.path.parent().unwrap_or_else(|| Path::new("."));
        return load_credentials(auth_home);
    }
    update_document(&mut credentials)?;
    let bytes = serde_json::to_vec_pretty(&credentials.document).map_err(|_| {
        OAuthError::new(
            OAuthErrorKind::CredentialWrite,
            "Codex OAuth credentials could not be updated safely.",
        )
    })?;
    atomic_replace(&credentials.path, &bytes)?;
    credentials.original_bytes = bytes;
    Ok(credentials)
}

fn update_document(credentials: &mut LoadedCredentials) -> Result<(), OAuthError> {
    let root = credentials.document.as_object_mut().ok_or_else(|| {
        OAuthError::new(
            OAuthErrorKind::InvalidCredentials,
            "Codex OAuth credentials have an invalid structure.",
        )
    })?;
    let tokens = root
        .entry("tokens")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            OAuthError::new(
                OAuthErrorKind::InvalidCredentials,
                "Codex OAuth credentials have an invalid token structure.",
            )
        })?;
    tokens.insert(
        "access_token".to_string(),
        Value::String(credentials.access_token.clone()),
    );
    tokens.insert(
        "refresh_token".to_string(),
        Value::String(credentials.refresh_token.clone()),
    );
    if let Some(id_token) = &credentials.id_token {
        tokens.insert("id_token".to_string(), Value::String(id_token.clone()));
    }
    if let Some(account_id) = &credentials.account_id {
        tokens.insert("account_id".to_string(), Value::String(account_id.clone()));
    }
    root.insert(
        "last_refresh".to_string(),
        Value::String(Utc::now().to_rfc3339()),
    );
    Ok(())
}

fn atomic_replace(path: &Path, contents: &[u8]) -> Result<(), OAuthError> {
    let parent = path.parent().ok_or_else(|| {
        OAuthError::new(
            OAuthErrorKind::CredentialWrite,
            "Codex OAuth credential location is invalid.",
        )
    })?;
    let temp = parent.join(format!(
        ".auth.json.tokenstack-{}-{}.tmp",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(contents)?;
        file.sync_all()?;
        replace_file(&temp, path)?;
        Ok::<(), std::io::Error>(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(|_| {
        OAuthError::new(
            OAuthErrorKind::CredentialWrite,
            "Codex OAuth credentials could not be updated safely.",
        )
    })
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn normalize_snapshot(
    started: DateTime<Utc>,
    usage: UsageResponse,
    reset: Option<ResetCreditsResponse>,
) -> Result<AccountSnapshot, OAuthError> {
    let windows = usage
        .rate_limit
        .map(|limits| {
            [
                limits.primary_window.map(|window| ("Session", window)),
                limits.secondary_window.map(|window| ("Weekly", window)),
            ]
            .into_iter()
            .flatten()
            .map(|(label, window)| normalize_window(label, window))
            .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    if windows.is_empty() {
        return Err(OAuthError::new(
            OAuthErrorKind::InvalidResponse,
            "Codex OAuth usage did not include rate-limit windows.",
        ));
    }
    let reset_credits = reset
        .map(|reset| {
            let expires_at_utc = reset
                .credits
                .iter()
                .filter(|credit| credit.status == "available")
                .filter_map(|credit| credit.expires_at.clone())
                .min();
            AccountResetCreditsSnapshot {
                available_count: Some(reset.available_count),
                expires_at_utc,
                credits: Some(
                    reset
                        .credits
                        .into_iter()
                        .map(|credit| AccountResetCreditDetail {
                            id: credit.id,
                            reset_type: credit.reset_type,
                            status: credit.status,
                            granted_at_utc: credit.granted_at,
                            expires_at_utc: credit.expires_at,
                            title: credit.title,
                            description: credit.description,
                        })
                        .collect(),
                ),
            }
        })
        .unwrap_or_default();
    Ok(AccountSnapshot {
        status: AccountRefreshStatus::Connected,
        launch: AccountLaunchDiagnostics {
            selected_executable: String::new(),
            argv_prefix: Vec::new(),
            mode: CodexLaunchMode::OAuthApi,
            candidates: Vec::new(),
        },
        diagnostics: AccountRefreshDiagnostics {
            started_at_utc: started.to_rfc3339(),
            completed_at_utc: Utc::now().to_rfc3339(),
            first_failing_stage: None,
            redacted_error_code: None,
            redacted_error_message: String::new(),
            stderr_tail: String::new(),
            used_last_good_snapshot: false,
            schema_fingerprint: OAUTH_SCHEMA_FINGERPRINT.to_string(),
            exit_code: None,
            child_terminated: true,
        },
        account: AccountIdentitySnapshot {
            account_label: None,
            plan: usage.plan_type,
        },
        usage: AccountUsageSnapshot::default(),
        reset_credits,
        rate_limits: vec![AccountRateLimitBucket {
            bucket_id: "codex-oauth".to_string(),
            display_name: "Codex".to_string(),
            windows,
        }],
        methods: vec![AccountMethodSnapshot {
            method: "account/rateLimits/read".to_string(),
            status: MethodStatus::Ok,
            redacted_error: None,
        }],
    })
}

fn normalize_window(
    label: &str,
    window: WindowSnapshot,
) -> Result<AccountRateLimitWindow, OAuthError> {
    if !window.used_percent.is_finite()
        || !(0.0..=100.0).contains(&window.used_percent)
        || window.limit_window_seconds <= 0
    {
        return Err(OAuthError::new(
            OAuthErrorKind::InvalidResponse,
            "Codex OAuth usage included an invalid rate-limit window.",
        ));
    }
    let resets_at_utc = Utc
        .timestamp_opt(window.reset_at, 0)
        .single()
        .ok_or_else(|| {
            OAuthError::new(
                OAuthErrorKind::InvalidResponse,
                "Codex OAuth usage included an invalid reset timestamp.",
            )
        })?
        .to_rfc3339();
    Ok(AccountRateLimitWindow {
        window_duration_mins: Some(window.limit_window_seconds / 60),
        window_label: label.to_string(),
        used_percent: window.used_percent,
        remaining_percent: 100.0 - window.used_percent,
        resets_at_utc: Some(resets_at_utc),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;
    use tiny_http::{Header, Response, Server};

    #[test]
    fn oauth_refresh_preserves_auth_fields_and_maps_authoritative_windows() {
        let auth_home = tempdir().unwrap();
        std::fs::write(
            auth_home.path().join("auth.json"),
            r#"{
              "tokens": {
                "access_token": "old-a",
                "refresh_token": "old-r",
                "id_token": "old-id",
                "account_id": "account-fixture"
              },
              "last_refresh": "2020-01-01T00:00:00Z",
              "preserved": {"setting": true}
            }"#,
        )
        .unwrap();
        let server = Server::http("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", server.server_addr());
        let observations = Arc::new(Mutex::new(Vec::<String>::new()));
        let observations_for_server = Arc::clone(&observations);
        let handle = std::thread::spawn(move || {
            for _ in 0..3 {
                let mut request = server.recv().unwrap();
                let authorization = request
                    .headers()
                    .iter()
                    .find(|header| header.field.equiv("Authorization"))
                    .map(|header| header.value.as_str().to_string())
                    .unwrap_or_default();
                observations_for_server
                    .lock()
                    .unwrap()
                    .push(format!("{} {authorization}", request.url()));
                let (status, body) = match request.url() {
                    "/token" => {
                        let mut request_body = String::new();
                        request
                            .as_reader()
                            .read_to_string(&mut request_body)
                            .unwrap();
                        assert!(request_body.contains("old-r"));
                        (
                            200,
                            r#"{"access_token":"new-a","refresh_token":"new-r","id_token":"new-id"}"#,
                        )
                    }
                    "/usage" => (
                        200,
                        r#"{"plan_type":"pro","rate_limit":{"primary_window":{"used_percent":25,"reset_at":1784400000,"limit_window_seconds":18000},"secondary_window":{"used_percent":40,"reset_at":1785004800,"limit_window_seconds":604800}}}"#,
                    ),
                    "/credits" => (
                        200,
                        r#"{"available_count":2,"credits":[{"id":"credit-1","reset_type":"manual","status":"available","granted_at":"2026-07-01T00:00:00Z","expires_at":"2026-08-01T00:00:00Z","title":"Reset","description":"Synthetic"}]}"#,
                    ),
                    _ => (404, "{}"),
                };
                let response = Response::from_string(body)
                    .with_status_code(status)
                    .with_header(Header::from_bytes("Content-Type", "application/json").unwrap());
                request.respond(response).unwrap();
            }
        });

        let snapshot = refresh_oauth_snapshot(&CodexOAuthConfig::for_test(
            auth_home.path().to_path_buf(),
            &base_url,
        ))
        .unwrap();
        handle.join().unwrap();

        assert_eq!(snapshot.account.plan.as_deref(), Some("pro"));
        assert_eq!(snapshot.launch.mode, CodexLaunchMode::OAuthApi);
        assert_eq!(snapshot.rate_limits[0].windows.len(), 2);
        assert_eq!(snapshot.rate_limits[0].windows[0].used_percent, 25.0);
        assert_eq!(snapshot.reset_credits.available_count, Some(2));
        assert_eq!(snapshot.reset_credits.credits.as_ref().unwrap().len(), 1);
        let conn = crate::db::open_memory().unwrap();
        crate::db::insert_account_snapshot(&conn, &snapshot).unwrap();
        let summary = crate::analytics::build_dashboard_summary(&conn, "remote").unwrap();
        assert_eq!(summary.rate_limit_windows.len(), 2);
        assert!(summary
            .connectors
            .iter()
            .find(|connector| connector.id == "rate-limit-windows")
            .unwrap()
            .detail
            .contains("OAuth"));
        let observations = observations.lock().unwrap();
        assert!(observations
            .iter()
            .any(|entry| entry == "/usage Bearer new-a"));
        assert!(observations
            .iter()
            .any(|entry| entry == "/credits Bearer new-a"));

        let updated: Value =
            serde_json::from_slice(&std::fs::read(auth_home.path().join("auth.json")).unwrap())
                .unwrap();
        assert_eq!(updated["tokens"]["access_token"], "new-a");
        assert_eq!(updated["tokens"]["refresh_token"], "new-r");
        assert_eq!(updated["preserved"]["setting"], true);
        assert!(auth_home.path().read_dir().unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("tokenstack")));
    }

    #[test]
    fn credential_errors_never_include_auth_paths_or_token_values() {
        let auth_home = tempdir().unwrap();
        std::fs::write(
            auth_home.path().join("auth.json"),
            b"synthetic-secret-token",
        )
        .unwrap();

        let error = refresh_oauth_snapshot(&CodexOAuthConfig::for_test(
            auth_home.path().to_path_buf(),
            "http://127.0.0.1:1",
        ))
        .unwrap_err();

        assert_eq!(error.kind, OAuthErrorKind::InvalidCredentials);
        assert!(!error.public_message.contains("auth.json"));
        assert!(!error.public_message.contains("synthetic-secret-token"));
        assert!(!error
            .public_message
            .contains(&auth_home.path().display().to_string()));
    }

    #[test]
    fn invalid_windows_fail_closed() {
        let error = normalize_snapshot(
            Utc::now(),
            UsageResponse {
                plan_type: Some("pro".to_string()),
                rate_limit: Some(RateLimitDetails {
                    primary_window: Some(WindowSnapshot {
                        used_percent: 101.0,
                        reset_at: 1,
                        limit_window_seconds: 300,
                    }),
                    secondary_window: None,
                }),
            },
            None,
        )
        .unwrap_err();

        assert_eq!(error.kind, OAuthErrorKind::InvalidResponse);
    }
}
