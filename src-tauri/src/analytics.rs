use chrono::{DateTime, Datelike, TimeZone, Utc};
use chrono_tz::America::New_York;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageDto {
    pub metric_key: String,
    pub source_kind: String,
    pub coverage_percent: i64,
    pub confidence: String,
    pub last_evidence_at_utc: String,
    pub formula_version: String,
    pub missing_facets: Vec<String>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricDto {
    pub key: String,
    pub label: String,
    pub value: String,
    pub delta: String,
    pub status: String,
    pub coverage: CoverageDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapDayDto {
    pub date: String,
    pub weekday: String,
    pub tokens: i64,
    pub intensity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummaryDto {
    pub generated_at_utc: String,
    pub data_mode: String,
    pub last_refresh_label: String,
    pub refresh_status: String,
    pub timezone: String,
    pub metrics: Vec<MetricDto>,
    pub heatmap: Vec<HeatmapDayDto>,
    pub reset_credits: Vec<ResetCreditDto>,
    pub coverage: Vec<CoverageDto>,
    pub connectors: Vec<ConnectorDto>,
    pub sessions: Vec<SessionDto>,
    pub rate_limit_windows: Vec<RateLimitWindowDto>,
    pub next_reset: NextResetDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetCreditDto {
    pub id: String,
    pub credit_count: i64,
    pub expires_at_utc: String,
    pub expires_at_ny: String,
    pub days_remaining: i64,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorDto {
    pub id: String,
    pub name: String,
    pub detail: String,
    pub status: String,
    pub read_only: bool,
    pub safety_class: String,
    pub last_run_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub id: String,
    pub start_time: String,
    pub duration: String,
    pub tokens: String,
    pub peak_tokens: String,
    pub mode: String,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitWindowDto {
    pub id: String,
    pub window: String,
    pub limit: String,
    pub used: String,
    pub remaining: String,
    pub resets_in: String,
    pub progress_percent: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextResetDto {
    pub label: String,
    pub expires_at_ny: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DataMode {
    Local,
    Remote,
    Combined,
}

impl DataMode {
    fn parse(value: &str) -> Self {
        match value {
            "local" => Self::Local,
            "remote" => Self::Remote,
            _ => Self::Combined,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
            Self::Combined => "combined",
        }
    }

    fn includes_local(self) -> bool {
        matches!(self, Self::Local | Self::Combined)
    }

    fn includes_remote(self) -> bool {
        matches!(self, Self::Remote | Self::Combined)
    }
}

struct LoadedResetCredit {
    dto: ResetCreditDto,
    captured_at_utc: String,
}

pub fn build_dashboard_summary(
    conn: &Connection,
    data_mode: &str,
) -> rusqlite::Result<DashboardSummaryDto> {
    let data_mode = DataMode::parse(data_mode);
    let generated_at_utc = Utc::now().to_rfc3339();
    let (lifetime, today, month, peak) = if data_mode.includes_local() {
        let lifetime: i64 = conn.query_row(
            "SELECT COALESCE(SUM(total_tokens), 0) FROM usage_events",
            [],
            |row| row.get(0),
        )?;
        let today = today_tokens_ny(conn, Utc::now())?;
        let month = month_to_date_tokens_ny(conn, Utc::now())?;
        let peak = conn.query_row(
            "SELECT COALESCE(MAX(total_tokens), 0) FROM usage_events",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        (lifetime, today, month, peak)
    } else {
        (0, 0, 0, 0)
    };
    let coverage = load_coverage(conn)?;
    let local_coverage = coverage
        .iter()
        .find(|entry| entry.metric_key == "local-usage")
        .cloned()
        .unwrap_or_else(default_local_coverage);
    let loaded_reset_credits = if data_mode.includes_remote() {
        load_reset_credits(conn)?
    } else {
        Vec::new()
    };
    let reset_credit_dtos = loaded_reset_credits
        .iter()
        .map(|credit| credit.dto.clone())
        .collect::<Vec<_>>();
    let latest_reset_run = if data_mode.includes_remote() {
        latest_connector_run(conn, "known-reset-credit")?
    } else {
        None
    };
    let reset_coverage = reset_coverage(&loaded_reset_credits, latest_reset_run.as_ref());
    let reset_credit_count = reset_credit_dtos
        .iter()
        .map(|credit| credit.credit_count)
        .sum::<i64>();
    let rate_limit_windows = if data_mode.includes_remote() {
        load_rate_limit_windows(conn)?
    } else {
        Vec::new()
    };
    let latest_undocumented_run = if data_mode.includes_remote() {
        latest_connector_run(conn, "undocumented-rate-limits")?
    } else {
        None
    };
    let undocumented_coverage =
        undocumented_coverage(&rate_limit_windows, latest_undocumented_run.as_ref());

    Ok(DashboardSummaryDto {
        generated_at_utc,
        data_mode: data_mode.label().to_string(),
        last_refresh_label: "just now".to_string(),
        refresh_status: "idle".to_string(),
        timezone: "America/New_York".to_string(),
        metrics: vec![
            metric(
                "lifetime",
                "Lifetime tokens",
                lifetime,
                "Imported local history",
                &local_coverage,
            ),
            metric(
                "today",
                "Today",
                today,
                "America/New_York bucket",
                &local_coverage,
            ),
            metric(
                "month",
                "This month",
                month,
                "Month-to-date",
                &local_coverage,
            ),
            metric(
                "peak",
                "Peak session",
                peak,
                "Largest imported event",
                &local_coverage,
            ),
            MetricDto {
                key: "reset".to_string(),
                label: "Reset credits".to_string(),
                value: reset_credit_count.to_string(),
                delta: if reset_credit_count > 0 {
                    "Available"
                } else {
                    "No snapshot"
                }
                .to_string(),
                status: if reset_credit_count > 0 {
                    "positive"
                } else {
                    "neutral"
                }
                .to_string(),
                coverage: reset_coverage.clone(),
            },
        ],
        heatmap: if data_mode.includes_local() {
            heatmap(conn)?
        } else {
            empty_heatmap()
        },
        reset_credits: reset_credit_dtos.clone(),
        coverage: if coverage.is_empty() {
            vec![
                default_local_coverage(),
                reset_coverage.clone(),
                undocumented_coverage.clone(),
            ]
        } else {
            let mut all = coverage;
            all.push(reset_coverage.clone());
            all.push(undocumented_coverage.clone());
            all
        },
        connectors: load_connectors(conn, data_mode)?,
        sessions: if data_mode.includes_local() {
            load_sessions(conn)?
        } else {
            Vec::new()
        },
        rate_limit_windows,
        next_reset: next_reset(&reset_credit_dtos),
    })
}

pub fn today_tokens_ny(conn: &Connection, now: DateTime<Utc>) -> rusqlite::Result<i64> {
    let local = now.with_timezone(&New_York);
    let start_local = New_York
        .with_ymd_and_hms(local.year(), local.month(), local.day(), 0, 0, 0)
        .single()
        .expect("valid New York midnight");
    let end_local = start_local + chrono::Duration::days(1);
    tokens_between(
        conn,
        start_local.with_timezone(&Utc),
        end_local.with_timezone(&Utc),
    )
}

pub fn month_to_date_tokens_ny(conn: &Connection, now: DateTime<Utc>) -> rusqlite::Result<i64> {
    let local = now.with_timezone(&New_York);
    let start_local = New_York
        .with_ymd_and_hms(local.year(), local.month(), 1, 0, 0, 0)
        .single()
        .expect("valid New York month start");
    tokens_between(conn, start_local.with_timezone(&Utc), now)
}

pub fn format_reset_expiration_ny(expires_at_utc: DateTime<Utc>) -> String {
    expires_at_utc
        .with_timezone(&New_York)
        .format("%b %-d, %Y, %-I:%M %p %Z")
        .to_string()
}

fn tokens_between(
    conn: &Connection,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(total_tokens), 0) FROM usage_events WHERE occurred_at_utc >= ?1 AND occurred_at_utc < ?2",
        [start.to_rfc3339(), end.to_rfc3339()],
        |row| row.get(0),
    )
}

fn load_coverage(conn: &Connection) -> rusqlite::Result<Vec<CoverageDto>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT metric_key, source_kind, coverage_percent, confidence, last_evidence_at_utc,
               formula_version, missing_facets_json, explanation
        FROM source_coverage ORDER BY id DESC LIMIT 4
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        let missing: String = row.get(6)?;
        let missing_facets = serde_json::from_str(&missing).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
        Ok(CoverageDto {
            metric_key: row.get(0)?,
            source_kind: row.get(1)?,
            coverage_percent: row.get(2)?,
            confidence: row.get(3)?,
            last_evidence_at_utc: row.get(4)?,
            formula_version: row.get(5)?,
            missing_facets,
            explanation: row.get(7)?,
        })
    })?;
    rows.collect()
}

fn metric(key: &str, label: &str, value: i64, delta: &str, coverage: &CoverageDto) -> MetricDto {
    MetricDto {
        key: key.to_string(),
        label: label.to_string(),
        value: compact_tokens(value),
        delta: delta.to_string(),
        status: if value > 0 { "positive" } else { "neutral" }.to_string(),
        coverage: coverage.clone(),
    }
}

fn compact_tokens(value: i64) -> String {
    if value >= 1_000_000_000 {
        format!("{:.2}B", value as f64 / 1_000_000_000.0)
    } else if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else {
        value.to_string()
    }
}

fn heatmap(conn: &Connection) -> rusqlite::Result<Vec<HeatmapDayDto>> {
    let mut stmt = conn.prepare(
        "SELECT substr(occurred_at_utc, 1, 10) AS day, SUM(total_tokens) FROM usage_events GROUP BY day ORDER BY day LIMIT 112",
    )?;
    let rows = stmt.query_map([], |row| {
        let tokens: i64 = row.get(1)?;
        Ok(HeatmapDayDto {
            date: row.get(0)?,
            weekday: "Mon".to_string(),
            tokens,
            intensity: (tokens / 50_000_000).clamp(0, 5),
        })
    })?;
    let mut days: Vec<_> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    while days.len() < 112 {
        days.push(HeatmapDayDto {
            date: format!("2026-07-{:02}", (days.len() % 28) + 1),
            weekday: "Mon".to_string(),
            tokens: 0,
            intensity: 0,
        });
    }
    Ok(days)
}

fn empty_heatmap() -> Vec<HeatmapDayDto> {
    (0..112)
        .map(|index| HeatmapDayDto {
            date: format!("2026-07-{:02}", (index % 28) + 1),
            weekday: "Mon".to_string(),
            tokens: 0,
            intensity: 0,
        })
        .collect()
}

fn default_local_coverage() -> CoverageDto {
    CoverageDto {
        metric_key: "local-history".to_string(),
        source_kind: "Local history".to_string(),
        coverage_percent: 0,
        confidence: "unavailable".to_string(),
        last_evidence_at_utc: Utc::now().to_rfc3339(),
        formula_version: "coverage-v1".to_string(),
        missing_facets: vec!["local usage events".to_string()],
        explanation: "No local usage events have been imported yet.".to_string(),
    }
}

fn reset_coverage(
    credits: &[LoadedResetCredit],
    latest_run: Option<&ConnectorRunRow>,
) -> CoverageDto {
    let latest_failed = latest_run
        .map(|run| connector_status(&run.status) == "degraded")
        .unwrap_or(false);
    if credits.is_empty() {
        let latest_run_at = latest_run.map(connector_run_evidence_at);
        return CoverageDto {
            metric_key: "reset-credits".to_string(),
            source_kind: "Reset credits".to_string(),
            coverage_percent: 0,
            confidence: "unavailable".to_string(),
            last_evidence_at_utc: latest_run_at.unwrap_or_else(|| Utc::now().to_rfc3339()),
            formula_version: "coverage-v1".to_string(),
            missing_facets: vec!["successful reset-credit connector snapshot".to_string()],
            explanation: if latest_failed {
                "Latest reset-credit connector attempt failed and no last-good snapshot is stored yet."
            } else {
                "No schema-valid reset-credit connector snapshot is stored yet."
            }
            .to_string(),
        };
    }

    if latest_failed {
        return CoverageDto {
            metric_key: "reset-credits".to_string(),
            source_kind: "Reset credits".to_string(),
            coverage_percent: 60,
            confidence: "low".to_string(),
            last_evidence_at_utc: latest_run
                .map(connector_run_evidence_at)
                .expect("latest_failed requires a latest connector run"),
            formula_version: "coverage-v1".to_string(),
            missing_facets: vec!["fresh successful reset-credit connector snapshot".to_string()],
            explanation:
                "Showing the last-good reset-credit snapshot because the latest connector refresh failed."
                    .to_string(),
        };
    }

    let confidence = if credits.iter().all(|credit| credit.dto.confidence == "high") {
        "high"
    } else {
        "medium"
    };
    CoverageDto {
        metric_key: "reset-credits".to_string(),
        source_kind: "Reset credits".to_string(),
        coverage_percent: if confidence == "high" { 100 } else { 80 },
        confidence: confidence.to_string(),
        last_evidence_at_utc: credits
            .iter()
            .map(|credit| credit.captured_at_utc.as_str())
            .max()
            .expect("reset coverage is built only for nonempty credit snapshots")
            .to_string(),
        formula_version: "coverage-v1".to_string(),
        missing_facets: Vec::new(),
        explanation:
            "Reset-credit connector snapshots are stored, schema-valid, and freshness checked."
                .to_string(),
    }
}

fn undocumented_coverage(
    rate_limit_windows: &[RateLimitWindowDto],
    latest_run: Option<&ConnectorRunRow>,
) -> CoverageDto {
    let latest_failed = latest_run
        .map(|run| connector_status(&run.status) == "degraded")
        .unwrap_or(false);
    let latest_run_at = latest_run.map(connector_run_evidence_at);

    if rate_limit_windows.is_empty() {
        return CoverageDto {
            metric_key: "rate-limit-windows".to_string(),
            source_kind: "Rate-limit windows".to_string(),
            coverage_percent: 0,
            confidence: "unavailable".to_string(),
            last_evidence_at_utc: latest_run_at.unwrap_or_else(|| Utc::now().to_rfc3339()),
            formula_version: "coverage-v1".to_string(),
            missing_facets: vec!["successful rate-limit window snapshot".to_string()],
            explanation: if latest_failed {
                "Latest rate-limit window refresh failed and no previous snapshot is stored yet."
            } else {
                "No rate-limit window snapshot is stored yet."
            }
            .to_string(),
        };
    }

    if latest_failed {
        return CoverageDto {
            metric_key: "rate-limit-windows".to_string(),
            source_kind: "Rate-limit windows".to_string(),
            coverage_percent: 35,
            confidence: "low".to_string(),
            last_evidence_at_utc: latest_run_at
                .expect("latest_failed requires a latest connector run"),
            formula_version: "coverage-v1".to_string(),
            missing_facets: vec!["fresh rate-limit window snapshot".to_string()],
            explanation:
                "Showing the last stored rate-limit windows because the latest refresh failed."
                    .to_string(),
        };
    }

    CoverageDto {
        metric_key: "rate-limit-windows".to_string(),
        source_kind: "Rate-limit windows".to_string(),
        coverage_percent: 68,
        confidence: "medium".to_string(),
        last_evidence_at_utc: latest_run_at.unwrap_or_else(|| Utc::now().to_rfc3339()),
        formula_version: "coverage-v1".to_string(),
        missing_facets: vec!["additional source confirmation".to_string()],
        explanation:
            "Rate-limit windows are stored with conservative confidence until more evidence is available."
                .to_string(),
    }
}

fn load_reset_credits(conn: &Connection) -> rusqlite::Result<Vec<LoadedResetCredit>> {
    let Some(run_id) = latest_successful_connector_run_id(conn, "known-reset-credit")? else {
        return Ok(Vec::new());
    };
    let mut stmt = conn.prepare(
        r#"
        SELECT id, credit_count, expires_at_utc, confidence, captured_at_utc
        FROM reset_credit_batches
        WHERE connector_run_id = ?1
        ORDER BY expires_at_utc ASC
        LIMIT 12
        "#,
    )?;
    let rows = stmt.query_map([run_id], |row| {
        let id: i64 = row.get(0)?;
        let expires_raw: String = row.get(2)?;
        let expires_at_utc = parse_db_utc(&expires_raw, 2)?;
        let days_remaining = (expires_at_utc - Utc::now()).num_days().max(0);
        Ok(LoadedResetCredit {
            dto: ResetCreditDto {
                id: format!("reset-{id}"),
                credit_count: row.get(1)?,
                expires_at_utc: expires_at_utc.to_rfc3339(),
                expires_at_ny: format_reset_expiration_ny(expires_at_utc),
                days_remaining,
                confidence: row.get(3)?,
            },
            captured_at_utc: row.get(4)?,
        })
    })?;
    rows.collect()
}

fn load_connectors(conn: &Connection, data_mode: DataMode) -> rusqlite::Result<Vec<ConnectorDto>> {
    let local_count: i64 = if data_mode.includes_local() {
        conn.query_row("SELECT COUNT(*) FROM usage_events", [], |row| row.get(0))?
    } else {
        0
    };
    let local_run = latest_import_run(conn)?;
    let known_run = if data_mode.includes_remote() {
        latest_connector_run(conn, "known-reset-credit")?
    } else {
        None
    };
    let undocumented_run = if data_mode.includes_remote() {
        latest_connector_run(conn, "undocumented-rate-limits")?
    } else {
        None
    };

    Ok(vec![
        ConnectorDto {
            id: "local".to_string(),
            name: "Local Codex history".to_string(),
            detail: local_connector_detail(local_count, local_run.as_ref()),
            status: local_connector_status(local_count, local_run.as_ref()),
            read_only: true,
            safety_class: "Local".to_string(),
            last_run_at_utc: local_run
                .map(|run| run.completed_at_utc)
                .unwrap_or_else(no_evidence_timestamp),
        },
        ConnectorDto {
            id: "known-reset-credit".to_string(),
            name: "Reset credits".to_string(),
            detail: connector_detail(
                known_run.as_ref(),
                "Latest reset-credit snapshot checked",
                "No reset-credit snapshot yet",
            ),
            status: known_run
                .as_ref()
                .map(|run| connector_status(&run.status))
                .unwrap_or_else(|| "unavailable".to_string()),
            read_only: true,
            safety_class: "Snapshot".to_string(),
            last_run_at_utc: known_run
                .map(|run| run.completed_at_utc.unwrap_or(run.started_at_utc))
                .unwrap_or_else(no_evidence_timestamp),
        },
        ConnectorDto {
            id: "rate-limit-windows".to_string(),
            name: "Rate-limit windows".to_string(),
            detail: connector_detail(
                undocumented_run.as_ref(),
                "Latest rate-limit window snapshot checked",
                "No rate-limit window snapshot yet",
            ),
            status: undocumented_run
                .as_ref()
                .map(|run| connector_status(&run.status))
                .unwrap_or_else(|| "unavailable".to_string()),
            read_only: true,
            safety_class: "Snapshot".to_string(),
            last_run_at_utc: undocumented_run
                .map(|run| run.completed_at_utc.unwrap_or(run.started_at_utc))
                .unwrap_or_else(no_evidence_timestamp),
        },
    ])
}

struct ImportRunRow {
    completed_at_utc: String,
    files_seen: i64,
    events_seen: i64,
    events_imported: i64,
}

fn latest_import_run(conn: &Connection) -> rusqlite::Result<Option<ImportRunRow>> {
    conn.query_row(
        r#"
        SELECT completed_at_utc, files_seen, events_seen, events_imported
        FROM import_runs
        ORDER BY id DESC
        LIMIT 1
        "#,
        [],
        |row| {
            Ok(ImportRunRow {
                completed_at_utc: row.get(0)?,
                files_seen: row.get(1)?,
                events_seen: row.get(2)?,
                events_imported: row.get(3)?,
            })
        },
    )
    .optional()
}

fn local_connector_detail(local_count: i64, run: Option<&ImportRunRow>) -> String {
    if local_count > 0 {
        return format!("{local_count} local events imported");
    }
    match run {
        Some(run) if run.files_seen == 0 => {
            "Checked local Codex folders; no parseable local history found".to_string()
        }
        Some(run) if run.events_seen > 0 && run.events_imported == 0 => {
            "Checked local Codex history; no parseable token events found".to_string()
        }
        Some(_) => "Checked local Codex history; no parseable local history found".to_string(),
        None => "No local history imported yet".to_string(),
    }
}

fn local_connector_status(local_count: i64, run: Option<&ImportRunRow>) -> String {
    if local_count > 0 {
        "connected"
    } else if run.is_some() {
        "degraded"
    } else {
        "unavailable"
    }
    .to_string()
}

fn no_evidence_timestamp() -> String {
    String::new()
}

fn connector_status(status: &str) -> String {
    match status {
        "complete" | "connected" => "connected",
        "failed" | "degraded" => "degraded",
        _ => "unavailable",
    }
    .to_string()
}

fn connector_detail(
    run: Option<&ConnectorRunRow>,
    success_detail: &str,
    unavailable_detail: &str,
) -> String {
    match run {
        None => unavailable_detail.to_string(),
        Some(run) if matches!(run.status.as_str(), "complete" | "connected") => {
            success_detail.to_string()
        }
        Some(run) if run.redacted_error_code.as_deref() == Some("auth_unavailable") => {
            "Auth unavailable; snapshot refresh skipped".to_string()
        }
        Some(_) => "Snapshot refresh needs attention".to_string(),
    }
}

struct ConnectorRunRow {
    started_at_utc: String,
    completed_at_utc: Option<String>,
    status: String,
    redacted_error_code: Option<String>,
}

fn latest_connector_run(
    conn: &Connection,
    connector_id: &str,
) -> rusqlite::Result<Option<ConnectorRunRow>> {
    conn.query_row(
        r#"
        SELECT started_at_utc, completed_at_utc, status, redacted_error_code
        FROM connector_runs
        WHERE connector_id = ?1
        ORDER BY id DESC
        LIMIT 1
        "#,
        [connector_id],
        |row| {
            Ok(ConnectorRunRow {
                started_at_utc: row.get(0)?,
                completed_at_utc: row.get(1)?,
                status: row.get(2)?,
                redacted_error_code: row.get(3)?,
            })
        },
    )
    .optional()
}

fn connector_run_evidence_at(run: &ConnectorRunRow) -> String {
    run.completed_at_utc
        .clone()
        .unwrap_or_else(|| run.started_at_utc.clone())
}

fn latest_successful_connector_run_id(
    conn: &Connection,
    connector_id: &str,
) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        r#"
        SELECT id
        FROM connector_runs
        WHERE connector_id = ?1 AND status = 'complete'
        ORDER BY id DESC
        LIMIT 1
        "#,
        [connector_id],
        |row| row.get(0),
    )
    .optional()
}

fn load_sessions(conn: &Connection) -> rusqlite::Result<Vec<SessionDto>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT session_uid, MIN(occurred_at_utc), MAX(occurred_at_utc),
               SUM(total_tokens), MAX(total_tokens), COALESCE(MAX(mode), 'unknown')
        FROM usage_events
        GROUP BY session_uid
        ORDER BY MAX(occurred_at_utc) DESC
        LIMIT 8
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        let start_raw: String = row.get(1)?;
        let end_raw: String = row.get(2)?;
        let start = parse_db_utc(&start_raw, 1)?;
        let end = parse_db_utc(&end_raw, 2)?;
        let total: i64 = row.get(3)?;
        let peak: i64 = row.get(4)?;
        Ok(SessionDto {
            id: row.get(0)?,
            start_time: start
                .with_timezone(&New_York)
                .format("%b %-d, %-I:%M %p")
                .to_string(),
            duration: format_duration(end - start),
            tokens: compact_tokens(total),
            peak_tokens: compact_tokens(peak),
            mode: row.get(5)?,
            sources: vec!["Local history".to_string()],
        })
    })?;
    rows.collect()
}

