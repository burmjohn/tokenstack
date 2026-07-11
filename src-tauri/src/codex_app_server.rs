#![allow(clippy::result_large_err)]

use crate::codex_runtime::CodexLaunchSpec;
#[cfg(test)]
pub use crate::codex_runtime::{discover_codex_runtimes, CodexRuntimeSettings};
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
pub const APP_SERVER_SCHEMA_FINGERPRINT: &str =
    "codex-app-server-v2@c4318c386de365bd0dd9595a08d55a30bb142d11";

#[derive(Debug, Clone)]
pub struct CodexAppServerConfig {
    pub explicit_runtime: Option<CodexLaunchSpec>,
    pub initialize_timeout: Duration,
    pub request_timeout: Duration,
    pub whole_refresh_timeout: Duration,
}

impl Default for CodexAppServerConfig {
    fn default() -> Self {
        Self {
            explicit_runtime: None,
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
    RuntimeValidation,
    ListenStdioNoMcp,
    PlainAppServerFallback,
}

impl CodexLaunchMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeValidation => "runtime_validation",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AccountConnectorFailureClass {
    Transport,
    Method,
}

#[derive(Debug, Clone)]
pub struct AccountConnectorError {
    pub kind: AccountConnectorErrorKind,
    pub stage: String,
    pub public_message: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub child_terminated: bool,
    pub launch: AccountLaunchDiagnostics,
    pub(crate) failure_class: AccountConnectorFailureClass,
}

impl AccountConnectorError {
    pub(crate) fn new(
        kind: AccountConnectorErrorKind,
        stage: impl Into<String>,
        message: impl AsRef<str>,
    ) -> Self {
        Self {
            kind,
            stage: stage.into(),
            public_message: redact_sensitive(message.as_ref()),
            exit_code: None,
            timed_out: false,
            child_terminated: false,
            launch: AccountLaunchDiagnostics::validation(String::new(), Vec::new()),
            failure_class: AccountConnectorFailureClass::Transport,
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
            exit_code: None,
            timed_out: false,
            child_terminated: false,
            launch: AccountLaunchDiagnostics::validation(String::new(), Vec::new()),
            failure_class: AccountConnectorFailureClass::Method,
        }
    }

    fn with_resolution_launch(
        mut self,
        selected: &Path,
        argv_prefix: &[String],
        candidates: Vec<String>,
    ) -> Self {
        self.launch = AccountLaunchDiagnostics {
            selected_executable: selected.display().to_string(),
            argv_prefix: argv_prefix.to_vec(),
            mode: CodexLaunchMode::RuntimeValidation,
            candidates,
        };
        self
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
    pub argv_prefix: Vec<String>,
    pub mode: CodexLaunchMode,
    pub candidates: Vec<String>,
}

impl AccountLaunchDiagnostics {
    pub(crate) fn validation(selected_executable: String, argv_prefix: Vec<String>) -> Self {
        Self {
            selected_executable,
            argv_prefix,
            mode: CodexLaunchMode::RuntimeValidation,
            candidates: Vec::new(),
        }
    }
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
    pub schema_fingerprint: String,
    pub exit_code: Option<i32>,
    pub child_terminated: bool,
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
    pub daily_buckets_status: OptionalCollectionStatus,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionalCollectionStatus {
    #[default]
    Absent,
    Null,
    Present,
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
    pub credits: Option<Vec<AccountResetCreditDetail>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountResetCreditDetail {
    pub id: String,
    pub reset_type: String,
    pub status: String,
    pub granted_at_utc: String,
    pub expires_at_utc: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
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
    pub window_duration_mins: Option<i64>,
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
    argv_prefix: Vec<String>,
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
    let executable = resolve_codex_executable(config.explicit_runtime.as_ref())?;

    let primary = run_refresh_attempt(
        &executable,
        CodexLaunchMode::ListenStdioNoMcp,
        &config,
        deadline,
        started,
    );

    match primary {
        Ok(snapshot) => Ok(snapshot),
        Err(error)
            if error.kind == AccountConnectorErrorKind::UnsupportedCli
                && error.exit_code.is_some()
                && error.child_terminated =>
        {
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

pub fn validate_codex_app_server_runtime(
    spec: &CodexLaunchSpec,
) -> Result<(), AccountConnectorError> {
    let executable = resolve_codex_executable(Some(spec))?;
    let mut client = JsonRpcClient::spawn(
        &executable.path,
        &executable.argv_prefix,
        CodexLaunchMode::ListenStdioNoMcp,
    )?;
    client.request(
        "initialize",
        Some(initialize_params()),
        Duration::from_secs(8),
    )?;
    client.notify("initialized", Some(json!({})))?;
    Ok(())
}

fn run_refresh_attempt(
    executable: &CodexExecutable,
    mode: CodexLaunchMode,
    config: &CodexAppServerConfig,
    deadline: Instant,
    started: DateTime<Utc>,
) -> Result<AccountSnapshot, AccountConnectorError> {
    run_refresh_attempt_inner(executable, mode, config, deadline, started).map_err(|mut error| {
        error.launch = AccountLaunchDiagnostics {
            selected_executable: executable.path.display().to_string(),
            argv_prefix: executable.argv_prefix.clone(),
            mode,
            candidates: executable.candidates.clone(),
        };
        error
    })
}

fn run_refresh_attempt_inner(
    executable: &CodexExecutable,
    mode: CodexLaunchMode,
    config: &CodexAppServerConfig,
    deadline: Instant,
    started: DateTime<Utc>,
) -> Result<AccountSnapshot, AccountConnectorError> {
    let mut client = JsonRpcClient::spawn(&executable.path, &executable.argv_prefix, mode)?;
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
        Err(mut error) if mode == CodexLaunchMode::ListenStdioNoMcp => {
            thread::sleep(Duration::from_millis(25));
            let stderr = client.stderr_tail();
            if error.exit_code.is_some()
                && error.child_terminated
                && looks_like_arg_rejection(&stderr)
            {
                error.kind = AccountConnectorErrorKind::UnsupportedCli;
                return Err(error);
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
        Err(error) if error.failure_class == AccountConnectorFailureClass::Method => {
            status = AccountRefreshStatus::Degraded;
            first_failure = Some(("account/read".to_string(), error.public_message.clone()));
            methods.push(method_failed("account/read", &error.public_message));
            AccountIdentitySnapshot::default()
        }
        Err(error) => return Err(error),
    };

    let (rate_limits, reset_credits) = match request_rate_limits(&mut client, config, deadline) {
        Ok(normalized) => {
            methods.push(method_ok("account/rateLimits/read"));
            (normalized.rate_limits, normalized.reset_credits)
        }
        Err(error) if error.failure_class == AccountConnectorFailureClass::Method => {
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
        Err(error) => return Err(error),
    };

    let usage = match request_usage(&mut client, config, deadline) {
        Ok(usage) => {
            methods.push(method_ok("account/usage/read"));
            usage
        }
        Err(error) if error.failure_class == AccountConnectorFailureClass::Method => {
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
        Err(error) => return Err(error),
    };

    let (first_failing_stage, redacted_error_message) = first_failure
        .map(|(stage, message)| (Some(stage), message))
        .unwrap_or_else(|| (None, String::new()));

    let stderr_tail = client.settled_stderr_tail();
    let (exit_code, child_terminated) = client.shutdown();

    Ok(AccountSnapshot {
        status,
        launch: AccountLaunchDiagnostics {
            selected_executable: executable.path.display().to_string(),
            argv_prefix: executable.argv_prefix.clone(),
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
            stderr_tail,
            used_last_good_snapshot: false,
            schema_fingerprint: APP_SERVER_SCHEMA_FINGERPRINT.to_string(),
            exit_code,
            child_terminated,
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
    normalize_account(&result)
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
    normalize_usage(&result)
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
    explicit: Option<&CodexLaunchSpec>,
) -> Result<CodexExecutable, AccountConnectorError> {
    let mut candidates = Vec::new();
    if let Some(spec) = explicit {
        let path = &spec.executable_path;
        candidates.push(path.display().to_string());
        if path.exists() {
            return Ok(CodexExecutable {
                path: path.to_path_buf(),
                argv_prefix: spec.argv_prefix.clone(),
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
        )
        .with_resolution_launch(path, &spec.argv_prefix, candidates));
    }

    if let Some(path) = env::var_os("TOKENSTACK_CODEX_BIN").map(PathBuf::from) {
        candidates.push(path.display().to_string());
        if path.exists() {
            return Ok(CodexExecutable {
                path,
                argv_prefix: Vec::new(),
                candidates,
            });
        }
        return Err(AccountConnectorError::new(
            AccountConnectorErrorKind::MissingCli,
            "resolve_codex",
            "TOKENSTACK_CODEX_BIN points to a missing Codex executable",
        )
        .with_resolution_launch(&path, &[], candidates));
    }

    for candidate in path_candidates("codex") {
        candidates.push(candidate.display().to_string());
        if candidate.exists() {
            return Ok(CodexExecutable {
                path: candidate,
                argv_prefix: Vec::new(),
                candidates,
            });
        }
    }

    for candidate in fallback_codex_candidates("codex") {
        let label = candidate.display().to_string();
        if candidates.iter().any(|seen| seen == &label) {
            continue;
        }
        candidates.push(label);
        if candidate.exists() {
            return Ok(CodexExecutable {
                path: candidate,
                argv_prefix: Vec::new(),
                candidates,
            });
        }
    }

    Err(AccountConnectorError::new(
        AccountConnectorErrorKind::MissingCli,
        "resolve_codex",
        "Codex CLI was not found in PATH or common app install locations. Configure the Codex executable or set TOKENSTACK_CODEX_BIN.",
    )
    .with_resolution_launch(Path::new(""), &[], candidates))
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

fn fallback_codex_candidates(binary: &str) -> Vec<PathBuf> {
    let suffixes = executable_suffixes();
    fallback_codex_candidate_dirs()
        .into_iter()
        .flat_map(|dir| {
            suffixes
                .iter()
                .map(move |suffix| dir.join(format!("{binary}{suffix}")))
        })
        .collect()
}

#[cfg(windows)]
fn fallback_codex_candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.to_path_buf());
        }
    }
    if let Some(appdata) = env::var_os("APPDATA").map(PathBuf::from) {
        dirs.push(appdata.join("npm"));
    }
    if let Some(local_appdata) = env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        dirs.push(local_appdata.join("Programs").join("Codex"));
        dirs.push(local_appdata.join("Programs").join("OpenAI Codex"));
    }
    if let Some(program_files) = env::var_os("ProgramFiles").map(PathBuf::from) {
        dirs.push(program_files.join("Codex"));
        dirs.push(program_files.join("OpenAI Codex"));
    }
    if let Some(user_profile) = env::var_os("USERPROFILE").map(PathBuf::from) {
        dirs.push(user_profile.join(".local").join("bin"));
    }
    dirs
}

#[cfg(not(windows))]
fn fallback_codex_candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.to_path_buf());
        }
    }
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".local").join("bin"));
    }
    dirs.push(PathBuf::from("/usr/local/bin"));
    dirs.push(PathBuf::from("/opt/homebrew/bin"));
    dirs
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
    fn spawn(
        path: &Path,
        argv_prefix: &[String],
        mode: CodexLaunchMode,
    ) -> Result<Self, AccountConnectorError> {
        let mut args = argv_prefix.to_vec();
        args.push("app-server".to_string());
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
        if let Err(error) = self.process.send(&request, method) {
            return Err(self.process.complete_error(error));
        }
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
        self.process
            .send(&notification, method)
            .map_err(|error| self.process.complete_error(error))
    }

    fn stderr_tail(&self) -> String {
        self.process.stderr_tail()
    }

    fn settled_stderr_tail(&self) -> String {
        // stdout and stderr are drained on separate threads. Give stderr a brief,
        // bounded chance to catch up after the final matching response.
        thread::sleep(Duration::from_millis(25));
        self.stderr_tail()
    }

    fn shutdown(&mut self) -> (Option<i32>, bool) {
        self.process.shutdown()
    }
}

struct AppServerProcess {
    child: Child,
    stdin: Option<ChildStdin>,
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
                    let mut keep_from = buffer.len() - 4_096;
                    while !buffer.is_char_boundary(keep_from) {
                        keep_from += 1;
                    }
                    *buffer = buffer[keep_from..].to_string();
                }
            }
        });

