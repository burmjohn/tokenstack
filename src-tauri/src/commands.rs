use crate::analytics::{build_dashboard_summary, DashboardSummaryDto};
use crate::codex_app_server::{
    refresh_account_snapshot, validate_codex_app_server_runtime, AccountConnectorError,
    AccountConnectorErrorKind, CodexAppServerConfig,
};
use crate::codex_runtime::{
    discover_codex_runtimes, discover_codex_runtimes_with, select_codex_runtime as select_runtime,
    CodexLaunchSpec, CodexRuntimeSettings, CodexRuntimeSource, RuntimeDiscoveryContext,
};
use crate::db::{insert_account_refresh_error, insert_account_snapshot, open_path};
use crate::discovery::{default_app_data_dir, default_auth_home, default_local_history_roots};
use crate::importers::LocalHistoryImporter;
use crate::settings::{
    clear_configured_runtime as clear_runtime_setting, load_configured_runtime,
    save_configured_runtime, ConfiguredCodexRuntime,
};
use crate::telemetry::redact_sensitive;
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use tauri_plugin_dialog::DialogExt;

static REFRESH_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static EXPORT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[tauri::command]
pub fn get_dashboard_summary(data_mode: String) -> Result<DashboardSummaryDto, String> {
    get_dashboard_summary_from_path(data_mode, default_app_data_dir())
}

#[tauri::command]
pub fn refresh_all(data_mode: String) -> Result<DashboardSummaryDto, String> {
    refresh_all_with_auth_home(
        data_mode,
        default_app_data_dir(),
        default_local_history_roots(),
        default_auth_home(),
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRuntimeCandidateDto {
    display_path: String,
    source: CodexRuntimeSource,
    exists: bool,
    executable: Option<bool>,
    version: Option<String>,
    validation_error: Option<String>,
    configured: bool,
    selected: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CodexRuntimeSelectionDto {
    display_path: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRuntimeValidationDto {
    valid: bool,
    version: Option<String>,
    error: Option<String>,
}

#[tauri::command]
pub fn list_codex_runtimes() -> Result<Vec<CodexRuntimeCandidateDto>, String> {
    let conn = open_app_database(&default_app_data_dir())?;
    let configured = load_configured_runtime(&conn).map_err(|e| e.to_string())?;
    let candidates = discover_codex_runtimes(&CodexRuntimeSettings {
        configured_runtime: configured.as_ref().map(|v| v.launch.clone()),
    });
    let selected_launch = configured
        .as_ref()
        .map(|runtime| runtime.launch.clone())
        .or_else(|| crate::codex_runtime::select_codex_runtime(&candidates).cloned());
    Ok(candidates
        .into_iter()
        .map(|v| CodexRuntimeCandidateDto {
            configured: configured.as_ref().is_some_and(|runtime| {
                runtime.display_path == v.display_path && runtime.launch == v.launch
            }),
            selected: selected_launch
                .as_ref()
                .is_some_and(|launch| *launch == v.launch),
            display_path: path_label(&v.display_path),
            source: v.source,
            exists: v.exists,
            executable: v.executable,
            version: v.version,
            validation_error: v.validation_error,
        })
        .collect())
}

#[tauri::command]
pub fn select_codex_runtime(
    selection: CodexRuntimeSelectionDto,
) -> Result<CodexRuntimeValidationDto, String> {
    let conn = open_app_database(&default_app_data_dir())?;
    let settings = load_configured_runtime(&conn).map_err(|e| e.to_string())?;
    let candidates = discover_codex_runtimes(&CodexRuntimeSettings {
        configured_runtime: settings.map(|v| v.launch),
    });
    select_codex_runtime_in(&conn, selection, &candidates, |spec| {
        validate_codex_app_server_runtime(spec).map_err(|error| error.public_message)
    })
}

fn select_codex_runtime_in(
    conn: &Connection,
    selection: CodexRuntimeSelectionDto,
    candidates: &[crate::codex_runtime::CodexRuntimeCandidate],
    validate: impl FnOnce(&CodexLaunchSpec) -> Result<(), String>,
) -> Result<CodexRuntimeValidationDto, String> {
    let selected_path = selection.display_path;
    let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.display_path == selected_path)
    else {
        return Ok(CodexRuntimeValidationDto {
            valid: false,
            version: None,
            error: Some("runtime candidate is not backend-discovered".into()),
        });
    };
    persist_validated_runtime(
        conn,
        selected_path,
        candidate.launch.clone(),
        candidate.source,
        candidate.version.clone(),
        validate,
    )
}

fn persist_validated_runtime(
    conn: &Connection,
    display_path: PathBuf,
    launch: CodexLaunchSpec,
    source: CodexRuntimeSource,
    version: Option<String>,
    validate: impl FnOnce(&CodexLaunchSpec) -> Result<(), String>,
) -> Result<CodexRuntimeValidationDto, String> {
    match validate(&launch) {
        Ok(()) => {
            let stored = ConfiguredCodexRuntime {
                display_path,
                launch,
                source,
                validated_at_utc: chrono::Utc::now().to_rfc3339(),
                version: version
                    .clone()
                    .unwrap_or_else(|| "app-server handshake verified".into()),
            };
            save_configured_runtime(conn, &stored).map_err(|e| e.to_string())?;
            Ok(CodexRuntimeValidationDto {
                valid: true,
                version: Some(stored.version),
                error: None,
            })
        }
        Err(error) => Ok(CodexRuntimeValidationDto {
            valid: false,
            version: None,
            error: Some(error),
        }),
    }
}

#[tauri::command]
pub async fn choose_codex_runtime(
    app: tauri::AppHandle,
) -> Result<CodexRuntimeValidationDto, String> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choose a Codex runtime")
            .blocking_pick_file()
    })
    .await
    .map_err(|error| error.to_string())?;
    let Some(path) = picked else {
        return Ok(CodexRuntimeValidationDto {
            valid: false,
            version: None,
            error: Some("selection cancelled".into()),
        });
    };
    let path = path
        .into_path()
        .map_err(|_| "selected item is not a native file path".to_string())?;
    let conn = open_app_database(&default_app_data_dir())?;
    choose_codex_runtime_in(&conn, path, |spec| {
        validate_codex_app_server_runtime(spec).map_err(|error| error.public_message)
    })
}

fn choose_codex_runtime_in(
    conn: &Connection,
    path: PathBuf,
    validate: impl FnOnce(&CodexLaunchSpec) -> Result<(), String>,
) -> Result<CodexRuntimeValidationDto, String> {
    let launch = CodexLaunchSpec {
        executable_path: path.clone(),
        argv_prefix: Vec::new(),
    };
    persist_validated_runtime(
        conn,
        path,
        launch,
        CodexRuntimeSource::Configured,
        None,
        validate,
    )
}

#[tauri::command]
pub fn clear_codex_runtime(data_mode: String) -> Result<DashboardSummaryDto, String> {
    let conn = open_app_database(&default_app_data_dir())?;
    clear_runtime_setting(&conn).map_err(|e| e.to_string())?;
    drop(conn);
    refresh_all(data_mode)
}

#[tauri::command]
pub fn validate_codex_runtime() -> Result<CodexRuntimeValidationDto, String> {
    let conn = open_app_database(&default_app_data_dir())?;
    let candidates = discover_codex_runtimes(&CodexRuntimeSettings::default());
    validate_codex_runtime_in(&conn, &candidates, |spec| {
        validate_codex_app_server_runtime(spec).map_err(|error| error.public_message)
    })
}

