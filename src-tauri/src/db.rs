use crate::codex_app_server::{AccountConnectorError, AccountMethodSnapshot, AccountSnapshot};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use sha2::{Digest, Sha256};
use std::path::Path;

pub const MIGRATIONS: &[&str] = &[
    r#"
CREATE TABLE IF NOT EXISTS app_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at_utc TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS import_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_kind TEXT NOT NULL,
  started_at_utc TEXT NOT NULL,
  completed_at_utc TEXT,
  status TEXT NOT NULL,
  files_seen INTEGER NOT NULL DEFAULT 0,
  events_seen INTEGER NOT NULL DEFAULT 0,
  events_imported INTEGER NOT NULL DEFAULT 0,
  warning_count INTEGER NOT NULL DEFAULT 0,
  warnings_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS source_documents (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_kind TEXT NOT NULL,
  path_hash TEXT NOT NULL,
  safe_label TEXT NOT NULL,
  first_seen_at_utc TEXT NOT NULL,
  last_seen_at_utc TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  last_offset INTEGER NOT NULL DEFAULT 0,
  redaction_level TEXT NOT NULL,
  UNIQUE(source_kind, path_hash)
);

CREATE TABLE IF NOT EXISTS usage_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  event_uid TEXT NOT NULL UNIQUE,
  source_document_id INTEGER NOT NULL,
  session_uid TEXT NOT NULL,
  occurred_at_utc TEXT NOT NULL,
  model TEXT,
  mode TEXT,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  cache_read_tokens INTEGER NOT NULL DEFAULT 0,
  cache_write_tokens INTEGER NOT NULL DEFAULT 0,
  total_tokens INTEGER NOT NULL DEFAULT 0,
  raw_event_kind TEXT NOT NULL,
  confidence TEXT NOT NULL,
  metadata_json_redacted TEXT NOT NULL DEFAULT '{}',
  FOREIGN KEY(source_document_id) REFERENCES source_documents(id)
);

CREATE TABLE IF NOT EXISTS sessions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_uid TEXT NOT NULL UNIQUE,
  started_at_utc TEXT NOT NULL,
  ended_at_utc TEXT NOT NULL,
  duration_seconds INTEGER NOT NULL,
  total_tokens INTEGER NOT NULL,
  peak_tokens INTEGER NOT NULL,
  model_mix_json TEXT NOT NULL,
  mode_labels_json TEXT NOT NULL,
  source_summary_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS connector_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  connector_id TEXT NOT NULL,
  started_at_utc TEXT NOT NULL,
  completed_at_utc TEXT,
  status TEXT NOT NULL,
  endpoint_id TEXT,
  http_status INTEGER,
  redacted_error_code TEXT,
  redacted_error_message TEXT
);

CREATE TABLE IF NOT EXISTS reset_credit_batches (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  connector_run_id INTEGER NOT NULL,
  captured_at_utc TEXT NOT NULL,
  credit_count INTEGER NOT NULL,
  expires_at_utc TEXT NOT NULL,
  source_connector_id TEXT NOT NULL,
  confidence TEXT NOT NULL,
  raw_batch_hash TEXT NOT NULL,
  FOREIGN KEY(connector_run_id) REFERENCES connector_runs(id)
);

CREATE TABLE IF NOT EXISTS rate_limit_windows (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  connector_run_id INTEGER NOT NULL,
  captured_at_utc TEXT NOT NULL,
  window_key TEXT NOT NULL,
  limit_tokens INTEGER NOT NULL,
  used_tokens INTEGER NOT NULL,
  remaining_tokens INTEGER NOT NULL,
  resets_at_utc TEXT NOT NULL,
  confidence TEXT NOT NULL,
  FOREIGN KEY(connector_run_id) REFERENCES connector_runs(id)
);

CREATE TABLE IF NOT EXISTS refresh_snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  trigger TEXT NOT NULL,
  started_at_utc TEXT NOT NULL,
  completed_at_utc TEXT,
  status TEXT NOT NULL,
  connector_summary_json TEXT NOT NULL,
  dashboard_summary_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS source_coverage (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  snapshot_id INTEGER,
  metric_key TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  coverage_percent INTEGER NOT NULL CHECK(coverage_percent >= 0 AND coverage_percent <= 100),
  confidence TEXT NOT NULL,
  last_evidence_at_utc TEXT NOT NULL,
  formula_version TEXT NOT NULL,
  required_facets_json TEXT NOT NULL,
  missing_facets_json TEXT NOT NULL,
  explanation TEXT NOT NULL,
  FOREIGN KEY(snapshot_id) REFERENCES refresh_snapshots(id)
);

