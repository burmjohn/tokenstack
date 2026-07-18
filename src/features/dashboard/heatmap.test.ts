import { describe, expect, it } from "vitest";
import { aggregateHeatmap, heatmapMonthLabels, mondayOffset } from "./heatmap";

const days = [
  { date: "2026-06-29", weekday: "Mon", tokens: 10, intensity: 1 },
  { date: "2026-07-01", weekday: "Wed", tokens: 20, intensity: 1 },
  { date: "2026-07-06", weekday: "Mon", tokens: 70, intensity: 2 },
];

describe("heatmap range helpers", () => {
  it("derives month labels from calendar dates", () => {
    expect(heatmapMonthLabels(days)).toEqual([
      { key: "2026-06", label: "Jun" },
      { key: "2026-07", label: "Jul" },
    ]);
  });

  it("aggregates weekly and monthly totals", () => {
    expect(aggregateHeatmap(days, "weekly").map(({ key, tokens }) => ({ key, tokens }))).toEqual([
      { key: "2026-06-29", tokens: 30 },
      { key: "2026-07-06", tokens: 70 },
    ]);
    expect(aggregateHeatmap(days, "monthly").map(({ key, tokens }) => ({ key, tokens }))).toEqual([
      { key: "2026-06", tokens: 10 },
      { key: "2026-07", tokens: 90 },
    ]);
  });

  it("uses a Monday-first calendar offset", () => {
    expect(mondayOffset("2026-07-01")).toBe(2);
  });
});