fn validate_codex_runtime_in(
    conn: &Connection,
    candidates: &[crate::codex_runtime::CodexRuntimeCandidate],
    validate: impl FnOnce(&CodexLaunchSpec) -> Result<(), String>,
) -> Result<CodexRuntimeValidationDto, String> {
    let runtime = match load_configured_runtime(conn).map_err(|e| e.to_string())? {
        Some(runtime) => runtime,
        None => {
            let Some(candidate) = candidates
                .iter()
                .find(|candidate| candidate.executable == Some(true))
            else {
                return Ok(CodexRuntimeValidationDto {
                    valid: false,
                    version: None,
                    error: Some("no valid automatic runtime found".into()),
                });
            };
            return Ok(match validate(&candidate.launch) {
                Ok(()) => CodexRuntimeValidationDto {
                    valid: true,
                    version: candidate
                        .version
                        .clone()
                        .or_else(|| Some("app-server handshake verified".into())),
                    error: None,
                },
                Err(error) => CodexRuntimeValidationDto {
                    valid: false,
                    version: None,
                    error: Some(error),
                },
            });
        }
    };
    Ok(match validate(&runtime.launch) {
        Ok(()) => CodexRuntimeValidationDto {
            valid: true,
            version: Some(runtime.version),
            error: None,
        },
        Err(error) => CodexRuntimeValidationDto {
            valid: false,
            version: None,
            error: Some(error),
        },
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupDiagnosticsDto {
    pub schema_version: i64,
    pub data_mode: String,
    pub app_data_dir: String,
    pub database_path: String,
    pub auth_home: String,
    pub selected_codex_executable: Option<String>,
    pub configured_codex_runtime_display: Option<String>,
    pub codex_launch_mode: Option<String>,
    pub first_failing_account_stage: Option<String>,
    pub last_successful_account_refresh: Option<String>,
    pub usage_event_count: i64,
    pub usage_total_tokens: i64,
    pub source_document_count: i64,
    pub local_roots: Vec<LocalRootDiagnosticsDto>,
    pub latest_import_run: Option<ImportRunDiagnosticsDto>,
    pub connector_runs: Vec<ConnectorRunDiagnosticsDto>,
    pub runtime_candidates: Vec<RuntimeCandidateDiagnosticsDto>,
    pub selected_runtime: Option<SelectedRuntimeDiagnosticsDto>,
    pub latest_account_run: Option<AccountRunDiagnosticsDto>,
    pub displayed_metrics: Vec<DisplayedMetricDiagnosticsDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCandidateDiagnosticsDto {
    pub display_path: String,
    pub native_executable_path: String,
    pub argv_prefix: Vec<String>,
    pub source: CodexRuntimeSource,
    pub exists: bool,
    pub executable: Option<bool>,
    pub version: Option<String>,
    pub validation_status: String,
    pub validation_error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedRuntimeDiagnosticsDto {
    pub display_path: String,
    pub native_executable_path: String,
    pub argv_prefix: Vec<String>,
    pub source: CodexRuntimeSource,
    pub version: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRunDiagnosticsDto {
    pub status: String,
    pub started_at_utc: String,
    pub completed_at_utc: String,
    pub duration_ms: Option<i64>,
    pub selected_executable: Option<String>,
    pub selected_display_path: Option<String>,
    pub argv_prefix: Vec<String>,
    pub runtime_source: Option<String>,
    pub launch_mode: Option<String>,
    pub launch_attempts: Vec<String>,
    pub first_failing_stage: Option<String>,
    pub error_code: Option<String>,
    pub error_message: String,
    pub exit_code: Option<i64>,
    pub timed_out: bool,
    pub child_terminated: Option<bool>,
    pub used_last_good_snapshot: bool,
    pub method_statuses: serde_json::Value,
    pub schema_fingerprint: Option<String>,
    pub account_bucket_ids: Vec<String>,
    pub daily_bucket_count: Option<i64>,
    pub reset_credit_count: Option<i64>,
    pub mcp_disabled: bool,
    pub initialize_status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayedMetricDiagnosticsDto {
    pub key: String,
    pub source: String,
    pub freshness: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRootDiagnosticsDto {
    pub path: String,
    pub exists: bool,
    pub is_directory: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRunDiagnosticsDto {
    pub completed_at_utc: String,
    pub files_seen: i64,
    pub events_seen: i64,
    pub events_imported: i64,
    pub warning_count: usize,
    pub warning_samples: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorRunDiagnosticsDto {
    pub connector_id: String,
    pub status: String,
    pub completed_at_utc: String,
    pub endpoint_id: Option<String>,
    pub http_status: Option<i64>,
    pub redacted_error_code: Option<String>,
    pub redacted_error_message: Option<String>,
}

#[tauri::command]
pub fn get_setup_diagnostics() -> Result<SetupDiagnosticsDto, String> {
    setup_diagnostics_from_parts(
        default_app_data_dir(),
        default_local_history_roots(),
        default_auth_home(),
    )
}

#[tauri::command]
pub fn save_text_export(filename: String, contents: String) -> Result<String, String> {
    save_text_export_to_dir(&filename, &contents, &default_download_dir())
        .map(|path| path_label(&path))
}

#[tauri::command]
pub fn save_binary_export(filename: String, contents: Vec<u8>) -> Result<String, String> {
    save_export_bytes_to_dir(&filename, &contents, &default_download_dir())
        .map(|path| path_label(&path))
}

#[tauri::command]
pub fn export_diagnostics(data_mode: String) -> Result<String, String> {
    if !matches!(data_mode.as_str(), "local" | "remote" | "combined") {
        return Err("invalid diagnostics data mode".into());
    }
    let mut diagnostics = get_setup_diagnostics()?;
    diagnostics.data_mode = data_mode.clone();
    let database_path = PathBuf::from(&diagnostics.database_path);
    diagnostics.displayed_metrics = if database_path.exists() {
        displayed_metric_diagnostics_for_mode(
            &open_path(&database_path).map_err(|error| error.to_string())?,
            &data_mode,
        )
        .map_err(|error| error.to_string())?
    } else {
        Vec::new()
    };
    export_diagnostics_to_dir(&diagnostics, &default_app_data_dir().join("diagnostics"))
        .map(|path| path_label(&path))
}

pub fn get_dashboard_summary_from_path(
    data_mode: String,
    app_data_dir: PathBuf,
) -> Result<DashboardSummaryDto, String> {
    let conn = open_app_database(&app_data_dir)?;
    build_dashboard_summary(&conn, &data_mode).map_err(|error| error.to_string())
}

fn setup_diagnostics_from_parts(
    app_data_dir: PathBuf,
    roots: Vec<PathBuf>,
    auth_home: PathBuf,
) -> Result<SetupDiagnosticsDto, String> {
    setup_diagnostics_from_parts_with_context(
        app_data_dir,
        roots,
        auth_home,
        &RuntimeDiscoveryContext::from_current_process(),
    )
}

fn setup_diagnostics_from_parts_with_context(
    app_data_dir: PathBuf,
    roots: Vec<PathBuf>,
    auth_home: PathBuf,
    runtime_context: &RuntimeDiscoveryContext,
) -> Result<SetupDiagnosticsDto, String> {
    let database_path = app_data_dir.join("tokenstack.sqlite3");
    let local_roots = roots.into_iter().map(local_root_diagnostics).collect();

    let (
        latest_import_run,
        connector_runs,
        account_refresh,
        usage_event_count,
        usage_total_tokens,
        source_document_count,
    ) = if database_path.exists() {
        let conn = open_path(&database_path).map_err(|error| error.to_string())?;
        (
            latest_import_run_diagnostics(&conn).map_err(|error| error.to_string())?,
            connector_run_diagnostics(&conn).map_err(|error| error.to_string())?,
            latest_account_refresh_diagnostics(&conn).map_err(|error| error.to_string())?,
            count_rows(&conn, "usage_events").map_err(|error| error.to_string())?,
            usage_total_tokens(&conn).map_err(|error| error.to_string())?,
            count_rows(&conn, "source_documents").map_err(|error| error.to_string())?,
        )
    } else {
        (None, Vec::new(), None, 0, 0, 0)
    };

    let configured_runtime = if database_path.exists() {
        load_configured_runtime(&open_path(&database_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?
    } else {
        None
    };
    let candidates = discover_codex_runtimes_with(
        &CodexRuntimeSettings {
            configured_runtime: configured_runtime
                .as_ref()
                .map(|runtime| runtime.launch.clone()),
        },
        runtime_context,
    );
    let runtime_candidates = candidates
        .iter()
        .map(|candidate| RuntimeCandidateDiagnosticsDto {
            display_path: path_label(&candidate.display_path),
            native_executable_path: path_label(&candidate.launch.executable_path),
            argv_prefix: candidate.launch.argv_prefix.clone(),
            source: candidate.source,
            exists: candidate.exists,
            executable: candidate.executable,
            version: candidate.version.clone(),
            validation_status: if candidate.validation_error.is_none()
                && candidate.executable == Some(true)
            {
                "valid".into()
            } else if candidate.exists {
                "invalid".into()
            } else {
                "missing".into()
            },
            validation_error: candidate.validation_error.clone(),
        })
        .collect();
    let latest_selected_runtime: Option<(String, String, Vec<String>, CodexRuntimeSource)> =
        if database_path.exists() {
            open_path(&database_path).map_err(|error| error.to_string())?.query_row(
            "SELECT selected_codex_executable, runtime_display_path, argv_prefix_json, runtime_source FROM account_refresh_runs ORDER BY id DESC LIMIT 1", [], |row| {
                let native: Option<String> = row.get(0)?; let display: Option<String> = row.get(1)?; let argv: String = row.get(2)?; let source: Option<String> = row.get(3)?;
                Ok(native.map(|native| (display.unwrap_or_else(|| native.clone()), native, serde_json::from_str(&argv).unwrap_or_default(), runtime_source_from_label(source.as_deref()))))
            }
        ).optional().map_err(|error| error.to_string())?.flatten()
        } else {
            None
        };
    let selected_runtime = configured_runtime
        .as_ref()
        .map(|runtime| SelectedRuntimeDiagnosticsDto {
            display_path: path_label(&runtime.display_path),
            native_executable_path: path_label(&runtime.launch.executable_path),
            argv_prefix: runtime.launch.argv_prefix.clone(),
            source: runtime.source,
            version: runtime.version.clone(),
        })
        .or_else(|| {
            latest_selected_runtime.map(|(display, native, argv_prefix, source)| {
                let version = candidates
                    .iter()
                    .find(|candidate| {
                        path_label(&candidate.display_path) == display
                            && path_label(&candidate.launch.executable_path) == native
                            && candidate.launch.argv_prefix == argv_prefix
                    })
                    .and_then(|candidate| candidate.version.clone())
                    .unwrap_or_else(|| "unknown".into());
                SelectedRuntimeDiagnosticsDto {
                    display_path: display,
                    native_executable_path: native,
                    argv_prefix,
                    source,
                    version,
                }
            })
        });
    let configured_display = configured_runtime
        .as_ref()
        .map(|runtime| path_label(&runtime.display_path));
    let selected_executable = selected_runtime
        .as_ref()
        .map(|runtime| runtime.native_executable_path.clone());
    let selected_launch_mode = account_refresh
        .as_ref()
        .and_then(|refresh| refresh.launch_mode.clone());
    Ok(SetupDiagnosticsDto {
        schema_version: 2,
        data_mode: "combined".into(),
        app_data_dir: path_label(&app_data_dir),
        database_path: path_label(&database_path),
        auth_home: path_label(&auth_home),
        selected_codex_executable: selected_executable,
        configured_codex_runtime_display: configured_display,
        codex_launch_mode: selected_launch_mode,
        first_failing_account_stage: account_refresh
            .as_ref()
            .and_then(|refresh| refresh.first_failing_stage.clone()),
        last_successful_account_refresh: latest_successful_account_refresh(&database_path)
            .map_err(|error| error.to_string())?,
        usage_event_count,
        usage_total_tokens,
        source_document_count,
        local_roots,
        latest_import_run,
        connector_runs,
        runtime_candidates,
        selected_runtime,
        latest_account_run: if database_path.exists() {
            latest_account_run_diagnostics(
                &open_path(&database_path).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?
        } else {
            None
        },
        displayed_metrics: if database_path.exists() {
            displayed_metric_diagnostics(
                &open_path(&database_path).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?
        } else {
            Vec::new()
        },
    })
}

fn latest_account_run_diagnostics(
    conn: &Connection,
) -> rusqlite::Result<Option<AccountRunDiagnosticsDto>> {
    conn.query_row(
        r#"SELECT id, started_at_utc, completed_at_utc, status, selected_codex_executable,
                  launch_mode, executable_candidates_json, first_failing_stage,
                  redacted_error_code, redacted_error_message, used_last_good_snapshot,
                  method_statuses_json, exit_code, timed_out, child_terminated,
                  runtime_display_path, argv_prefix_json, runtime_source
           FROM account_refresh_runs ORDER BY id DESC LIMIT 1"#,
        [],
        |row| {
            let run_id: i64 = row.get(0)?;
            let started: String = row.get(1)?;
            let completed: String = row.get(2)?;
            let error_code: Option<String> = row.get(8)?;
            let error_message: String = row.get(9)?;
            let duration_ms = chrono::DateTime::parse_from_rfc3339(&started).ok().and_then(|start|
                chrono::DateTime::parse_from_rfc3339(&completed).ok().map(|end| (end - start).num_milliseconds()));
            let schema_fingerprint = conn.query_row(
                "SELECT schema_fingerprint FROM account_method_attempts WHERE refresh_run_id = ?1 ORDER BY id DESC LIMIT 1",
                [run_id], |method_row| method_row.get(0)).optional()?;
            let method_statuses: serde_json::Value = serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_else(|_| serde_json::json!([]));
            let method_ok = |name: &str| method_statuses.as_array().is_some_and(|items| items.iter().any(|item| item["method"] == name && item["status"] == "ok"));
            let account_bucket_ids = conn.prepare("SELECT bucket_id FROM account_rate_limit_buckets WHERE refresh_run_id = ?1 ORDER BY bucket_id")?
                .query_map([run_id], |item| item.get(0))?.collect::<rusqlite::Result<Vec<String>>>()?;
            let daily_bucket_count = if method_ok("account/usage/read") { Some(conn.query_row("SELECT COUNT(*) FROM account_daily_usage_buckets WHERE refresh_run_id = ?1", [run_id], |item| item.get(0))?) } else { None };
            let reset_credit_count = if method_ok("account/rateLimits/read") { conn.query_row("SELECT available_count FROM account_reset_credit_snapshots WHERE refresh_run_id = ?1", [run_id], |item| item.get(0)).optional()? } else { None };
            let timeout_text = format!("{} {}", error_code.as_deref().unwrap_or_default(), error_message).to_lowercase();
            let status: String = row.get(3)?;
            let failing_stage: Option<String> = row.get(7)?;
            let initialize_status = if failing_stage.as_deref().is_some_and(|stage| stage.contains("initialize")) {
                "failed"
            } else if failing_stage.as_deref().is_some_and(|stage| stage.contains("spawn") || stage.contains("resolve") || stage.contains("discover")) {
                "not_attempted"
            } else if status == "connected" || status == "partial" || method_statuses.as_array().is_some_and(|items| items.iter().any(|item| item["method"].as_str().is_some_and(|method| method.starts_with("account/")))) {
                "ok"
            } else { "unknown" };
            Ok(AccountRunDiagnosticsDto {
                status, started_at_utc: started, completed_at_utc: completed, duration_ms,
                selected_executable: row.get(4)?, launch_mode: row.get(5)?,
                selected_display_path: row.get(15)?, argv_prefix: serde_json::from_str(&row.get::<_, String>(16)?).unwrap_or_default(), runtime_source: row.get(17)?,
                launch_attempts: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(6)?).unwrap_or_default(),
                first_failing_stage: failing_stage, error_code, error_message,
                exit_code: row.get(12)?, timed_out: row.get::<_, i64>(13)? != 0 || timeout_text.contains("timeout") || timeout_text.contains("timed out"),
                child_terminated: row.get::<_, Option<i64>>(14)?.map(|value| value != 0), used_last_good_snapshot: row.get::<_, i64>(10)? != 0,
                method_statuses, schema_fingerprint, account_bucket_ids, daily_bucket_count, reset_credit_count,
                mcp_disabled: row.get::<_, Option<String>>(5)?.as_deref() == Some("listen_stdio_no_mcp"),
                initialize_status: initialize_status.into(),
            })
        },
    ).optional()
}

fn runtime_source_from_label(source: Option<&str>) -> CodexRuntimeSource {
    match source {
        Some("configured") => CodexRuntimeSource::Configured,
        Some("environment") => CodexRuntimeSource::Environment,
        Some("codex_app") => CodexRuntimeSource::CodexApp,
        Some("npm") => CodexRuntimeSource::Npm,
        Some("standalone") => CodexRuntimeSource::Standalone,
        Some("msix") => CodexRuntimeSource::Msix,
        _ => CodexRuntimeSource::Path,
    }
}

fn displayed_metric_diagnostics(
    conn: &Connection,
) -> rusqlite::Result<Vec<DisplayedMetricDiagnosticsDto>> {
    displayed_metric_diagnostics_for_mode(conn, "combined")
}

fn displayed_metric_diagnostics_for_mode(
    conn: &Connection,
    data_mode: &str,
) -> rusqlite::Result<Vec<DisplayedMetricDiagnosticsDto>> {
    let summary = build_dashboard_summary(conn, data_mode)?;
    let connector_freshness = summary
        .connectors
        .iter()
        .map(|connector| (connector.id.as_str(), connector.freshness.as_str()))
        .collect::<std::collections::HashMap<_, _>>();
    Ok(summary
        .metrics
        .into_iter()
        .chain(summary.account_metrics)
        .chain(summary.local_metrics)
        .map(|metric| {
            let connector_id = match metric.coverage.metric_key.as_str() {
                "account-usage" => Some("account-usage"),
                "reset-credits" => Some("known-reset-credit"),
                "rate-limit-windows" => Some("rate-limit-windows"),
                "local-usage" | "local-history" => Some("local"),
                _ => None,
            };
            let freshness =
                if metric.value == "Unavailable" || metric.coverage.confidence == "unavailable" {
                    "unavailable"
                } else {
                    connector_id
                        .and_then(|id| connector_freshness.get(id).copied())
                        .unwrap_or("fresh")
                };
            DisplayedMetricDiagnosticsDto {
                key: metric.key,
                source: metric.coverage.source_kind,
                freshness: freshness.to_string(),
                status: metric.status,
            }
        })
        .collect())
}

#[derive(Debug)]
struct AccountRefreshDiagnosticsRow {
    first_failing_stage: Option<String>,
    launch_mode: Option<String>,
}

fn local_root_diagnostics(path: PathBuf) -> LocalRootDiagnosticsDto {
    let metadata = std::fs::metadata(&path).ok();
    LocalRootDiagnosticsDto {
        path: path_label(&path),
        exists: metadata.is_some(),
        is_directory: metadata
            .as_ref()
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false),
    }
}

fn latest_import_run_diagnostics(
    conn: &Connection,
) -> rusqlite::Result<Option<ImportRunDiagnosticsDto>> {
    conn.query_row(
        r#"
        SELECT COALESCE(completed_at_utc, started_at_utc), files_seen, events_seen,
               events_imported, warning_count, warnings_json
        FROM import_runs
        ORDER BY id DESC
        LIMIT 1
        "#,
        [],
        |row| {
            let warning_count: i64 = row.get(4)?;
            let warnings_json: String = row.get(5)?;
            let warnings = parse_warning_samples(&warnings_json);
            Ok(ImportRunDiagnosticsDto {
                completed_at_utc: row.get(0)?,
                files_seen: row.get(1)?,
                events_seen: row.get(2)?,
                events_imported: row.get(3)?,
                warning_count: warning_count.max(0) as usize,
                warning_samples: warnings.into_iter().take(20).collect(),
            })
        },
    )
    .optional()
}

fn connector_run_diagnostics(
    conn: &Connection,
) -> rusqlite::Result<Vec<ConnectorRunDiagnosticsDto>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT connector_id, status, COALESCE(completed_at_utc, started_at_utc),
               endpoint_id, http_status, redacted_error_code, redacted_error_message
        FROM connector_runs
        ORDER BY id DESC
        LIMIT 8
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ConnectorRunDiagnosticsDto {
            connector_id: row.get(0)?,
            status: row.get(1)?,
            completed_at_utc: row.get(2)?,
            endpoint_id: row.get(3)?,
            http_status: row.get(4)?,
            redacted_error_code: row.get(5)?,
            redacted_error_message: row.get(6)?,
        })
    })?;
    rows.collect()
}

fn latest_account_refresh_diagnostics(
    conn: &Connection,
) -> rusqlite::Result<Option<AccountRefreshDiagnosticsRow>> {
    conn.query_row(
        r#"
        SELECT first_failing_stage, launch_mode
        FROM account_refresh_runs
        ORDER BY id DESC
        LIMIT 1
        "#,
        [],
        |row| {
            Ok(AccountRefreshDiagnosticsRow {
                first_failing_stage: row.get(0)?,
                launch_mode: row.get(1)?,
            })
        },
    )
    .optional()
}

fn latest_successful_account_refresh(database_path: &Path) -> rusqlite::Result<Option<String>> {
    if !database_path.exists() {
        return Ok(None);
    }
    let conn = open_path(database_path)?;
    conn.query_row(
        r#"
        SELECT completed_at_utc
        FROM account_refresh_runs
        WHERE status = 'connected'
        ORDER BY id DESC
        LIMIT 1
        "#,
        [],
        |row| row.get(0),
    )
    .optional()
}

fn parse_warning_samples(warnings_json: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(warnings_json).unwrap_or_default()
}

fn count_rows(conn: &Connection, table: &str) -> rusqlite::Result<i64> {
    let query = format!("SELECT COUNT(*) FROM {table}");
    conn.query_row(&query, [], |row| row.get(0))
}

fn usage_total_tokens(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(total_tokens), 0) FROM usage_events",
        [],
        |row| row.get(0),
    )
}

fn path_label(path: &Path) -> String {
    path.display().to_string()
}

fn save_text_export_to_dir(
    filename: &str,
    contents: &str,
    target_dir: &Path,
) -> Result<PathBuf, String> {
    save_export_bytes_to_dir(filename, contents.as_bytes(), target_dir)
}

fn save_export_bytes_to_dir(
    filename: &str,
    contents: &[u8],
    target_dir: &Path,
) -> Result<PathBuf, String> {
    validate_export_filename(filename)?;
    std::fs::create_dir_all(target_dir).map_err(|error| error.to_string())?;
    let sequence = EXPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp_path = target_dir.join(format!(
        ".{filename}.{}.{}.tmp",
        std::process::id(),
        sequence
    ));
    {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|error| {
                format!(
                    "could not create diagnostics temporary file {}: {error}",
                    path_label(&temp_path)
                )
            })?;
        file.write_all(contents)
            .and_then(|_| file.sync_all())
            .map_err(|error| {
                let _ = std::fs::remove_file(&temp_path);
                format!(
                    "could not write export temporary file {}: {error}",
                    path_label(&temp_path)
                )
            })?;
    }
    let filename_path = Path::new(filename);
    let stem = filename_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "invalid export filename".to_string())?;
    let extension = filename_path.extension().and_then(|value| value.to_str());
    let mut saved_path = None;
    for suffix in 0..10_000 {
        let candidate_name = if suffix == 0 {
            filename.to_string()
        } else if let Some(extension) = extension {
            format!("{stem}-{suffix}.{extension}")
        } else {
            format!("{stem}-{suffix}")
        };
        let candidate = target_dir.join(candidate_name);
        match std::fs::hard_link(&temp_path, &candidate) {
            Ok(()) => {
                saved_path = Some(candidate);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                let _ = std::fs::remove_file(&temp_path);
                return Err(format!(
                    "could not finalize export file {}: {error}",
                    path_label(&candidate)
                ));
            }
        }
    }
    let target_path = saved_path.ok_or_else(|| {
        let _ = std::fs::remove_file(&temp_path);
        "could not allocate a unique export filename".to_string()
    })?;
    std::fs::remove_file(&temp_path)
        .map_err(|error| format!("could not remove export temporary file: {error}"))?;
    Ok(target_path)
}

fn validate_export_filename(filename: &str) -> Result<(), String> {
    let valid = !filename.is_empty()
        && filename.len() <= 160
        && filename.starts_with("tokenstack-")
        && filename.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        });
    if valid {
        Ok(())
    } else {
        Err("invalid export filename".to_string())
    }
}

fn default_download_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(path).join("Downloads");
    }
    if let Some(path) = std::env::var_os("HOME") {
        return PathBuf::from(path).join("Downloads");
    }
    default_app_data_dir()
}