fn load_rate_limit_windows(conn: &Connection) -> rusqlite::Result<Vec<RateLimitWindowDto>> {
    let Some(run_id) = latest_successful_connector_run_id(conn, "undocumented-rate-limits")? else {
        return Ok(Vec::new());
    };
    let mut stmt = conn.prepare(
        r#"
        SELECT id, window_key, limit_tokens, used_tokens, remaining_tokens, resets_at_utc
        FROM rate_limit_windows
        WHERE connector_run_id = ?1
        ORDER BY resets_at_utc ASC
        LIMIT 8
        "#,
    )?;
    let rows = stmt.query_map([run_id], |row| {
        let id: i64 = row.get(0)?;
        let limit: i64 = row.get(2)?;
        let used: i64 = row.get(3)?;
        let remaining: i64 = row.get(4)?;
        let resets_raw: String = row.get(5)?;
        let resets_at = parse_db_utc(&resets_raw, 5)?;
        Ok(RateLimitWindowDto {
            id: format!("rate-limit-{id}"),
            window: row.get(1)?,
            limit: compact_tokens(limit),
            used: compact_tokens(used),
            remaining: format!(
                "{} ({:.0}%)",
                compact_tokens(remaining),
                percent(remaining, limit)
            ),
            resets_in: format_resets_in(resets_at - Utc::now()),
            progress_percent: percent(used, limit).round() as i64,
        })
    })?;
    rows.collect()
}

