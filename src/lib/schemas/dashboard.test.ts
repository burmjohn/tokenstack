import { describe, expect, it } from "vitest";
import { createMockDashboardSummary } from "../api/mockData";
import { dashboardSummarySchema } from "./dashboard";

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