fn refresh_all_with_auth_home(
    data_mode: String,
    app_data_dir: PathBuf,
    roots: Vec<PathBuf>,
    _auth_home: PathBuf,
) -> Result<DashboardSummaryDto, String> {
    refresh_all_with_context(
        data_mode,
        app_data_dir,
        roots,
        &RuntimeDiscoveryContext::from_current_process(),
    )
}

fn refresh_all_with_context(
    data_mode: String,
    app_data_dir: PathBuf,
    roots: Vec<PathBuf>,
    runtime_context: &RuntimeDiscoveryContext,
) -> Result<DashboardSummaryDto, String> {
    let _refresh_guard = refresh_lock()
        .lock()
        .map_err(|_| "refresh lock is poisoned".to_string())?;
    let conn = open_app_database(&app_data_dir)?;
    let importer = LocalHistoryImporter::new(roots);
    let _summary = importer
        .import_into(&conn)
        .map_err(|error| error.to_string())?;
    if data_mode != "local" {
        let refresh_result = account_config_from_conn_with_context(&conn, runtime_context)
            .map_err(|message| {
                AccountConnectorError::new(
                    AccountConnectorErrorKind::MissingCli,
                    "discover",
                    message,
                )
            })
            .and_then(refresh_account_snapshot);
        match refresh_result {
            Ok(snapshot) => insert_account_snapshot(&conn, &snapshot),
            Err(error) => insert_account_refresh_error(&conn, &error),
        }
        .map_err(|error| error.to_string())?;
    }
    build_dashboard_summary(&conn, &data_mode).map_err(|error| error.to_string())
}

