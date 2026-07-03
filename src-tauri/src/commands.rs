use crate::analytics::{build_dashboard_summary, DashboardSummaryDto};
use crate::auth::{parse_auth_document, AuthHandle, AuthLocator};
use crate::connectors::{
    ConnectorRunResult, KnownResetCreditsConnector, UndocumentedRateLimitsConnector,
};
use crate::db::{
    insert_connector_run, insert_rate_limit_window, insert_reset_credit_batch, open_path,
    NewRateLimitWindow,
};
use crate::discovery::{default_app_data_dir, default_auth_home, default_local_history_roots};
use crate::importers::LocalHistoryImporter;
use crate::telemetry::public_error;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use url::Url;

static REFRESH_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

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
pub struct SetupDiagnosticsDto {
    pub app_data_dir: String,
    pub database_path: String,
    pub auth_home: String,
    pub local_roots: Vec<LocalRootDiagnosticsDto>,
    pub latest_import_run: Option<ImportRunDiagnosticsDto>,
    pub connector_runs: Vec<ConnectorRunDiagnosticsDto>,
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorRunDiagnosticsDto {
    pub connector_id: String,
    pub status: String,
    pub completed_at_utc: String,
    pub redacted_error_code: Option<String>,
}

#[tauri::command]
pub fn get_setup_diagnostics() -> Result<SetupDiagnosticsDto, String> {
    setup_diagnostics_from_parts(
        default_app_data_dir(),
        default_local_history_roots(),
        default_auth_home(),
    )
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
    let database_path = app_data_dir.join("tokenstack.sqlite3");
    let local_roots = roots.into_iter().map(local_root_diagnostics).collect();

    let (latest_import_run, connector_runs) = if database_path.exists() {
        let conn = open_path(&database_path).map_err(|error| error.to_string())?;
        (
            latest_import_run_diagnostics(&conn).map_err(|error| error.to_string())?,
            connector_run_diagnostics(&conn).map_err(|error| error.to_string())?,
        )
    } else {
        (None, Vec::new())
    };

    Ok(SetupDiagnosticsDto {
        app_data_dir: path_label(&app_data_dir),
        database_path: path_label(&database_path),
        auth_home: path_label(&auth_home),
        local_roots,
        latest_import_run,
        connector_runs,
    })
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
               events_imported, warnings_json
        FROM import_runs
        ORDER BY id DESC
        LIMIT 1
        "#,
        [],
        |row| {
            let warnings_json: String = row.get(4)?;
            let warning_count = serde_json::from_str::<Vec<String>>(&warnings_json)
                .map(|warnings| warnings.len())
                .unwrap_or(0);
            Ok(ImportRunDiagnosticsDto {
                completed_at_utc: row.get(0)?,
                files_seen: row.get(1)?,
                events_seen: row.get(2)?,
                events_imported: row.get(3)?,
                warning_count,
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
        SELECT connector_id, status, COALESCE(completed_at_utc, started_at_utc), redacted_error_code
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
            redacted_error_code: row.get(3)?,
        })
    })?;
    rows.collect()
}

fn path_label(path: &Path) -> String {
    path.display().to_string()
}

fn refresh_all_with_auth_home(
    data_mode: String,
    app_data_dir: PathBuf,
    roots: Vec<PathBuf>,
    auth_home: PathBuf,
) -> Result<DashboardSummaryDto, String> {
    let _refresh_guard = refresh_lock()
        .lock()
        .map_err(|_| "refresh lock is poisoned".to_string())?;
    let conn = open_app_database(&app_data_dir)?;
    let importer = LocalHistoryImporter::new(roots);
    let _summary = importer
        .import_into(&conn)
        .map_err(|error| error.to_string())?;
    refresh_remote_connectors(&conn, &auth_home)?;
    build_dashboard_summary(&conn, &data_mode).map_err(|error| error.to_string())
}

fn refresh_lock() -> &'static Mutex<()> {
    REFRESH_LOCK.get_or_init(|| Mutex::new(()))
}

fn open_app_database(app_data_dir: &Path) -> Result<rusqlite::Connection, String> {
    std::fs::create_dir_all(app_data_dir).map_err(|error| error.to_string())?;
    open_path(&app_data_dir.join("tokenstack.sqlite3")).map_err(|error| error.to_string())
}

