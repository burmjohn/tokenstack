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

export const dashboardSummarySchema = z.object({
  generatedAtUtc: z.string(),
  dataMode: dataModeSchema,
  lastRefreshLabel: z.string(),
  refreshStatus: z.enum(["idle", "pending", "stale", "degraded", "failed"]),
  timezone: z.literal("America/New_York"),
  metrics: z.array(metricSchema),
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
});

export type DashboardSummary = z.infer<typeof dashboardSummarySchema>;
export type MetricCoverage = z.infer<typeof coverageSchema>;

export const setupDiagnosticsSchema = z.object({
  appDataDir: z.string(),
  databasePath: z.string(),
  authHome: z.string(),
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
});

export type SetupDiagnostics = z.infer<typeof setupDiagnosticsSchema>;