fn account_config_from_conn(conn: &Connection) -> Result<CodexAppServerConfig, String> {
    account_config_from_conn_with_context(conn, &RuntimeDiscoveryContext::from_current_process())
}

fn account_config_from_conn_with_context(
    conn: &Connection,
    context: &RuntimeDiscoveryContext,
) -> Result<CodexAppServerConfig, String> {
    let configured = load_configured_runtime(conn).map_err(|error| error.to_string())?;
    let explicit_runtime = if let Some(runtime) = configured {
        runtime.launch
    } else {
        let candidates = discover_codex_runtimes_with(&CodexRuntimeSettings::default(), context);
        select_runtime(&candidates)
            .cloned()
            .ok_or_else(|| "no validated Codex runtime found through typed discovery".to_string())?
    };
    Ok(CodexAppServerConfig {
        explicit_runtime: Some(explicit_runtime),
        ..CodexAppServerConfig::default()
    })
}

fn refresh_lock() -> &'static Mutex<()> {
    REFRESH_LOCK.get_or_init(|| Mutex::new(()))
}

fn open_app_database(app_data_dir: &Path) -> Result<rusqlite::Connection, String> {
    std::fs::create_dir_all(app_data_dir).map_err(|error| error.to_string())?;
    open_path(&app_data_dir.join("tokenstack.sqlite3")).map_err(|error| error.to_string())
}