CREATE TABLE IF NOT EXISTS account_refresh_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  started_at_utc TEXT NOT NULL,
  completed_at_utc TEXT NOT NULL,
  status TEXT NOT NULL,
  selected_codex_executable TEXT,
  launch_mode TEXT,
  executable_candidates_json TEXT NOT NULL DEFAULT '[]',
  first_failing_stage TEXT,
  redacted_error_code TEXT,
  redacted_error_message TEXT NOT NULL DEFAULT '',
  stderr_tail TEXT NOT NULL DEFAULT '',
  used_last_good_snapshot INTEGER NOT NULL DEFAULT 0,
  method_statuses_json TEXT NOT NULL DEFAULT '[]',
  exit_code INTEGER,
  timed_out INTEGER NOT NULL DEFAULT 0,
  child_terminated INTEGER,
  argv_prefix_json TEXT NOT NULL DEFAULT '[]',
  runtime_source TEXT,
  runtime_display_path TEXT
);

CREATE TABLE IF NOT EXISTS account_identity_snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  refresh_run_id INTEGER NOT NULL,
  account_label TEXT,
  plan TEXT,
  FOREIGN KEY(refresh_run_id) REFERENCES account_refresh_runs(id)
);

CREATE TABLE IF NOT EXISTS account_usage_snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  refresh_run_id INTEGER NOT NULL,
  lifetime_tokens INTEGER,
  FOREIGN KEY(refresh_run_id) REFERENCES account_refresh_runs(id)
);

CREATE TABLE IF NOT EXISTS account_daily_usage_buckets (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  refresh_run_id INTEGER NOT NULL,
  usage_date TEXT NOT NULL,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  total_tokens INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY(refresh_run_id) REFERENCES account_refresh_runs(id)
);

CREATE TABLE IF NOT EXISTS account_reset_credit_snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  refresh_run_id INTEGER NOT NULL,
  available_count INTEGER,
  expires_at_utc TEXT,
  FOREIGN KEY(refresh_run_id) REFERENCES account_refresh_runs(id)
);

CREATE TABLE IF NOT EXISTS account_rate_limit_buckets (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  refresh_run_id INTEGER NOT NULL,
  bucket_id TEXT NOT NULL,
  display_name TEXT NOT NULL,
  FOREIGN KEY(refresh_run_id) REFERENCES account_refresh_runs(id)
);

CREATE TABLE IF NOT EXISTS account_rate_limit_windows (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  bucket_row_id INTEGER NOT NULL,
  window_duration_mins INTEGER NOT NULL,
  window_label TEXT NOT NULL,
  used_percent REAL NOT NULL,
  remaining_percent REAL NOT NULL,
  resets_at_utc TEXT,
  FOREIGN KEY(bucket_row_id) REFERENCES account_rate_limit_buckets(id)
);
"#,
    r#"
CREATE TABLE IF NOT EXISTS codex_runtime_settings (
  singleton_key INTEGER PRIMARY KEY CHECK(singleton_key = 1),
  display_path TEXT NOT NULL,
  executable_path TEXT NOT NULL,
  argv_prefix_json TEXT NOT NULL,
  source TEXT NOT NULL,
  validated_at_utc TEXT NOT NULL,
  version TEXT NOT NULL
);
"#,
    r#"
CREATE TABLE IF NOT EXISTS account_method_attempts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  refresh_run_id INTEGER NOT NULL,
  method TEXT NOT NULL,
  status TEXT NOT NULL,
  redacted_error TEXT,
  captured_at_utc TEXT NOT NULL,
  schema_fingerprint TEXT NOT NULL DEFAULT '',
  FOREIGN KEY(refresh_run_id) REFERENCES account_refresh_runs(id)
);

CREATE TABLE IF NOT EXISTS account_reset_credit_details (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  reset_credit_snapshot_id INTEGER NOT NULL,
  credit_id TEXT NOT NULL,
  reset_type TEXT NOT NULL,
  status TEXT NOT NULL,
  granted_at_utc TEXT NOT NULL,
  expires_at_utc TEXT,
  title TEXT,
  description TEXT,
  FOREIGN KEY(reset_credit_snapshot_id) REFERENCES account_reset_credit_snapshots(id)
);
"#,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub event_uid: String,
    pub source_document_id: i64,
    pub session_uid: String,
    pub occurred_at_utc: DateTime<Utc>,
    pub model: Option<String>,
    pub mode: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub total_tokens: i64,
    pub raw_event_kind: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRunSummary {
    pub files_seen: usize,
    pub events_seen: usize,
    pub events_imported: usize,
    pub warning_count: usize,
    pub warnings: Vec<String>,
}

#[allow(dead_code)]
pub fn open_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    run_migrations(&conn)?;
    Ok(conn)
}

pub fn open_path(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    run_migrations(&conn)?;
    Ok(conn)
}

pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    for migration in MIGRATIONS {
        conn.execute_batch(migration)?;
    }
    let has_warning_count = conn
        .prepare("PRAGMA table_info(import_runs)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .any(|column| column == "warning_count");
    if !has_warning_count {
        conn.execute(
            "ALTER TABLE import_runs ADD COLUMN warning_count INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    ensure_column(
        conn,
        "account_usage_snapshots",
        "captured_at_utc",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(conn, "account_refresh_runs", "exit_code", "INTEGER")?;
    ensure_column(
        conn,
        "account_refresh_runs",
        "timed_out",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(conn, "account_refresh_runs", "child_terminated", "INTEGER")?;
    ensure_column(
        conn,
        "account_refresh_runs",
        "argv_prefix_json",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(conn, "account_refresh_runs", "runtime_source", "TEXT")?;
    ensure_column(conn, "account_refresh_runs", "runtime_display_path", "TEXT")?;
    ensure_column(
        conn,
        "account_usage_snapshots",
        "schema_fingerprint",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "account_reset_credit_snapshots",
        "captured_at_utc",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "account_reset_credit_snapshots",
        "schema_fingerprint",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "account_rate_limit_buckets",
        "captured_at_utc",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "account_rate_limit_buckets",
        "schema_fingerprint",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    declaration: &str,
) -> rusqlite::Result<()> {
    let columns = conn
        .prepare(&format!("PRAGMA table_info({table})"))?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if !columns.iter().any(|existing| existing == column) {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {declaration}"),
            [],
        )?;
    }
    Ok(())
}

pub fn upsert_source_document(
    conn: &Connection,
    source_kind: &str,
    path_hash: &str,
    safe_label: &str,
    content_hash: &str,
    last_offset: i64,
) -> rusqlite::Result<i64> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        r#"
        INSERT INTO source_documents
          (source_kind, path_hash, safe_label, first_seen_at_utc, last_seen_at_utc, content_hash, last_offset, redaction_level)
        VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, 'path-hash')
        ON CONFLICT(source_kind, path_hash) DO UPDATE SET
          last_seen_at_utc = excluded.last_seen_at_utc,
          content_hash = excluded.content_hash,
          last_offset = excluded.last_offset
        "#,
        params![source_kind, path_hash, safe_label, now, content_hash, last_offset],
    )?;
    conn.query_row(
        "SELECT id FROM source_documents WHERE source_kind = ?1 AND path_hash = ?2",
        params![source_kind, path_hash],
        |row| row.get(0),
    )
}

pub fn insert_usage_event(conn: &Connection, event: &UsageEvent) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        r#"
        INSERT OR IGNORE INTO usage_events
          (event_uid, source_document_id, session_uid, occurred_at_utc, model, mode,
           input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, total_tokens,
           raw_event_kind, confidence, metadata_json_redacted)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, '{}')
        "#,
        params![
            event.event_uid,
            event.source_document_id,
            event.session_uid,
            event.occurred_at_utc.to_rfc3339(),
            event.model,
            event.mode,
            event.input_tokens,
            event.output_tokens,
            event.cache_read_tokens,
            event.cache_write_tokens,
            event.total_tokens,
            event.raw_event_kind,
            event.confidence
        ],
    )?;
    Ok(changed == 1)
}

#[allow(dead_code)]
pub fn count_usage_events(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM usage_events", [], |row| row.get(0))
}

#[allow(dead_code)]
pub fn usage_total(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(total_tokens), 0) FROM usage_events",
        [],
        |row| row.get(0),
    )
}

#[allow(dead_code)]
pub fn usage_event_by_uid(
    conn: &Connection,
    event_uid: &str,
) -> rusqlite::Result<Option<UsageEvent>> {
    conn.query_row(
        r#"
        SELECT event_uid, source_document_id, session_uid, occurred_at_utc, model, mode,
               input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, total_tokens,
               raw_event_kind, confidence
        FROM usage_events WHERE event_uid = ?1
        "#,
        [event_uid],
        |row| {
            let occurred: String = row.get(3)?;
            Ok(UsageEvent {
                event_uid: row.get(0)?,
                source_document_id: row.get(1)?,
                session_uid: row.get(2)?,
                occurred_at_utc: DateTime::parse_from_rfc3339(&occurred)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|err| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    })?,
                model: row.get(4)?,
                mode: row.get(5)?,
                input_tokens: row.get(6)?,
                output_tokens: row.get(7)?,
                cache_read_tokens: row.get(8)?,
                cache_write_tokens: row.get(9)?,
                total_tokens: row.get(10)?,
                raw_event_kind: row.get(11)?,
                confidence: row.get(12)?,
            })
        },
    )
    .optional()
}

