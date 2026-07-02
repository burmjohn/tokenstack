import type { DashboardSummary, DataMode } from "../schemas/dashboard";

const now = "2026-07-02T19:30:00Z";

const coverage = [
  {
    metricKey: "local-history",
    sourceKind: "Local history",
    coveragePercent: 72,
    confidence: "medium" as const,
    lastEvidenceAtUtc: now,
    formulaVersion: "coverage-v1",
    missingFacets: ["some archived sessions"],
    explanation: "Local JSONL token events are present and deduplicated; unknown shapes lower confidence.",
  },
  {
    metricKey: "rate-limits",
    sourceKind: "Rate limits",
    coveragePercent: 92,
    confidence: "high" as const,
    lastEvidenceAtUtc: now,
    formulaVersion: "coverage-v1",
    missingFacets: ["one stale window"],
    explanation: "Read-only rate-limit windows are schema-valid with one stale freshness facet.",
  },
  {
    metricKey: "reset-credits",
    sourceKind: "Reset credits",
    coveragePercent: 100,
    confidence: "high" as const,
    lastEvidenceAtUtc: now,
    formulaVersion: "coverage-v1",
    missingFacets: [],
    explanation: "Reset-credit count and expiration timestamps are schema-valid and fresh.",
  },
  {
    metricKey: "undocumented",
    sourceKind: "Undocumented (RO)",
    coveragePercent: 68,
    confidence: "medium" as const,
    lastEvidenceAtUtc: now,
    formulaVersion: "coverage-v1",
    missingFacets: ["documented source contract"],
    explanation: "Endpoint is registered as read-only with a response schema, but confidence remains conservative.",
  },
];

export function createMockDashboardSummary(dataMode: DataMode = "combined"): DashboardSummary {
  const heatmap = Array.from({ length: 112 }, (_, index) => {
    const monthBand = Math.floor(index / 16);
    const day = (index % 28) + 1;
    const tokens = index < 58 ? 0 : 24_000_000 + ((index * 17) % 86) * 1_000_000;
    return {
      date: `2026-${String(Math.min(7, monthBand + 1)).padStart(2, "0")}-${String(day).padStart(2, "0")}`,
      weekday: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][index % 7],
      tokens,
      intensity: tokens === 0 ? 0 : Math.min(5, Math.max(1, Math.round(tokens / 22_000_000))),
    };
  });

  return {
    generatedAtUtc: now,
    dataMode,
    lastRefreshLabel: "2m ago",
    refreshStatus: "idle",
    timezone: "America/New_York",
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
        coverage: coverage[2],
      },
    ],
    heatmap,
    resetCredits: [
      {
        id: "reset-1",
        creditCount: 4,
        expiresAtUtc: "2026-07-28T18:14:00Z",
        expiresAtNy: "Jul 28, 2026, 2:14 PM EDT",
        daysRemaining: 22,
        confidence: "high",
      },
      {
        id: "reset-2",
        creditCount: 3,
        expiresAtUtc: "2026-08-25T18:14:00Z",
        expiresAtNy: "Aug 25, 2026, 2:14 PM EDT",
        daysRemaining: 50,
        confidence: "high",
      },
      {
        id: "reset-3",
        creditCount: 2,
        expiresAtUtc: "2026-09-22T18:14:00Z",
        expiresAtNy: "Sep 22, 2026, 2:14 PM EDT",
        daysRemaining: 78,
        confidence: "high",
      },
      {
        id: "reset-4",
        creditCount: 1,
        expiresAtUtc: "2026-10-20T18:14:00Z",
        expiresAtNy: "Oct 20, 2026, 2:14 PM EDT",
        daysRemaining: 106,
        confidence: "high",
      },
    ],
    coverage,
    connectors: [
      {
        id: "local",
        name: "Local Codex CLI",
        detail: "~/.codex/history/*.jsonl",
        status: "connected",
        readOnly: true,
        safetyClass: "Read-only",
        lastRunAtUtc: now,
      },
      {
        id: "known-reset-credit",
        name: "Known read-only endpoint",
        detail: "/wham/rate-limit-reset-credits",
        status: "connected",
        readOnly: true,
        safetyClass: "Read-only",
        lastRunAtUtc: now,
      },
      {
        id: "undocumented-ro",
        name: "Undocumented (RO)",
        detail: "registered schema-gated endpoint",
        status: "connected",
        readOnly: true,
        safetyClass: "Read-only",
        lastRunAtUtc: now,
      },
    ],
    sessions: [
      { id: "s1", startTime: "Jun 14, 1:12 PM", duration: "47m 23s", tokens: "512.3M", peakTokens: "1.72B", mode: "deep-research", sources: ["CLI", "Cloud"] },
      { id: "s2", startTime: "Jun 14, 10:01 AM", duration: "32m 11s", tokens: "286.4M", peakTokens: "1.04B", mode: "code-review", sources: ["CLI", "Cloud"] },
      { id: "s3", startTime: "Jun 13, 7:43 PM", duration: "1h 05m", tokens: "1.12B", peakTokens: "1.48B", mode: "architect", sources: ["CLI"] },
      { id: "s4", startTime: "Jun 13, 3:28 PM", duration: "24m 17s", tokens: "198.7M", peakTokens: "742.1M", mode: "executor", sources: ["CLI", "Cloud"] },
      { id: "s5", startTime: "Jun 13, 11:02 AM", duration: "19m 05s", tokens: "143.9M", peakTokens: "512.3M", mode: "explore", sources: ["CLI"] },
    ],
    rateLimitWindows: [
      { id: "1m", window: "1m", limit: "20.0B", used: "11.2B", remaining: "8.8B (44%)", resetsIn: "00:14", progressPercent: 56 },
      { id: "1h", window: "1h", limit: "500.0B", used: "231.4B", remaining: "268.6B (54%)", resetsIn: "14:22", progressPercent: 46 },
      { id: "24h", window: "24h", limit: "2.00T", used: "1.12T", remaining: "880.0B (44%)", resetsIn: "06:14:22", progressPercent: 56 },
      { id: "30d", window: "30d", limit: "20.00T", used: "3.62T", remaining: "16.38T (82%)", resetsIn: "16d 03h", progressPercent: 18 },
    ],
    nextReset: {
      label: "22d 03h 14m",
      expiresAtNy: "Jul 28, 2026, 2:14 PM EDT",
      timezone: "America/New_York",
    },
  };
}