fn diagnostics_filename() -> String {
    format!(
        "tokenstack-diagnostics-{}-{:06}.json",
        chrono::Utc::now().format("%Y-%m-%dT%H%M%S%.9fZ"),
        EXPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}

fn export_diagnostics_to_dir(
    diagnostics: &SetupDiagnosticsDto,
    target_dir: &Path,
) -> Result<PathBuf, String> {
    let contents = sanitized_diagnostics_json(diagnostics)?;
    save_text_export_to_dir(&diagnostics_filename(), &contents, target_dir)
}

pub(crate) fn run_packaged_smoke(
    runtime: Option<PathBuf>,
    app_data_dir: PathBuf,
    diagnostics_dir: PathBuf,
) -> Result<PathBuf, String> {
    let conn = open_app_database(&app_data_dir)?;
    if let Some(runtime) = runtime {
        if !runtime.is_file() {
            return Err("packaged smoke runtime is not a file".into());
        }
        let launch = CodexLaunchSpec {
            executable_path: runtime.clone(),
            argv_prefix: Vec::new(),
        };
        validate_codex_app_server_runtime(&launch).map_err(|error| error.to_string())?;
        save_configured_runtime(
            &conn,
            &ConfiguredCodexRuntime {
                display_path: runtime,
                launch: launch.clone(),
                source: CodexRuntimeSource::Configured,
                validated_at_utc: chrono::Utc::now().to_rfc3339(),
                version: "packaged-smoke-validated".into(),
            },
        )
        .map_err(|error| error.to_string())?;
    }
    let snapshot = refresh_account_snapshot(account_config_from_conn(&conn)?)
        .map_err(|error| error.to_string())?;
    let method_ok = |method: &str| {
        snapshot.methods.iter().any(|entry| {
            entry.method == method && entry.status == crate::codex_app_server::MethodStatus::Ok
        })
    };
    if !method_ok("account/read")
        || !method_ok("account/rateLimits/read")
        || !method_ok("account/usage/read")
        || snapshot.usage.lifetime_tokens != Some(987_654_321)
        || snapshot.reset_credits.available_count != Some(3)
    {
        return Err("packaged smoke received unexpected account method results".into());
    }
    insert_account_snapshot(&conn, &snapshot).map_err(|error| error.to_string())?;
    drop(conn);
    let diagnostics = setup_diagnostics_from_parts(
        app_data_dir,
        Vec::new(),
        PathBuf::from("[smoke-auth-home-not-read]"),
    )?;
    let path = export_diagnostics_to_dir(&diagnostics, &diagnostics_dir)?;
    let contents = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let parsed: serde_json::Value =
        serde_json::from_str(&contents).map_err(|error| error.to_string())?;
    if parsed["redaction"]["status"] != "sanitized"
        || parsed["diagnostics"]["latestAccountRun"]["initializeStatus"] != "ok"
        || parsed["diagnostics"]["latestAccountRun"]["childTerminated"] != true
    {
        return Err("packaged smoke diagnostics did not prove cleanup and sanitization".into());
    }
    Ok(path)
}

fn sanitized_diagnostics_json(diagnostics: &SetupDiagnosticsDto) -> Result<String, String> {
    let mut value = serde_json::json!({
        "schemaVersion": 2,
        "generatedAtUtc": chrono::Utc::now().to_rfc3339(),
        "app": {
            "name": "TokenStack",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "runtime": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "timezone": "America/New_York",
        },
        "redaction": {
            "status": "sanitized",
            "excluded": ["auth tokens", "cookies", "prompt bodies", "response bodies", "raw JSONL conversation content", "account-identifying labels"],
        },
        "diagnostics": diagnostics,
    });
    sanitize_json_value(&mut value);
    serde_json::to_string_pretty(&value).map_err(|error| error.to_string())
}

fn sanitize_json_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) => {
            *text = redact_sensitive(text);
        }
        serde_json::Value::Array(items) => {
            for item in items {
                sanitize_json_value(item);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                let normalized = key.to_ascii_lowercase();
                if matches!(
                    normalized.as_str(),
                    "token"
                        | "authtoken"
                        | "accesstoken"
                        | "refreshtoken"
                        | "cookie"
                        | "cookies"
                        | "prompt"
                        | "promptbody"
                        | "responsebody"
                        | "rawresponse"
                        | "rawjsonl"
                        | "accountlabel"
                ) {
                    *value = serde_json::Value::String("[REDACTED]".into());
                } else {
                    sanitize_json_value(value);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_path;
    use crate::settings::{load_configured_runtime, save_configured_runtime};
    use std::fs;
    use std::io::Write;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    fn compile_fake_codex(scenario: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/fake_codex.rs");
        let bin = dir.path().join(if cfg!(windows) {
            format!("fake codex {scenario}.exe")
        } else {
            format!("fake codex {scenario}")
        });
        let output = std::process::Command::new("rustc")
            .arg("--edition=2021")
            .arg(source)
            .arg("-o")
            .arg(&bin)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        (dir, bin)
    }

    fn selected(path: &str, prefix: &str) -> ConfiguredCodexRuntime {
        ConfiguredCodexRuntime {
            display_path: PathBuf::from(path),
            launch: CodexLaunchSpec {
                executable_path: PathBuf::from(path),
                argv_prefix: vec![prefix.into()],
            },
            source: CodexRuntimeSource::Npm,
            validated_at_utc: "2026-07-10T12:00:00Z".into(),
            version: "codex 1".into(),
        }
    }

    #[test]
    fn failed_validation_preserves_last_working_selection() {
        let conn = crate::db::open_memory().unwrap();
        let working = selected("fixture-a-node", "fixture-a.js");
        save_configured_runtime(&conn, &working).unwrap();
        let attempted = CodexRuntimeSelectionDto {
            display_path: PathBuf::from("missing.cmd"),
        };

        let result = select_codex_runtime_in(&conn, attempted, &[], |spec| {
            assert!(spec.argv_prefix.is_empty());
            Err("missing runtime".into())
        })
        .unwrap();

        assert!(!result.valid);
        assert_eq!(load_configured_runtime(&conn).unwrap(), Some(working));
    }

    #[test]
    fn unknown_renderer_path_is_rejected_without_execution() {
        let conn = crate::db::open_memory().unwrap();
        let executed = std::cell::Cell::new(false);
        let result = select_codex_runtime_in(
            &conn,
            CodexRuntimeSelectionDto {
                display_path: PathBuf::from("C:/untrusted/codex.exe"),
            },
            &[],
            |_| {
                executed.set(true);
                Ok(())
            },
        )
        .unwrap();

        assert!(!result.valid);
        assert_eq!(
            result.error.as_deref(),
            Some("runtime candidate is not backend-discovered")
        );
        assert!(!executed.get());
        assert!(load_configured_runtime(&conn).unwrap().is_none());
    }

    #[test]
    fn testing_automatic_candidate_does_not_create_configured_override() {
        let conn = crate::db::open_memory().unwrap();
        let launch = CodexLaunchSpec {
            executable_path: PathBuf::from("C:/Codex/codex.exe"),
            argv_prefix: Vec::new(),
        };
        let candidate = crate::codex_runtime::CodexRuntimeCandidate {
            display_path: launch.executable_path.clone(),
            launch: launch.clone(),
            source: CodexRuntimeSource::Path,
            exists: true,
            executable: Some(true),
            version: Some("codex 1.2.3".into()),
            validation_error: None,
            validation: None,
        };

        let result = validate_codex_runtime_in(&conn, &[candidate], |actual| {
            assert_eq!(actual, &launch);
            Ok(())
        })
        .unwrap();

        assert!(result.valid);
        assert_eq!(result.version.as_deref(), Some("codex 1.2.3"));
        assert!(load_configured_runtime(&conn).unwrap().is_none());
    }

    #[test]
    fn ipc_selection_rejects_frontend_supplied_executable_and_prefix() {
        let payload = serde_json::json!({
            "displayPath": "codex.cmd",
            "launch": { "executablePath": "cmd.exe", "argvPrefix": ["/c", "anything"] }
        });
        assert!(serde_json::from_value::<CodexRuntimeSelectionDto>(payload).is_err());
    }

    #[test]
    fn version_success_without_app_server_handshake_cannot_replace_selection() {
        let conn = crate::db::open_memory().unwrap();
        let working = selected("fixture-a-node", "fixture-a.js");
        save_configured_runtime(&conn, &working).unwrap();
        let (_dir, bin) = compile_fake_codex("version_only");
        let spec = CodexLaunchSpec {
            executable_path: bin.clone(),
            argv_prefix: Vec::new(),
        };
        assert!(crate::codex_runtime::validate_codex_runtime(&spec).is_ok());

        let result = select_codex_runtime_in(
            &conn,
            CodexRuntimeSelectionDto { display_path: bin },
            &[],
            |runtime| {
                validate_codex_app_server_runtime(runtime).map_err(|error| error.public_message)
            },
        )
        .unwrap();

        assert!(!result.valid);
        assert_eq!(load_configured_runtime(&conn).unwrap(), Some(working));
    }

    #[test]
    fn restart_switch_and_clear_feed_complete_runtime_spec_to_refresh_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.sqlite3");
        save_configured_runtime(
            &open_path(&path).unwrap(),
            &selected("fixture-a-node", "fixture-a.js"),
        )
        .unwrap();
        let reopened = open_path(&path).unwrap();
        assert_eq!(
            account_config_from_conn(&reopened)
                .unwrap()
                .explicit_runtime
                .unwrap()
                .argv_prefix,
            ["fixture-a.js"]
        );
        save_configured_runtime(&reopened, &selected("fixture-b-node", "fixture-b.js")).unwrap();
        assert_eq!(
            account_config_from_conn(&reopened)
                .unwrap()
                .explicit_runtime
                .unwrap()
                .argv_prefix,
            ["fixture-b.js"]
        );
        clear_runtime_setting(&reopened).unwrap();
        assert!(account_config_from_conn_with_context(
            &reopened,
            &RuntimeDiscoveryContext::isolated(&[], Vec::new())
        )
        .is_err());
    }

    #[test]
    fn restart_refreshes_fixture_a_switches_b_then_clear_uses_automatic_runtime() {
        let db_dir = tempdir().unwrap();
        let db = db_dir.path().join("settings.sqlite3");
        let (_a_dir, a) = compile_fake_codex("happy");
        let (_b_dir, b) = compile_fake_codex("happy");
        let runtime = |path: &Path| ConfiguredCodexRuntime {
            display_path: path.to_path_buf(),
            launch: CodexLaunchSpec {
                executable_path: path.to_path_buf(),
                argv_prefix: Vec::new(),
            },
            source: CodexRuntimeSource::Configured,
            validated_at_utc: "2026-07-10T12:00:00Z".into(),
            version: "fake".into(),
        };

        save_configured_runtime(&open_path(&db).unwrap(), &runtime(&a)).unwrap();
        let reopened = open_path(&db).unwrap();
        assert_eq!(
            refresh_account_snapshot(account_config_from_conn(&reopened).unwrap())
                .unwrap()
                .launch
                .selected_executable,
            a.display().to_string()
        );
        save_configured_runtime(&reopened, &runtime(&b)).unwrap();
        drop(reopened);
        let reopened = open_path(&db).unwrap();
        assert_eq!(
            refresh_account_snapshot(account_config_from_conn(&reopened).unwrap())
                .unwrap()
                .launch
                .selected_executable,
            b.display().to_string()
        );
        clear_runtime_setting(&reopened).unwrap();

        let automatic_config = account_config_from_conn_with_context(
            &reopened,
            &RuntimeDiscoveryContext::isolated(
                &[("TOKENSTACK_CODEX_BIN", a.as_path())],
                Vec::new(),
            ),
        )
        .unwrap();
        let automatic = refresh_account_snapshot(automatic_config).unwrap();
        assert_eq!(
            automatic.launch.selected_executable,
            a.display().to_string()
        );
    }

    #[test]
    fn production_config_uses_typed_localappdata_and_npm_launch_specs() {
        let conn = crate::db::open_memory().unwrap();
        let root = tempdir().unwrap();
        let local = root.path().join("local");
        let app_bin = local.join("OpenAI/Codex/bin/codex.exe");
        std::fs::create_dir_all(app_bin.parent().unwrap()).unwrap();
        let (_fake_dir, fake) = compile_fake_codex("happy");
        std::fs::copy(&fake, &app_bin).unwrap();
        let app_config = account_config_from_conn_with_context(
            &conn,
            &RuntimeDiscoveryContext::isolated(&[("LOCALAPPDATA", &local)], Vec::new()),
        )
        .unwrap();
        assert_eq!(
            app_config.explicit_runtime.unwrap(),
            CodexLaunchSpec {
                executable_path: app_bin,
                argv_prefix: Vec::new()
            }
        );

        let roaming = root.path().join("roaming");
        let npm = roaming.join("npm");
        let entrypoint = npm.join("node_modules/@openai/codex/bin/codex.js");
        std::fs::create_dir_all(entrypoint.parent().unwrap()).unwrap();
        std::fs::copy(&fake, npm.join("node.exe")).unwrap();
        std::fs::write(&entrypoint, "// fixture").unwrap();
        std::fs::write(
            npm.join("codex.cmd"),
            "@\"%dp0%\\node.exe\" \"%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js\" %*\r\n",
        )
        .unwrap();
        let npm_config = account_config_from_conn_with_context(
            &conn,
            &RuntimeDiscoveryContext::isolated(&[("APPDATA", &roaming)], Vec::new()),
        )
        .unwrap();
        assert_eq!(
            npm_config.explicit_runtime.unwrap(),
            CodexLaunchSpec {
                executable_path: npm.join("node.exe"),
                argv_prefix: vec![entrypoint.to_string_lossy().into_owned()]
            }
        );
    }

    #[test]
    fn refresh_persists_imported_history_for_later_summary_calls() {
        let app_dir = tempdir().unwrap();
        let history_dir = tempdir().unwrap();
        let mut file = fs::File::create(history_dir.path().join("history.jsonl")).unwrap();
        writeln!(
            file,
            r#"{{"id":"persisted-event","type":"token_count","timestamp":"2026-07-02T18:00:00Z","session_id":"s1","usage":{{"total_tokens":321}}}}"#
        )
        .unwrap();

        let auth_home = tempdir().unwrap();

        let refreshed = refresh_all_with_auth_home(
            "local".to_string(),
            app_dir.path().to_path_buf(),
            vec![history_dir.path().to_path_buf()],
            auth_home.path().to_path_buf(),
        )
        .unwrap();
        let reopened =
            get_dashboard_summary_from_path("local".to_string(), app_dir.path().to_path_buf())
                .unwrap();

        let local =
            get_dashboard_summary_from_path("local".to_string(), app_dir.path().to_path_buf())
                .unwrap();

        assert_eq!(refreshed.metrics[0].value, "321");
        assert_eq!(reopened.metrics[0].value, "321");
        assert_eq!(reopened.metrics[0].label, "Local history tokens");
        assert_eq!(local.metrics[0].value, "321");
        let conn = open_app_database(app_dir.path()).unwrap();
        let account_refresh_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM account_refresh_runs", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(account_refresh_count, 0);
    }

    #[test]
    fn combined_refresh_keeps_local_summary_when_runtime_discovery_fails() {
        let app_dir = tempdir().unwrap();
        let history_dir = tempdir().unwrap();
        let mut file = fs::File::create(history_dir.path().join("history.jsonl")).unwrap();
        writeln!(
            file,
            r#"{{"id":"degraded-event","type":"token_count","timestamp":"2026-07-18T18:00:00Z","session_id":"s1","usage":{{"total_tokens":654}}}}"#
        )
        .unwrap();
        let context = RuntimeDiscoveryContext::isolated(&[], Vec::new());

        let refreshed = refresh_all_with_context(
            "combined".to_string(),
            app_dir.path().to_path_buf(),
            vec![history_dir.path().to_path_buf()],
            &context,
        )
        .unwrap();

        assert_eq!(refreshed.metrics[0].value, "654");
        assert_eq!(
            refreshed
                .connectors
                .iter()
                .find(|connector| connector.id == "account-usage")
                .unwrap()
                .status,
            "degraded"
        );
        let conn = open_app_database(app_dir.path()).unwrap();
        let (status, stage): (String, Option<String>) = conn
            .query_row(
                "SELECT status, first_failing_stage FROM account_refresh_runs ORDER BY id DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "unavailable");
        assert_eq!(stage.as_deref(), Some("discover"));
    }

    #[test]
    fn backend_refresh_lock_blocks_concurrent_refreshes() {
        let guard = refresh_lock().lock().unwrap();
        let started = Arc::new(AtomicBool::new(false));
        let completed = Arc::new(AtomicBool::new(false));
        let started_for_thread = Arc::clone(&started);
        let completed_for_thread = Arc::clone(&completed);

        let handle = thread::spawn(move || {
            started_for_thread.store(true, Ordering::SeqCst);
            let _guard = refresh_lock().lock().unwrap();
            completed_for_thread.store(true, Ordering::SeqCst);
        });

        while !started.load(Ordering::SeqCst) {
            thread::yield_now();
        }
        thread::sleep(Duration::from_millis(25));
        assert!(!completed.load(Ordering::SeqCst));

        drop(guard);
        handle.join().unwrap();
        assert!(completed.load(Ordering::SeqCst));
    }

    #[test]
    fn setup_diagnostics_reports_checked_roots_without_existing_database() {
        let app_dir = tempdir().unwrap();
        let auth_home = tempdir().unwrap();
        let existing_root = tempdir().unwrap();
        let missing_root = existing_root.path().join("missing-root");

        let diagnostics = setup_diagnostics_from_parts(
            app_dir.path().to_path_buf(),
            vec![existing_root.path().to_path_buf(), missing_root.clone()],
            auth_home.path().to_path_buf(),
        )
        .unwrap();

        assert_eq!(
            diagnostics.app_data_dir,
            app_dir.path().display().to_string()
        );
        assert!(diagnostics.database_path.ends_with("tokenstack.sqlite3"));
        assert_eq!(
            diagnostics.auth_home,
            auth_home.path().display().to_string()
        );
        assert!(diagnostics.latest_import_run.is_none());
        assert_eq!(diagnostics.usage_event_count, 0);
        assert_eq!(diagnostics.usage_total_tokens, 0);
        assert_eq!(diagnostics.source_document_count, 0);
        assert_eq!(diagnostics.local_roots.len(), 2);
        assert!(diagnostics.local_roots[0].exists);
        assert!(diagnostics.local_roots[0].is_directory);
        assert_eq!(
            diagnostics.local_roots[0].path,
            existing_root.path().display().to_string()
        );
        assert!(!diagnostics.local_roots[1].exists);
        assert!(!diagnostics.local_roots[1].is_directory);
        assert_eq!(
            diagnostics.local_roots[1].path,
            missing_root.display().to_string()
        );
    }

    #[test]
    fn setup_diagnostics_reports_latest_import_and_connector_runs() {
        let app_dir = tempdir().unwrap();
        let auth_home = tempdir().unwrap();
        let conn = open_app_database(app_dir.path()).unwrap();
        crate::db::insert_import_run(
            &conn,
            &crate::db::ImportRunSummary {
                files_seen: 4,
                events_seen: 3,
                events_imported: 2,
                warning_count: 1,
                warnings: vec![
                    "history.jsonl:2 unknown event shape skipped (type=message; keys=timestamp,type)"
                        .to_string(),
                ],
            },
        )
        .unwrap();
        crate::db::insert_connector_run(
            &conn,
            "known-reset-credit",
            "failed",
            Some("known-reset-credit"),
            None,
            Some("auth_unavailable"),
            Some("auth document is unavailable"),
        )
        .unwrap();
        drop(conn);

        let diagnostics = setup_diagnostics_from_parts(
            app_dir.path().to_path_buf(),
            Vec::new(),
            auth_home.path().to_path_buf(),
        )
        .unwrap();
        let latest_import = diagnostics.latest_import_run.unwrap();

        assert_eq!(latest_import.files_seen, 4);
        assert_eq!(latest_import.events_seen, 3);
        assert_eq!(latest_import.events_imported, 2);
        assert_eq!(latest_import.warning_count, 1);
        assert_eq!(latest_import.warning_samples.len(), 1);
        assert!(latest_import.warning_samples[0].contains("type=message"));
        assert_eq!(diagnostics.usage_event_count, 0);
        assert_eq!(diagnostics.usage_total_tokens, 0);
        assert_eq!(diagnostics.source_document_count, 0);
        assert_eq!(diagnostics.connector_runs.len(), 1);
        assert_eq!(
            diagnostics.connector_runs[0].connector_id,
            "known-reset-credit"
        );
        assert_eq!(
            diagnostics.connector_runs[0].endpoint_id.as_deref(),
            Some("known-reset-credit")
        );
        assert_eq!(
            diagnostics.connector_runs[0].redacted_error_code.as_deref(),
            Some("auth_unavailable")
        );
        assert_eq!(
            diagnostics.connector_runs[0]
                .redacted_error_message
                .as_deref(),
            Some("auth document is unavailable")
        );
    }

    #[test]
    fn save_text_export_writes_safe_file_to_target_directory() {
        let target_dir = tempdir().unwrap();

        let saved_path = save_text_export_to_dir(
            "tokenstack-diagnostics-2026-07-03.json",
            "{\"diagnostics\":true}",
            target_dir.path(),
        )
        .unwrap();

        assert_eq!(
            saved_path,
            target_dir
                .path()
                .join("tokenstack-diagnostics-2026-07-03.json")
        );
        assert_eq!(
            fs::read_to_string(saved_path).unwrap(),
            "{\"diagnostics\":true}"
        );
    }

    #[test]
    fn repeated_and_binary_exports_use_unique_files_without_overwriting() {
        let target_dir = tempdir().unwrap();
        let filename = "tokenstack-usage-2026-07-18.csv";

        let first = save_text_export_to_dir(filename, "first", target_dir.path()).unwrap();
        let second = save_text_export_to_dir(filename, "second", target_dir.path()).unwrap();
        let binary = save_export_bytes_to_dir(
            "tokenstack-badge-2026-07-18.png",
            &[137, 80, 78, 71],
            target_dir.path(),
        )
        .unwrap();

        assert_ne!(first, second);
        assert_eq!(first.file_name().unwrap(), filename);
        assert_eq!(
            second.file_name().unwrap(),
            "tokenstack-usage-2026-07-18-1.csv"
        );
        assert_eq!(fs::read_to_string(first).unwrap(), "first");
        assert_eq!(fs::read_to_string(second).unwrap(), "second");
        assert_eq!(fs::read(binary).unwrap(), [137, 80, 78, 71]);
    }

    #[test]
    fn displayed_metric_freshness_uses_coverage_instead_of_presentation_status() {
        let conn = crate::db::open_memory().unwrap();

        let metrics = displayed_metric_diagnostics_for_mode(&conn, "remote").unwrap();

        assert!(!metrics.is_empty());
        assert!(metrics
            .iter()
            .all(|metric| metric.freshness == "unavailable"));
    }

    #[test]
    fn save_text_export_rejects_path_traversal_filenames() {
        let target_dir = tempdir().unwrap();

        let error =
            save_text_export_to_dir("../auth.json", "secret", target_dir.path()).unwrap_err();

        assert!(error.contains("invalid export filename"));
        assert!(!target_dir.path().join("auth.json").exists());
    }

    #[test]
    fn diagnostics_export_writes_sanitized_file() {
        let target_dir = tempdir().unwrap();
        let diagnostics = SetupDiagnosticsDto {
            schema_version: 2,
            data_mode: "combined".to_string(),
            app_data_dir: target_dir.path().display().to_string(),
            database_path: target_dir
                .path()
                .join("tokenstack.sqlite3")
                .display()
                .to_string(),
            auth_home: "C:\\Users\\TokenStack".to_string(),
            selected_codex_executable: Some("C:\\Program Files\\Codex\\codex.exe".to_string()),
            configured_codex_runtime_display: Some(
                "C:\\Program Files\\Codex\\codex.exe".to_string(),
            ),
            codex_launch_mode: Some("listen_stdio_no_mcp".to_string()),
            first_failing_account_stage: Some("account/usage/read".to_string()),
            last_successful_account_refresh: Some("2026-07-03T12:00:00Z".to_string()),
            usage_event_count: 0,
            usage_total_tokens: 0,
            source_document_count: 0,
            local_roots: Vec::new(),
            latest_import_run: None,
            connector_runs: vec![ConnectorRunDiagnosticsDto {
                connector_id: "account-usage".to_string(),
                status: "failed".to_string(),
                completed_at_utc: "2026-07-03T12:00:00Z".to_string(),
                endpoint_id: None,
                http_status: None,
                redacted_error_code: Some("protocol_error".to_string()),
                redacted_error_message: Some("authorization synthetic".to_string()),
            }],
            runtime_candidates: Vec::new(),
            selected_runtime: None,
            latest_account_run: None,
            displayed_metrics: Vec::new(),
        };

        let path = export_diagnostics_to_dir(&diagnostics, target_dir.path()).unwrap();
        let contents = fs::read_to_string(path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();

        assert_eq!(parsed["schemaVersion"], 2);
        assert_eq!(parsed["redaction"]["status"], "sanitized");
        assert!(contents.contains("codex.exe"));
        assert!(!contents.contains("authorization synthetic"));
        assert!(contents.contains("[REDACTED]"));
        assert!(target_dir.path().read_dir().unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp")));
    }

    #[test]
    fn diagnostics_export_redacts_secret_bearing_keys_and_values() {
        let authorization = [
            "Authorization:",
            "Bearer",
            "eyJhbGciOiJIUzI1NiJ9.private.signature",
        ]
        .join(" ");
        let mut value = serde_json::json!({
            "token": "sk-secret-value",
            "cookie": "session=private",
            "prompt": "summarize my private repository",
            "responseBody": {"accountLabel": "person@example.com"},
            "stderrTail": authorization,
            "safe": "app-server initialize timed out"
        });

        sanitize_json_value(&mut value);
        let text = serde_json::to_string(&value).unwrap();

        assert!(!text.contains("sk-secret-value"));
        assert!(!text.contains("person@example.com"));
        assert!(!text.contains("private repository"));
        assert!(!text.contains("eyJhbGci"));
        assert!(text.contains("app-server initialize timed out"));
    }

    #[test]
    fn concurrent_diagnostics_exports_are_distinct_parseable_and_leave_no_temps() {
        let target = Arc::new(tempdir().unwrap());
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let target = Arc::clone(&target);
                thread::spawn(move || {
                    let diagnostics = setup_diagnostics_from_parts(
                        target.path().join("app"),
                        vec![],
                        target.path().join("auth"),
                    )
                    .unwrap();
                    export_diagnostics_to_dir(&diagnostics, target.path()).unwrap()
                })
            })
            .collect();
        let paths: std::collections::HashSet<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(paths.len(), 8);
        for path in paths {
            serde_json::from_str::<serde_json::Value>(&fs::read_to_string(path).unwrap()).unwrap();
        }
        assert!(target.path().read_dir().unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp")));
    }

    #[test]
    fn diagnostics_reconstructs_persisted_automatic_npm_runtime_after_discovery_disappears() {
        let dir = tempdir().unwrap();
        let conn = open_app_database(dir.path()).unwrap();
        conn.execute("INSERT INTO account_refresh_runs (started_at_utc, completed_at_utc, status, selected_codex_executable, launch_mode, executable_candidates_json, method_statuses_json, argv_prefix_json, runtime_source, runtime_display_path) VALUES ('2026-07-10T00:00:00Z','2026-07-10T00:00:01Z','connected','C:\\Program Files\\nodejs\\node.exe','listen_stdio_no_mcp','[]','[]','[\"C:\\\\Users\\\\Test\\\\codex.js\"]','npm','C:\\Users\\Test\\codex.cmd')", []).unwrap();
        drop(conn);
        let diagnostics = setup_diagnostics_from_parts_with_context(
            dir.path().to_path_buf(),
            vec![],
            dir.path().join("auth"),
            &RuntimeDiscoveryContext::isolated(&[], Vec::new()),
        )
        .unwrap();
        let selected = diagnostics.selected_runtime.unwrap();
        assert_eq!(selected.source, CodexRuntimeSource::Npm);
        assert_eq!(
            selected.native_executable_path,
            "C:\\Program Files\\nodejs\\node.exe"
        );
        assert_eq!(selected.display_path, "C:\\Users\\Test\\codex.cmd");
        assert_eq!(selected.argv_prefix, vec!["C:\\Users\\Test\\codex.js"]);
        assert_eq!(
            diagnostics.selected_codex_executable.as_deref(),
            Some("C:\\Program Files\\nodejs\\node.exe")
        );
        assert_eq!(
            diagnostics.codex_launch_mode.as_deref(),
            Some("listen_stdio_no_mcp")
        );
        assert_eq!(
            diagnostics
                .latest_account_run
                .as_ref()
                .unwrap()
                .initialize_status,
            "ok"
        );
    }
}