pub fn insert_import_run(conn: &Connection, summary: &ImportRunSummary) -> rusqlite::Result<i64> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        r#"
        INSERT INTO import_runs
          (source_kind, started_at_utc, completed_at_utc, status, files_seen, events_seen, events_imported, warning_count, warnings_json)
        VALUES ('local-codex-history', ?1, ?1, 'complete', ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            now,
            summary.files_seen as i64,
            summary.events_seen as i64,
            summary.events_imported as i64,
            summary.warning_count as i64,
            serde_json::to_string(&summary.warnings).unwrap_or_else(|_| "[]".to_string())
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn record_source_coverage(
    conn: &Connection,
    metric_key: &str,
    source_kind: &str,
    coverage_percent: i64,
    confidence: &str,
    missing_facets: &[String],
    explanation: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        r#"
        INSERT INTO source_coverage
          (metric_key, source_kind, coverage_percent, confidence, last_evidence_at_utc,
           formula_version, required_facets_json, missing_facets_json, explanation)
        VALUES (?1, ?2, ?3, ?4, ?5, 'coverage-v1', '["local usage events","parseable token fields","dedupe key","selected date range"]', ?6, ?7)
        "#,
        params![
            metric_key,
            source_kind,
            coverage_percent,
            confidence,
            Utc::now().to_rfc3339(),
            serde_json::to_string(missing_facets).unwrap_or_else(|_| "[]".to_string()),
            explanation
        ],
    )?;
    Ok(())
}

#[cfg(test)]
pub fn insert_connector_run(
    conn: &Connection,
    connector_id: &str,
    status: &str,
    endpoint_id: Option<&str>,
    http_status: Option<i64>,
    redacted_error_code: Option<&str>,
    redacted_error_message: Option<&str>,
) -> rusqlite::Result<i64> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        r#"
        INSERT INTO connector_runs
          (connector_id, started_at_utc, completed_at_utc, status, endpoint_id, http_status,
           redacted_error_code, redacted_error_message)
        VALUES (?1, ?2, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            connector_id,
            now,
            status,
            endpoint_id,
            http_status,
            redacted_error_code,
            redacted_error_message
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

#[cfg(test)]
pub fn insert_reset_credit_batch(
    conn: &Connection,
    connector_run_id: i64,
    credit_count: i64,
    expires_at_utc: DateTime<Utc>,
    source_connector_id: &str,
    confidence: &str,
) -> rusqlite::Result<()> {
    let captured_at_utc = Utc::now().to_rfc3339();
    let raw_batch_hash = hash_reset_batch(credit_count, expires_at_utc, source_connector_id);
    conn.execute(
        r#"
        INSERT INTO reset_credit_batches
          (connector_run_id, captured_at_utc, credit_count, expires_at_utc,
           source_connector_id, confidence, raw_batch_hash)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            connector_run_id,
            captured_at_utc,
            credit_count,
            expires_at_utc.to_rfc3339(),
            source_connector_id,
            confidence,
            raw_batch_hash
        ],
    )?;
    Ok(())
}

#[cfg(test)]
pub struct NewRateLimitWindow<'a> {
    pub connector_run_id: i64,
    pub window_key: &'a str,
    pub limit_tokens: i64,
    pub used_tokens: i64,
    pub remaining_tokens: i64,
    pub resets_at_utc: DateTime<Utc>,
    pub confidence: &'a str,
}

#[cfg(test)]
pub fn insert_rate_limit_window(
    conn: &Connection,
    window: &NewRateLimitWindow<'_>,
) -> rusqlite::Result<()> {
    let captured_at_utc = Utc::now().to_rfc3339();
    conn.execute(
        r#"
        INSERT INTO rate_limit_windows
          (connector_run_id, captured_at_utc, window_key, limit_tokens, used_tokens,
           remaining_tokens, resets_at_utc, confidence)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            window.connector_run_id,
            captured_at_utc,
            window.window_key,
            window.limit_tokens,
            window.used_tokens,
            window.remaining_tokens,
            window.resets_at_utc.to_rfc3339(),
            window.confidence
        ],
    )?;
    Ok(())
}

pub fn insert_account_snapshot(
    conn: &Connection,
    snapshot: &AccountSnapshot,
) -> rusqlite::Result<i64> {
    let transaction = conn.unchecked_transaction()?;
    let run_id = insert_account_snapshot_inner(&transaction, snapshot)?;
    transaction.commit()?;
    Ok(run_id)
}

