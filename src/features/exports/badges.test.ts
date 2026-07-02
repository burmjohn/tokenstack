import { describe, expect, it } from "vitest";
import { createMockDashboardSummary } from "../../lib/api/mockData";
import { BADGE_HEIGHT, BADGE_LAYOUTS, BADGE_WIDTH, buildBadgeFilename, buildBadgeLayoutModel } from "./badges";

describe("badge export model", () => {
  it("defines required output dimensions and layouts", () => {
    expect(BADGE_WIDTH).toBe(1200);
    expect(BADGE_HEIGHT).toBe(630);
    expect(BADGE_LAYOUTS.map((layout) => layout.id)).toEqual(["compact", "usage", "profile"]);
  });

  it.each([
    ["compact", "Usage badge", "38.1B", ["Today", "Reset credits", "Timezone"]],
    ["usage", "Monthly output", "3.62B", ["Peak session", "Month delta", "Coverage"]],
    ["profile", "Usage profile", "38.1B", ["This month", "Today", "Reset credits", "Coverage", "Peak session", "Timezone"]],
  ] as const)("builds %s layout with public copy", (layout, label, heroValue, statLabels) => {
    const model = buildBadgeLayoutModel(createMockDashboardSummary(), layout);
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
    const summary = createMockDashboardSummary();
    const compact = buildBadgeLayoutModel(summary, "compact");
    const usage = buildBadgeLayoutModel(summary, "usage");

    expect(compact.footer).toBe("2026 snapshot");
    expect(usage.sparkline).toHaveLength(24);
    expect(usage.sparkline.some((point) => point > 0)).toBe(true);
  });

  it("builds required badge filenames", () => {
    expect(buildBadgeFilename("compact", new Date("2026-07-02T19:30:00Z"))).toBe("tokenstack-badge-compact-2026-07-02.png");
    expect(buildBadgeFilename("usage", new Date("2026-07-02T19:30:00Z"))).toBe("tokenstack-badge-usage-2026-07-02.png");
    expect(buildBadgeFilename("profile", new Date("2026-07-02T19:30:00Z"))).toBe("tokenstack-badge-profile-2026-07-02.png");
  });
});
