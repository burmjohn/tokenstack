import { describe, expect, it } from "vitest";
import { createMockDashboardSummary } from "../../lib/api/mockData";
import type { DashboardSummary } from "../../lib/schemas/dashboard";
import { buildDashboardUsageCsv, buildUsageCsvFilename, escapeCsvField } from "./csv";

describe("export CSV helpers", () => {
  it("escapes commas, quotes, newlines, and carriage returns", () => {
    expect(escapeCsvField("plain")).toBe("plain");
    expect(escapeCsvField("alpha,beta")).toBe('"alpha,beta"');
    expect(escapeCsvField('alpha "beta"')).toBe('"alpha ""beta"""');
    expect(escapeCsvField("alpha\nbeta")).toBe('"alpha\nbeta"');
    expect(escapeCsvField("alpha\rbeta")).toBe('"alpha\rbeta"');
  });

  it("writes sections in the committed design order", () => {
    const csv = buildDashboardUsageCsv(createMockDashboardSummary(), new Date("2026-07-02T19:30:00Z"));
    const sections = csv.split(/\n\n/).map((section) => section.split("\n")[0]);

    expect(sections).toEqual([
      "metadata",
      "metrics",
      "daily_usage",
      "reset_credits",
      "recent_sessions",
      "rate_limit_windows",
      "source_coverage",
    ]);
  });

  it("includes representative dashboard rows", () => {
    const csv = buildDashboardUsageCsv(createExportFixtureSummary(), new Date("2026-07-02T19:30:00Z"));

    expect(csv).toContain("generated_at_utc,data_mode,refresh_status,timezone,last_refresh_label");
    expect(csv).toContain("2026-07-02T19:30:00.000Z,combined,idle,America/New_York,2m ago");
    expect(csv).toContain("source_family,metric_key,label,value,delta,status,coverage_percent,confidence,source_kind");
    expect(csv).toContain("account,account-lifetime,Account lifetime,99B");
    expect(csv).toContain("local_history,local-lifetime,Local lifetime,38.1B");
    expect(csv).toContain("date,weekday,tokens,intensity");
    expect(csv).toContain("credit_count,expires_at_utc,expires_at_new_york,days_remaining,confidence");
    expect(csv).toContain("start_time,duration,tokens,peak_tokens,mode,sources");
    expect(csv).toContain('"Jun 14, 1:12 PM",47m 23s,512.3M,1.72B,deep-research,CLI; Cloud');
    expect(csv).toContain("window,limit,used,remaining,reset_countdown,progress_percent");
    expect(csv).toContain("1m,20.0B,11.2B,8.8B (44%),00:14,56");
    expect(csv).toContain("metric_key,source_kind,coverage_percent,confidence,last_evidence_at_utc,formula_version,missing_facets,explanation");
  });

  it("builds the required dated filename", () => {
    expect(buildUsageCsvFilename(new Date("2026-07-02T19:30:00Z"))).toBe("tokenstack-usage-2026-07-02.csv");
  });
});

function createExportFixtureSummary(): DashboardSummary {
  const summary = createMockDashboardSummary("combined");
  const coverage = [
    {
      ...summary.coverage[0],
      coveragePercent: 72,
      confidence: "medium" as const,
      lastEvidenceAtUtc: "2026-07-02T19:30:00Z",
      missingFacets: ["some archived sessions"],
      explanation: "Local usage events are present and deduplicated; unknown shapes lower confidence.",
    },
    {
      ...summary.coverage[1],
      coveragePercent: 100,
      confidence: "high" as const,
      lastEvidenceAtUtc: "2026-07-02T19:30:00Z",
      missingFacets: [],
      explanation: "Reset-credit snapshots are stored, schema-valid, and freshness checked.",
    },
    {
      ...summary.coverage[2],
      coveragePercent: 68,
      confidence: "medium" as const,
      lastEvidenceAtUtc: "2026-07-02T19:30:00Z",
      missingFacets: ["additional source confirmation"],
      explanation: "Rate-limit windows are stored with conservative confidence until more evidence is available.",
    },
  ];

  return {
    ...summary,
    lastRefreshLabel: "2m ago",
    metrics: [
      {
        key: "lifetime",
        label: "Lifetime tokens",
        value: "38.1B",
        delta: "12.4% vs last 30 days",
        status: "positive",
        coverage: coverage[0],
      },
      {
        key: "today",
        label: "Today",
        value: "128.7M",
        delta: "8.6% vs yesterday",
        status: "positive",
        coverage: coverage[0],
      },
      {
        key: "month",
        label: "This month",
        value: "3.62B",
        delta: "18.3% vs last month",
        status: "positive",
        coverage: coverage[0],
      },
      {
        key: "peak",
        label: "Peak session",
        value: "1.72B",
        delta: "Jun 4, 1:44 PM",
        status: "neutral",
        coverage: coverage[0],
      },
      {
        key: "reset",
        label: "Reset credits",
        value: "4",
        delta: "Available",
        status: "positive",
        coverage: coverage[1],
      },
    ],
    accountMetrics: [{ ...summary.metrics[0], key: "account-lifetime", label: "Account lifetime", value: "99B", coverage: coverage[0] }],
    localMetrics: [
      { ...summary.metrics[0], key: "local-lifetime", label: "Local lifetime", value: "38.1B", coverage: coverage[0] },
      { ...summary.metrics[1], key: "local-today", label: "Local today", value: "128.7M", coverage: coverage[0] },
      { ...summary.metrics[2], key: "local-month", label: "Local this month", value: "3.62B", coverage: coverage[0] },
      { ...summary.metrics[3], key: "local-peak", label: "Local peak session", value: "1.72B", coverage: coverage[0] },
    ],
    resetCredits: [
      {
        id: "reset-1",
        creditCount: 4,
        expiresAtUtc: "2026-07-28T18:14:00Z",
        expiresAtNy: "Jul 28, 2026, 2:14 PM EDT",
        daysRemaining: 22,
        confidence: "high",
      },
    ],
    sessions: [
      {
        id: "s1",
        startTime: "Jun 14, 1:12 PM",
        duration: "47m 23s",
        tokens: "512.3M",
        peakTokens: "1.72B",
        mode: "deep-research",
        sources: ["CLI", "Cloud"],
      },
    ],
    rateLimitWindows: [
      { id: "1m", window: "1m", limit: "20.0B", used: "11.2B", remaining: "8.8B (44%)", resetsIn: "00:14", progressPercent: 56 },
    ],
    coverage,
  };
}