        Ok(Self {
            child,
            stdin: Some(stdin),
            stdout_rx,
            stderr,
        })
    }

    fn send(&mut self, value: &Value, stage: &str) -> Result<(), AccountConnectorError> {
        let stdin = self.stdin.as_mut().ok_or_else(|| {
            AccountConnectorError::new(
                AccountConnectorErrorKind::Protocol,
                stage,
                "app-server stdin is closed",
            )
        })?;
        serde_json::to_writer(&mut *stdin, value).map_err(|error| {
            AccountConnectorError::new(
                AccountConnectorErrorKind::Protocol,
                stage,
                format!("failed to encode JSON-RPC request: {error}"),
            )
        })?;
        stdin.write_all(b"\n").map_err(|error| {
            AccountConnectorError::new(
                AccountConnectorErrorKind::Protocol,
                stage,
                format!("failed to write JSON-RPC request: {error}"),
            )
        })?;
        stdin.flush().map_err(|error| {
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
                let mut error = AccountConnectorError::new(
                    AccountConnectorErrorKind::Timeout,
                    stage,
                    format!("{stage} timed out"),
                );
                error.timed_out = true;
                return Err(self.complete_error(error));
            }
            match self.stdout_rx.recv_timeout(deadline - now) {
                Ok(line) => {
                    let value = serde_json::from_str::<Value>(&line).map_err(|error| {
                        AccountConnectorError::new(
                            AccountConnectorErrorKind::Protocol,
                            stage,
                            format!(
                                "malformed JSON-RPC output: {error}; {}",
                                redact_sensitive(&line)
                            ),
                        )
                    });
                    let value = match value {
                        Ok(value) => value,
                        Err(error) => return Err(self.complete_error(error)),
                    };
                    if is_server_request(&value) {
                        self.reject_server_request(&value, stage)?;
                        continue;
                    }
                    if value.get("id").and_then(Value::as_i64) != Some(id) {
                        continue;
                    }
                    if let Some(error) = value.get("error") {
                        let error = json_rpc_error(stage, error);
                        if error.kind == AccountConnectorErrorKind::LoggedOut {
                            return Err(self.complete_error(error));
                        }
                        return Err(error);
                    }
                    return value
                        .get("result")
                        .cloned()
                        .ok_or_else(|| {
                            AccountConnectorError::new(
                                AccountConnectorErrorKind::Protocol,
                                stage,
                                "JSON-RPC response is missing result",
                            )
                        })
                        .map_err(|error| self.complete_error(error));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let mut error = AccountConnectorError::new(
                        AccountConnectorErrorKind::Timeout,
                        stage,
                        format!("{stage} timed out"),
                    );
                    error.timed_out = true;
                    return Err(self.complete_error(error));
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let stderr = self.stderr_tail();
                    let kind = if looks_like_arg_rejection(&stderr) {
                        AccountConnectorErrorKind::UnsupportedCli
                    } else {
                        AccountConnectorErrorKind::Protocol
                    };
                    let error = AccountConnectorError::new(
                        kind,
                        stage,
                        if stderr.is_empty() {
                            "app-server exited before responding".to_string()
                        } else {
                            stderr
                        },
                    );
                    return Err(self.complete_error(error));
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
            .map(|buffer| redact_sensitive(&buffer))
            .unwrap_or_default()
    }

    fn wait_for_exit_code(&mut self, timeout: Duration) -> Option<i32> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => return status.code(),
                Ok(None) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(5));
                }
                Ok(None) | Err(_) => return None,
            }
        }
    }

    fn complete_error(&mut self, mut error: AccountConnectorError) -> AccountConnectorError {
        let (exit_code, child_terminated) = self.shutdown();
        error.exit_code = exit_code;
        error.child_terminated = child_terminated;
        error
    }

    fn shutdown(&mut self) -> (Option<i32>, bool) {
        self.stdin.take();
        if let Ok(Some(status)) = self.child.try_wait() {
            return (status.code(), true);
        }
        if let Some(exit_code) = self.wait_for_exit_code(Duration::from_millis(100)) {
            return (Some(exit_code), true);
        }
        if self.child.kill().is_err() {
            return (None, false);
        }
        match self.child.wait() {
            Ok(status) => (status.code(), true),
            Err(_) => (None, false),
        }
    }
}

