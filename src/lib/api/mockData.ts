import type { DashboardSummary, DataMode, SetupDiagnostics } from "../schemas/dashboard";

const now = "2026-07-02T19:30:00Z";

const coverage = [
  {
    metricKey: "local-history",
    sourceKind: "Local history",
    coveragePercent: 0,
    confidence: "unavailable" as const,
    lastEvidenceAtUtc: now,
    formulaVersion: "coverage-v1",
    missingFacets: ["local usage events"],
    explanation: "No local usage events have been imported yet.",
  },
  {
    metricKey: "reset-credits",
    sourceKind: "Reset credits",
    coveragePercent: 0,
    confidence: "unavailable" as const,
    lastEvidenceAtUtc: now,
    formulaVersion: "coverage-v1",
    missingFacets: ["reset-credit snapshot"],
    explanation: "No reset-credit snapshot is stored yet.",
  },
  {
    metricKey: "rate-limit-windows",
    sourceKind: "Rate-limit windows",
    coveragePercent: 0,
    confidence: "unavailable" as const,
    lastEvidenceAtUtc: now,
    formulaVersion: "coverage-v1",
    missingFacets: ["rate-limit window snapshot"],
    explanation: "No rate-limit window snapshot is stored yet.",
  },
];

export function createMockDashboardSummary(dataMode: DataMode = "combined"): DashboardSummary {
  const heatmap = Array.from({ length: 112 }, (_, index) => {
    const monthBand = Math.floor(index / 16);
    const day = (index % 28) + 1;

    return {
      date: `2026-${String(Math.min(7, monthBand + 1)).padStart(2, "0")}-${String(day).padStart(2, "0")}`,
      weekday: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][index % 7],
      tokens: 0,
      intensity: 0,
    };
  });

  return {
    generatedAtUtc: now,
    dataMode,
    lastRefreshLabel: "not yet",
    refreshStatus: "idle",
    timezone: "America/New_York",
    metrics: [
      {
        key: "lifetime",
        label: "Lifetime tokens",
        value: "0",
        delta: "No imported history",
        status: "neutral",
        coverage: coverage[0],
      },
      {
        key: "today",
        label: "Today",
        value: "0",
        delta: "America/New_York bucket",
        status: "neutral",
        coverage: coverage[0],
      },
      {
        key: "month",
        label: "This month",
        value: "0",
        delta: "Month-to-date",
        status: "neutral",
        coverage: coverage[0],
      },
      {
        key: "peak",
        label: "Peak session",
        value: "0",
        delta: "No imported events",
        status: "neutral",
        coverage: coverage[0],
      },
      {
        key: "reset",
        label: "Reset credits",
        value: "0",
        delta: "No snapshot",
        status: "neutral",
        coverage: coverage[1],
      },
    ],
    heatmap,
    resetCredits: [],
    coverage,
    connectors: [
      {
        id: "local",
        name: "Local Codex history",
        detail: "No local history imported yet",
        status: "unavailable",
        readOnly: true,
        safetyClass: "Local",
        lastRunAtUtc: "",
      },
      {
        id: "known-reset-credit",
        name: "Reset credits",
        detail: "No reset-credit snapshot yet",
        status: "unavailable",
        readOnly: true,
        safetyClass: "Snapshot",
        lastRunAtUtc: "",
      },
      {
        id: "rate-limit-windows",
        name: "Rate-limit windows",
        detail: "No rate-limit window snapshot yet",
        status: "unavailable",
        readOnly: true,
        safetyClass: "Snapshot",
        lastRunAtUtc: "",
      },
    ],
    sessions: [],
    rateLimitWindows: [],
    nextReset: {
      label: "No reset-credit snapshot",
      expiresAtNy: "",
      timezone: "America/New_York",
    },
  };
}

export function createMockSetupDiagnostics(): SetupDiagnostics {
  return {
    appDataDir: "~/.local/share/tokenstack",
    databasePath: "~/.local/share/tokenstack/tokenstack.sqlite3",
    authHome: "~",
    usageEventCount: 0,
    usageTotalTokens: 0,
    sourceDocumentCount: 0,
    localRoots: [
      {
        path: "~/.codex/sessions",
        exists: false,
        isDirectory: false,
      },
      {
        path: "~/.codex/history",
        exists: false,
        isDirectory: false,
      },
      {
        path: "~/.codex/archive",
        exists: false,
        isDirectory: false,
      },
      {
        path: "~/.codex/archived_sessions",
        exists: false,
        isDirectory: false,
      },
    ],
    latestImportRun: null,
    connectorRuns: [],
  };
}
