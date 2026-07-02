import type { DashboardSummary } from "../../lib/schemas/dashboard";

export function escapeCsvField(value: unknown): string {
  const field = String(value ?? "");
  if (!/[",\n\r]/.test(field)) {
    return field;
  }
  return `"${field.replaceAll('"', '""')}"`;
}

export function buildDashboardUsageCsv(summary: DashboardSummary, generatedAt = new Date()): string {
  const sections = [
    buildSection(
      "metadata",
      ["generated_at_utc", "data_mode", "refresh_status", "timezone", "last_refresh_label"],
      [[generatedAt.toISOString(), summary.dataMode, summary.refreshStatus, summary.timezone, summary.lastRefreshLabel]],
    ),
    buildSection(
      "metrics",
      ["metric_key", "label", "value", "delta", "status", "coverage_percent", "confidence", "source_kind"],
      summary.metrics.map((metric) => [
        metric.key,
        metric.label,
        metric.value,
        metric.delta,
        metric.status,
        metric.coverage.coveragePercent,
        metric.coverage.confidence,
        metric.coverage.sourceKind,
      ]),
    ),
    buildSection(
      "daily_usage",
      ["date", "weekday", "tokens", "intensity"],
      summary.heatmap.map((day) => [day.date, day.weekday, day.tokens, day.intensity]),
    ),
    buildSection(
      "reset_credits",
      ["credit_count", "expires_at_utc", "expires_at_new_york", "days_remaining", "confidence"],
      summary.resetCredits.map((credit) => [credit.creditCount, credit.expiresAtUtc, credit.expiresAtNy, credit.daysRemaining, credit.confidence]),
    ),
    buildSection(
      "recent_sessions",
      ["start_time", "duration", "tokens", "peak_tokens", "mode", "sources"],
      summary.sessions.map((session) => [session.startTime, session.duration, session.tokens, session.peakTokens, session.mode, session.sources.join("; ")]),
    ),
    buildSection(
      "rate_limit_windows",
      ["window", "limit", "used", "remaining", "reset_countdown", "progress_percent"],
      summary.rateLimitWindows.map((window) => [window.window, window.limit, window.used, window.remaining, window.resetsIn, window.progressPercent]),
    ),
    buildSection(
      "source_coverage",
      ["metric_key", "source_kind", "coverage_percent", "confidence", "last_evidence_at_utc", "formula_version", "missing_facets", "explanation"],
      summary.coverage.map((coverage) => [
        coverage.metricKey,
        coverage.sourceKind,
        coverage.coveragePercent,
        coverage.confidence,
        coverage.lastEvidenceAtUtc,
        coverage.formulaVersion,
        coverage.missingFacets.join("; "),
        coverage.explanation,
      ]),
    ),
  ];

  return sections.join("\n\n");
}

export function buildUsageCsvFilename(generatedAt = new Date()): string {
  return `tokenstack-usage-${formatDate(generatedAt)}.csv`;
}

function buildSection(name: string, headers: string[], rows: unknown[][]): string {
  return [name, headers.join(","), ...rows.map((row) => row.map(escapeCsvField).join(","))].join("\n");
}

function formatDate(date: Date): string {
  return date.toISOString().slice(0, 10);
}