impl Drop for AppServerProcess {
    fn drop(&mut self) {
        self.shutdown();
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
        let mut result = AccountConnectorError::new(
            AccountConnectorErrorKind::Protocol,
            stage,
            format!("JSON-RPC error {code}: {message}"),
        );
        result.failure_class = AccountConnectorFailureClass::Method;
        result
    }
}

fn is_logged_out_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("not logged in")
        || lower.contains("login required")
        || lower.contains("unauthenticated")
}

fn looks_like_arg_rejection(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    let recognized_parser_error = lower.contains("error: unexpected argument")
        || lower.contains("error: unrecognized option")
        || lower.contains("error: unknown option");
    let rejected_primary_flag = lower.contains("'--listen'")
        || lower.contains("\"--listen\"")
        || lower.contains("'mcp_servers'")
        || lower.contains("\"mcp_servers\"");
    recognized_parser_error && rejected_primary_flag
}

fn normalize_account(value: &Value) -> Result<AccountIdentitySnapshot, AccountConnectorError> {
    let object = value
        .as_object()
        .ok_or_else(|| account_schema_error("response must be an object"))?;
    if !object
        .get("requiresOpenaiAuth")
        .is_some_and(Value::is_boolean)
    {
        return Err(account_schema_error(
            "required requiresOpenaiAuth boolean is missing",
        ));
    }
    let account = object
        .get("account")
        .ok_or_else(|| account_schema_error("required account field is missing"))?;
    if account.is_null() {
        return Ok(AccountIdentitySnapshot::default());
    }
    let account = account
        .as_object()
        .ok_or_else(|| account_schema_error("account must be an object or null"))?;
    let account_type = account
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| account_schema_error("account type is missing"))?;
    match account_type {
        "apiKey" => {}
        "chatgpt" => {
            if !account.contains_key("email") {
                return Err(account_schema_error("chatgpt account email is missing"));
            }
            if !account.get("planType").is_some_and(Value::is_string) {
                return Err(account_schema_error("chatgpt account planType is missing"));
            }
        }
        "amazonBedrock" => {
            if !account.contains_key("credentialSource") {
                return Err(account_schema_error(
                    "amazonBedrock credentialSource is missing",
                ));
            }
        }
        _ => return Err(account_schema_error("account type is unsupported")),
    }
    Ok(AccountIdentitySnapshot {
        account_label: account
            .get("email")
            .or_else(|| account.get("label"))
            .and_then(Value::as_str)
            .map(redact_account_label),
        plan: account
            .get("planType")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn account_schema_error(message: &str) -> AccountConnectorError {
    let mut error = AccountConnectorError::new(
        AccountConnectorErrorKind::Protocol,
        "account/read",
        format!("schema mismatch: {message}"),
    );
    error.failure_class = AccountConnectorFailureClass::Method;
    error
}

fn normalize_usage(value: &Value) -> Result<AccountUsageSnapshot, AccountConnectorError> {
    let summary = value
        .get("summary")
        .ok_or_else(|| usage_schema_error("required summary is missing"))?;
    if summary.get("lifetimeTokens").is_none() && summary.get("lifetime_tokens").is_none() {
        return Err(usage_schema_error(
            "required lifetime token field is missing",
        ));
    }
    let lifetime_tokens = summary
        .get("lifetimeTokens")
        .or_else(|| summary.get("lifetime_tokens"))
        .and_then(Value::as_i64);
    let daily_value = value
        .get("dailyUsageBuckets")
        .or_else(|| value.get("daily_usage_buckets"))
        .or_else(|| value.get("dailyUsage"));
    let daily_buckets_status = match daily_value {
        None => OptionalCollectionStatus::Absent,
        Some(Value::Null) => OptionalCollectionStatus::Null,
        Some(Value::Array(_)) => OptionalCollectionStatus::Present,
        Some(_) => {
            return Err(usage_schema_error(
                "daily usage buckets must be an array or null",
            ));
        }
    };
    let daily_buckets = daily_value
        .and_then(Value::as_array)
        .map(|buckets| {
            buckets
                .iter()
                .map(normalize_daily_bucket)
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();

    Ok(AccountUsageSnapshot {
        lifetime_tokens,
        daily_buckets,
        daily_buckets_status,
    })
}

fn normalize_daily_bucket(value: &Value) -> Result<AccountDailyUsageBucket, AccountConnectorError> {
    let date = value
        .get("date")
        .or_else(|| value.get("day"))
        .or_else(|| value.get("startDate"))
        .and_then(Value::as_str)
        .ok_or_else(|| usage_schema_error("daily bucket required startDate is missing"))?;
    let total_tokens = value
        .get("totalTokens")
        .or_else(|| value.get("total_tokens"))
        .or_else(|| value.get("tokens"))
        .and_then(Value::as_i64)
        .ok_or_else(|| usage_schema_error("daily bucket required tokens is missing or invalid"))?;
    Ok(AccountDailyUsageBucket {
        date: date.to_string(),
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
        total_tokens,
    })
}

fn usage_schema_error(message: &str) -> AccountConnectorError {
    let mut error = AccountConnectorError::new(
        AccountConnectorErrorKind::Protocol,
        "account/usage/read",
        format!("schema mismatch: {message}"),
    );
    error.failure_class = AccountConnectorFailureClass::Method;
    error
}

fn normalize_rate_limits(value: &Value) -> Result<RateLimitNormalization, AccountConnectorError> {
    let reset_credits = normalize_reset_credits(value)?;
    let mut rate_limits = if let Some(map_value) = value.get("rateLimitsByLimitId") {
        if map_value.is_null() {
            normalize_rate_limits_fallback(value.get("rateLimits"))?
        } else if let Some(map) = map_value.as_object() {
            let mut buckets = map
                .iter()
                .map(|(id, bucket)| normalize_rate_limit_bucket(id, bucket))
                .collect::<Result<Vec<_>, _>>()?;
            buckets.sort_by(|left, right| {
                bucket_sort_key(&left.bucket_id).cmp(&bucket_sort_key(&right.bucket_id))
            });
            buckets
        } else {
            return Err(rate_limit_schema_error(
                "rateLimitsByLimitId must be an object or null",
            ));
        }
    } else {
        normalize_rate_limits_fallback(value.get("rateLimits"))?
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

fn normalize_rate_limits_fallback(
    value: Option<&Value>,
) -> Result<Vec<AccountRateLimitBucket>, AccountConnectorError> {
    match value {
        Some(Value::Object(_)) => Ok(vec![normalize_rate_limit_bucket("codex", value.unwrap())?]),
        Some(Value::Array(items)) => normalize_rate_limit_array(items),
        Some(_) => Err(rate_limit_schema_error("rateLimits must be an object")),
        None => Err(rate_limit_schema_error(
            "required rateLimits container is missing",
        )),
    }
}

fn normalize_rate_limit_bucket(
    id: &str,
    value: &Value,
) -> Result<AccountRateLimitBucket, AccountConnectorError> {
    let object = value
        .as_object()
        .ok_or_else(|| rate_limit_schema_error("rate-limit bucket must be an object"))?;
    for field in ["primary", "secondary"] {
        if !object.contains_key(field) {
            return Err(rate_limit_schema_error(&format!(
                "rate-limit bucket required field {field} is missing"
            )));
        }
    }
    let mut windows = Vec::new();
    for field in ["primary", "secondary"] {
        if let Some(window) = object.get(field).filter(|window| !window.is_null()) {
            windows.push(normalize_rate_limit_window(window)?);
        }
    }
    Ok(AccountRateLimitBucket {
        bucket_id: id.to_string(),
        display_name: value
            .get("displayName")
            .or_else(|| value.get("limitName"))
            .or_else(|| value.get("name"))
            .and_then(Value::as_str)
            .unwrap_or(id)
            .to_string(),
        windows,
    })
}

fn normalize_rate_limit_array(
    items: &[Value],
) -> Result<Vec<AccountRateLimitBucket>, AccountConnectorError> {
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
        let window = normalize_rate_limit_window(item)?;
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
    Ok(buckets)
}

fn normalize_rate_limit_window(
    value: &Value,
) -> Result<AccountRateLimitWindow, AccountConnectorError> {
    let object = value
        .as_object()
        .ok_or_else(|| rate_limit_schema_error("rate-limit window must be an object"))?;
    if !object.contains_key("usedPercent") && !object.contains_key("used_percent") {
        return Err(rate_limit_schema_error(
            "rate-limit window required field usedPercent is missing",
        ));
    }
    if !object.contains_key("windowDurationMins")
        && !object.contains_key("window_duration_mins")
        && !object.contains_key("durationMinutes")
    {
        return Err(rate_limit_schema_error(
            "rate-limit window required field windowDurationMins is missing",
        ));
    }
    let window_duration_mins = value
        .get("windowDurationMins")
        .or_else(|| value.get("window_duration_mins"))
        .or_else(|| value.get("durationMinutes"))
        .and_then(Value::as_i64);
    let used_percent = value
        .get("usedPercent")
        .or_else(|| value.get("used_percent"))
        .and_then(Value::as_f64)
        .ok_or_else(|| rate_limit_schema_error("rate-limit window usedPercent must be numeric"))?
        .clamp(0.0, 100.0);
    Ok(AccountRateLimitWindow {
        window_duration_mins,
        window_label: window_duration_mins
            .map(window_label)
            .unwrap_or_else(|| "unknown".to_string()),
        used_percent,
        remaining_percent: (100.0 - used_percent).clamp(0.0, 100.0),
        resets_at_utc: value
            .get("resetsAt")
            .or_else(|| value.get("resetAt"))
            .or_else(|| value.get("resets_at"))
            .and_then(timestamp_string),
    })
}

fn rate_limit_schema_error(message: &str) -> AccountConnectorError {
    let mut error = AccountConnectorError::new(
        AccountConnectorErrorKind::Protocol,
        "account/rateLimits/read",
        format!("schema mismatch: {message}"),
    );
    error.failure_class = AccountConnectorFailureClass::Method;
    error
}

fn normalize_reset_credits(
    value: &Value,
) -> Result<AccountResetCreditsSnapshot, AccountConnectorError> {
    let credits = value.get("rateLimitResetCredits").unwrap_or(&Value::Null);
    if credits.is_null() {
        return Ok(AccountResetCreditsSnapshot::default());
    }
    if !credits.get("availableCount").is_some_and(Value::is_i64) {
        return Err(rate_limit_schema_error(
            "reset-credit required availableCount is missing or invalid",
        ));
    }
    if credits.get("credits").is_none() {
        return Err(rate_limit_schema_error(
            "reset-credit required credits field is missing",
        ));
    }
    let detail_rows = match credits.get("credits") {
        Some(Value::Null) => None,
        Some(Value::Array(rows)) => Some(
            rows.iter()
                .map(normalize_reset_credit)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        _ => {
            return Err(rate_limit_schema_error(
                "reset-credit credits must be array or null",
            ))
        }
    };
    Ok(AccountResetCreditsSnapshot {
        available_count: credits
            .get("availableCount")
            .or_else(|| credits.get("available_count"))
            .and_then(Value::as_i64),
        expires_at_utc: credits
            .get("expiresAt")
            .or_else(|| credits.get("expires_at"))
            .and_then(timestamp_string),
        credits: detail_rows,
    })
}

fn normalize_reset_credit(
    value: &Value,
) -> Result<AccountResetCreditDetail, AccountConnectorError> {
    let required_string = |field: &str| {
        value
            .get(field)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| rate_limit_schema_error(&format!("reset-credit {field} is missing")))
    };
    let granted_at_utc = value
        .get("grantedAt")
        .and_then(timestamp_string)
        .ok_or_else(|| rate_limit_schema_error("reset-credit grantedAt is missing or invalid"))?;
    for nullable in ["expiresAt", "title", "description"] {
        if value.get(nullable).is_none() {
            return Err(rate_limit_schema_error(&format!(
                "reset-credit {nullable} is missing"
            )));
        }
    }
    Ok(AccountResetCreditDetail {
        id: required_string("id")?,
        reset_type: required_string("resetType")?,
        status: required_string("status")?,
        granted_at_utc,
        expires_at_utc: value.get("expiresAt").and_then(timestamp_string),
        title: value
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_string),
        description: value
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn timestamp_string(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string).or_else(|| {
        value
            .as_i64()
            .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
            .map(|timestamp| timestamp.to_rfc3339())
    })
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
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::Duration;
    use tempfile::tempdir;

    struct FakeCodex {
        _dir: tempfile::TempDir,
        bin: PathBuf,
        rpc_log: PathBuf,
        launch_log: PathBuf,
        pid_log: PathBuf,
    }

    impl FakeCodex {
        fn new(scenario: &str) -> Self {
            let dir = tempdir().unwrap();
            let source =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/fake_codex.rs");
            let bin = dir.path().join(if cfg!(windows) {
                format!("fake codex {scenario}.exe")
            } else {
                format!("fake codex {scenario}")
            });
            let rpc_log = dir.path().join(format!("{scenario}.rpc.log"));
            let launch_log = dir.path().join(format!("{scenario}.launch.log"));
            let pid_log = dir.path().join(format!("{scenario}.pid.log"));
            let compile = Command::new("rustc")
                .arg("--edition=2021")
                .arg(&source)
                .arg("-o")
                .arg(&bin)
                .output()
                .unwrap();
            assert!(
                compile.status.success(),
                "failed to compile {}: {}",
                source.display(),
                String::from_utf8_lossy(&compile.stderr)
            );
            Self {
                _dir: dir,
                bin,
                rpc_log,
                launch_log,
                pid_log,
            }
        }

        fn rpc_log(&self) -> String {
            fs::read_to_string(&self.rpc_log).unwrap_or_default()
        }

        fn launch_log(&self) -> String {
            fs::read_to_string(&self.launch_log).unwrap_or_default()
        }

        fn pid(&self) -> u32 {
            fs::read_to_string(&self.pid_log)
                .unwrap()
                .lines()
                .last()
                .unwrap()
                .parse()
                .unwrap()
        }
    }

    fn test_config(fake: &FakeCodex) -> CodexAppServerConfig {
        CodexAppServerConfig {
            explicit_runtime: Some(CodexLaunchSpec {
                executable_path: fake.bin.clone(),
                argv_prefix: Vec::new(),
            }),
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
        assert!(snapshot.diagnostics.child_terminated);

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
        let fake = FakeCodex::new("unsupported_argument");
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
    fn typed_argv_prefix_is_launched_and_retained_in_diagnostics() {
        let fake = FakeCodex::new("early_exit");
        let mut config = test_config(&fake);
        config.explicit_runtime.as_mut().unwrap().argv_prefix = vec!["fixed-prefix".to_string()];

        let error = refresh_account_snapshot(config).unwrap_err();

        assert!(fake.launch_log().starts_with("fixed-prefix app-server"));
        assert_eq!(error.launch.argv_prefix, vec!["fixed-prefix"]);
    }

    #[test]
    fn initialize_timeout_never_uses_compatibility_fallback() {
        let fake = FakeCodex::new("hung_initialize");
        let error = refresh_account_snapshot(test_config(&fake)).unwrap_err();

        assert_eq!(error.kind, AccountConnectorErrorKind::Timeout);
        assert_eq!(fake.launch_log().lines().count(), 1);
        assert!(!process_is_running(fake.pid()));
    }

    #[test]
    fn request_timeout_aborts_refresh_with_cleanup_and_no_later_requests() {
        let fake = FakeCodex::new("request_timeout");
        let error = refresh_account_snapshot(test_config(&fake)).unwrap_err();

        assert_eq!(error.kind, AccountConnectorErrorKind::Timeout);
        assert_eq!(error.stage, "account/rateLimits/read");
        assert!(error.timed_out);
        assert!(error.child_terminated);
        assert_eq!(fake.launch_log().lines().count(), 1);
        assert!(!fake.rpc_log().contains(r#""method":"account/usage/read""#));
        assert!(!process_is_running(fake.pid()));
    }

    #[test]
    fn malformed_method_response_aborts_with_cleanup_metadata() {
        let fake = FakeCodex::new("malformed_request");
        let error = refresh_account_snapshot(test_config(&fake)).unwrap_err();

        assert_eq!(error.kind, AccountConnectorErrorKind::Protocol);
        assert_eq!(error.stage, "account/rateLimits/read");
        assert!(error.child_terminated);
        assert!(!fake.rpc_log().contains(r#""method":"account/usage/read""#));
        assert!(!process_is_running(fake.pid()));
    }

    #[test]
    fn unrelated_stderr_that_echoes_supported_args_never_triggers_fallback() {
        let fake = FakeCodex::new("stderr_echo_args");
        let snapshot = refresh_account_snapshot(test_config(&fake)).unwrap();

        assert_eq!(snapshot.launch.mode, CodexLaunchMode::ListenStdioNoMcp);
        assert_eq!(fake.launch_log().lines().count(), 1);
    }

    #[test]
    fn early_exit_that_echoes_supported_args_never_triggers_fallback() {
        let fake = FakeCodex::new("stderr_echo_args_exit");
        let error = refresh_account_snapshot(test_config(&fake)).unwrap_err();

        assert_eq!(error.kind, AccountConnectorErrorKind::Protocol);
        assert_eq!(error.exit_code, Some(19));
        assert_eq!(fake.launch_log().lines().count(), 1);
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
        assert!(error.timed_out);
        assert!(error.child_terminated);
        assert_ne!(error.exit_code, Some(0));
    }

    #[test]
    fn malformed_app_server_output_is_a_protocol_error() {
        let fake = FakeCodex::new("malformed");
        let error = refresh_account_snapshot(test_config(&fake)).unwrap_err();

        assert_eq!(error.kind, AccountConnectorErrorKind::Protocol);
        assert_eq!(error.stage, "initialize");
        assert!(error.public_message.contains("malformed JSON-RPC output"));
    }

    #[test]
    fn wrong_id_response_is_ignored_until_matching_response_arrives() {
        let fake = FakeCodex::new("wrong_id");
        let snapshot = refresh_account_snapshot(test_config(&fake)).unwrap();

        assert_eq!(snapshot.status, AccountRefreshStatus::Connected);
        assert_eq!(snapshot.usage.lifetime_tokens, Some(987_654_321));
    }

    #[test]
    fn notification_interleaving_does_not_consume_pending_response() {
        let fake = FakeCodex::new("notification");
        let snapshot = refresh_account_snapshot(test_config(&fake)).unwrap();

        assert_eq!(snapshot.status, AccountRefreshStatus::Connected);
        assert_eq!(snapshot.reset_credits.available_count, Some(3));
    }

    #[test]
    fn rate_limit_failure_keeps_usage_as_partial_snapshot() {
        let fake = FakeCodex::new("partial_rate_limits");
        let snapshot = refresh_account_snapshot(test_config(&fake)).unwrap();

        assert_eq!(snapshot.status, AccountRefreshStatus::Degraded);
        assert_eq!(
            snapshot.method_status("account/rateLimits/read"),
            Some(MethodStatus::Failed)
        );
        assert_eq!(
            snapshot.method_status("account/usage/read"),
            Some(MethodStatus::Ok)
        );
        assert_eq!(snapshot.usage.lifetime_tokens, Some(987_654_321));
        assert!(snapshot.rate_limits.is_empty());
        assert_eq!(snapshot.reset_credits.available_count, None);
    }

    #[test]
    fn account_profile_failure_keeps_other_facets_as_partial_snapshot() {
        let fake = FakeCodex::new("partial_account_failure");
        let snapshot = refresh_account_snapshot(test_config(&fake)).unwrap();

        assert_eq!(snapshot.status, AccountRefreshStatus::Degraded);
        assert_eq!(
            snapshot.method_status("account/read"),
            Some(MethodStatus::Failed)
        );
        assert_eq!(
            snapshot.method_status("account/rateLimits/read"),
            Some(MethodStatus::Ok)
        );
        assert_eq!(
            snapshot.method_status("account/usage/read"),
            Some(MethodStatus::Ok)
        );
        assert_eq!(snapshot.account.account_label, None);
        assert_eq!(snapshot.usage.lifetime_tokens, Some(987_654_321));
    }

    #[test]
    fn server_requests_are_rejected_without_disrupting_the_pending_response() {
        let fake = FakeCodex::new("server_request");
        let snapshot = refresh_account_snapshot(test_config(&fake)).unwrap();

        assert_eq!(snapshot.status, AccountRefreshStatus::Connected);
        assert!(fake.rpc_log().lines().any(|line| {
            serde_json::from_str::<Value>(line).is_ok_and(|value| {
                value.get("id").and_then(Value::as_i64) == Some(77)
                    && value.pointer("/error/code").and_then(Value::as_i64) == Some(-32601)
            })
        }));
    }

    #[test]
    fn stderr_is_redacted_and_bounded() {
        let fake = FakeCodex::new("stderr_flood");
        let snapshot = refresh_account_snapshot(test_config(&fake)).unwrap();

        assert!(snapshot.diagnostics.stderr_tail.len() <= 4_096);
        assert!(!snapshot
            .diagnostics
            .stderr_tail
            .contains("secret-token-value"));
        assert!(snapshot.diagnostics.stderr_tail.contains("[REDACTED]"));
    }

    #[test]
    fn unicode_stderr_suffix_is_bounded_without_splitting_utf8() {
        let fake = FakeCodex::new("unicode_stderr_flood");
        let snapshot = refresh_account_snapshot(test_config(&fake)).unwrap();

        assert!(snapshot.diagnostics.stderr_tail.len() <= 4_096);
        assert!(!snapshot
            .diagnostics
            .stderr_tail
            .contains("secret-token-value"));
    }

    #[test]
    fn split_line_stderr_credentials_are_redacted_as_one_suffix() {
        let fake = FakeCodex::new("split_line_secret");
        let snapshot = refresh_account_snapshot(test_config(&fake)).unwrap();

        assert!(!snapshot.diagnostics.stderr_tail.contains("eyJhbGci"));
        assert!(snapshot.diagnostics.stderr_tail.contains("[REDACTED]"));
    }

    #[cfg(not(windows))]
    fn process_is_running(pid: u32) -> bool {
        PathBuf::from(format!("/proc/{pid}")).exists()
    }

    #[cfg(windows)]
    fn process_is_running(pid: u32) -> bool {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
    }

    #[test]
    fn early_exit_captures_child_exit_code() {
        let fake = FakeCodex::new("early_exit");
        let error = refresh_account_snapshot(test_config(&fake)).unwrap_err();

        assert_eq!(error.kind, AccountConnectorErrorKind::Protocol);
        assert_eq!(error.exit_code, Some(17));
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
            "rateLimitResetCredits": { "availableCount": 0, "credits": [] }
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
    fn generated_rate_limit_fixture_prefers_map_and_parses_credit_details() {
        let value: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/app_server/rate_limits_current.json"
        ))
        .unwrap();

        let normalized = normalize_rate_limits(&value).unwrap();

        assert_eq!(normalized.rate_limits.len(), 2);
        assert_eq!(normalized.rate_limits[0].bucket_id, "codex");
        assert_eq!(normalized.rate_limits[0].windows.len(), 2);
        assert_eq!(
            normalized.rate_limits[1].windows[0].window_label,
            "42-minute"
        );
        assert_eq!(normalized.reset_credits.available_count, Some(2));
        assert_eq!(normalized.reset_credits.credits.as_ref().unwrap().len(), 2);
        assert_eq!(
            normalized.reset_credits.credits.as_ref().unwrap()[0].expires_at_utc,
            None
        );
    }

    #[test]
    fn generated_usage_fixture_preserves_explicit_zero_daily_tokens() {
        let value: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/app_server/usage_current.json"
        ))
        .unwrap();

        let normalized = normalize_usage(&value).unwrap();

        assert_eq!(normalized.lifetime_tokens, Some(1234));
        let buckets = &normalized.daily_buckets;
        assert_eq!(buckets[0].date, "2026-07-09");
        assert_eq!(buckets[0].total_tokens, 0);
        assert_eq!(
            normalized.daily_buckets_status,
            OptionalCollectionStatus::Present
        );
    }

    #[test]
    fn generated_rate_limit_fallback_fixture_uses_single_snapshot() {
        let value: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/app_server/rate_limits_fallback.json"
        ))
        .unwrap();

        let normalized = normalize_rate_limits(&value).unwrap();

        assert_eq!(normalized.rate_limits.len(), 1);
        assert_eq!(normalized.rate_limits[0].bucket_id, "codex");
        assert_eq!(normalized.rate_limits[0].windows[0].used_percent, 7.5);
        assert_eq!(normalized.reset_credits.available_count, None);
    }

    #[test]
    fn generated_missing_optional_fixture_preserves_null_and_explicit_zero() {
        let value: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/app_server/rate_limits_missing_optional.json"
        ))
        .unwrap();

        let normalized = normalize_rate_limits(&value).unwrap();

        assert_eq!(
            normalized.rate_limits[0].windows[0].window_duration_mins,
            None
        );
        assert_eq!(normalized.rate_limits[0].windows[0].window_label, "unknown");
        assert_eq!(normalized.reset_credits.available_count, Some(0));
        assert!(normalized.reset_credits.credits.is_none());
    }

    #[test]
    fn generated_malformed_required_window_field_is_a_method_error() {
        let value: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/app_server/rate_limits_malformed.json"
        ))
        .unwrap();

        let error = normalize_rate_limits(&value).unwrap_err();

        assert_eq!(error.stage, "account/rateLimits/read");
        assert!(error.public_message.contains("usedPercent"));
    }

    #[test]
    fn missing_or_wrong_rate_limit_containers_are_method_errors() {
        for value in [
            serde_json::json!({}),
            serde_json::json!({ "rateLimits": {}, "rateLimitsByLimitId": [] }),
        ] {
            assert!(normalize_rate_limits(&value).is_err(), "accepted {value}");
        }
    }

    #[test]
    fn partial_method_fixture_keeps_valid_facets_and_rejects_only_usage() {
        let value: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/app_server/partial_methods.json"
        ))
        .unwrap();

        assert!(normalize_account(&value["account/read"]).is_ok());
        assert!(normalize_rate_limits(&value["account/rateLimits/read"]).is_ok());
        assert!(normalize_usage(&value["account/usage/read"]).is_err());
    }

    #[test]
    fn malformed_account_required_shape_is_a_method_error() {
        for value in [
            serde_json::json!({}),
            serde_json::json!({ "account": {}, "requiresOpenaiAuth": true }),
            serde_json::json!({ "account": { "type": "chatgpt", "planType": "pro" }, "requiresOpenaiAuth": true }),
        ] {
            let error = normalize_account(&value).unwrap_err();
            assert_eq!(error.stage, "account/read");
        }
        assert!(normalize_account(&serde_json::json!({
            "account": null,
            "requiresOpenaiAuth": true
        }))
        .is_ok());
    }

    #[test]
    fn malformed_daily_row_fails_usage_method_without_rejecting_explicit_zero() {
        for row in [
            serde_json::json!({ "tokens": 1 }),
            serde_json::json!({ "startDate": "2026-07-10" }),
            serde_json::json!({ "startDate": "2026-07-10", "tokens": "1" }),
        ] {
            let error = normalize_usage(&serde_json::json!({
                "summary": { "lifetimeTokens": 0 },
                "dailyUsageBuckets": [row]
            }))
            .unwrap_err();
            assert_eq!(error.stage, "account/usage/read");
        }
        assert_eq!(
            normalize_usage(&serde_json::json!({
                "summary": { "lifetimeTokens": 0 },
                "dailyUsageBuckets": [{ "startDate": "2026-07-10", "tokens": 0 }]
            }))
            .unwrap()
            .daily_buckets[0]
                .total_tokens,
            0
        );
    }

    #[test]
    fn malformed_reset_credit_detail_fails_rate_limit_method() {
        let mut value: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/app_server/rate_limits_fallback.json"
        ))
        .unwrap();
        value["rateLimitResetCredits"] = serde_json::json!({
            "availableCount": 1,
            "credits": [{ "id": "synthetic", "resetType": "primary" }]
        });

        let error = normalize_rate_limits(&value).unwrap_err();

        assert_eq!(error.stage, "account/rateLimits/read");
        assert!(error.public_message.contains("reset-credit"));
    }

    #[test]
    fn nullable_and_absent_usage_buckets_remain_distinguishable() {
        let null_buckets = normalize_usage(&serde_json::json!({
            "summary": { "lifetimeTokens": null },
            "dailyUsageBuckets": null
        }))
        .unwrap();
        let empty_buckets = normalize_usage(&serde_json::json!({
            "summary": { "lifetimeTokens": 0 },
            "dailyUsageBuckets": []
        }))
        .unwrap();

        assert_eq!(
            null_buckets.daily_buckets_status,
            OptionalCollectionStatus::Null
        );
        assert_eq!(
            empty_buckets.daily_buckets_status,
            OptionalCollectionStatus::Present
        );
        assert!(empty_buckets.daily_buckets.is_empty());
        assert!(normalize_usage(&serde_json::json!({ "summary": {} })).is_err());
    }

    #[test]
    fn consume_reset_credit_method_is_not_a_client_method() {
        let fake = FakeCodex::new("happy");
        refresh_account_snapshot(test_config(&fake)).unwrap();

        let rpc_log = fake.rpc_log();
        let outbound_account_methods = rpc_log
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter_map(|message| message.get("method")?.as_str().map(str::to_string))
            .filter(|method| method.starts_with("account/"))
            .collect::<Vec<_>>();

        assert_eq!(outbound_account_methods, ACCOUNT_READ_METHODS);
        assert!(!rpc_log.contains(&forbidden_consume_method()));
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

    #[test]
    fn expected_red_configured_runtime_is_not_discovered_until_task_two() {
        let configured = CodexLaunchSpec {
            executable_path: PathBuf::from("configured-codex"),
            argv_prefix: vec!["fixed-prefix".to_string()],
        };
        let candidates = discover_codex_runtimes(&CodexRuntimeSettings {
            configured_runtime: Some(configured.clone()),
        });

        assert_eq!(
            candidates.first().map(|candidate| &candidate.launch),
            Some(&configured)
        );
    }

    #[test]
    fn missing_configured_runtime_error_retains_production_resolution_diagnostics() {
        let missing = tempfile::tempdir()
            .unwrap()
            .path()
            .join("Missing Codex")
            .join("codex.exe");
        let error = resolve_codex_executable(Some(&CodexLaunchSpec {
            executable_path: missing.clone(),
            argv_prefix: vec!["fixed-entry.js".to_string()],
        }))
        .unwrap_err();

        assert_eq!(
            error.launch.selected_executable,
            missing.display().to_string()
        );
        assert_eq!(error.launch.argv_prefix, vec!["fixed-entry.js"]);
        assert_eq!(error.launch.candidates, vec![missing.display().to_string()]);
    }
}
