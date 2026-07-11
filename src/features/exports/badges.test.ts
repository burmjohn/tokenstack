import { describe, expect, it } from "vitest";
import { createMockDashboardSummary } from "../../lib/api/mockData";
import type { DashboardSummary } from "../../lib/schemas/dashboard";
import { BADGE_HEIGHT, BADGE_LAYOUTS, BADGE_WIDTH, buildBadgeFilename, buildBadgeLayoutModel } from "./badges";

describe("badge export model", () => {
  it("defines required output dimensions and layouts", () => {
    expect(BADGE_WIDTH).toBe(1200);
    expect(BADGE_HEIGHT).toBe(630);
    expect(BADGE_LAYOUTS.map((layout) => layout.id)).toEqual(["compact", "usage", "profile"]);
  });

  it.each([
    ["compact", "Usage badge", "38.1B", ["Local today", "Account reset credits", "Timezone"]],
    ["usage", "Local monthly output", "3.62B", ["Local today", "Local peak session", "Coverage"]],
    ["profile", "Usage profile", "38.1B", ["Local this month", "Local today", "Account reset credits", "Coverage", "Timezone"]],
  ] as const)("builds %s layout with public copy", (layout, label, heroValue, statLabels) => {
    const model = buildBadgeLayoutModel(createExportFixtureSummary(), layout);
    const copy = JSON.stringify(model);

    expect(model.label).toBe(label);
    expect(model.heroValue).toBe(heroValue);
    expect(model.brand).toBe("TokenStack");
    expect(model.stats.map((stat) => stat.label)).toEqual(statLabels);
    expect(copy).not.toContain("Read-only");
    expect(copy).not.toContain("/consume");
    expect(copy).not.toContain("safety");
  });

  it("adds a year label and sparkline values from dashboard heatmap", () => {
    const summary = createExportFixtureSummary();
    const compact = buildBadgeLayoutModel(summary, "compact");
    const usage = buildBadgeLayoutModel(summary, "usage");

    expect(compact.footer).toBe("2026 snapshot");
    expect(usage.sparkline).toHaveLength(24);
    expect(usage.sparkline.some((point) => point > 0)).toBe(true);
  });

  it("labels account and local metric families in combined exports", () => {
    const summary = createExportFixtureSummary();
    summary.accountMetrics = [{ ...summary.metrics[0], key: "account-lifetime", label: "Account lifetime", value: "99B" }];
    summary.localMetrics = [{ ...summary.metrics[0], key: "local-lifetime", label: "Local lifetime", value: "38.1B" }];

    const model = buildBadgeLayoutModel(summary, "profile");

    expect(model.stats).toEqual(expect.arrayContaining([
      { label: "Account lifetime", value: "99B" },
      { label: "Local lifetime", value: "38.1B" },
    ]));
  });

  it("keeps combined compact and usage metrics explicitly sourced", () => {
    const summary = createExportFixtureSummary();
    summary.accountMetrics = [
      { ...summary.metrics[0], key: "account-lifetime", value: "99B" },
      { ...summary.metrics[1], key: "account-today", value: "9B" },
      { ...summary.metrics[2], key: "account-month", value: "90B" },
    ];

    const compact = buildBadgeLayoutModel(summary, "compact");
    const usage = buildBadgeLayoutModel(summary, "usage");

    expect(compact.heroLabel).toBe("Account lifetime tokens");
    expect(compact.stats[0]).toEqual({ label: "Account today", value: "9B" });
    expect(usage.heroLabel).toBe("Account tokens this month");
    expect(usage.stats[0]).toEqual({ label: "Account today", value: "9B" });
  });

  it("uses account today and month coherently for remote-only badges", () => {
    const summary = createExportFixtureSummary();
    summary.dataMode = "remote";
    summary.localMetrics = [];
    summary.accountMetrics = [
      { ...summary.metrics[0], key: "account-lifetime", value: "99B" },
      { ...summary.metrics[1], key: "account-today", value: "9B" },
      { ...summary.metrics[2], key: "account-month", value: "90B" },
    ];

    const profile = buildBadgeLayoutModel(summary, "profile");

    expect(profile.heroValue).toBe("99B");
    expect(profile.heroLabel).toBe("Account lifetime tokens");
    expect(profile.stats).toEqual(expect.arrayContaining([
      { label: "Account today", value: "9B" },
      { label: "Account this month", value: "90B" },
    ]));
    expect(profile.stats.some((stat) => stat.label.startsWith("Local"))).toBe(false);
  });

  it("builds required badge filenames", () => {
    expect(buildBadgeFilename("compact", new Date("2026-07-02T19:30:00Z"))).toBe("tokenstack-badge-compact-2026-07-02.png");
    expect(buildBadgeFilename("usage", new Date("2026-07-02T19:30:00Z"))).toBe("tokenstack-badge-usage-2026-07-02.png");
    expect(buildBadgeFilename("profile", new Date("2026-07-02T19:30:00Z"))).toBe("tokenstack-badge-profile-2026-07-02.png");
  });
});