fn insert_account_snapshot_inner(
    conn: &Connection,
    snapshot: &AccountSnapshot,
) -> rusqlite::Result<i64> {
    let run_id = insert_account_refresh_run(
        conn,
        &AccountRefreshRunInsert {
            started_at_utc: &snapshot.diagnostics.started_at_utc,
            completed_at_utc: &snapshot.diagnostics.completed_at_utc,
            status: snapshot.status.as_str(),
            selected_codex_executable: Some(&snapshot.launch.selected_executable),
            launch_mode: Some(snapshot.launch.mode.as_str()),
            executable_candidates: &snapshot.launch.candidates,
            first_failing_stage: snapshot.diagnostics.first_failing_stage.as_deref(),
            redacted_error_code: snapshot.diagnostics.redacted_error_code.as_deref(),
            redacted_error_message: &snapshot.diagnostics.redacted_error_message,
            stderr_tail: &snapshot.diagnostics.stderr_tail,
            used_last_good_snapshot: snapshot.diagnostics.used_last_good_snapshot,
            method_statuses: &snapshot.methods,
            schema_fingerprint: &snapshot.diagnostics.schema_fingerprint,
            exit_code: snapshot.diagnostics.exit_code,
            timed_out: false,
            child_terminated: Some(snapshot.diagnostics.child_terminated),
            argv_prefix: &snapshot.launch.argv_prefix,
            runtime_source: infer_runtime_source(&snapshot.launch.argv_prefix),
            runtime_display_path: infer_runtime_display(
                &snapshot.launch.selected_executable,
                &snapshot.launch.argv_prefix,
            ),
        },
    )?;

    conn.execute(
        r#"
        INSERT INTO account_identity_snapshots (refresh_run_id, account_label, plan)
        VALUES (?1, ?2, ?3)
        "#,
        params![
            run_id,
            snapshot.account.account_label,
            snapshot.account.plan
        ],
    )?;
    if method_succeeded(&snapshot.methods, "account/usage/read") {
        conn.execute(
            r#"
        INSERT INTO account_usage_snapshots
          (refresh_run_id, lifetime_tokens, captured_at_utc, schema_fingerprint)
        VALUES (?1, ?2, ?3, ?4)
        "#,
            params![
                run_id,
                snapshot.usage.lifetime_tokens,
                snapshot.diagnostics.completed_at_utc,
                snapshot.diagnostics.schema_fingerprint
            ],
        )?;
        for bucket in &snapshot.usage.daily_buckets {
            conn.execute(
                r#"
            INSERT INTO account_daily_usage_buckets
              (refresh_run_id, usage_date, input_tokens, output_tokens, total_tokens)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
                params![
                    run_id,
                    bucket.date,
                    bucket.input_tokens,
                    bucket.output_tokens,
                    bucket.total_tokens
                ],
            )?;
        }
    }
    if method_succeeded(&snapshot.methods, "account/rateLimits/read") {
        conn.execute(
            r#"
        INSERT INTO account_reset_credit_snapshots
          (refresh_run_id, available_count, expires_at_utc, captured_at_utc, schema_fingerprint)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
            params![
                run_id,
                snapshot.reset_credits.available_count,
                snapshot.reset_credits.expires_at_utc,
                snapshot.diagnostics.completed_at_utc,
                snapshot.diagnostics.schema_fingerprint
            ],
        )?;
        let reset_snapshot_id = conn.last_insert_rowid();
        if let Some(details) = &snapshot.reset_credits.credits {
            for detail in details {
                conn.execute(
                    r#"
                    INSERT INTO account_reset_credit_details
                      (reset_credit_snapshot_id, credit_id, reset_type, status, granted_at_utc,
                       expires_at_utc, title, description)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    "#,
                    params![
                        reset_snapshot_id,
                        detail.id,
                        detail.reset_type,
                        detail.status,
                        detail.granted_at_utc,
                        detail.expires_at_utc,
                        detail.title,
                        detail.description
                    ],
                )?;
            }
        }
        for bucket in &snapshot.rate_limits {
            conn.execute(
                r#"
            INSERT INTO account_rate_limit_buckets
              (refresh_run_id, bucket_id, display_name, captured_at_utc, schema_fingerprint)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
                params![
                    run_id,
                    bucket.bucket_id,
                    bucket.display_name,
                    snapshot.diagnostics.completed_at_utc,
                    snapshot.diagnostics.schema_fingerprint
                ],
            )?;
            let bucket_row_id = conn.last_insert_rowid();
            for window in &bucket.windows {
                conn.execute(
                    r#"
                INSERT INTO account_rate_limit_windows
                  (bucket_row_id, window_duration_mins, window_label, used_percent,
                   remaining_percent, resets_at_utc)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                    params![
                        bucket_row_id,
                        window.window_duration_mins,
                        window.window_label,
                        window.used_percent,
                        window.remaining_percent,
                        window.resets_at_utc
                    ],
                )?;
            }
        }
    }

    Ok(run_id)
}

pub fn insert_account_refresh_error(
    conn: &Connection,
    error: &AccountConnectorError,
) -> rusqlite::Result<i64> {
    let now = Utc::now().to_rfc3339();
    insert_account_refresh_run(
        conn,
        &AccountRefreshRunInsert {
            started_at_utc: &now,
            completed_at_utc: &now,
            status: "unavailable",
            selected_codex_executable: (!error.launch.selected_executable.is_empty())
                .then_some(error.launch.selected_executable.as_str()),
            launch_mode: Some(error.launch.mode.as_str()),
            executable_candidates: &error.launch.candidates,
            first_failing_stage: Some(&error.stage),
            redacted_error_code: Some(account_error_code(error)),
            redacted_error_message: &error.public_message,
            stderr_tail: "",
            used_last_good_snapshot: false,
            method_statuses: &[AccountMethodSnapshot {
                method: error.stage.clone(),
                status: crate::codex_app_server::MethodStatus::Failed,
                redacted_error: Some(error.public_message.clone()),
            }],
            schema_fingerprint: "",
            exit_code: error.exit_code,
            timed_out: error.timed_out,
            child_terminated: Some(error.child_terminated),
            argv_prefix: &error.launch.argv_prefix,
            runtime_source: infer_runtime_source(&error.launch.argv_prefix),
            runtime_display_path: infer_runtime_display(
                &error.launch.selected_executable,
                &error.launch.argv_prefix,
            ),
        },
    )
}

