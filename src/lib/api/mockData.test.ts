import { describe, expect, it } from "vitest";
import { createMockDashboardSummary } from "./mockData";

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
