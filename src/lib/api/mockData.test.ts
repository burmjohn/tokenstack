import { describe, expect, it } from "vitest";
import { createMockDashboardSummary, createMockSetupDiagnostics } from "./mockData";

describe("createMockDashboardSummary", () => {
  it("uses an empty local dashboard instead of fake usage values", () => {
    const summary = createMockDashboardSummary("combined");

    expect(summary.metrics.map((metric) => metric.value)).toEqual(["0", "0", "0", "0", "0"]);
    expect(summary.resetCredits).toEqual([]);
    expect(summary.sessions).toEqual([]);
    expect(summary.rateLimitWindows).toEqual([]);
    expect(summary.nextReset.label).toBe("No reset-credit snapshot");
    expect(summary.connectors.every((connector) => connector.status === "unavailable")).toBe(true);
  });
});

describe("createMockSetupDiagnostics", () => {
  it("uses empty sanitized diagnostics outside Tauri", () => {
    const diagnostics = createMockSetupDiagnostics();

    expect(diagnostics.localRoots.length).toBeGreaterThan(0);
    expect(diagnostics.latestImportRun).toBeNull();
    expect(diagnostics.connectorRuns).toEqual([]);
    expect(diagnostics.usageEventCount).toBe(0);
    expect(diagnostics.usageTotalTokens).toBe(0);
  });
});