function createExportFixtureSummary(): DashboardSummary {
  const summary = createMockDashboardSummary();
  const coverage = [
    {
      ...summary.coverage[0],
      coveragePercent: 72,
      confidence: "medium" as const,
      lastEvidenceAtUtc: "2026-07-02T19:30:00Z",
      missingFacets: ["some archived sessions"],
      explanation: "Local usage events are present and deduplicated; unknown shapes lower confidence.",
    },
    {
      ...summary.coverage[1],
      coveragePercent: 100,
      confidence: "high" as const,
      lastEvidenceAtUtc: "2026-07-02T19:30:00Z",
      missingFacets: [],
      explanation: "Reset-credit snapshots are stored, schema-valid, and freshness checked.",
    },
    {
      ...summary.coverage[2],
      coveragePercent: 68,
      confidence: "medium" as const,
      lastEvidenceAtUtc: "2026-07-02T19:30:00Z",
      missingFacets: ["additional source confirmation"],
      explanation: "Rate-limit windows are stored with conservative confidence until more evidence is available.",
    },
  ];

  return {
    ...summary,
    lastRefreshLabel: "2m ago",
    metrics: [
      {
        key: "lifetime",
        label: "Lifetime tokens",
        value: "38.1B",
        delta: "12.4% vs last 30 days",
        status: "positive",
        coverage: coverage[0],
      },
      {
        key: "today",
        label: "Today",
        value: "128.7M",
        delta: "8.6% vs yesterday",
        status: "positive",
        coverage: coverage[0],
      },
      {
        key: "month",
        label: "This month",
        value: "3.62B",
        delta: "18.3% vs last month",
        status: "positive",
        coverage: coverage[0],
      },
      {
        key: "peak",
        label: "Peak session",
        value: "1.72B",
        delta: "Jun 4, 1:44 PM",
        status: "neutral",
        coverage: coverage[0],
      },
      {
        key: "reset",
        label: "Reset credits",
        value: "4",
        delta: "Available",
        status: "positive",
        coverage: coverage[1],
      },
    ],
    localMetrics: [
      { key: "local-lifetime", label: "Local lifetime", value: "38.1B", delta: "Imported local history", status: "positive", coverage: coverage[0] },
      { key: "local-today", label: "Local today", value: "128.7M", delta: "Local day", status: "positive", coverage: coverage[0] },
      { key: "local-month", label: "Local this month", value: "3.62B", delta: "Local month", status: "positive", coverage: coverage[0] },
      { key: "local-peak", label: "Local peak session", value: "1.72B", delta: "Local peak", status: "neutral", coverage: coverage[0] },
    ],
    heatmap: summary.heatmap.map((day, index) => {
      const tokens = index < 88 ? 0 : 24_000_000 + ((index * 17) % 86) * 1_000_000;
      return {
        ...day,
        tokens,
        intensity: tokens === 0 ? 0 : Math.min(5, Math.max(1, Math.round(tokens / 22_000_000))),
      };
    }),
    coverage,
  };
}