struct AccountRefreshRunInsert<'a> {
    started_at_utc: &'a str,
    completed_at_utc: &'a str,
    status: &'a str,
    selected_codex_executable: Option<&'a str>,
    launch_mode: Option<&'a str>,
    executable_candidates: &'a [String],
    first_failing_stage: Option<&'a str>,
    redacted_error_code: Option<&'a str>,
    redacted_error_message: &'a str,
    stderr_tail: &'a str,
    used_last_good_snapshot: bool,
    method_statuses: &'a [AccountMethodSnapshot],
    schema_fingerprint: &'a str,
    exit_code: Option<i32>,
    timed_out: bool,
    child_terminated: Option<bool>,
    argv_prefix: &'a [String],
    runtime_source: Option<&'a str>,
    runtime_display_path: &'a str,
}

fn insert_account_refresh_run(
    conn: &Connection,
    insert: &AccountRefreshRunInsert<'_>,
) -> rusqlite::Result<i64> {
    conn.execute(
        r#"
        INSERT INTO account_refresh_runs
          (started_at_utc, completed_at_utc, status, selected_codex_executable, launch_mode,
           executable_candidates_json, first_failing_stage, redacted_error_code,
           redacted_error_message, stderr_tail, used_last_good_snapshot, method_statuses_json,
           exit_code, timed_out, child_terminated, argv_prefix_json, runtime_source, runtime_display_path)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
        "#,
        params![
            insert.started_at_utc,
            insert.completed_at_utc,
            insert.status,
            insert.selected_codex_executable,
            insert.launch_mode,
            serde_json::to_string(insert.executable_candidates)
                .unwrap_or_else(|_| "[]".to_string()),
            insert.first_failing_stage,
            insert.redacted_error_code,
            insert.redacted_error_message,
            insert.stderr_tail,
            if insert.used_last_good_snapshot { 1 } else { 0 },
            serde_json::to_string(insert.method_statuses).unwrap_or_else(|_| "[]".to_string()),
            insert.exit_code,
            if insert.timed_out { 1 } else { 0 },
            insert
                .child_terminated
                .map(|value| if value { 1 } else { 0 }),
            serde_json::to_string(insert.argv_prefix).unwrap_or_else(|_| "[]".into()),
            insert.runtime_source,
            insert.runtime_display_path,
        ],
    )?;
    let run_id = conn.last_insert_rowid();
    for method in insert.method_statuses {
        conn.execute(
            r#"
            INSERT INTO account_method_attempts
              (refresh_run_id, method, status, redacted_error, captured_at_utc, schema_fingerprint)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                run_id,
                method.method,
                method_status_name(method.status),
                method.redacted_error,
                insert.completed_at_utc,
                insert.schema_fingerprint
            ],
        )?;
    }
    Ok(run_id)
}

fn infer_runtime_source(argv_prefix: &[String]) -> Option<&str> {
    (!argv_prefix.is_empty()).then_some("npm")
}

fn infer_runtime_display<'a>(selected: &'a str, argv_prefix: &'a [String]) -> &'a str {
    argv_prefix.first().map(String::as_str).unwrap_or(selected)
}

fn method_succeeded(methods: &[AccountMethodSnapshot], method: &str) -> bool {
    methods.iter().any(|attempt| {
        attempt.method == method && attempt.status == crate::codex_app_server::MethodStatus::Ok
    })
}

fn method_status_name(status: crate::codex_app_server::MethodStatus) -> &'static str {
    match status {
        crate::codex_app_server::MethodStatus::Ok => "ok",
        crate::codex_app_server::MethodStatus::Failed => "failed",
        crate::codex_app_server::MethodStatus::Skipped => "skipped",
    }
}

fn account_error_code(error: &AccountConnectorError) -> &'static str {
    match error.kind {
        crate::codex_app_server::AccountConnectorErrorKind::MissingCli => "missing_cli",
        crate::codex_app_server::AccountConnectorErrorKind::UnsupportedCli => "unsupported_cli",
        crate::codex_app_server::AccountConnectorErrorKind::LoggedOut => "logged_out",
        crate::codex_app_server::AccountConnectorErrorKind::Timeout => "timeout",
        crate::codex_app_server::AccountConnectorErrorKind::Protocol => "protocol_error",
        crate::codex_app_server::AccountConnectorErrorKind::Spawn => "spawn_failed",
    }
}

