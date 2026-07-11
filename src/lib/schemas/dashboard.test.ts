import { describe, expect, it } from "vitest";
import { createMockDashboardSummary, createMockSetupDiagnostics } from "../api/mockData";
import { codexRuntimeCandidateSchema, codexRuntimeSelectionSchema, dashboardSummarySchema, setupDiagnosticsSchema } from "./dashboard";

describe("Codex runtime schemas", () => {
  it("keeps npm launch details backend-only and exposes explicit selection state", () => {
    const launch = { executablePath: "C:\\Program Files\\nodejs\\node.exe", argvPrefix: ["C:\\Users\\Test\\codex.js"] };
    const parsed = codexRuntimeCandidateSchema.parse({ displayPath: "C:\\Users\\Test\\codex.cmd", source: "npm", exists: true, executable: true, version: "codex 1", validationError: null, configured: true, selected: true });
    expect(parsed).not.toHaveProperty("launch");
    expect(parsed).toMatchObject({ configured: true, selected: true });
    expect(codexRuntimeSelectionSchema.parse({ displayPath: "C:\\Users\\Test\\codex.cmd" }).displayPath).toContain("codex.cmd");
    expect(() => codexRuntimeSelectionSchema.parse({ displayPath: "codex.cmd", launch })).toThrow();
  });
});

describe("dashboardSummarySchema", () => {
  it("accepts sanitized dashboard payloads", () => {
    expect(dashboardSummarySchema.parse(createMockDashboardSummary("combined")).timezone).toBe("America/New_York");
  });

  it("rejects malformed coverage values", () => {
    const payload = createMockDashboardSummary("combined");
    payload.coverage[0].coveragePercent = 101;
    expect(() => dashboardSummarySchema.parse(payload)).toThrow();
    payload.coverage[0].coveragePercent = 86;
  });

  it("preserves explicit connector freshness and last-good age", () => {
    const payload = createMockDashboardSummary("combined");
    Object.assign(payload.connectors[0], { freshness: "stale", ageSeconds: 45 });
    const parsed = dashboardSummarySchema.parse(payload);
    expect(parsed.connectors[0].freshness).toBe("stale");
    expect(parsed.connectors[0].ageSeconds).toBe(45);
  });

  it("preserves distinct account and local metric families", () => {
    const payload = createMockDashboardSummary("combined");
    payload.accountMetrics = [{ ...payload.metrics[0], key: "account-lifetime", label: "Account lifetime" }];
    payload.localMetrics = [{ ...payload.metrics[0], key: "local-lifetime", label: "Local lifetime" }];

    const parsed = dashboardSummarySchema.parse(payload);

    expect(parsed.accountMetrics.map((metric) => metric.key)).toEqual(["account-lifetime"]);
    expect(parsed.localMetrics.map((metric) => metric.key)).toEqual(["local-lifetime"]);
  });

  it("rejects duplicate metric, connector, and coverage keys", () => {
    const payload = createMockDashboardSummary("combined");
    payload.accountMetrics = [payload.metrics[0], payload.metrics[0]];
    payload.connectors.push(payload.connectors[0]);
    payload.coverage.push(payload.coverage[0]);

    expect(() => dashboardSummarySchema.parse(payload)).toThrow(/unique/i);
  });

  it("accepts older payloads without separated metrics or configured runtime display", () => {
    const payload = createMockDashboardSummary("combined") as Record<string, unknown>;
    payload.coverage = [...(payload.coverage as unknown[]).slice(0, 3)];
    delete payload.accountMetrics;
    delete payload.localMetrics;
    expect(dashboardSummarySchema.parse(payload)).toMatchObject({ accountMetrics: [], localMetrics: [] });

    const diagnostics = createMockSetupDiagnostics() as unknown as Record<string, unknown>;
    delete diagnostics.configuredCodexRuntimeDisplay;
    expect(setupDiagnosticsSchema.parse(diagnostics).configuredCodexRuntimeDisplay).toBeNull();
  });
});

describe("setupDiagnosticsSchema", () => {
  it("accepts sanitized local setup diagnostics", () => {
    const payload = {
      appDataDir: "C:\\Users\\John\\AppData\\Roaming\\TokenStack",
      databasePath: "C:\\Users\\John\\AppData\\Roaming\\TokenStack\\tokenstack.sqlite3",
      authHome: "C:\\Users\\John",
      selectedCodexExecutable: "C:\\Program Files\\Codex\\codex.exe",
      configuredCodexRuntimeDisplay: "C:\\Program Files\\Codex\\codex.exe",
      codexLaunchMode: "listen_stdio_no_mcp",
      firstFailingAccountStage: null,
      lastSuccessfulAccountRefresh: "2026-07-03T12:00:00Z",
      usageEventCount: 307,
      usageTotalTokens: 22800000,
      sourceDocumentCount: 23,
      localRoots: [
        {
          path: "C:\\Users\\John\\.codex\\sessions",
          exists: true,
          isDirectory: true,
        },
      ],
      latestImportRun: {
        completedAtUtc: "2026-07-03T12:00:00Z",
        filesSeen: 2,
        eventsSeen: 2,
        eventsImported: 1,
        warningCount: 1,
        warningSamples: ["history.jsonl:2 unknown event shape skipped (type=message; keys=timestamp,type)"],
      },
      connectorRuns: [
        {
          connectorId: "known-reset-credit",
          status: "failed",
          endpointId: "known-reset-credit",
          httpStatus: null,
          completedAtUtc: "2026-07-03T12:00:00Z",
          redactedErrorCode: "auth_unavailable",
          redactedErrorMessage: "auth document is unavailable",
        },
      ],
    };

    const diagnostics = setupDiagnosticsSchema.parse(payload);
    expect(diagnostics.localRoots[0].exists).toBe(true);
    expect(diagnostics.latestImportRun?.warningSamples[0]).toContain("type=message");
  });
});
