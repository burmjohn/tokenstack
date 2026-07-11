import { z } from "zod";

export const dataModeSchema = z.enum(["local", "remote", "combined"]);
export type DataMode = z.infer<typeof dataModeSchema>;

export const coverageSchema = z.object({
  metricKey: z.string(),
  sourceKind: z.string(),
  coveragePercent: z.number().min(0).max(100),
  confidence: z.enum(["high", "medium", "low", "unavailable"]),
  lastEvidenceAtUtc: z.string(),
  formulaVersion: z.string(),
  missingFacets: z.array(z.string()),
  explanation: z.string(),
});

export const metricSchema = z.object({
  key: z.string(),
  label: z.string(),
  value: z.string(),
  delta: z.string(),
  status: z.enum(["positive", "neutral", "warning"]),
  coverage: coverageSchema,
});

export const heatmapDaySchema = z.object({
  date: z.string(),
  weekday: z.string(),
  tokens: z.number(),
  intensity: z.number().min(0).max(5),
});

export const resetCreditSchema = z.object({
  id: z.string(),
  creditCount: z.number(),
  expiresAtUtc: z.string(),
  expiresAtNy: z.string(),
  daysRemaining: z.number(),
  confidence: z.enum(["high", "medium", "low", "unavailable"]),
});

export const connectorSchema = z.object({
  id: z.string(),
  name: z.string(),
  detail: z.string(),
  status: z.enum(["connected", "degraded", "unavailable"]),
  readOnly: z.boolean(),
  safetyClass: z.string(),
  lastRunAtUtc: z.string(),
  freshness: z.enum(["fresh", "stale", "unavailable"]).default("unavailable"),
  ageSeconds: z.number().int().nonnegative().nullable().default(null),
});

export const sessionSchema = z.object({
  id: z.string(),
  startTime: z.string(),
  duration: z.string(),
  tokens: z.string(),
  peakTokens: z.string(),
  mode: z.string(),
  sources: z.array(z.string()),
});

export const rateLimitWindowSchema = z.object({
  id: z.string(),
  window: z.string(),
  limit: z.string(),
  used: z.string(),
  remaining: z.string(),
  resetsIn: z.string(),
  progressPercent: z.number().min(0).max(100),
});

const uniqueBy = <T>(items: T[], key: (item: T) => string) => new Set(items.map(key)).size === items.length;

export const dashboardSummarySchema = z.object({
  generatedAtUtc: z.string(),
  dataMode: dataModeSchema,
  lastRefreshLabel: z.string(),
  refreshStatus: z.enum(["idle", "pending", "stale", "degraded", "failed"]),
  timezone: z.literal("America/New_York"),
  metrics: z.array(metricSchema),
  accountMetrics: z.array(metricSchema).default([]),
  localMetrics: z.array(metricSchema).default([]),
  heatmap: z.array(heatmapDaySchema),
  resetCredits: z.array(resetCreditSchema),
  coverage: z.array(coverageSchema),
  connectors: z.array(connectorSchema),
  sessions: z.array(sessionSchema),
  rateLimitWindows: z.array(rateLimitWindowSchema),
  nextReset: z.object({
    label: z.string(),
    expiresAtNy: z.string(),
    timezone: z.literal("America/New_York"),
  }),
}).superRefine((summary, context) => {
  const keyedCollections = [
    ["metrics", summary.metrics, (item: z.infer<typeof metricSchema>) => item.key],
    ["accountMetrics", summary.accountMetrics, (item: z.infer<typeof metricSchema>) => item.key],
    ["localMetrics", summary.localMetrics, (item: z.infer<typeof metricSchema>) => item.key],
    ["connectors", summary.connectors, (item: z.infer<typeof connectorSchema>) => item.id],
    ["coverage", summary.coverage, (item: z.infer<typeof coverageSchema>) => item.metricKey],
  ] as const;
  for (const [path, items, key] of keyedCollections) {
    if (!uniqueBy(items as never[], key as (item: never) => string)) {
      context.addIssue({ code: "custom", message: `${path} keys must be unique`, path: [path] });
    }
  }
});

export type DashboardSummary = z.infer<typeof dashboardSummarySchema>;
export type MetricCoverage = z.infer<typeof coverageSchema>;