fn refresh_remote_connectors(conn: &Connection, auth_home: &Path) -> Result<(), String> {
    let base_url = Url::parse("https://chatgpt.com").map_err(|error| error.to_string())?;
    match load_auth_handle(auth_home) {
        Ok(auth) => {
            let reset_result = KnownResetCreditsConnector::new(base_url.clone()).fetch(&auth);
            persist_connector_result(conn, &reset_result).map_err(|error| error.to_string())?;
            let rate_limit_result = UndocumentedRateLimitsConnector::new(base_url).fetch(&auth);
            persist_connector_result(conn, &rate_limit_result).map_err(|error| error.to_string())
        }
        Err(error) => {
            for connector_id in ["known-reset-credit", "undocumented-rate-limits"] {
                let result = ConnectorRunResult {
                    connector_id: connector_id.to_string(),
                    status: "failed".to_string(),
                    batches: Vec::new(),
                    rate_limit_windows: Vec::new(),
                    redacted_error: Some(public_error("auth_unavailable", &error)),
                };
                persist_connector_result(conn, &result).map_err(|error| error.to_string())?;
            }
            Ok(())
        }
    }
}

fn load_auth_handle(auth_home: &Path) -> Result<AuthHandle, String> {
    let locator = AuthLocator::new(auth_home.to_path_buf());
    for candidate in locator.candidate_paths() {
        if !candidate.exists() {
            continue;
        }
        let allowed = locator
            .allowed_path(&candidate)
            .map_err(|error| error.to_string())?;
        let text = std::fs::read_to_string(allowed).map_err(|error| error.to_string())?;
        return parse_auth_document(&text).map_err(|error| error.to_string());
    }
    Err("auth document is unavailable".to_string())
}

fn persist_connector_result(
    conn: &Connection,
    result: &ConnectorRunResult,
) -> rusqlite::Result<()> {
    let run_id = insert_connector_run(
        conn,
        &result.connector_id,
        &result.status,
        Some(result.connector_id.as_str()),
        None,
        result
            .redacted_error
            .as_ref()
            .map(|error| error.code.as_str()),
        result
            .redacted_error
            .as_ref()
            .map(|error| error.message.as_str()),
    )?;
    for batch in &result.batches {
        insert_reset_credit_batch(
            conn,
            run_id,
            batch.credit_count,
            batch.expires_at_utc,
            &result.connector_id,
            &batch.confidence,
        )?;
    }
    for window in &result.rate_limit_windows {
        insert_rate_limit_window(
            conn,
            &NewRateLimitWindow {
                connector_run_id: run_id,
                window_key: &window.window_key,
                limit_tokens: window.limit_tokens,
                used_tokens: window.used_tokens,
                remaining_tokens: window.remaining_tokens,
                resets_at_utc: window.resets_at_utc,
                confidence: &window.confidence,
            },
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

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
            "combined".to_string(),
            app_dir.path().to_path_buf(),
            vec![history_dir.path().to_path_buf()],
            auth_home.path().to_path_buf(),
        )
        .unwrap();
        let reopened =
            get_dashboard_summary_from_path("combined".to_string(), app_dir.path().to_path_buf())
                .unwrap();

        assert_eq!(refreshed.metrics[0].value, "321");
        assert_eq!(reopened.metrics[0].value, "321");
        let connector = reopened
            .connectors
            .iter()
            .find(|connector| connector.id == "known-reset-credit")
            .unwrap();
        assert_eq!(connector.status, "degraded");
        let rate_limit_windows = reopened
            .connectors
            .iter()
            .find(|connector| connector.id == "rate-limit-windows")
            .unwrap();
        assert_eq!(rate_limit_windows.status, "degraded");

        let conn = open_app_database(app_dir.path()).unwrap();
        let stored_endpoint: String = conn
            .query_row(
                r#"
                SELECT endpoint_id
                FROM connector_runs
                WHERE connector_id = 'undocumented-rate-limits'
                ORDER BY id DESC
                LIMIT 1
                "#,
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_endpoint, "undocumented-rate-limits");
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
                warnings: vec!["ignored unknown event shape".to_string()],
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
        assert_eq!(diagnostics.connector_runs.len(), 1);
        assert_eq!(
            diagnostics.connector_runs[0].connector_id,
            "known-reset-credit"
        );
        assert_eq!(
            diagnostics.connector_runs[0].redacted_error_code.as_deref(),
            Some("auth_unavailable")
        );
    }
}