fn next_reset(reset_credits: &[ResetCreditDto]) -> NextResetDto {
    reset_credits
        .first()
        .map(|credit| NextResetDto {
            label: format!("{}d remaining", credit.days_remaining),
            expires_at_ny: credit.expires_at_ny.clone(),
            timezone: "America/New_York".to_string(),
        })
        .unwrap_or_else(|| NextResetDto {
            label: "No reset-credit snapshot".to_string(),
            expires_at_ny: "Unavailable".to_string(),
            timezone: "America/New_York".to_string(),
        })
}

fn parse_db_utc(input: &str, column: usize) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(input)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                column,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })
}

fn format_duration(duration: chrono::Duration) -> String {
    let seconds = duration.num_seconds().max(0);
    let minutes = seconds / 60;
    let hours = minutes / 60;
    if hours > 0 {
        format!("{hours}h {:02}m", minutes % 60)
    } else {
        format!("{minutes}m {:02}s", seconds % 60)
    }
}

fn format_resets_in(duration: chrono::Duration) -> String {
    let minutes = duration.num_minutes().max(0);
    let days = minutes / 1440;
    let hours = (minutes % 1440) / 60;
    if days > 0 {
        format!("{days}d {hours:02}h")
    } else {
        format!("{hours:02}h {:02}m", minutes % 60)
    }
}

