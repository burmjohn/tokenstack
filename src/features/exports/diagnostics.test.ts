import { describe, expect, it } from "vitest";
import { createMockSetupDiagnostics } from "../../lib/api/mockData";
import { buildSetupDiagnosticsFilename, buildSetupDiagnosticsJson } from "./diagnostics";

describe("diagnostics export helpers", () => {
  it("writes sanitized diagnostics JSON for support handoff", () => {
    const diagnostics = {
      ...createMockSetupDiagnostics(),
      usageEventCount: 307,
      usageTotalTokens: 22_800_000,
      latestImportRun: {
        completedAtUtc: "2026-07-03T12:00:00Z",
        filesSeen: 23,
        eventsSeen: 2402,
        eventsImported: 0,
        warningCount: 2095,
        warningSamples: ["history.jsonl:2 unknown event shape skipped (type=message; keys=timestamp,type)"],
      },
      connectorRuns: [
        {
          connectorId: "known-reset-credit",
          status: "failed",
          endpointId: "known-reset-credit",
          httpStatus: null,
          completedAtUtc: "2026-07-03T12:00:00Z",
          redactedErrorCode: "connector_failed",
          redactedErrorMessage: "HTTP status client error (401 Unauthorized)",
        },
      ],
    };

    const json = buildSetupDiagnosticsJson(diagnostics, new Date("2026-07-03T12:34:56Z"));

    expect(json).toContain('"generatedAtUtc": "2026-07-03T12:34:56.000Z"');
    expect(json).toContain('"usageTotalTokens": 22800000');
    expect(json).toContain("401 Unauthorized");
    expect(json).toContain("type=message");
    expect(json).not.toContain("Bearer ");
  });

  it("builds a dated diagnostics filename", () => {
    expect(buildSetupDiagnosticsFilename(new Date("2026-07-03T12:34:56Z"))).toBe(
      "tokenstack-diagnostics-2026-07-03.json",
    );
  });
});
