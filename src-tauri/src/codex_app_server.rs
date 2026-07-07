use crate::telemetry::redact_sensitive;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(test)]
pub const ACCOUNT_READ_METHODS: [&str; 3] = [
    "account/read",
    "account/rateLimits/read",
    "account/usage/read",
];

#[derive(Debug, Clone)]
pub struct CodexAppServerConfig {
    pub explicit_codex_path: Option<PathBuf>,
    pub initialize_timeout: Duration,
    pub request_timeout: Duration,
    pub whole_refresh_timeout: Duration,
}

impl Default for CodexAppServerConfig {
    fn default() -> Self {
        Self {
            explicit_codex_path: None,
            initialize_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(12),
            whole_refresh_timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountRefreshStatus {
    Connected,
    Degraded,
    Unavailable,
}

impl AccountRefreshStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Degraded => "degraded",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexLaunchMode {
    ListenStdioNoMcp,
    PlainAppServerFallback,
}

impl CodexLaunchMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ListenStdioNoMcp => "listen_stdio_no_mcp",
            Self::PlainAppServerFallback => "plain_app_server_fallback",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MethodStatus {
    Ok,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountConnectorErrorKind {
    MissingCli,
    UnsupportedCli,
    LoggedOut,
    Timeout,
    Protocol,
    Spawn,
}

#[derive(Debug, Clone)]
pub struct AccountConnectorError {
    pub kind: AccountConnectorErrorKind,
    pub stage: String,
    pub public_message: String,
}

impl AccountConnectorError {
    fn new(
        kind: AccountConnectorErrorKind,
        stage: impl Into<String>,
        message: impl AsRef<str>,
    ) -> Self {
        Self {
            kind,
            stage: stage.into(),
            public_message: redact_sensitive(message.as_ref()),
        }
    }

    fn logged_out(stage: impl Into<String>, message: impl AsRef<str>) -> Self {
        Self {
            kind: AccountConnectorErrorKind::LoggedOut,
            stage: stage.into(),
            public_message: format!(
                "Codex login required: {}",
                redact_sensitive(message.as_ref())
            ),
        }
    }
}

impl std::fmt::Display for AccountConnectorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.stage, self.public_message)
    }
}

impl std::error::Error for AccountConnectorError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSnapshot {
    pub status: AccountRefreshStatus,
    pub launch: AccountLaunchDiagnostics,
    pub diagnostics: AccountRefreshDiagnostics,
    pub account: AccountIdentitySnapshot,
    pub usage: AccountUsageSnapshot,
    pub reset_credits: AccountResetCreditsSnapshot,
    pub rate_limits: Vec<AccountRateLimitBucket>,
    pub methods: Vec<AccountMethodSnapshot>,
}

impl AccountSnapshot {
    #[cfg(test)]
    pub fn method_status(&self, method: &str) -> Option<MethodStatus> {
        self.methods
            .iter()
            .find(|entry| entry.method == method)
            .map(|entry| entry.status)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountLaunchDiagnostics {
    pub selected_executable: String,
    pub mode: CodexLaunchMode,
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRefreshDiagnostics {
    pub started_at_utc: String,
    pub completed_at_utc: String,
    pub first_failing_stage: Option<String>,
    pub redacted_error_code: Option<String>,
    pub redacted_error_message: String,
    pub stderr_tail: String,
    pub used_last_good_snapshot: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountIdentitySnapshot {
    pub account_label: Option<String>,
    pub plan: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUsageSnapshot {
    pub lifetime_tokens: Option<i64>,
    pub daily_buckets: Vec<AccountDailyUsageBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDailyUsageBucket {
    pub date: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountResetCreditsSnapshot {
    pub available_count: Option<i64>,
    pub expires_at_utc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRateLimitBucket {
    pub bucket_id: String,
    pub display_name: String,
    pub windows: Vec<AccountRateLimitWindow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRateLimitWindow {
    pub window_duration_mins: i64,
    pub window_label: String,
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub resets_at_utc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountMethodSnapshot {
    pub method: String,
    pub status: MethodStatus,
    pub redacted_error: Option<String>,
}

#[derive(Debug, Clone)]
struct CodexExecutable {
    path: PathBuf,
    candidates: Vec<String>,
}

#[derive(Debug)]
struct RateLimitNormalization {
    reset_credits: AccountResetCreditsSnapshot,
    rate_limits: Vec<AccountRateLimitBucket>,
}

pub fn refresh_account_snapshot(
    config: CodexAppServerConfig,
) -> Result<AccountSnapshot, AccountConnectorError> {
    let started = Utc::now();
    let deadline = Instant::now() + config.whole_refresh_timeout;
    let executable = resolve_codex_executable(config.explicit_codex_path.as_deref())?;

    let primary = run_refresh_attempt(
        &executable,
        CodexLaunchMode::ListenStdioNoMcp,
        &config,
        deadline,
        started,
    );

    match primary {
        Ok(snapshot) => Ok(snapshot),
        Err(error) if error.kind == AccountConnectorErrorKind::UnsupportedCli => {
            run_refresh_attempt(
                &executable,
                CodexLaunchMode::PlainAppServerFallback,
                &config,
                deadline,
                started,
            )
        }
        Err(error) => Err(error),
    }
}

fn run_refresh_attempt(
    executable: &CodexExecutable,
    mode: CodexLaunchMode,
    config: &CodexAppServerConfig,
    deadline: Instant,
    started: DateTime<Utc>,
) -> Result<AccountSnapshot, AccountConnectorError> {
    let mut client = JsonRpcClient::spawn(&executable.path, mode)?;
    let initialize_timeout =
        remaining_timeout(deadline, config.initialize_timeout).ok_or_else(|| {
            AccountConnectorError::new(
                AccountConnectorErrorKind::Timeout,
                "initialize",
                "refresh deadline exceeded",
            )
        })?;
    let initialize = client.request("initialize", Some(initialize_params()), initialize_timeout);
    match initialize {
        Ok(_) => {}
        Err(error) if mode == CodexLaunchMode::ListenStdioNoMcp => {
            thread::sleep(Duration::from_millis(25));
            let stderr = client.stderr_tail();
            if error.looks_like_unsupported_cli(&stderr)
                || looks_like_arg_rejection(&format!("{} {stderr}", error.public_message))
            {
                return Err(AccountConnectorError::new(
                    AccountConnectorErrorKind::UnsupportedCli,
                    "launch",
                    if stderr.is_empty() {
                        error.public_message
                    } else {
                        stderr
                    },
                ));
            }
            return Err(error);
        }
        Err(error) => return Err(error),
    }
    client.notify("initialized", Some(json!({})))?;

    let mut methods = Vec::new();
    let mut first_failure: Option<(String, String)> = None;
    let mut status = AccountRefreshStatus::Connected;

    let account = match request_account_read(&mut client, config, deadline) {
        Ok(account) => {
            methods.push(method_ok("account/read"));
            account
        }
        Err(error) if error.kind == AccountConnectorErrorKind::LoggedOut => return Err(error),
        Err(error) => return Err(error),
    };

    let (rate_limits, reset_credits) = match request_rate_limits(&mut client, config, deadline) {
        Ok(normalized) => {
            methods.push(method_ok("account/rateLimits/read"));
            (normalized.rate_limits, normalized.reset_credits)
        }
        Err(error) => {
            status = AccountRefreshStatus::Degraded;
            first_failure = Some((
                "account/rateLimits/read".to_string(),
                error.public_message.clone(),
            ));
            methods.push(method_failed(
                "account/rateLimits/read",
                &error.public_message,
            ));
            (Vec::new(), AccountResetCreditsSnapshot::default())
        }
    };

    let usage = match request_usage(&mut client, config, deadline) {
        Ok(usage) => {
            methods.push(method_ok("account/usage/read"));
            usage
        }
        Err(error) => {
            status = AccountRefreshStatus::Degraded;
            if first_failure.is_none() {
                first_failure = Some((
                    "account/usage/read".to_string(),
                    error.public_message.clone(),
                ));
            }
            methods.push(method_failed("account/usage/read", &error.public_message));
            AccountUsageSnapshot::default()
        }
    };

    let (first_failing_stage, redacted_error_message) = first_failure
        .map(|(stage, message)| (Some(stage), message))
        .unwrap_or_else(|| (None, String::new()));

    Ok(AccountSnapshot {
        status,
        launch: AccountLaunchDiagnostics {
            selected_executable: executable.path.display().to_string(),
            mode,
            candidates: executable.candidates.clone(),
        },
        diagnostics: AccountRefreshDiagnostics {
            started_at_utc: started.to_rfc3339(),
            completed_at_utc: Utc::now().to_rfc3339(),
            first_failing_stage,
            redacted_error_code: if redacted_error_message.is_empty() {
                None
            } else {
                Some("account_method_failed".to_string())
            },
            redacted_error_message,
            stderr_tail: client.stderr_tail(),
            used_last_good_snapshot: false,
        },
        account,
        usage,
        reset_credits,
        rate_limits,
        methods,
    })
}

fn request_account_read(
    client: &mut JsonRpcClient,
    config: &CodexAppServerConfig,
    deadline: Instant,
) -> Result<AccountIdentitySnapshot, AccountConnectorError> {
    let timeout = remaining_timeout(deadline, config.request_timeout).ok_or_else(|| {
        AccountConnectorError::new(
            AccountConnectorErrorKind::Timeout,
            "account/read",
            "refresh deadline exceeded",
        )
    })?;
    let result = client.request("account/read", None, timeout)?;
    Ok(normalize_account(&result))
}

fn request_rate_limits(
    client: &mut JsonRpcClient,
    config: &CodexAppServerConfig,
    deadline: Instant,
) -> Result<RateLimitNormalization, AccountConnectorError> {
    let timeout = remaining_timeout(deadline, config.request_timeout).ok_or_else(|| {
        AccountConnectorError::new(
            AccountConnectorErrorKind::Timeout,
            "account/rateLimits/read",
            "refresh deadline exceeded",
        )
    })?;
    let result = client.request("account/rateLimits/read", None, timeout)?;
    normalize_rate_limits(&result)
}

fn request_usage(
    client: &mut JsonRpcClient,
    config: &CodexAppServerConfig,
    deadline: Instant,
) -> Result<AccountUsageSnapshot, AccountConnectorError> {
    let timeout = remaining_timeout(deadline, config.request_timeout).ok_or_else(|| {
        AccountConnectorError::new(
            AccountConnectorErrorKind::Timeout,
            "account/usage/read",
            "refresh deadline exceeded",
        )
    })?;
    let result = client.request("account/usage/read", None, timeout)?;
    Ok(normalize_usage(&result))
}

fn method_ok(method: &str) -> AccountMethodSnapshot {
    AccountMethodSnapshot {
        method: method.to_string(),
        status: MethodStatus::Ok,
        redacted_error: None,
    }
}

fn method_failed(method: &str, error: &str) -> AccountMethodSnapshot {
    AccountMethodSnapshot {
        method: method.to_string(),
        status: MethodStatus::Failed,
        redacted_error: Some(redact_sensitive(error)),
    }
}

fn initialize_params() -> Value {
    json!({
        "clientInfo": {
            "name": "tokenstack",
            "version": env!("CARGO_PKG_VERSION")
        },
        "capabilities": {
            "experimentalApi": true
        }
    })
}

fn remaining_timeout(deadline: Instant, preferred: Duration) -> Option<Duration> {
    let now = Instant::now();
    if now >= deadline {
        return None;
    }
    Some((deadline - now).min(preferred))
}

fn resolve_codex_executable(
    explicit: Option<&Path>,
) -> Result<CodexExecutable, AccountConnectorError> {
    let mut candidates = Vec::new();
    if let Some(path) = explicit {
        candidates.push(path.display().to_string());
        if path.exists() {
            return Ok(CodexExecutable {
                path: path.to_path_buf(),
                candidates,
            });
        }
        return Err(AccountConnectorError::new(
            AccountConnectorErrorKind::MissingCli,
            "resolve_codex",
            format!(
                "configured Codex executable does not exist: {}",
                path.display()
            ),
        ));
    }

    if let Some(path) = env::var_os("TOKENSTACK_CODEX_BIN").map(PathBuf::from) {
        candidates.push(path.display().to_string());
        if path.exists() {
            return Ok(CodexExecutable { path, candidates });
        }
        return Err(AccountConnectorError::new(
            AccountConnectorErrorKind::MissingCli,
            "resolve_codex",
            "TOKENSTACK_CODEX_BIN points to a missing Codex executable",
        ));
    }

    for candidate in path_candidates("codex") {
        candidates.push(candidate.display().to_string());
        if candidate.exists() {
            return Ok(CodexExecutable {
                path: candidate,
                candidates,
            });
        }
    }

    Err(AccountConnectorError::new(
        AccountConnectorErrorKind::MissingCli,
        "resolve_codex",
        "Codex CLI was not found on PATH. Configure the Codex executable or set TOKENSTACK_CODEX_BIN.",
    ))
}

fn path_candidates(binary: &str) -> Vec<PathBuf> {
    let Some(paths) = env::var_os("PATH") else {
        return Vec::new();
    };
    let suffixes = executable_suffixes();
    env::split_paths(&paths)
        .flat_map(|dir| {
            suffixes
                .iter()
                .map(move |suffix| dir.join(format!("{binary}{suffix}")))
        })
        .collect()
}

#[cfg(windows)]
fn executable_suffixes() -> Vec<String> {
    env::var("PATHEXT")
        .unwrap_or_else(|_| ".EXE;.CMD;.BAT".to_string())
        .split(';')
        .map(|suffix| suffix.to_ascii_lowercase())
        .chain(std::iter::once(String::new()))
        .collect()
}

#[cfg(not(windows))]
fn executable_suffixes() -> Vec<String> {
    vec![String::new()]
}

struct JsonRpcClient {
    process: AppServerProcess,
    next_id: i64,
}

impl JsonRpcClient {
    fn spawn(path: &Path, mode: CodexLaunchMode) -> Result<Self, AccountConnectorError> {
        let mut args = vec!["app-server".to_string()];
        if mode == CodexLaunchMode::ListenStdioNoMcp {
            args.extend([
                "--listen".to_string(),
                "stdio://".to_string(),
                "-c".to_string(),
                "mcp_servers={}".to_string(),
            ]);
        }
        let process = AppServerProcess::spawn(path, &args, mode)?;
        Ok(Self {
            process,
            next_id: 1,
        })
    }

    fn request(
        &mut self,
        method: &str,
        params: Option<Value>,
        timeout: Duration,
    ) -> Result<Value, AccountConnectorError> {
        let id = self.next_id;
        self.next_id += 1;
        let mut request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method
        });
        if let Some(params) = params {
            request["params"] = params;
        }
        self.process.send(&request, method)?;
        self.process.await_response(id, method, timeout)
    }

    fn notify(&mut self, method: &str, params: Option<Value>) -> Result<(), AccountConnectorError> {
        let mut notification = json!({
            "jsonrpc": "2.0",
            "method": method
        });
        if let Some(params) = params {
            notification["params"] = params;
        }
        self.process.send(&notification, method)
    }

    fn stderr_tail(&self) -> String {
        self.process.stderr_tail()
    }
}

struct AppServerProcess {
    child: Child,
    stdin: ChildStdin,
    stdout_rx: mpsc::Receiver<String>,
    stderr: Arc<Mutex<String>>,
}

impl AppServerProcess {
    fn spawn(
        path: &Path,
        args: &[String],
        mode: CodexLaunchMode,
    ) -> Result<Self, AccountConnectorError> {
        let mut child = Command::new(path)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                AccountConnectorError::new(
                    AccountConnectorErrorKind::Spawn,
                    "launch",
                    format!("failed to spawn Codex app-server ({mode:?}): {error}"),
                )
            })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            AccountConnectorError::new(
                AccountConnectorErrorKind::Spawn,
                "launch",
                "failed to open app-server stdin",
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            AccountConnectorError::new(
                AccountConnectorErrorKind::Spawn,
                "launch",
                "failed to open app-server stdout",
            )
        })?;
        let stderr_pipe = child.stderr.take().ok_or_else(|| {
            AccountConnectorError::new(
                AccountConnectorErrorKind::Spawn,
                "launch",
                "failed to open app-server stderr",
            )
        })?;

        let (stdout_tx, stdout_rx) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => {
                        if stdout_tx.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let stderr = Arc::new(Mutex::new(String::new()));
        let stderr_for_thread = Arc::clone(&stderr);
        thread::spawn(move || {
            for line in BufReader::new(stderr_pipe).lines().map_while(Result::ok) {
                let mut buffer = stderr_for_thread.lock().expect("stderr buffer poisoned");
                if !buffer.is_empty() {
                    buffer.push('\n');
                }
                buffer.push_str(&redact_sensitive(&line));
                if buffer.len() > 4_096 {
                    let keep_from = buffer.len() - 4_096;
                    *buffer = buffer[keep_from..].to_string();
                }
            }
        });

        Ok(Self {
            child,
            stdin,
            stdout_rx,
            stderr,
        })
    }

    fn send(&mut self, value: &Value, stage: &str) -> Result<(), AccountConnectorError> {
        serde_json::to_writer(&mut self.stdin, value).map_err(|error| {
            AccountConnectorError::new(
                AccountConnectorErrorKind::Protocol,
                stage,
                format!("failed to encode JSON-RPC request: {error}"),
            )
        })?;
        self.stdin.write_all(b"\n").map_err(|error| {
            AccountConnectorError::new(
                AccountConnectorErrorKind::Protocol,
                stage,
                format!("failed to write JSON-RPC request: {error}"),
            )
        })?;
        self.stdin.flush().map_err(|error| {
            AccountConnectorError::new(
                AccountConnectorErrorKind::Protocol,
                stage,
                format!("failed to flush JSON-RPC request: {error}"),
            )
        })
    }

    fn await_response(
        &mut self,
        id: i64,
        stage: &str,
        timeout: Duration,
    ) -> Result<Value, AccountConnectorError> {
        let deadline = Instant::now() + timeout;
        loop {
            let now = Instant::now();
            if now >= deadline {
                self.terminate();
                return Err(AccountConnectorError::new(
                    AccountConnectorErrorKind::Timeout,
                    stage,
                    format!("{stage} timed out"),
                ));
            }
            match self.stdout_rx.recv_timeout(deadline - now) {
                Ok(line) => {
                    let value: Value = serde_json::from_str(&line).map_err(|error| {
                        AccountConnectorError::new(
                            AccountConnectorErrorKind::Protocol,
                            stage,
                            format!(
                                "malformed JSON-RPC output: {error}; {}",
                                redact_sensitive(&line)
                            ),
                        )
                    })?;
                    if is_server_request(&value) {
                        self.reject_server_request(&value, stage)?;
                        continue;
                    }
                    if value.get("id").and_then(Value::as_i64) != Some(id) {
                        continue;
                    }
                    if let Some(error) = value.get("error") {
                        return Err(json_rpc_error(stage, error));
                    }
                    return value.get("result").cloned().ok_or_else(|| {
                        AccountConnectorError::new(
                            AccountConnectorErrorKind::Protocol,
                            stage,
                            "JSON-RPC response is missing result",
                        )
                    });
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    self.terminate();
                    return Err(AccountConnectorError::new(
                        AccountConnectorErrorKind::Timeout,
                        stage,
                        format!("{stage} timed out"),
                    ));
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let stderr = self.stderr_tail();
                    let kind = if looks_like_arg_rejection(&stderr) {
                        AccountConnectorErrorKind::UnsupportedCli
                    } else {
                        AccountConnectorErrorKind::Protocol
                    };
                    return Err(AccountConnectorError::new(
                        kind,
                        stage,
                        if stderr.is_empty() {
                            "app-server exited before responding".to_string()
                        } else {
                            stderr
                        },
                    ));
                }
            }
        }
    }

    fn reject_server_request(
        &mut self,
        request: &Value,
        stage: &str,
    ) -> Result<(), AccountConnectorError> {
        let Some(id) = request.get("id").cloned() else {
            return Ok(());
        };
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": "TokenStack does not provide interactive app-server handlers"
            }
        });
        self.send(&response, stage)
    }

    fn stderr_tail(&self) -> String {
        self.stderr
            .lock()
            .map(|buffer| buffer.clone())
            .unwrap_or_default()
    }

    fn terminate(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for AppServerProcess {
    fn drop(&mut self) {
        self.terminate();
    }
}

fn is_server_request(value: &Value) -> bool {
    value.get("id").is_some()
        && value.get("method").and_then(Value::as_str).is_some()
        && value.get("result").is_none()
        && value.get("error").is_none()
}

fn json_rpc_error(stage: &str, error: &Value) -> AccountConnectorError {
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("JSON-RPC method failed");
    let code = error
        .get("code")
        .map(Value::to_string)
        .unwrap_or_else(|| "unknown".to_string());
    if is_logged_out_message(message) {
        AccountConnectorError::logged_out(stage, message)
    } else {
        AccountConnectorError::new(
            AccountConnectorErrorKind::Protocol,
            stage,
            format!("JSON-RPC error {code}: {message}"),
        )
    }
}

fn is_logged_out_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("not logged in")
        || lower.contains("login required")
        || lower.contains("unauthenticated")
}

impl AccountConnectorError {
    fn looks_like_unsupported_cli(&self, stderr: &str) -> bool {
        self.kind == AccountConnectorErrorKind::UnsupportedCli || looks_like_arg_rejection(stderr)
    }
}

fn looks_like_arg_rejection(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("unexpected argument")
        || lower.contains("unrecognized option")
        || lower.contains("unknown option")
        || lower.contains("--listen")
        || lower.contains("mcp_servers")
}

fn normalize_account(value: &Value) -> AccountIdentitySnapshot {
    let account = value.get("account").unwrap_or(value);
    AccountIdentitySnapshot {
        account_label: account
            .get("email")
            .or_else(|| account.get("label"))
            .and_then(Value::as_str)
            .map(redact_account_label),
        plan: account
            .get("plan")
            .or_else(|| account.get("planType"))
            .and_then(Value::as_str)
            .map(str::to_string),
    }
}

fn normalize_usage(value: &Value) -> AccountUsageSnapshot {
    let summary = value.get("summary").unwrap_or(value);
    let lifetime_tokens = summary
        .get("lifetimeTokens")
        .or_else(|| summary.get("lifetime_tokens"))
        .and_then(Value::as_i64);
    let daily_buckets = value
        .get("dailyUsageBuckets")
        .or_else(|| value.get("daily_usage_buckets"))
        .or_else(|| value.get("dailyUsage"))
        .and_then(Value::as_array)
        .map(|buckets| {
            buckets
                .iter()
                .filter_map(normalize_daily_bucket)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    AccountUsageSnapshot {
        lifetime_tokens,
        daily_buckets,
    }
}

fn normalize_daily_bucket(value: &Value) -> Option<AccountDailyUsageBucket> {
    Some(AccountDailyUsageBucket {
        date: value
            .get("date")
            .or_else(|| value.get("day"))
            .and_then(Value::as_str)?
            .to_string(),
        input_tokens: value
            .get("inputTokens")
            .or_else(|| value.get("input_tokens"))
            .and_then(Value::as_i64)
            .unwrap_or(0),
        output_tokens: value
            .get("outputTokens")
            .or_else(|| value.get("output_tokens"))
            .and_then(Value::as_i64)
            .unwrap_or(0),
        total_tokens: value
            .get("totalTokens")
            .or_else(|| value.get("total_tokens"))
            .and_then(Value::as_i64)
            .unwrap_or(0),
    })
}

fn normalize_rate_limits(value: &Value) -> Result<RateLimitNormalization, AccountConnectorError> {
    let reset_credits = normalize_reset_credits(value);
    let mut rate_limits =
        if let Some(map) = value.get("rateLimitsByLimitId").and_then(Value::as_object) {
            let mut buckets = map
                .iter()
                .map(|(id, bucket)| normalize_rate_limit_bucket(id, bucket))
                .collect::<Vec<_>>();
            buckets.sort_by(|left, right| {
                bucket_sort_key(&left.bucket_id).cmp(&bucket_sort_key(&right.bucket_id))
            });
            buckets
        } else if let Some(items) = value.get("rateLimits").and_then(Value::as_array) {
            normalize_rate_limit_array(items)
        } else {
            Vec::new()
        };
    for bucket in &mut rate_limits {
        bucket
            .windows
            .sort_by_key(|window| window.window_duration_mins);
    }
    Ok(RateLimitNormalization {
        reset_credits,
        rate_limits,
    })
}

fn normalize_rate_limit_bucket(id: &str, value: &Value) -> AccountRateLimitBucket {
    let windows = value
        .get("windows")
        .or_else(|| value.get("limits"))
        .and_then(Value::as_array)
        .map(|windows| {
            windows
                .iter()
                .filter_map(normalize_rate_limit_window)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| normalize_rate_limit_window(value).into_iter().collect());
    AccountRateLimitBucket {
        bucket_id: id.to_string(),
        display_name: value
            .get("displayName")
            .or_else(|| value.get("name"))
            .and_then(Value::as_str)
            .unwrap_or(id)
            .to_string(),
        windows,
    }
}

fn normalize_rate_limit_array(items: &[Value]) -> Vec<AccountRateLimitBucket> {
    let mut grouped: BTreeMap<String, AccountRateLimitBucket> = BTreeMap::new();
    for item in items {
        let id = item
            .get("limitId")
            .or_else(|| item.get("limit_id"))
            .or_else(|| item.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("codex")
            .to_string();
        let display_name = item
            .get("displayName")
            .or_else(|| item.get("name"))
            .and_then(Value::as_str)
            .unwrap_or(&id)
            .to_string();
        let Some(window) = normalize_rate_limit_window(item) else {
            continue;
        };
        grouped
            .entry(id.clone())
            .or_insert_with(|| AccountRateLimitBucket {
                bucket_id: id,
                display_name,
                windows: Vec::new(),
            })
            .windows
            .push(window);
    }
    let mut buckets = grouped.into_values().collect::<Vec<_>>();
    buckets.sort_by(|left, right| {
        bucket_sort_key(&left.bucket_id).cmp(&bucket_sort_key(&right.bucket_id))
    });
    buckets
}

fn normalize_rate_limit_window(value: &Value) -> Option<AccountRateLimitWindow> {
    let window_duration_mins = value
        .get("windowDurationMins")
        .or_else(|| value.get("window_duration_mins"))
        .or_else(|| value.get("durationMinutes"))
        .and_then(Value::as_i64)?;
    let used_percent = value
        .get("usedPercent")
        .or_else(|| value.get("used_percent"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 100.0);
    Some(AccountRateLimitWindow {
        window_duration_mins,
        window_label: window_label(window_duration_mins),
        used_percent,
        remaining_percent: (100.0 - used_percent).clamp(0.0, 100.0),
        resets_at_utc: value
            .get("resetsAt")
            .or_else(|| value.get("resetAt"))
            .or_else(|| value.get("resets_at"))
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn normalize_reset_credits(value: &Value) -> AccountResetCreditsSnapshot {
    let credits = value.get("rateLimitResetCredits").unwrap_or(&Value::Null);
    AccountResetCreditsSnapshot {
        available_count: credits
            .get("availableCount")
            .or_else(|| credits.get("available_count"))
            .and_then(Value::as_i64),
        expires_at_utc: credits
            .get("expiresAt")
            .or_else(|| credits.get("expires_at"))
            .and_then(Value::as_str)
            .map(str::to_string),
    }
}

fn bucket_sort_key(bucket_id: &str) -> (u8, &str) {
    if bucket_id == "codex" {
        (0, bucket_id)
    } else {
        (1, bucket_id)
    }
}

fn window_label(minutes: i64) -> String {
    match minutes {
        300 => "5-hour".to_string(),
        10080 => "7-day".to_string(),
        value if value % 1440 == 0 => format!("{}-day", value / 1440),
        value if value % 60 == 0 => format!("{}-hour", value / 60),
        value => format!("{value}-minute"),
    }
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
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use tempfile::tempdir;

    struct FakeCodex {
        _dir: tempfile::TempDir,
        bin: PathBuf,
        rpc_log: PathBuf,
        launch_log: PathBuf,
    }

    impl FakeCodex {
        fn new(scenario: &str) -> Self {
            let dir = tempdir().unwrap();
            let bin = dir.path().join("codex");
            let rpc_log = dir.path().join("rpc.log");
            let launch_log = dir.path().join("launch.log");
            fs::write(&bin, fake_codex_script(scenario, &rpc_log, &launch_log)).unwrap();
            let mut permissions = fs::metadata(&bin).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&bin, permissions).unwrap();
            Self {
                _dir: dir,
                bin,
                rpc_log,
                launch_log,
            }
        }

        fn rpc_log(&self) -> String {
            fs::read_to_string(&self.rpc_log).unwrap_or_default()
        }

        fn launch_log(&self) -> String {
            fs::read_to_string(&self.launch_log).unwrap_or_default()
        }
    }

    fn fake_codex_script(scenario: &str, rpc_log: &Path, launch_log: &Path) -> String {
        format!(
            r#"#!/bin/sh
scenario="{scenario}"
rpc_log="{rpc_log}"
launch_log="{launch_log}"
printf '%s\n' "$*" >> "$launch_log"
if [ "$scenario" = "old_cli" ]; then
  for arg in "$@"; do
    if [ "$arg" = "--listen" ]; then
      echo "error: unexpected argument '--listen'" >&2
      exit 2
    fi
  done
fi
if [ "$scenario" = "hung_initialize" ]; then
  sleep 30
  exit 0
fi
while IFS= read -r line; do
  printf '%s\n' "$line" >> "$rpc_log"
  case "$line" in
    *'"method":"initialize"'*)
      echo '{{"jsonrpc":"2.0","method":"account/rateLimits/updated","params":{{}}}}'
      echo '{{"jsonrpc":"2.0","id":999,"result":{{"ignored":true}}}}'
      echo '{{"jsonrpc":"2.0","id":1,"result":{{"serverInfo":{{"name":"fake-codex"}}}}}}'
      ;;
    *'"method":"account/read"'*)
      if [ "$scenario" = "logged_out" ]; then
        echo '{{"jsonrpc":"2.0","id":2,"error":{{"code":-32001,"message":"not logged in; run codex login"}}}}'
      else
        echo '{{"jsonrpc":"2.0","id":2,"result":{{"account":{{"email":"person@example.invalid","plan":"Pro"}}}}}}'
      fi
      ;;
    *'"method":"account/rateLimits/read"'*)
      echo '{{"jsonrpc":"2.0","id":3,"result":{{"rateLimitsByLimitId":{{"gpt-5.5":{{"displayName":"GPT-5.5","windows":[{{"windowDurationMins":60,"usedPercent":10.0,"resetsAt":"2026-07-07T01:00:00Z"}}]}},"codex":{{"displayName":"Codex","windows":[{{"windowDurationMins":300,"usedPercent":25.5,"resetsAt":"2026-07-07T05:00:00Z"}},{{"windowDurationMins":10080,"usedPercent":40.0,"resetsAt":"2026-07-14T00:00:00Z"}}]}}}},"rateLimitResetCredits":{{"availableCount":3,"expiresAt":"2026-07-28T18:14:00Z"}}}}}}'
      ;;
    *'"method":"account/usage/read"'*)
      if [ "$scenario" = "partial_usage_failure" ]; then
        echo '{{"jsonrpc":"2.0","id":4,"error":{{"code":-32050,"message":"usage temporarily unavailable Bearer should-redact"}}}}'
      else
        echo '{{"jsonrpc":"2.0","id":4,"result":{{"summary":{{"lifetimeTokens":987654321}},"dailyUsageBuckets":[{{"date":"2026-07-07","inputTokens":10,"outputTokens":20,"totalTokens":30}}]}}}}'
      fi
      ;;
  esac
done
"#,
            scenario = scenario,
            rpc_log = rpc_log.display(),
            launch_log = launch_log.display()
        )
    }

    fn test_config(fake: &FakeCodex) -> CodexAppServerConfig {
        CodexAppServerConfig {
            explicit_codex_path: Some(fake.bin.clone()),
            initialize_timeout: Duration::from_millis(300),
            request_timeout: Duration::from_millis(300),
            whole_refresh_timeout: Duration::from_secs(2),
        }
    }

    #[test]
    fn fake_app_server_happy_path_normalizes_account_snapshot() {
        let fake = FakeCodex::new("happy");
        let snapshot = refresh_account_snapshot(test_config(&fake)).unwrap();

        assert_eq!(snapshot.status, AccountRefreshStatus::Connected);
        assert_eq!(snapshot.launch.mode, CodexLaunchMode::ListenStdioNoMcp);
        assert_eq!(
            snapshot.launch.selected_executable,
            fake.bin.display().to_string()
        );
        assert_eq!(snapshot.usage.lifetime_tokens, Some(987_654_321));
        assert_eq!(snapshot.usage.daily_buckets[0].total_tokens, 30);
        assert_eq!(snapshot.reset_credits.available_count, Some(3));
        assert_eq!(snapshot.rate_limits[0].bucket_id, "codex");
        assert_eq!(snapshot.rate_limits[0].windows[0].window_label, "5-hour");
        assert_eq!(snapshot.rate_limits[0].windows[0].remaining_percent, 74.5);
        assert_eq!(snapshot.rate_limits[1].bucket_id, "gpt-5.5");

        let rpc = fake.rpc_log();
        assert!(rpc.contains(r#""method":"initialize""#));
        assert!(rpc.contains(r#""experimentalApi":true"#));
        assert!(rpc.contains(r#""method":"initialized""#));
        assert!(rpc.contains(r#""method":"account/read""#));
        assert!(rpc.contains(r#""method":"account/rateLimits/read""#));
        assert!(rpc.contains(r#""method":"account/usage/read""#));
        assert!(!rpc.contains(&forbidden_consume_method()));
    }

    #[test]
    fn old_cli_argument_rejection_falls_back_to_plain_app_server() {
        let fake = FakeCodex::new("old_cli");
        let snapshot = refresh_account_snapshot(test_config(&fake)).unwrap();

        assert_eq!(snapshot.status, AccountRefreshStatus::Connected);
        assert_eq!(
            snapshot.launch.mode,
            CodexLaunchMode::PlainAppServerFallback
        );
        let launch_log = fake.launch_log();
        assert!(launch_log.contains("app-server --listen stdio:// -c mcp_servers={}"));
        assert!(launch_log.contains("app-server\n"));
    }

    #[test]
    fn usage_failure_keeps_rate_limits_as_partial_snapshot() {
        let fake = FakeCodex::new("partial_usage_failure");
        let snapshot = refresh_account_snapshot(test_config(&fake)).unwrap();

        assert_eq!(snapshot.status, AccountRefreshStatus::Degraded);
        assert_eq!(
            snapshot.method_status("account/rateLimits/read"),
            Some(MethodStatus::Ok)
        );
        assert_eq!(
            snapshot.method_status("account/usage/read"),
            Some(MethodStatus::Failed)
        );
        assert_eq!(snapshot.usage.lifetime_tokens, None);
        assert_eq!(snapshot.reset_credits.available_count, Some(3));
        assert!(snapshot
            .diagnostics
            .first_failing_stage
            .as_deref()
            .unwrap()
            .contains("account/usage/read"));
        assert!(!snapshot
            .diagnostics
            .redacted_error_message
            .contains("Bearer should-redact"));
    }

    #[test]
    fn logged_out_account_read_maps_to_logged_out_failure() {
        let fake = FakeCodex::new("logged_out");
        let error = refresh_account_snapshot(test_config(&fake)).unwrap_err();

        assert_eq!(error.kind, AccountConnectorErrorKind::LoggedOut);
        assert!(error.public_message.contains("Codex login required"));
        assert!(!error.public_message.contains("person@example.invalid"));
    }

    #[test]
    fn hung_app_server_times_out_and_reports_initialize_stage() {
        let fake = FakeCodex::new("hung_initialize");
        let error = refresh_account_snapshot(test_config(&fake)).unwrap_err();

        assert_eq!(error.kind, AccountConnectorErrorKind::Timeout);
        assert_eq!(error.stage, "initialize");
    }

    #[test]
    fn rate_limits_array_fallback_and_unknown_windows_are_preserved() {
        let value = serde_json::json!({
            "rateLimits": [
                {
                    "limitId": "other",
                    "displayName": "Other bucket",
                    "windowDurationMins": 42,
                    "usedPercent": 110.0,
                    "resetsAt": "2026-07-07T06:00:00Z"
                }
            ],
            "rateLimitResetCredits": { "availableCount": 0 }
        });

        let normalized = normalize_rate_limits(&value).unwrap();

        assert_eq!(normalized.reset_credits.available_count, Some(0));
        assert_eq!(normalized.rate_limits[0].bucket_id, "other");
        assert_eq!(
            normalized.rate_limits[0].windows[0].window_label,
            "42-minute"
        );
        assert_eq!(normalized.rate_limits[0].windows[0].used_percent, 100.0);
        assert_eq!(normalized.rate_limits[0].windows[0].remaining_percent, 0.0);
    }

    #[test]
    fn consume_reset_credit_method_is_not_a_client_method() {
        let fake = FakeCodex::new("happy");
        refresh_account_snapshot(test_config(&fake)).unwrap();

        assert!(!fake.rpc_log().contains(&forbidden_consume_method()));
        assert!(!ACCOUNT_READ_METHODS.contains(&forbidden_consume_method().as_str()));
    }

    fn forbidden_consume_method() -> String {
        [
            "account",
            &format!("{}{}", "rateLimitReset", "Credit"),
            "consume",
        ]
        .join("/")
    }
}