#[cfg(test)]
fn hash_reset_batch(
    credit_count: i64,
    expires_at_utc: DateTime<Utc>,
    source_connector_id: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!(
        "{source_connector_id}:{credit_count}:{}",
        expires_at_utc.to_rfc3339()
    ));
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn migrations_create_schema_from_empty_db() {
        let conn = open_memory().unwrap();
        let tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('usage_events', 'source_coverage', 'reset_credit_batches')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 3);
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = open_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
    }

    #[test]
    fn migrations_add_typed_codex_runtime_settings_without_replacing_existing_schema() {
        let conn = open_memory().unwrap();
        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(codex_runtime_settings)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert_eq!(
            columns,
            [
                "singleton_key",
                "display_path",
                "executable_path",
                "argv_prefix_json",
                "source",
                "validated_at_utc",
                "version",
            ]
        );
    }

    #[test]
    fn migrations_add_per_method_attempts_and_separate_reset_credit_details() {
        let conn = open_memory().unwrap();
        let tables: Vec<String> = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('account_method_attempts', 'account_reset_credit_details') ORDER BY name",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            tables,
            ["account_method_attempts", "account_reset_credit_details"]
        );

        for (table, expected) in [
            (
                "account_usage_snapshots",
                vec!["captured_at_utc", "schema_fingerprint"],
            ),
            (
                "account_reset_credit_snapshots",
                vec!["captured_at_utc", "schema_fingerprint"],
            ),
            (
                "account_rate_limit_buckets",
                vec!["captured_at_utc", "schema_fingerprint"],
            ),
        ] {
            let columns: Vec<String> = conn
                .prepare(&format!("PRAGMA table_info({table})"))
                .unwrap()
                .query_map([], |row| row.get(1))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            for column in expected {
                assert!(columns.iter().any(|actual| actual == column));
            }
        }
    }

    #[test]
    fn failed_refresh_persists_launch_candidates_and_method_error() {
        let conn = open_memory().unwrap();
        let error = AccountConnectorError {
            kind: crate::codex_app_server::AccountConnectorErrorKind::Spawn,
            stage: "spawn".to_string(),
            public_message: "access denied".to_string(),
            exit_code: Some(23),
            timed_out: true,
            child_terminated: true,
            launch: crate::codex_app_server::AccountLaunchDiagnostics {
                selected_executable: "C:\\Program Files\\Codex\\codex.exe".to_string(),
                argv_prefix: Vec::new(),
                mode: crate::codex_app_server::CodexLaunchMode::RuntimeValidation,
                candidates: vec![
                    "configured:C:\\Program Files\\Codex\\codex.exe:access_denied".to_string(),
                ],
            },
            failure_class: crate::codex_app_server::AccountConnectorFailureClass::Transport,
        };
        let run_id = insert_account_refresh_error(&conn, &error).unwrap();
        let (selected, candidates): (Option<String>, String) = conn
            .query_row(
                "SELECT selected_codex_executable, executable_candidates_json FROM account_refresh_runs WHERE id = ?1",
                [run_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            selected.as_deref(),
            Some("C:\\Program Files\\Codex\\codex.exe")
        );
        let process: (Option<i64>, i64, i64) = conn.query_row(
            "SELECT exit_code, timed_out, child_terminated FROM account_refresh_runs WHERE id = ?1",
            [run_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
        assert_eq!(process, (Some(23), 1, 1));
        assert!(candidates.contains("access_denied"));
        let method_error: String = conn
            .query_row(
                "SELECT redacted_error FROM account_method_attempts WHERE refresh_run_id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(method_error, "access denied");
    }

    #[test]
    fn successful_rate_limit_method_persists_reset_summary_and_detail_separately() {
        use crate::codex_app_server::*;
        let conn = open_memory().unwrap();
        let now = Utc::now().to_rfc3339();
        let snapshot = AccountSnapshot {
            status: AccountRefreshStatus::Connected,
            launch: AccountLaunchDiagnostics {
                selected_executable: "codex".to_string(),
                argv_prefix: Vec::new(),
                mode: CodexLaunchMode::ListenStdioNoMcp,
                candidates: vec!["codex".to_string()],
            },
            diagnostics: AccountRefreshDiagnostics {
                started_at_utc: now.clone(),
                completed_at_utc: now,
                first_failing_stage: None,
                redacted_error_code: None,
                redacted_error_message: String::new(),
                stderr_tail: String::new(),
                used_last_good_snapshot: false,
                schema_fingerprint: APP_SERVER_SCHEMA_FINGERPRINT.to_string(),
                exit_code: None,
                child_terminated: true,
            },
            account: AccountIdentitySnapshot::default(),
            usage: AccountUsageSnapshot::default(),
            reset_credits: AccountResetCreditsSnapshot {
                available_count: Some(1),
                expires_at_utc: Some("2026-08-01T00:00:00Z".to_string()),
                credits: Some(vec![AccountResetCreditDetail {
                    id: "credit-1".to_string(),
                    reset_type: "weekly".to_string(),
                    status: "available".to_string(),
                    granted_at_utc: "2026-07-01T00:00:00Z".to_string(),
                    expires_at_utc: Some("2026-08-01T00:00:00Z".to_string()),
                    title: Some("Reset".to_string()),
                    description: None,
                }]),
            },
            rate_limits: Vec::new(),
            methods: vec![AccountMethodSnapshot {
                method: "account/rateLimits/read".to_string(),
                status: MethodStatus::Ok,
                redacted_error: None,
            }],
        };
        insert_account_snapshot(&conn, &snapshot).unwrap();
        let summary_count: i64 = conn
            .query_row(
                "SELECT available_count FROM account_reset_credit_snapshots",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let detail_id: String = conn
            .query_row(
                "SELECT credit_id FROM account_reset_credit_details",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(summary_count, 1);
        assert_eq!(detail_id, "credit-1");
        let child_terminated: i64 = conn
            .query_row(
                "SELECT child_terminated FROM account_refresh_runs",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(child_terminated, 1);

        let rollback_conn = open_memory().unwrap();
        rollback_conn
            .execute_batch(
                "CREATE TRIGGER reject_identity BEFORE INSERT ON account_identity_snapshots BEGIN SELECT RAISE(ABORT, 'forced mid-write failure'); END;",
            )
            .unwrap();
        assert!(insert_account_snapshot(&rollback_conn, &snapshot).is_err());
        for table in ["account_refresh_runs", "account_method_attempts"] {
            let count: i64 = rollback_conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "{table} escaped transaction rollback");
        }
    }

    #[test]
    fn usage_events_roundtrip() {
        let conn = open_memory().unwrap();
        let doc_id =
            upsert_source_document(&conn, "local", "hash", "history.jsonl", "content", 120)
                .unwrap();
        let event = UsageEvent {
            event_uid: "event-1".to_string(),
            source_document_id: doc_id,
            session_uid: "session-1".to_string(),
            occurred_at_utc: Utc.with_ymd_and_hms(2026, 7, 2, 18, 0, 0).unwrap(),
            model: Some("gpt-5.5".to_string()),
            mode: Some("executor".to_string()),
            input_tokens: 10,
            output_tokens: 20,
            cache_read_tokens: 5,
            cache_write_tokens: 1,
            total_tokens: 36,
            raw_event_kind: "token_count".to_string(),
            confidence: "high".to_string(),
        };
        assert!(insert_usage_event(&conn, &event).unwrap());
        assert!(!insert_usage_event(&conn, &event).unwrap());
        let stored = usage_event_by_uid(&conn, "event-1").unwrap().unwrap();
        assert_eq!(stored.total_tokens, 36);
    }

    #[test]
    fn foreign_keys_and_unique_constraints_prevent_duplicates() {
        let conn = open_memory().unwrap();
        let doc_id =
            upsert_source_document(&conn, "local", "hash", "history.jsonl", "content", 120)
                .unwrap();
        let event = UsageEvent {
            event_uid: "event-unique".to_string(),
            source_document_id: doc_id,
            session_uid: "session-1".to_string(),
            occurred_at_utc: Utc::now(),
            model: None,
            mode: None,
            input_tokens: 1,
            output_tokens: 1,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 2,
            raw_event_kind: "token_count".to_string(),
            confidence: "high".to_string(),
        };
        assert!(insert_usage_event(&conn, &event).unwrap());
        assert!(!insert_usage_event(&conn, &event).unwrap());
        assert_eq!(count_usage_events(&conn).unwrap(), 1);
    }

    #[test]
    fn connector_runs_and_reset_credit_batches_roundtrip() {
        let conn = open_memory().unwrap();
        let run_id = insert_connector_run(
            &conn,
            "known-reset-credit",
            "complete",
            Some("known-reset-credit"),
            Some(200),
            None,
            None,
        )
        .unwrap();
        insert_reset_credit_batch(
            &conn,
            run_id,
            4,
            Utc.with_ymd_and_hms(2026, 7, 28, 18, 14, 0).unwrap(),
            "known-reset-credit",
            "high",
        )
        .unwrap();
        insert_rate_limit_window(
            &conn,
            &NewRateLimitWindow {
                connector_run_id: run_id,
                window_key: "gpt-5",
                limit_tokens: 1000,
                used_tokens: 250,
                remaining_tokens: 750,
                resets_at_utc: Utc.with_ymd_and_hms(2026, 7, 3, 18, 14, 0).unwrap(),
                confidence: "medium",
            },
        )
        .unwrap();

        let stored: i64 = conn
            .query_row(
                "SELECT credit_count FROM reset_credit_batches WHERE connector_run_id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored, 4);
        let remaining: i64 = conn
            .query_row(
                "SELECT remaining_tokens FROM rate_limit_windows WHERE connector_run_id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 750);
    }
}
