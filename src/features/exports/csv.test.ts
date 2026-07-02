import { describe, expect, it } from "vitest";
import { createMockDashboardSummary } from "../../lib/api/mockData";
import { buildDashboardUsageCsv, buildUsageCsvFilename, escapeCsvField } from "./csv";

describe("export CSV helpers", () => {
  it("escapes commas, quotes, newlines, and carriage returns", () => {
    expect(escapeCsvField("plain")).toBe("plain");
    expect(escapeCsvField("alpha,beta")).toBe('"alpha,beta"');
    expect(escapeCsvField('alpha "beta"')).toBe('"alpha ""beta"""');
    expect(escapeCsvField("alpha\nbeta")).toBe('"alpha\nbeta"');
    expect(escapeCsvField("alpha\rbeta")).toBe('"alpha\rbeta"');
  });

  it("writes sections in the committed design order", () => {
    const csv = buildDashboardUsageCsv(createMockDashboardSummary(), new Date("2026-07-02T19:30:00Z"));
    const sections = csv.split(/\n\n/).map((section) => section.split("\n")[0]);

    expect(sections).toEqual([
      "metadata",
      "metrics",
      "daily_usage",
      "reset_credits",
      "recent_sessions",
      "rate_limit_windows",
      "source_coverage",
    ]);
  });

  it("includes representative dashboard rows", () => {
    const csv = buildDashboardUsageCsv(createMockDashboardSummary("combined"), new Date("2026-07-02T19:30:00Z"));

    expect(csv).toContain("generated_at_utc,data_mode,refresh_status,timezone,last_refresh_label");
    expect(csv).toContain("2026-07-02T19:30:00.000Z,combined,idle,America/New_York,2m ago");
    expect(csv).toContain("metric_key,label,value,delta,status,coverage_percent,confidence,source_kind");
    expect(csv).toContain("lifetime,Lifetime tokens,38.1B,12.4% vs last 30 days,positive,72,medium,Local history");
    expect(csv).toContain("date,weekday,tokens,intensity");
    expect(csv).toContain("credit_count,expires_at_utc,expires_at_new_york,days_remaining,confidence");
    expect(csv).toContain("start_time,duration,tokens,peak_tokens,mode,sources");
    expect(csv).toContain('"Jun 14, 1:12 PM",47m 23s,512.3M,1.72B,deep-research,CLI; Cloud');
    expect(csv).toContain("window,limit,used,remaining,reset_countdown,progress_percent");
    expect(csv).toContain("1m,20.0B,11.2B,8.8B (44%),00:14,56");
    expect(csv).toContain("metric_key,source_kind,coverage_percent,confidence,last_evidence_at_utc,formula_version,missing_facets,explanation");
  });

  it("builds the required dated filename", () => {
    expect(buildUsageCsvFilename(new Date("2026-07-02T19:30:00Z"))).toBe("tokenstack-usage-2026-07-02.csv");
  });
});
