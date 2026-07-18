import type { DashboardSummary } from "../../lib/schemas/dashboard";

type HeatmapDay = DashboardSummary["heatmap"][number];
export type HeatmapRange = "daily" | "weekly" | "monthly";

export type HeatmapBucket = {
  key: string;
  label: string;
  tokens: number;
  intensity: number;
};

const parseDay = (date: string) => new Date(`${date}T00:00:00Z`);

export function heatmapMonthLabels(days: HeatmapDay[]) {
  const formatter = new Intl.DateTimeFormat("en-US", { month: "short", timeZone: "UTC" });
  const labels: { key: string; label: string }[] = [];
  for (const day of days) {
    const key = day.date.slice(0, 7);
    if (labels.at(-1)?.key !== key) {
      labels.push({ key, label: formatter.format(parseDay(day.date)) });
    }
  }
  return labels;
}

export function aggregateHeatmap(days: HeatmapDay[], range: Exclude<HeatmapRange, "daily">): HeatmapBucket[] {
  const formatter = range === "weekly"
    ? new Intl.DateTimeFormat("en-US", { month: "short", day: "numeric", timeZone: "UTC" })
    : new Intl.DateTimeFormat("en-US", { month: "short", year: "numeric", timeZone: "UTC" });
  const totals = new Map<string, { date: Date; tokens: number }>();

  for (const day of days) {
    const date = parseDay(day.date);
    if (range === "weekly") {
      const mondayOffset = (date.getUTCDay() + 6) % 7;
      date.setUTCDate(date.getUTCDate() - mondayOffset);
    } else {
      date.setUTCDate(1);
    }
    const key = date.toISOString().slice(0, range === "weekly" ? 10 : 7);
    const bucket = totals.get(key) ?? { date, tokens: 0 };
    bucket.tokens += day.tokens;
    totals.set(key, bucket);
  }

  const max = Math.max(0, ...Array.from(totals.values(), (bucket) => bucket.tokens));
  return Array.from(totals, ([key, bucket]) => ({
    key,
    label: formatter.format(bucket.date),
    tokens: bucket.tokens,
    intensity: bucket.tokens === 0 || max === 0 ? 0 : Math.max(1, Math.ceil((bucket.tokens / max) * 5)),
  }));
}

export function mondayOffset(date: string) {
  return (parseDay(date).getUTCDay() + 6) % 7;
}