fn percent(numerator: i64, denominator: i64) -> f64 {
    if denominator <= 0 {
        0.0
    } else {
        ((numerator as f64 / denominator as f64) * 100.0).clamp(0.0, 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{insert_usage_event, open_memory, upsert_source_document, UsageEvent};
    use chrono::TimeZone;

    fn seeded_conn() -> Connection {
        let conn = open_memory().unwrap();
        let doc_id =
            upsert_source_document(&conn, "local", "hash", "history.jsonl", "content", 1).unwrap();
        for (id, occurred, total) in [
            ("today", "2026-07-02T15:00:00Z", 100),
            ("month", "2026-07-01T15:00:00Z", 50),
            ("old", "2026-06-15T15:00:00Z", 25),
        ] {
            insert_usage_event(
                &conn,
                &UsageEvent {
                    event_uid: id.to_string(),
                    source_document_id: doc_id,
                    session_uid: id.to_string(),
                    occurred_at_utc: DateTime::parse_from_rfc3339(occurred)
                        .unwrap()
                        .with_timezone(&Utc),
                    model: None,
                    mode: None,
                    input_tokens: 0,
                    output_tokens: 0,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                    total_tokens: total,
                    raw_event_kind: "token_count".to_string(),
                    confidence: "high".to_string(),
                },
            )
            .unwrap();
        }
        conn
    }

    #[test]
    fn computes_lifetime_tokens() {
        let conn = seeded_conn();
        let summary = build_dashboard_summary(&conn, "combined").unwrap();
        assert_eq!(summary.metrics[0].value, "175");
    }

    #[test]
    fn computes_today_in_america_new_york() {
        let conn = seeded_conn();
        let now = Utc.with_ymd_and_hms(2026, 7, 2, 20, 0, 0).unwrap();
        assert_eq!(today_tokens_ny(&conn, now).unwrap(), 100);
    }

    #[test]
    fn computes_month_to_date_in_america_new_york() {
        let conn = seeded_conn();
        let now = Utc.with_ymd_and_hms(2026, 7, 2, 20, 0, 0).unwrap();
        assert_eq!(month_to_date_tokens_ny(&conn, now).unwrap(), 150);
    }

    #[test]
    fn formats_reset_expiration_in_america_new_york() {
        let expires = Utc.with_ymd_and_hms(2026, 7, 28, 18, 14, 0).unwrap();
        assert_eq!(
            format_reset_expiration_ny(expires),
            "Jul 28, 2026, 2:14 PM EDT"
        );
    }

    #[test]
    fn handles_dst_spring_forward() {
        let expires = Utc.with_ymd_and_hms(2026, 3, 8, 7, 30, 0).unwrap();
        assert!(format_reset_expiration_ny(expires).contains("EDT"));
    }

    #[test]
    fn handles_dst_fall_back() {
        let expires = Utc.with_ymd_and_hms(2026, 11, 1, 6, 30, 0).unwrap();
        assert!(format_reset_expiration_ny(expires).contains("EST"));
    }

    #[test]
    fn handles_zero_data_without_nan_or_crash() {
        let conn = open_memory().unwrap();
        let summary = build_dashboard_summary(&conn, "local").unwrap();
        assert_eq!(summary.metrics[0].value, "0");
        assert_eq!(summary.timezone, "America/New_York");
    }

    #[test]
    fn local_connector_reports_checked_empty_history_after_refresh() {
        let conn = open_memory().unwrap();
        crate::db::insert_import_run(
            &conn,
            &crate::db::ImportRunSummary {
                files_seen: 0,
                events_seen: 0,
                events_imported: 0,
                warnings: Vec::new(),
            },
        )
        .unwrap();

        let summary = build_dashboard_summary(&conn, "combined").unwrap();
        let local = summary
            .connectors
            .iter()
            .find(|connector| connector.id == "local")
            .unwrap();

        assert_eq!(local.status, "degraded");
        assert!(local.detail.contains("Checked"));
        assert!(local.detail.contains("no parseable local history"));
    }

    #[test]
    fn missing_reset_connector_evidence_does_not_overstate_coverage() {
        let conn = open_memory().unwrap();
        let summary = build_dashboard_summary(&conn, "combined").unwrap();
        let reset_metric = summary
            .metrics
            .iter()
            .find(|metric| metric.key == "reset")
            .unwrap();

        assert_eq!(reset_metric.value, "0");
        assert_eq!(reset_metric.coverage.coverage_percent, 0);
        assert_eq!(reset_metric.coverage.confidence, "unavailable");
        assert!(summary.reset_credits.is_empty());
        assert!(summary.rate_limit_windows.is_empty());
    }

    #[test]
    fn failed_remote_connectors_show_sanitized_error_details() {
        let conn = open_memory().unwrap();
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

        let summary = build_dashboard_summary(&conn, "combined").unwrap();
        let connector = summary
            .connectors
            .iter()
            .find(|connector| connector.id == "known-reset-credit")
            .unwrap();

        assert_eq!(connector.status, "degraded");
        assert!(connector.detail.contains("Auth unavailable"));
        assert!(!connector.detail.contains("auth.json"));
    }

    #[test]
    fn dashboard_summary_uses_user_facing_source_labels() {
        let conn = open_memory().unwrap();
        let summary = build_dashboard_summary(&conn, "combined").unwrap();
        let visible_copy = summary
            .connectors
            .iter()
            .flat_map(|connector| {
                [
                    connector.name.as_str(),
                    connector.detail.as_str(),
                    connector.safety_class.as_str(),
                ]
            })
            .chain(summary.coverage.iter().flat_map(|coverage| {
                [coverage.source_kind.as_str(), coverage.explanation.as_str()]
            }))
            .collect::<Vec<_>>()
            .join(" ");

        for internal_term in ["Read-only", "read-only", "Undocumented", "schema-gated"] {
            assert!(
                !visible_copy.contains(internal_term),
                "visible summary copy leaked internal term: {internal_term}"
            );
        }
    }

    #[test]
    fn data_mode_filters_local_and_remote_sources() {
        let conn = seeded_conn();
        let run_id = crate::db::insert_connector_run(
            &conn,
            "known-reset-credit",
            "complete",
            Some("known-reset-credit"),
            Some(200),
            None,
            None,
        )
        .unwrap();
        crate::db::insert_reset_credit_batch(
            &conn,
            run_id,
            4,
            Utc.with_ymd_and_hms(2026, 7, 28, 18, 14, 0).unwrap(),
            "known-reset-credit",
            "high",
        )
        .unwrap();

        let local = build_dashboard_summary(&conn, "local").unwrap();
        let remote = build_dashboard_summary(&conn, "remote").unwrap();
        let combined = build_dashboard_summary(&conn, "combined").unwrap();

        assert_eq!(local.metrics[0].value, "175");
        assert_eq!(local.metrics[4].value, "0");
        assert!(local.reset_credits.is_empty());
        assert_eq!(remote.metrics[0].value, "0");
        assert_eq!(remote.metrics[4].value, "4");
        assert!(remote.sessions.is_empty());
        assert_eq!(combined.metrics[0].value, "175");
        assert_eq!(combined.metrics[4].value, "4");
    }

    #[test]
    fn repeated_remote_refreshes_use_latest_successful_snapshot_only() {
        let conn = open_memory().unwrap();
        for _ in 0..2 {
            let reset_run_id = crate::db::insert_connector_run(
                &conn,
                "known-reset-credit",
                "complete",
                Some("known-reset-credit"),
                Some(200),
                None,
                None,
            )
            .unwrap();
            crate::db::insert_reset_credit_batch(
                &conn,
                reset_run_id,
                4,
                Utc.with_ymd_and_hms(2026, 7, 28, 18, 14, 0).unwrap(),
                "known-reset-credit",
                "high",
            )
            .unwrap();

            let rate_run_id = crate::db::insert_connector_run(
                &conn,
                "undocumented-rate-limits",
                "complete",
                Some("undocumented-rate-limits"),
                Some(200),
                None,
                None,
            )
            .unwrap();
            crate::db::insert_rate_limit_window(
                &conn,
                &crate::db::NewRateLimitWindow {
                    connector_run_id: rate_run_id,
                    window_key: "gpt-5",
                    limit_tokens: 1000,
                    used_tokens: 250,
                    remaining_tokens: 750,
                    resets_at_utc: Utc.with_ymd_and_hms(2026, 7, 3, 18, 14, 0).unwrap(),
                    confidence: "medium",
                },
            )
            .unwrap();
        }
        let failed_run_id = crate::db::insert_connector_run(
            &conn,
            "known-reset-credit",
            "failed",
            Some("known-reset-credit"),
            None,
            Some("connector_failed"),
            Some("synthetic failure"),
        )
        .unwrap();
        crate::db::insert_reset_credit_batch(
            &conn,
            failed_run_id,
            99,
            Utc.with_ymd_and_hms(2026, 12, 1, 18, 14, 0).unwrap(),
            "known-reset-credit",
            "low",
        )
        .unwrap();
        crate::db::insert_connector_run(
            &conn,
            "undocumented-rate-limits",
            "failed",
            Some("undocumented-rate-limits"),
            None,
            Some("connector_failed"),
            Some("synthetic failure"),
        )
        .unwrap();

        let summary = build_dashboard_summary(&conn, "remote").unwrap();
        let reset_metric = summary
            .metrics
            .iter()
            .find(|metric| metric.key == "reset")
            .unwrap();
        let undocumented_coverage = summary
            .coverage
            .iter()
            .find(|coverage| coverage.metric_key == "rate-limit-windows")
            .unwrap();

        assert_eq!(summary.metrics[4].value, "4");
        assert_eq!(summary.reset_credits.len(), 1);
        assert_eq!(reset_metric.coverage.coverage_percent, 60);
        assert_eq!(reset_metric.coverage.confidence, "low");
        assert!(reset_metric
            .coverage
            .explanation
            .contains("latest connector refresh failed"));
        assert_eq!(summary.rate_limit_windows.len(), 1);
        assert_eq!(summary.rate_limit_windows[0].remaining, "750 (75%)");
        assert_eq!(undocumented_coverage.coverage_percent, 35);
        assert_eq!(undocumented_coverage.confidence, "low");
    }
}
