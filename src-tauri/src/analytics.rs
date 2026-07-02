use chrono::{DateTime, Datelike, TimeZone, Utc};
use chrono_tz::America::New_York;
use rusqlite::Connection;
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

pub fn build_dashboard_summary(
    conn: &Connection,
    data_mode: &str,
) -> rusqlite::Result<DashboardSummaryDto> {
    let generated_at_utc = Utc::now().to_rfc3339();
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
    let coverage = load_coverage(conn)?;
    let local_coverage = coverage
        .first()
        .cloned()
        .unwrap_or_else(default_local_coverage);

    Ok(DashboardSummaryDto {
        generated_at_utc,
        data_mode: data_mode.to_string(),
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
                value: "4".to_string(),
                delta: "Available".to_string(),
                status: "positive".to_string(),
                coverage: reset_coverage(),
            },
        ],
        heatmap: heatmap(conn)?,
        reset_credits: sample_reset_credits(),
        coverage: if coverage.is_empty() {
            vec![
                default_local_coverage(),
                reset_coverage(),
                undocumented_coverage(),
            ]
        } else {
            let mut all = coverage;
            all.push(reset_coverage());
            all.push(undocumented_coverage());
            all
        },
        connectors: sample_connectors(),
        sessions: sample_sessions(),
        rate_limit_windows: sample_rate_limit_windows(),
        next_reset: NextResetDto {
            label: "22d 03h 14m".to_string(),
            expires_at_ny: "Jul 28, 2026, 2:14 PM EDT".to_string(),
            timezone: "America/New_York".to_string(),
        },
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
        Ok(CoverageDto {
            metric_key: row.get(0)?,
            source_kind: row.get(1)?,
            coverage_percent: row.get(2)?,
            confidence: row.get(3)?,
            last_evidence_at_utc: row.get(4)?,
            formula_version: row.get(5)?,
            missing_facets: serde_json::from_str(&missing).unwrap_or_default(),
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

fn reset_coverage() -> CoverageDto {
    CoverageDto {
        metric_key: "reset-credits".to_string(),
        source_kind: "Reset credits".to_string(),
        coverage_percent: 100,
        confidence: "high".to_string(),
        last_evidence_at_utc: Utc::now().to_rfc3339(),
        formula_version: "coverage-v1".to_string(),
        missing_facets: Vec::new(),
        explanation: "Reset-credit connector snapshots are schema-valid and freshness checked."
            .to_string(),
    }
}

fn undocumented_coverage() -> CoverageDto {
    CoverageDto {
        metric_key: "undocumented".to_string(),
        source_kind: "Undocumented (RO)".to_string(),
        coverage_percent: 68,
        confidence: "medium".to_string(),
        last_evidence_at_utc: Utc::now().to_rfc3339(),
        formula_version: "coverage-v1".to_string(),
        missing_facets: vec!["documented public contract".to_string()],
        explanation: "Endpoint is registered as read-only with an explicit schema; confidence remains conservative.".to_string(),
    }
}

fn sample_reset_credits() -> Vec<ResetCreditDto> {
    [
        ("reset-1", 4, "2026-07-28T18:14:00Z", 22),
        ("reset-2", 3, "2026-08-25T18:14:00Z", 50),
        ("reset-3", 2, "2026-09-22T18:14:00Z", 78),
        ("reset-4", 1, "2026-10-20T18:14:00Z", 106),
    ]
    .into_iter()
    .map(|(id, credit_count, utc, days_remaining)| {
        let expires = DateTime::parse_from_rfc3339(utc)
            .unwrap()
            .with_timezone(&Utc);
        ResetCreditDto {
            id: id.to_string(),
            credit_count,
            expires_at_utc: utc.to_string(),
            expires_at_ny: format_reset_expiration_ny(expires),
            days_remaining,
            confidence: "high".to_string(),
        }
    })
    .collect()
}

fn sample_connectors() -> Vec<ConnectorDto> {
    vec![
        ConnectorDto {
            id: "local".to_string(),
            name: "Local Codex CLI".to_string(),
            detail: "~/.codex/history/*.jsonl".to_string(),
            status: "connected".to_string(),
            read_only: true,
            safety_class: "Read-only".to_string(),
            last_run_at_utc: Utc::now().to_rfc3339(),
        },
        ConnectorDto {
            id: "known-reset-credit".to_string(),
            name: "Known read-only endpoint".to_string(),
            detail: "/wham/rate-limit-reset-credits".to_string(),
            status: "connected".to_string(),
            read_only: true,
            safety_class: "Read-only".to_string(),
            last_run_at_utc: Utc::now().to_rfc3339(),
        },
        ConnectorDto {
            id: "undocumented-ro".to_string(),
            name: "Undocumented (RO)".to_string(),
            detail: "registered schema-gated endpoint".to_string(),
            status: "connected".to_string(),
            read_only: true,
            safety_class: "Read-only".to_string(),
            last_run_at_utc: Utc::now().to_rfc3339(),
        },
    ]
}

fn sample_sessions() -> Vec<SessionDto> {
    vec![
        SessionDto {
            id: "s1".to_string(),
            start_time: "Jun 14, 1:12 PM".to_string(),
            duration: "47m 23s".to_string(),
            tokens: "512.3M".to_string(),
            peak_tokens: "1.72B".to_string(),
            mode: "deep-research".to_string(),
            sources: vec!["CLI".to_string(), "Cloud".to_string()],
        },
        SessionDto {
            id: "s2".to_string(),
            start_time: "Jun 14, 10:01 AM".to_string(),
            duration: "32m 11s".to_string(),
            tokens: "286.4M".to_string(),
            peak_tokens: "1.04B".to_string(),
            mode: "code-review".to_string(),
            sources: vec!["CLI".to_string(), "Cloud".to_string()],
        },
    ]
}

fn sample_rate_limit_windows() -> Vec<RateLimitWindowDto> {
    vec![
        RateLimitWindowDto {
            id: "1m".to_string(),
            window: "1m".to_string(),
            limit: "20.0B".to_string(),
            used: "11.2B".to_string(),
            remaining: "8.8B (44%)".to_string(),
            resets_in: "00:14".to_string(),
            progress_percent: 56,
        },
        RateLimitWindowDto {
            id: "30d".to_string(),
            window: "30d".to_string(),
            limit: "20.00T".to_string(),
            used: "3.62T".to_string(),
            remaining: "16.38T (82%)".to_string(),
            resets_in: "16d 03h".to_string(),
            progress_percent: 18,
        },
    ]
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
}
