use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MIGRATIONS: &[&str] = &[r#"
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
"#];

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
    pub warnings: Vec<String>,
}

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

pub fn count_usage_events(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM usage_events", [], |row| row.get(0))
}

pub fn usage_total(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(total_tokens), 0) FROM usage_events",
        [],
        |row| row.get(0),
    )
}

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
          (source_kind, started_at_utc, completed_at_utc, status, files_seen, events_seen, events_imported, warnings_json)
        VALUES ('local-codex-history', ?1, ?1, 'complete', ?2, ?3, ?4, ?5)
        "#,
        params![
            now,
            summary.files_seen as i64,
            summary.events_seen as i64,
            summary.events_imported as i64,
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
}
