use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::America::New_York;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub account_metrics: Vec<MetricDto>,
    pub local_metrics: Vec<MetricDto>,
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
    pub freshness: String,
    pub age_seconds: Option<i64>,
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

struct LoadedAccountUsage {
    lifetime_tokens: Option<i64>,
    today_tokens: i64,
    month_tokens: i64,
    captured_at_utc: String,
    degraded: bool,
}

pub fn build_dashboard_summary(
    conn: &Connection,
    data_mode: &str,
) -> rusqlite::Result<DashboardSummaryDto> {
    let data_mode = DataMode::parse(data_mode);
    let generated_at_utc = Utc::now().to_rfc3339();
    let local_event_count: i64 = if data_mode.includes_local() {
        conn.query_row("SELECT COUNT(*) FROM usage_events", [], |row| row.get(0))?
    } else {
        0
    };
    let (local_lifetime, local_today, local_month, peak) = if data_mode.includes_local() {
        let local_lifetime: i64 = conn.query_row(
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
        (local_lifetime, today, month, peak)
    } else {
        (0, 0, 0, 0)
    };
    let coverage = load_coverage(conn)?;
    let local_coverage = coverage
        .iter()
        .find(|entry| entry.metric_key == "local-usage")
        .cloned()
        .unwrap_or_else(default_local_coverage);
    let account_usage = if data_mode.includes_remote() {
        load_account_usage(conn)?
    } else {
        None
    };
    let account_coverage =
        account_usage_coverage(account_usage.as_ref(), latest_account_run(conn)?.as_ref());
    let (
        lifetime_value,
        today_value,
        month_value,
        lifetime_delta,
        today_delta,
        month_delta,
        profile_label,
        profile_coverage,
    ) = if data_mode.includes_remote() && account_usage.is_some() {
        (
            account_usage
                .as_ref()
                .and_then(|usage| usage.lifetime_tokens)
                .map(compact_tokens)
                .unwrap_or_else(|| "Unavailable".to_string()),
            account_usage
                .as_ref()
                .map(|usage| compact_tokens(usage.today_tokens))
                .unwrap_or_else(|| "Unavailable".to_string()),
            account_usage
                .as_ref()
                .map(|usage| compact_tokens(usage.month_tokens))
                .unwrap_or_else(|| "Unavailable".to_string()),
            "Codex account snapshot",
            "Account daily bucket",
            "Account month-to-date",
            "Account lifetime",
            account_coverage.clone(),
        )
    } else if data_mode.includes_local() {
        (
            compact_tokens(local_lifetime),
            compact_tokens(local_today),
            compact_tokens(local_month),
            if data_mode.includes_remote() {
                "Account unavailable; showing local history"
            } else {
                "Imported local history"
            },
            if data_mode.includes_remote() {
                "Account unavailable; showing local day"
            } else {
                "Local history day bucket"
            },
            if data_mode.includes_remote() {
                "Account unavailable; showing local month"
            } else {
                "Local history month-to-date"
            },
            "Local history tokens",
            local_coverage.clone(),
        )
    } else {
        (
            "Unavailable".to_string(),
            "Unavailable".to_string(),
            "Unavailable".to_string(),
            "No account snapshot",
            "No account snapshot",
            "No account snapshot",
            "Account lifetime",
            account_coverage.clone(),
        )
    };
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
        latest_account_run(conn)?
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
    let latest_rate_limit_run = if data_mode.includes_remote() {
        latest_account_run(conn)?
    } else {
        None
    };
    let rate_limit_coverage =
        rate_limit_coverage(&rate_limit_windows, latest_rate_limit_run.as_ref());
    let account_metrics = account_usage
        .as_ref()
        .filter(|_| data_mode.includes_remote())
        .map(|usage| {
            vec![
                metric_value(
                    "account-lifetime",
                    "Account lifetime",
                    usage
                        .lifetime_tokens
                        .map(compact_tokens)
                        .unwrap_or_else(|| "Unavailable".to_string()),
                    "Codex account snapshot",
                    &account_coverage,
                ),
                metric_value(
                    "account-today",
                    "Account today",
                    compact_tokens(usage.today_tokens),
                    "Account daily bucket",
                    &account_coverage,
                ),
                metric_value(
                    "account-month",
                    "Account this month",
                    compact_tokens(usage.month_tokens),
                    "Account month-to-date",
                    &account_coverage,
                ),
            ]
        })
        .unwrap_or_default();
    let local_metrics = if data_mode.includes_local() && local_event_count > 0 {
        vec![
            metric(
                "local-lifetime",
                "Local lifetime",
                local_lifetime,
                "Imported local history",
                &local_coverage,
            ),
            metric(
                "local-today",
                "Local today",
                local_today,
                "Local history day bucket",
                &local_coverage,
            ),
            metric(
                "local-month",
                "Local this month",
                local_month,
                "Local history month-to-date",
                &local_coverage,
            ),
            metric(
                "local-peak",
                "Local peak session",
                peak,
                "Largest imported event",
                &local_coverage,
            ),
        ]
    } else {
        Vec::new()
    };

    Ok(DashboardSummaryDto {
        generated_at_utc,
        data_mode: data_mode.label().to_string(),
        last_refresh_label: "just now".to_string(),
        refresh_status: "idle".to_string(),
        timezone: "America/New_York".to_string(),
        metrics: vec![
            metric_value(
                "lifetime",
                profile_label,
                lifetime_value,
                lifetime_delta,
                &profile_coverage,
            ),
            metric_value(
                "today",
                "Today",
                today_value,
                today_delta,
                &profile_coverage,
            ),
            metric_value(
                "month",
                "This month",
                month_value,
                month_delta,
                &profile_coverage,
            ),
            metric(
                "peak",
                "Peak local session",
                peak,
                "Largest imported event",
                &local_coverage,
            ),
            MetricDto {
                key: "reset".to_string(),
                label: "Reset credits".to_string(),
                value: if data_mode.includes_remote() && loaded_reset_credits.is_empty() {
                    "Unavailable".to_string()
                } else {
                    reset_credit_count.to_string()
                },
                delta: if !data_mode.includes_remote() {
                    "Remote account mode disabled"
                } else if reset_credit_count > 0 {
                    "Available"
                } else if loaded_reset_credits.is_empty() {
                    "No account snapshot"
                } else {
                    "Explicit zero"
                }
                .to_string(),
                status: if reset_credit_count > 0 {
                    "positive"
                } else if data_mode.includes_remote() && loaded_reset_credits.is_empty() {
                    "warning"
                } else {
                    "neutral"
                }
                .to_string(),
                coverage: reset_coverage.clone(),
            },
        ],
        account_metrics,
        local_metrics,
        heatmap: if data_mode.includes_local() {
            heatmap(conn)?
        } else {
            empty_heatmap()
        },
        reset_credits: reset_credit_dtos.clone(),
        coverage: if coverage.is_empty() {
            let mut generated = vec![default_local_coverage()];
            if data_mode.includes_remote() {
                generated.push(account_coverage.clone());
            }
            generated.push(reset_coverage.clone());
            generated.push(rate_limit_coverage.clone());
            generated
        } else {
            let mut all = coverage;
            if data_mode.includes_remote() {
                all.push(account_coverage.clone());
            }
            all.push(reset_coverage.clone());
            all.push(rate_limit_coverage.clone());
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
        FROM source_coverage
        WHERE id IN (SELECT MAX(id) FROM source_coverage GROUP BY metric_key)
        ORDER BY id DESC
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
    metric_value(key, label, compact_tokens(value), delta, coverage)
}

fn metric_value(
    key: &str,
    label: &str,
    value: String,
    delta: &str,
    coverage: &CoverageDto,
) -> MetricDto {
    let status = if value == "Unavailable" {
        "warning"
    } else if value != "0" {
        "positive"
    } else {
        "neutral"
    };
    MetricDto {
        key: key.to_string(),
        label: label.to_string(),
        value,
        delta: delta.to_string(),
        status: status.to_string(),
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
    heatmap_at(conn, Utc::now())
}

fn empty_heatmap() -> Vec<HeatmapDayDto> {
    calendar_heatmap(Utc::now(), &HashMap::new())
}

fn heatmap_at(conn: &Connection, now: DateTime<Utc>) -> rusqlite::Result<Vec<HeatmapDayDto>> {
    let end = now.with_timezone(&New_York).date_naive();
    let start = end - Duration::days(111);
    let mut totals = HashMap::<NaiveDate, i64>::new();
    let mut stmt = conn.prepare(
        "SELECT occurred_at_utc, total_tokens FROM usage_events ORDER BY occurred_at_utc",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (occurred_at_utc, tokens) = row?;
        let occurred = parse_db_utc(&occurred_at_utc, 0)?;
        let date = occurred.with_timezone(&New_York).date_naive();
        if (start..=end).contains(&date) {
            *totals.entry(date).or_default() += tokens;
        }
    }
    Ok(calendar_heatmap(now, &totals))
}

fn calendar_heatmap(now: DateTime<Utc>, totals: &HashMap<NaiveDate, i64>) -> Vec<HeatmapDayDto> {
    let end = now.with_timezone(&New_York).date_naive();
    let start = end - Duration::days(111);
    (0..112)
        .map(|offset| {
            let date = start + Duration::days(offset);
            let tokens = totals.get(&date).copied().unwrap_or_default();
            HeatmapDayDto {
                date: date.format("%Y-%m-%d").to_string(),
                weekday: date.format("%a").to_string(),
                tokens,
                intensity: if tokens == 0 {
                    0
                } else {
                    (tokens / 50_000_000 + 1).clamp(1, 5)
                },
            }
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

fn account_usage_coverage(
    usage: Option<&LoadedAccountUsage>,
    latest_run: Option<&ConnectorRunRow>,
) -> CoverageDto {
    let latest_run_at = latest_run.map(connector_run_evidence_at);
    match usage {
        Some(usage) if usage.lifetime_tokens.is_some() && !usage.degraded => CoverageDto {
            metric_key: "account-usage".to_string(),
            source_kind: "Codex account usage".to_string(),
            coverage_percent: 100,
            confidence: "high".to_string(),
            last_evidence_at_utc: usage.captured_at_utc.clone(),
            formula_version: "coverage-v2".to_string(),
            missing_facets: Vec::new(),
            explanation: "Profile totals come from the Codex app-server account usage snapshot."
                .to_string(),
        },
        Some(usage) if usage.lifetime_tokens.is_some() => CoverageDto {
            metric_key: "account-usage".to_string(),
            source_kind: "Codex account usage".to_string(),
            coverage_percent: 70,
            confidence: "low".to_string(),
            last_evidence_at_utc: usage.captured_at_utc.clone(),
            formula_version: "coverage-v2".to_string(),
            missing_facets: vec!["fresh account refresh".to_string()],
            explanation:
                "Showing the last-good account usage snapshot because the latest refresh degraded."
                    .to_string(),
        },
        _ => CoverageDto {
            metric_key: "account-usage".to_string(),
            source_kind: "Codex account usage".to_string(),
            coverage_percent: 0,
            confidence: "unavailable".to_string(),
            last_evidence_at_utc: latest_run_at.unwrap_or_else(|| Utc::now().to_rfc3339()),
            formula_version: "coverage-v2".to_string(),
            missing_facets: vec!["successful account/usage/read snapshot".to_string()],
            explanation:
                "No Codex app-server account usage snapshot is stored yet; local history is separate."
                    .to_string(),
        },
    }
}

fn reset_coverage(
    credits: &[LoadedResetCredit],
    latest_run: Option<&ConnectorRunRow>,
) -> CoverageDto {
    let latest_failed = latest_run
        .map(|run| connector_status(&run.status) != "connected")
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

fn rate_limit_coverage(
    rate_limit_windows: &[RateLimitWindowDto],
    latest_run: Option<&ConnectorRunRow>,
) -> CoverageDto {
    let latest_failed = latest_run
        .map(|run| connector_status(&run.status) != "connected")
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
    let Some(run_id) = latest_account_run_id_with_reset_credits(conn)? else {
        return Ok(Vec::new());
    };
    conn.query_row(
        r#"
        SELECT r.id, r.available_count,
               COALESCE(
                 r.expires_at_utc,
                 (
                   SELECT MIN(d.expires_at_utc)
                   FROM account_reset_credit_details d
                   WHERE d.reset_credit_snapshot_id = r.id
                     AND d.status = 'available'
                     AND d.expires_at_utc IS NOT NULL
                     AND d.expires_at_utc > a.completed_at_utc
                 )
               ),
               a.completed_at_utc
        FROM account_reset_credit_snapshots r
        JOIN account_refresh_runs a ON a.id = r.refresh_run_id
        WHERE r.refresh_run_id = ?1
        ORDER BY r.id DESC
        LIMIT 1
        "#,
        [run_id],
        |row| {
            let id: i64 = row.get(0)?;
            let credit_count: i64 = row.get(1)?;
            let expires_raw: Option<String> = row.get(2)?;
            let (expires_at_utc, expires_at_ny, days_remaining) =
                if let Some(expires_raw) = expires_raw {
                    let expires_at_utc = parse_db_utc(&expires_raw, 2)?;
                    (
                        expires_at_utc.to_rfc3339(),
                        format_reset_expiration_ny(expires_at_utc),
                        (expires_at_utc - Utc::now()).num_days().max(0),
                    )
                } else {
                    (String::new(), "Unavailable".to_string(), 0)
                };
            Ok(LoadedResetCredit {
                dto: ResetCreditDto {
                    id: format!("reset-{id}"),
                    credit_count,
                    expires_at_utc,
                    expires_at_ny,
                    days_remaining,
                    confidence: "high".to_string(),
                },
                captured_at_utc: row.get(3)?,
            })
        },
    )
    .optional()
    .map(|maybe| maybe.into_iter().collect())
}

fn load_account_usage(conn: &Connection) -> rusqlite::Result<Option<LoadedAccountUsage>> {
    let Some(run_id) = latest_account_run_id_with_usage(conn)? else {
        return Ok(None);
    };
    let (lifetime_tokens, captured_at_utc): (Option<i64>, String) = conn.query_row(
        r#"
        SELECT u.lifetime_tokens, r.completed_at_utc
        FROM account_usage_snapshots u
        JOIN account_refresh_runs r ON r.id = u.refresh_run_id
        WHERE u.refresh_run_id = ?1
        ORDER BY u.id DESC
        LIMIT 1
        "#,
        [run_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let degraded = facet_freshness(conn, "account/usage/read", Some(run_id))?.stale;
    let mut stmt = conn.prepare(
        r#"
        SELECT usage_date, total_tokens
        FROM account_daily_usage_buckets
        WHERE refresh_run_id = ?1
        "#,
    )?;
    let rows = stmt.query_map([run_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    let now = Utc::now().with_timezone(&New_York);
    let today_key = format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day());
    let month_prefix = format!("{:04}-{:02}-", now.year(), now.month());
    let mut today_tokens = 0;
    let mut month_tokens = 0;
    for row in rows {
        let (date, tokens) = row?;
        if date == today_key {
            today_tokens += tokens;
        }
        if date.starts_with(&month_prefix) {
            month_tokens += tokens;
        }
    }
    Ok(Some(LoadedAccountUsage {
        lifetime_tokens,
        today_tokens,
        month_tokens,
        captured_at_utc,
        degraded,
    }))
}

fn load_connectors(conn: &Connection, data_mode: DataMode) -> rusqlite::Result<Vec<ConnectorDto>> {
    let local_count: i64 = if data_mode.includes_local() {
        conn.query_row("SELECT COUNT(*) FROM usage_events", [], |row| row.get(0))?
    } else {
        0
    };
    let local_run = latest_import_run(conn)?;
    let usage_freshness = facet_freshness(
        conn,
        "account/usage/read",
        latest_account_run_id_with_usage(conn)?,
    )?;
    let reset_freshness = facet_freshness(
        conn,
        "account/rateLimits/read",
        latest_account_run_id_with_reset_credits(conn)?,
    )?;
    let rate_freshness = facet_freshness(
        conn,
        "account/rateLimits/read",
        latest_account_run_id_with_rate_limits(conn)?,
    )?;

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
            freshness: if local_count > 0 {
                "fresh"
            } else {
                "unavailable"
            }
            .to_string(),
            age_seconds: None,
        },
        ConnectorDto {
            id: "account-usage".to_string(),
            name: "Codex account usage".to_string(),
            detail: facet_connector_detail(
                &usage_freshness,
                "Latest account usage snapshot checked",
                "No account usage snapshot yet",
            ),
            status: account_connector_status_for_facet(&usage_freshness),
            read_only: true,
            safety_class: "Snapshot".to_string(),
            last_run_at_utc: usage_freshness
                .latest_attempt_at
                .clone()
                .unwrap_or_else(no_evidence_timestamp),
            freshness: usage_freshness.label().to_string(),
            age_seconds: usage_freshness.age_seconds,
        },
        ConnectorDto {
            id: "known-reset-credit".to_string(),
            name: "Reset credits".to_string(),
            detail: facet_connector_detail(
                &reset_freshness,
                "Latest reset-credit snapshot checked",
                "No reset-credit snapshot yet",
            ),
            status: account_connector_status_for_facet(&reset_freshness),
            read_only: true,
            safety_class: "Snapshot".to_string(),
            last_run_at_utc: reset_freshness
                .latest_attempt_at
                .clone()
                .unwrap_or_else(no_evidence_timestamp),
            freshness: reset_freshness.label().to_string(),
            age_seconds: reset_freshness.age_seconds,
        },
        ConnectorDto {
            id: "rate-limit-windows".to_string(),
            name: "Rate-limit windows".to_string(),
            detail: facet_connector_detail(
                &rate_freshness,
                "Latest rate-limit window snapshot checked",
                "No rate-limit window snapshot yet",
            ),
            status: account_connector_status_for_facet(&rate_freshness),
            read_only: true,
            safety_class: "Snapshot".to_string(),
            last_run_at_utc: rate_freshness
                .latest_attempt_at
                .clone()
                .unwrap_or_else(no_evidence_timestamp),
            freshness: rate_freshness.label().to_string(),
            age_seconds: rate_freshness.age_seconds,
        },
    ])
}

struct FacetFreshness {
    has_last_good: bool,
    stale: bool,
    age_seconds: Option<i64>,
    latest_attempt_at: Option<String>,
    latest_error: Option<String>,
    has_attempt: bool,
}

impl FacetFreshness {
    fn label(&self) -> &'static str {
        if !self.has_last_good {
            "unavailable"
        } else if self.stale {
            "stale"
        } else {
            "fresh"
        }
    }
}

fn facet_freshness(
    conn: &Connection,
    method: &str,
    last_good_run_id: Option<i64>,
) -> rusqlite::Result<FacetFreshness> {
    let latest_attempt: Option<(i64, String, Option<String>, String)> = conn
        .query_row(
            r#"
            SELECT refresh_run_id, status, redacted_error, captured_at_utc
            FROM account_method_attempts
            WHERE method = ?1 OR method NOT LIKE 'account/%'
            ORDER BY refresh_run_id DESC, id DESC
            LIMIT 1
            "#,
            [method],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let captured_at = last_good_run_id
        .map(|run_id| {
            conn.query_row(
                "SELECT completed_at_utc FROM account_refresh_runs WHERE id = ?1",
                [run_id],
                |row| row.get::<_, String>(0),
            )
        })
        .transpose()?;
    let age_seconds = captured_at.as_deref().and_then(|captured_at| {
        DateTime::parse_from_rfc3339(captured_at)
            .ok()
            .map(|captured| {
                (Utc::now() - captured.with_timezone(&Utc))
                    .num_seconds()
                    .max(0)
            })
    });
    let stale = match (last_good_run_id, latest_attempt.as_ref()) {
        (Some(good), Some((attempt, status, _, _))) => *attempt > good || status != "ok",
        _ => false,
    };
    Ok(FacetFreshness {
        has_last_good: last_good_run_id.is_some(),
        stale,
        age_seconds,
        latest_attempt_at: latest_attempt
            .as_ref()
            .map(|attempt| attempt.3.clone())
            .or_else(|| captured_at.clone()),
        latest_error: latest_attempt
            .as_ref()
            .and_then(|attempt| attempt.2.clone()),
        has_attempt: latest_attempt.is_some(),
    })
}

fn account_connector_status_for_facet(freshness: &FacetFreshness) -> String {
    if !freshness.has_last_good && !freshness.has_attempt {
        "unavailable"
    } else if !freshness.has_last_good || freshness.stale {
        "degraded"
    } else {
        "connected"
    }
    .to_string()
}

fn facet_connector_detail(
    freshness: &FacetFreshness,
    success_detail: &str,
    unavailable_detail: &str,
) -> String {
    if !freshness.has_attempt {
        if freshness.has_last_good {
            let legacy_label = success_detail
                .strip_prefix("Latest ")
                .unwrap_or(success_detail)
                .strip_suffix(" checked")
                .unwrap_or(success_detail);
            return format!("Stored {legacy_label} using legacy refresh metadata.");
        }
        return unavailable_detail.to_string();
    }
    if freshness.has_last_good && !freshness.stale {
        return success_detail.to_string();
    }
    if let Some(error) = &freshness.latest_error {
        let public_error = if error.contains("Codex CLI was not found") {
            "Codex CLI not found; configure the executable path or TOKENSTACK_CODEX_BIN."
                .to_string()
        } else {
            error.clone()
        };
        if freshness.has_last_good {
            return format!(
                "Using last-good account snapshot; latest attempt failed: {public_error}"
            );
        }
        return public_error;
    }
    if freshness.has_last_good {
        "Using last-good account snapshot; latest attempt did not succeed.".to_string()
    } else {
        "Account snapshot refresh needs attention.".to_string()
    }
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

struct ConnectorRunRow {
    started_at_utc: String,
    completed_at_utc: Option<String>,
    status: String,
}

fn latest_account_run(conn: &Connection) -> rusqlite::Result<Option<ConnectorRunRow>> {
    conn.query_row(
        r#"
        SELECT started_at_utc, completed_at_utc, status
        FROM account_refresh_runs
        ORDER BY id DESC
        LIMIT 1
        "#,
        [],
        |row| {
            Ok(ConnectorRunRow {
                started_at_utc: row.get(0)?,
                completed_at_utc: row.get(1)?,
                status: row.get(2)?,
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

fn latest_account_run_id_with_usage(conn: &Connection) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        r#"
        SELECT r.id
        FROM account_refresh_runs r
        JOIN account_usage_snapshots u ON u.refresh_run_id = r.id
        WHERE u.lifetime_tokens IS NOT NULL
        ORDER BY r.id DESC
        LIMIT 1
        "#,
        [],
        |row| row.get(0),
    )
    .optional()
}

fn latest_account_run_id_with_reset_credits(conn: &Connection) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        r#"
        SELECT r.id
        FROM account_refresh_runs r
        JOIN account_reset_credit_snapshots c ON c.refresh_run_id = r.id
        WHERE c.available_count IS NOT NULL
        ORDER BY r.id DESC
        LIMIT 1
        "#,
        [],
        |row| row.get(0),
    )
    .optional()
}

fn latest_account_run_id_with_rate_limits(conn: &Connection) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        r#"
        SELECT r.id
        FROM account_refresh_runs r
        JOIN account_rate_limit_buckets b ON b.refresh_run_id = r.id
        JOIN account_rate_limit_windows w ON w.bucket_row_id = b.id
        ORDER BY r.id DESC
        LIMIT 1
        "#,
        [],
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
    let Some(run_id) = latest_account_run_id_with_rate_limits(conn)? else {
        return Ok(Vec::new());
    };
    let mut stmt = conn.prepare(
        r#"
        SELECT w.id, b.bucket_id, b.display_name, w.window_label, w.used_percent,
               w.remaining_percent, w.resets_at_utc
        FROM account_rate_limit_windows w
        JOIN account_rate_limit_buckets b ON b.id = w.bucket_row_id
        WHERE b.refresh_run_id = ?1
        ORDER BY CASE WHEN b.bucket_id = 'codex' THEN 0 ELSE 1 END,
                 b.bucket_id ASC,
                 w.window_duration_mins ASC
        LIMIT 12
        "#,
    )?;
    let rows = stmt.query_map([run_id], |row| {
        let id: i64 = row.get(0)?;
        let bucket_id: String = row.get(1)?;
        let display_name: String = row.get(2)?;
        let used_percent: f64 = row.get(4)?;
        let remaining_percent: f64 = row.get(5)?;
        let resets_raw: Option<String> = row.get(6)?;
        let resets_in = if let Some(resets_raw) = resets_raw {
            let resets_at = parse_db_utc(&resets_raw, 6)?;
            format_resets_in(resets_at - Utc::now())
        } else {
            "Unavailable".to_string()
        };
        Ok(RateLimitWindowDto {
            id: format!("rate-limit-{id}"),
            window: format!("{display_name} ({bucket_id}) {}", row.get::<_, String>(3)?),
            limit: "Percent only".to_string(),
            used: format!("{used_percent:.0}%"),
            remaining: format!("{remaining_percent:.0}%"),
            resets_in,
            progress_percent: used_percent.round().clamp(0.0, 100.0) as i64,
        })
    })?;
    rows.collect()
}

fn next_reset(reset_credits: &[ResetCreditDto]) -> NextResetDto {
    reset_credits
        .iter()
        .filter(|credit| credit.credit_count > 0 && !credit.expires_at_utc.is_empty())
        .min_by(|left, right| left.expires_at_utc.cmp(&right.expires_at_utc))
        .map(|credit| NextResetDto {
            label: format!("{}d remaining", credit.days_remaining),
            expires_at_ny: credit.expires_at_ny.clone(),
            timezone: "America/New_York".to_string(),
        })
        .unwrap_or_else(|| {
            let has_available_credit = reset_credits.iter().any(|credit| credit.credit_count > 0);
            NextResetDto {
                label: if has_available_credit {
                    "Expiration unavailable"
                } else if reset_credits.is_empty() {
                    "No reset-credit snapshot"
                } else {
                    "No reset credits available"
                }
                .to_string(),
                expires_at_ny: "Unavailable".to_string(),
                timezone: "America/New_York".to_string(),
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex_app_server::{
        AccountConnectorError, AccountConnectorErrorKind, AccountDailyUsageBucket,
        AccountIdentitySnapshot, AccountLaunchDiagnostics, AccountMethodSnapshot,
        AccountRateLimitBucket, AccountRateLimitWindow, AccountRefreshDiagnostics,
        AccountRefreshStatus, AccountResetCreditsSnapshot, AccountSnapshot, AccountUsageSnapshot,
        CodexLaunchMode, MethodStatus,
    };
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

    fn insert_account_fixture(
        conn: &Connection,
        lifetime_tokens: Option<i64>,
        reset_credits: Option<i64>,
        rate_used_percent: Option<f64>,
        status: AccountRefreshStatus,
    ) {
        let now = Utc::now().to_rfc3339();
        let snapshot = AccountSnapshot {
            status,
            launch: AccountLaunchDiagnostics {
                selected_executable: "/usr/bin/codex".to_string(),
                argv_prefix: Vec::new(),
                mode: CodexLaunchMode::ListenStdioNoMcp,
                candidates: vec!["/usr/bin/codex".to_string()],
            },
            diagnostics: AccountRefreshDiagnostics {
                started_at_utc: now.clone(),
                completed_at_utc: now,
                first_failing_stage: if status == AccountRefreshStatus::Connected {
                    None
                } else {
                    Some("account/usage/read".to_string())
                },
                redacted_error_code: if status == AccountRefreshStatus::Connected {
                    None
                } else {
                    Some("account_method_failed".to_string())
                },
                redacted_error_message: String::new(),
                stderr_tail: String::new(),
                used_last_good_snapshot: false,
                schema_fingerprint: crate::codex_app_server::APP_SERVER_SCHEMA_FINGERPRINT
                    .to_string(),
                exit_code: None,
                child_terminated: true,
            },
            account: AccountIdentitySnapshot {
                account_label: Some("t***@example.*".to_string()),
                plan: Some("Pro".to_string()),
            },
            usage: AccountUsageSnapshot {
                lifetime_tokens,
                daily_buckets: vec![AccountDailyUsageBucket {
                    date: Utc::now().format("%Y-%m-%d").to_string(),
                    input_tokens: 10,
                    output_tokens: 20,
                    total_tokens: 30,
                }],
                daily_buckets_status: crate::codex_app_server::OptionalCollectionStatus::Present,
            },
            reset_credits: AccountResetCreditsSnapshot {
                available_count: reset_credits,
                expires_at_utc: Some("2026-07-28T18:14:00Z".to_string()),
                credits: None,
            },
            rate_limits: rate_used_percent
                .map(|used_percent| {
                    vec![AccountRateLimitBucket {
                        bucket_id: "codex".to_string(),
                        display_name: "Codex".to_string(),
                        windows: vec![AccountRateLimitWindow {
                            window_duration_mins: Some(300),
                            window_label: "5-hour".to_string(),
                            used_percent,
                            remaining_percent: 100.0 - used_percent,
                            resets_at_utc: Some("2026-07-07T05:00:00Z".to_string()),
                        }],
                    }]
                })
                .unwrap_or_default(),
            methods: vec![
                AccountMethodSnapshot {
                    method: "account/read".to_string(),
                    status: MethodStatus::Ok,
                    redacted_error: None,
                },
                AccountMethodSnapshot {
                    method: "account/usage/read".to_string(),
                    status: if lifetime_tokens.is_some() {
                        MethodStatus::Ok
                    } else {
                        MethodStatus::Failed
                    },
                    redacted_error: lifetime_tokens
                        .is_none()
                        .then(|| "usage unavailable".to_string()),
                },
                AccountMethodSnapshot {
                    method: "account/rateLimits/read".to_string(),
                    status: if reset_credits.is_some() || rate_used_percent.is_some() {
                        MethodStatus::Ok
                    } else {
                        MethodStatus::Failed
                    },
                    redacted_error: (reset_credits.is_none() && rate_used_percent.is_none())
                        .then(|| "rate limits unavailable".to_string()),
                },
            ],
        };
        crate::db::insert_account_snapshot(conn, &snapshot).unwrap();
    }

    #[test]
    fn computes_lifetime_tokens() {
        let conn = seeded_conn();
        let summary = build_dashboard_summary(&conn, "local").unwrap();
        assert_eq!(summary.metrics[0].value, "175");
    }

    #[test]
    fn heatmap_uses_the_latest_contiguous_calendar_window_with_real_weekdays() {
        let conn = open_memory().unwrap();
        let doc_id =
            upsert_source_document(&conn, "local", "heatmap", "history.jsonl", "content", 1)
                .unwrap();
        for (id, occurred, total) in [
            ("outside", "2025-12-01T12:00:00Z", 999),
            ("inside", "2026-07-17T12:00:00Z", 125),
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

        let days = heatmap_at(&conn, Utc.with_ymd_and_hms(2026, 7, 18, 16, 0, 0).unwrap()).unwrap();

        assert_eq!(days.len(), 112);
        assert_eq!(days.first().unwrap().date, "2026-03-29");
        assert_eq!(days.last().unwrap().date, "2026-07-18");
        assert_eq!(days.first().unwrap().weekday, "Sun");
        assert_eq!(
            days.iter()
                .find(|day| day.date == "2026-07-17")
                .unwrap()
                .tokens,
            125
        );
        assert_eq!(
            days.iter()
                .map(|day| &day.date)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            112
        );
        assert!(!days.iter().any(|day| day.tokens == 999));
    }

    #[test]
    fn reset_credit_detail_expiration_is_used_when_aggregate_expiration_is_missing() {
        let conn = open_memory().unwrap();
        insert_account_fixture(
            &conn,
            Some(100),
            Some(2),
            None,
            AccountRefreshStatus::Connected,
        );
        conn.execute(
            "UPDATE account_reset_credit_snapshots SET expires_at_utc = NULL",
            [],
        )
        .unwrap();
        let snapshot_id: i64 = conn
            .query_row("SELECT id FROM account_reset_credit_snapshots", [], |row| {
                row.get(0)
            })
            .unwrap();
        conn.execute(
            "INSERT INTO account_reset_credit_details (reset_credit_snapshot_id, credit_id, reset_type, status, granted_at_utc, expires_at_utc) VALUES (?1, 'detail-1', 'manual', 'available', '2026-07-01T00:00:00Z', '2036-07-28T18:14:00Z')",
            [snapshot_id],
        )
        .unwrap();

        let summary = build_dashboard_summary(&conn, "remote").unwrap();

        assert_eq!(
            summary.reset_credits[0].expires_at_utc,
            "2036-07-28T18:14:00+00:00"
        );
        assert_ne!(summary.next_reset.expires_at_ny, "Unavailable");
    }

    #[test]
    fn zero_reset_credits_do_not_claim_a_zero_day_expiration() {
        let conn = open_memory().unwrap();
        insert_account_fixture(
            &conn,
            Some(100),
            Some(0),
            None,
            AccountRefreshStatus::Connected,
        );

        let summary = build_dashboard_summary(&conn, "remote").unwrap();

        assert_eq!(summary.next_reset.label, "No reset credits available");
        assert_eq!(summary.next_reset.expires_at_ny, "Unavailable");
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
        assert!(summary.local_metrics.is_empty());
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
                warning_count: 0,
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

        assert_eq!(reset_metric.value, "Unavailable");
        assert_eq!(reset_metric.coverage.coverage_percent, 0);
        assert_eq!(reset_metric.coverage.confidence, "unavailable");
        assert!(summary.reset_credits.is_empty());
        assert!(summary.rate_limit_windows.is_empty());
    }

    #[test]
    fn combined_mode_falls_back_to_local_usage_when_account_snapshot_is_missing() {
        let conn = seeded_conn();
        crate::db::insert_account_refresh_error(
            &conn,
            &AccountConnectorError {
                kind: AccountConnectorErrorKind::MissingCli,
                stage: "resolve_codex".to_string(),
                public_message: "Codex CLI was not found".to_string(),
                exit_code: None,
                timed_out: false,
                child_terminated: false,
                launch: AccountLaunchDiagnostics::validation(String::new(), Vec::new()),
                failure_class: crate::codex_app_server::AccountConnectorFailureClass::Transport,
            },
        )
        .unwrap();

        let summary = build_dashboard_summary(&conn, "combined").unwrap();
        let lifetime = summary
            .metrics
            .iter()
            .find(|metric| metric.key == "lifetime")
            .unwrap();

        assert_eq!(lifetime.value, "175");
        assert_eq!(lifetime.label, "Local history tokens");
        assert!(lifetime.delta.contains("Account unavailable"));
        assert_ne!(lifetime.status, "warning");
        assert!(summary.account_metrics.is_empty());
        assert_eq!(summary.local_metrics[0].value, "175");
        assert_eq!(summary.local_metrics[0].label, "Local lifetime");
    }

    #[test]
    fn combined_mode_keeps_account_and_local_usage_as_separate_metric_families() {
        let conn = seeded_conn();
        insert_account_fixture(
            &conn,
            Some(999),
            Some(0),
            Some(25.0),
            AccountRefreshStatus::Connected,
        );

        let summary = build_dashboard_summary(&conn, "combined").unwrap();

        assert_eq!(summary.account_metrics.len(), 3);
        assert_eq!(summary.local_metrics.len(), 4);
        assert_eq!(summary.account_metrics[0].label, "Account lifetime");
        assert_eq!(summary.account_metrics[0].value, "999");
        assert_eq!(summary.local_metrics[0].label, "Local lifetime");
        assert_eq!(summary.local_metrics[0].value, "175");
        assert_eq!(summary.local_metrics[3].label, "Local peak session");
        assert_eq!(
            summary
                .metrics
                .iter()
                .find(|metric| metric.key == "reset")
                .unwrap()
                .value,
            "0"
        );
    }

    #[test]
    fn coverage_summary_uses_latest_row_per_metric() {
        let conn = open_memory().unwrap();
        crate::db::record_source_coverage(
            &conn,
            "local-usage",
            "Local history",
            25,
            "low",
            &["old parse result".to_string()],
            "Older local history coverage.",
        )
        .unwrap();
        crate::db::record_source_coverage(
            &conn,
            "local-usage",
            "Local history",
            72,
            "medium",
            &["all event shapes parseable".to_string()],
            "Latest local history coverage.",
        )
        .unwrap();

        let summary = build_dashboard_summary(&conn, "combined").unwrap();
        let local_rows = summary
            .coverage
            .iter()
            .filter(|coverage| coverage.metric_key == "local-usage")
            .collect::<Vec<_>>();

        assert_eq!(local_rows.len(), 1);
        assert_eq!(local_rows[0].coverage_percent, 72);
        assert_eq!(local_rows[0].explanation, "Latest local history coverage.");
    }

    #[test]
    fn failed_remote_connectors_show_sanitized_error_details() {
        let conn = open_memory().unwrap();
        crate::db::insert_account_refresh_error(
            &conn,
            &AccountConnectorError {
                kind: AccountConnectorErrorKind::MissingCli,
                stage: "resolve_codex".to_string(),
                public_message: "Codex CLI was not found".to_string(),
                exit_code: None,
                timed_out: false,
                child_terminated: false,
                launch: AccountLaunchDiagnostics::validation(String::new(), Vec::new()),
                failure_class: crate::codex_app_server::AccountConnectorFailureClass::Transport,
            },
        )
        .unwrap();

        let summary = build_dashboard_summary(&conn, "combined").unwrap();
        let connector = summary
            .connectors
            .iter()
            .find(|connector| connector.id == "account-usage")
            .unwrap();

        assert_eq!(connector.status, "degraded");
        assert!(connector.detail.contains("Codex CLI not found"));
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
        insert_account_fixture(
            &conn,
            Some(999),
            Some(4),
            Some(25.0),
            AccountRefreshStatus::Connected,
        );

        let local = build_dashboard_summary(&conn, "local").unwrap();
        let remote = build_dashboard_summary(&conn, "remote").unwrap();
        let combined = build_dashboard_summary(&conn, "combined").unwrap();

        assert_eq!(local.metrics[0].value, "175");
        assert_eq!(local.metrics[4].value, "0");
        assert!(local.reset_credits.is_empty());
        assert_eq!(remote.metrics[0].value, "999");
        assert_eq!(remote.metrics[4].value, "4");
        assert!(remote.sessions.is_empty());
        assert_eq!(combined.metrics[0].value, "999");
        assert_eq!(combined.metrics[4].value, "4");
    }

    #[test]
    fn repeated_remote_refreshes_use_latest_successful_snapshot_only() {
        let conn = open_memory().unwrap();
        for _ in 0..2 {
            insert_account_fixture(
                &conn,
                Some(100),
                Some(4),
                Some(25.0),
                AccountRefreshStatus::Connected,
            );
        }
        crate::db::insert_account_refresh_error(
            &conn,
            &AccountConnectorError {
                kind: AccountConnectorErrorKind::Timeout,
                stage: "initialize".to_string(),
                public_message: "initialize timed out".to_string(),
                exit_code: None,
                timed_out: true,
                child_terminated: true,
                launch: AccountLaunchDiagnostics::validation(String::new(), Vec::new()),
                failure_class: crate::codex_app_server::AccountConnectorFailureClass::Transport,
            },
        )
        .unwrap();

        let summary = build_dashboard_summary(&conn, "remote").unwrap();
        let reset_metric = summary
            .metrics
            .iter()
            .find(|metric| metric.key == "reset")
            .unwrap();
        let rate_limit_coverage = summary
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
        assert_eq!(summary.rate_limit_windows[0].remaining, "75%");
        assert_eq!(rate_limit_coverage.coverage_percent, 35);
        assert_eq!(rate_limit_coverage.confidence, "low");
    }

    #[test]
    fn partial_failure_degrades_only_failed_facet_and_recovery_clears_stale_state() {
        let conn = open_memory().unwrap();
        insert_account_fixture(
            &conn,
            Some(100),
            Some(4),
            Some(25.0),
            AccountRefreshStatus::Connected,
        );
        insert_account_fixture(&conn, Some(120), None, None, AccountRefreshStatus::Degraded);

        let partial = build_dashboard_summary(&conn, "remote").unwrap();
        let usage = partial
            .connectors
            .iter()
            .find(|item| item.id == "account-usage")
            .unwrap();
        let credits = partial
            .connectors
            .iter()
            .find(|item| item.id == "known-reset-credit")
            .unwrap();
        assert_eq!(usage.status, "connected");
        assert_eq!(usage.freshness, "fresh");
        assert_eq!(usage.detail, "Latest account usage snapshot checked");
        assert_eq!(credits.status, "degraded");
        assert_eq!(credits.freshness, "stale");
        assert!(credits.detail.contains("rate limits unavailable"));
        assert_ne!(usage.last_run_at_utc, "");
        assert_ne!(credits.last_run_at_utc, "");
        assert!(credits.age_seconds.is_some());
        assert_eq!(
            partial
                .metrics
                .iter()
                .find(|item| item.key == "reset")
                .unwrap()
                .value,
            "4"
        );

        insert_account_fixture(
            &conn,
            Some(130),
            Some(0),
            Some(5.0),
            AccountRefreshStatus::Connected,
        );
        let recovered = build_dashboard_summary(&conn, "remote").unwrap();
        let credits = recovered
            .connectors
            .iter()
            .find(|item| item.id == "known-reset-credit")
            .unwrap();
        assert_eq!(credits.status, "connected");
        assert_eq!(credits.freshness, "fresh");
        assert_eq!(
            recovered
                .metrics
                .iter()
                .find(|item| item.key == "reset")
                .unwrap()
                .value,
            "0"
        );
    }

    #[test]
    fn unrelated_newer_account_attempt_does_not_stale_usage_facet() {
        let conn = open_memory().unwrap();
        insert_account_fixture(
            &conn,
            Some(100),
            Some(4),
            Some(25.0),
            AccountRefreshStatus::Connected,
        );
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO account_refresh_runs (started_at_utc, completed_at_utc, status) VALUES (?1, ?1, 'degraded')",
            [&now],
        )
        .unwrap();
        let run_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO account_method_attempts (refresh_run_id, method, status, captured_at_utc) VALUES (?1, 'account/read', 'failed', ?2)",
            rusqlite::params![run_id, now],
        )
        .unwrap();

        let summary = build_dashboard_summary(&conn, "remote").unwrap();
        let usage = summary
            .connectors
            .iter()
            .find(|item| item.id == "account-usage")
            .unwrap();
        assert_eq!(usage.freshness, "fresh");
        assert_eq!(usage.status, "connected");
    }

    #[test]
    fn historical_snapshot_without_method_attempt_remains_connected_with_legacy_provenance() {
        let conn = open_memory().unwrap();
        insert_account_fixture(
            &conn,
            Some(100),
            Some(4),
            Some(25.0),
            AccountRefreshStatus::Connected,
        );
        let captured_at: String = conn
            .query_row(
                "SELECT completed_at_utc FROM account_refresh_runs ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        conn.execute("DELETE FROM account_method_attempts", [])
            .unwrap();

        let summary = build_dashboard_summary(&conn, "remote").unwrap();
        let usage = summary
            .connectors
            .iter()
            .find(|item| item.id == "account-usage")
            .unwrap();
        assert_eq!(usage.status, "connected");
        assert_eq!(usage.freshness, "fresh");
        assert_eq!(usage.last_run_at_utc, captured_at);
        assert!(usage.detail.contains("legacy refresh metadata"));
        assert!(usage.age_seconds.is_some());
    }
}