export const setupDiagnosticsSchema = z.object({
  schemaVersion: z.number().int().min(2).default(2),
  dataMode: z.enum(["local", "remote", "combined"]).default("combined"),
  appDataDir: z.string(),
  databasePath: z.string(),
  authHome: z.string(),
  selectedCodexExecutable: z.string().nullable(),
  configuredCodexRuntimeDisplay: z.string().nullable().default(null),
  codexLaunchMode: z.string().nullable(),
  firstFailingAccountStage: z.string().nullable(),
  lastSuccessfulAccountRefresh: z.string().nullable(),
  usageEventCount: z.number().int().nonnegative(),
  usageTotalTokens: z.number().int().nonnegative(),
  sourceDocumentCount: z.number().int().nonnegative(),
  localRoots: z.array(
    z.object({
      path: z.string(),
      exists: z.boolean(),
      isDirectory: z.boolean(),
    }),
  ),
  latestImportRun: z
    .object({
      completedAtUtc: z.string(),
      filesSeen: z.number().int().nonnegative(),
      eventsSeen: z.number().int().nonnegative(),
      eventsImported: z.number().int().nonnegative(),
      warningCount: z.number().int().nonnegative(),
      warningSamples: z.array(z.string()),
    })
    .nullable(),
  connectorRuns: z.array(
    z.object({
      connectorId: z.string(),
      status: z.string(),
      endpointId: z.string().nullable(),
      httpStatus: z.number().int().nullable(),
      completedAtUtc: z.string(),
      redactedErrorCode: z.string().nullable(),
      redactedErrorMessage: z.string().nullable(),
    }),
  ),
  runtimeCandidates: z.array(z.object({
    displayPath: z.string(),
    nativeExecutablePath: z.string(),
    argvPrefix: z.array(z.string()),
    source: z.enum(["configured", "environment", "path", "codex_app", "npm", "standalone", "msix"]),
    exists: z.boolean(),
    executable: z.boolean().nullable(),
    version: z.string().nullable(),
    validationStatus: z.enum(["valid", "invalid", "missing"]),
    validationError: z.string().nullable(),
  })).default([]),
  selectedRuntime: z.object({
    displayPath: z.string(), nativeExecutablePath: z.string(), argvPrefix: z.array(z.string()),
    source: z.enum(["configured", "environment", "path", "codex_app", "npm", "standalone", "msix"]),
    version: z.string(),
  }).nullable().default(null),
  latestAccountRun: z.object({
    status: z.string(), startedAtUtc: z.string(), completedAtUtc: z.string(), durationMs: z.number().int().nullable(),
    selectedExecutable: z.string().nullable(), selectedDisplayPath: z.string().nullable(), argvPrefix: z.array(z.string()), runtimeSource: z.string().nullable(),
    launchMode: z.string().nullable(), launchAttempts: z.array(z.string()),
    firstFailingStage: z.string().nullable(), errorCode: z.string().nullable(), errorMessage: z.string(),
    exitCode: z.number().int().nullable(), timedOut: z.boolean(), childTerminated: z.boolean().nullable(),
    usedLastGoodSnapshot: z.boolean(), methodStatuses: z.unknown(), schemaFingerprint: z.string().nullable(),
    accountBucketIds: z.array(z.string()), dailyBucketCount: z.number().int().nonnegative().nullable(),
    resetCreditCount: z.number().int().nonnegative().nullable(), mcpDisabled: z.boolean(),
    initializeStatus: z.enum(["ok", "failed", "not_attempted", "unknown"]),
  }).nullable().default(null),
  displayedMetrics: z.array(z.object({
    key: z.string(), source: z.string(), freshness: z.enum(["fresh", "stale", "unavailable"]), status: z.string(),
  })).default([]),
});

export type SetupDiagnostics = z.infer<typeof setupDiagnosticsSchema>;

export const codexLaunchSpecSchema = z.object({
  executablePath: z.string().min(1),
  argvPrefix: z.array(z.string()),
});

export const codexRuntimeSourceSchema = z.enum([
  "configured", "environment", "path", "codex_app", "npm", "standalone", "msix",
]);

export const codexRuntimeCandidateSchema = z.object({
  displayPath: z.string(),
  source: codexRuntimeSourceSchema,
  exists: z.boolean(),
  executable: z.boolean().nullable(),
  version: z.string().nullable(),
  validationError: z.string().nullable(),
  configured: z.boolean(),
  selected: z.boolean(),
});

export const codexRuntimeSelectionSchema = z.object({
  displayPath: z.string().min(1),
}).strict();

export const codexRuntimeValidationSchema = z.object({
  valid: z.boolean(),
  version: z.string().nullable(),
  error: z.string().nullable(),
});

export type CodexRuntimeCandidate = z.infer<typeof codexRuntimeCandidateSchema>;
export type CodexRuntimeSource = z.infer<typeof codexRuntimeSourceSchema>;
export type CodexRuntimeSelection = z.infer<typeof codexRuntimeSelectionSchema>;
export type CodexRuntimeValidation = z.infer<typeof codexRuntimeValidationSchema>;
