import { describe, expect, it } from "vitest";
import { createMockDashboardSummary } from "../api/mockData";
import { dashboardSummarySchema, setupDiagnosticsSchema } from "./dashboard";

describe("dashboardSummarySchema", () => {
  it("accepts sanitized dashboard payloads", () => {
    expect(dashboardSummarySchema.parse(createMockDashboardSummary("combined")).timezone).toBe("America/New_York");
  });

  it("rejects malformed coverage values", () => {
    const payload = createMockDashboardSummary("combined");
    payload.coverage[0].coveragePercent = 101;
    expect(() => dashboardSummarySchema.parse(payload)).toThrow();
  });
});

describe("setupDiagnosticsSchema", () => {
  it("accepts sanitized local setup diagnostics", () => {
    const payload = {
      appDataDir: "C:\\Users\\John\\AppData\\Roaming\\TokenStack",
      databasePath: "C:\\Users\\John\\AppData\\Roaming\\TokenStack\\tokenstack.sqlite3",
      authHome: "C:\\Users\\John",
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
      },
      connectorRuns: [
        {
          connectorId: "known-reset-credit",
          status: "failed",
          completedAtUtc: "2026-07-03T12:00:00Z",
          redactedErrorCode: "auth_unavailable",
        },
      ],
    };

    expect(setupDiagnosticsSchema.parse(payload).localRoots[0].exists).toBe(true);
  });
});
