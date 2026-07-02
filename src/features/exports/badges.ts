import type { DashboardSummary } from "../../lib/schemas/dashboard";

export const BADGE_WIDTH = 1200;
export const BADGE_HEIGHT = 630;

export const BADGE_LAYOUTS = [
  { id: "compact", label: "Compact" },
  { id: "usage", label: "Usage" },
  { id: "profile", label: "Profile" },
] as const;

export type BadgeLayoutId = (typeof BADGE_LAYOUTS)[number]["id"];

export type BadgeStat = {
  label: string;
  value: string;
};

export type BadgeLayoutModel = {
  layout: BadgeLayoutId;
  brand: "TokenStack";
  label: string;
  heroValue: string;
  heroLabel: string;
  stats: BadgeStat[];
  sparkline: number[];
  footer: string;
};

export async function loadBadgeLogo(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error("Unable to load badge logo."));
    image.src = src;
  });
}

export function buildBadgeLayoutModel(summary: DashboardSummary, layout: BadgeLayoutId): BadgeLayoutModel {
  const lifetime = getMetric(summary, "lifetime");
  const today = getMetric(summary, "today");
  const month = getMetric(summary, "month");
  const peak = getMetric(summary, "peak");
  const reset = getMetric(summary, "reset");
  const coverage = `${averageCoverage(summary)}%`;
  const year = new Date(summary.generatedAtUtc).getUTCFullYear();
  const sparkline = summary.heatmap.slice(-24).map((day) => day.intensity);

  if (layout === "usage") {
    return {
      layout,
      brand: "TokenStack",
      label: "Monthly output",
      heroValue: month.value,
      heroLabel: "tokens this month",
      stats: [
        { label: "Peak session", value: peak.value },
        { label: "Month delta", value: month.delta },
        { label: "Coverage", value: coverage },
      ],
      sparkline,
      footer: `${year} snapshot`,
    };
  }

  if (layout === "profile") {
    return {
      layout,
      brand: "TokenStack",
      label: "Usage profile",
      heroValue: lifetime.value,
      heroLabel: "lifetime tokens",
      stats: [
        { label: "This month", value: month.value },
        { label: "Today", value: today.value },
        { label: "Reset credits", value: reset.value },
        { label: "Coverage", value: coverage },
        { label: "Peak session", value: peak.value },
        { label: "Timezone", value: summary.timezone },
      ],
      sparkline,
      footer: `${year} snapshot`,
    };
  }

  return {
    layout,
    brand: "TokenStack",
    label: "Usage badge",
    heroValue: lifetime.value,
    heroLabel: "lifetime tokens",
    stats: [
      { label: "Today", value: today.value },
      { label: "Reset credits", value: reset.value },
      { label: "Timezone", value: summary.timezone },
    ],
    sparkline: [],
    footer: `${year} snapshot`,
  };
}

export function renderUsageBadge(canvas: HTMLCanvasElement, summary: DashboardSummary, layout: BadgeLayoutId, logo: CanvasImageSource | null): BadgeLayoutModel {
  const model = buildBadgeLayoutModel(summary, layout);
  canvas.width = BADGE_WIDTH;
  canvas.height = BADGE_HEIGHT;

  const context = canvas.getContext("2d");
  if (!context) {
    return model;
  }

  drawBadge(context, model, logo);
  return model;
}

export function buildBadgeFilename(layout: BadgeLayoutId, generatedAt = new Date()): string {
  return `tokenstack-badge-${layout}-${formatDate(generatedAt)}.png`;
}

function getMetric(summary: DashboardSummary, key: string) {
  const metric = summary.metrics.find((item) => item.key === key);
  if (!metric) {
    return { value: "0", delta: "Unavailable" };
  }
  return metric;
}

function averageCoverage(summary: DashboardSummary): number {
  if (summary.coverage.length === 0) {
    return 0;
  }
  return Math.round(summary.coverage.reduce((total, item) => total + item.coveragePercent, 0) / summary.coverage.length);
}

function formatDate(date: Date): string {
  return date.toISOString().slice(0, 10);
}

function drawBadge(context: CanvasRenderingContext2D, model: BadgeLayoutModel, logo: CanvasImageSource | null) {
  context.clearRect(0, 0, BADGE_WIDTH, BADGE_HEIGHT);
  context.fillStyle = model.layout === "profile" ? "#111828" : "#0d1424";
  context.fillRect(0, 0, BADGE_WIDTH, BADGE_HEIGHT);

  context.fillStyle = "#172237";
  context.fillRect(40, 40, BADGE_WIDTH - 80, BADGE_HEIGHT - 80);
  context.strokeStyle = "#33507a";
  context.lineWidth = 2;
  context.strokeRect(40, 40, BADGE_WIDTH - 80, BADGE_HEIGHT - 80);

  drawBrand(context, logo);

  context.fillStyle = "#8fb8ff";
  context.font = "600 30px Inter, system-ui, sans-serif";
  context.fillText(model.label, 86, 188);

  context.fillStyle = "#f6fbff";
  context.font = model.layout === "profile" ? "700 98px Inter, system-ui, sans-serif" : "700 118px Inter, system-ui, sans-serif";
  context.fillText(model.heroValue, 82, model.layout === "profile" ? 310 : 328);

  context.fillStyle = "#9fb2c9";
  context.font = "500 24px Inter, system-ui, sans-serif";
  context.fillText(model.heroLabel, 90, model.layout === "profile" ? 352 : 372);

  drawStats(context, model);
  if (model.sparkline.length > 0) {
    drawSparkline(context, model.sparkline, model.layout === "profile" ? 720 : 760, 432, 330, 86);
  }

  context.fillStyle = "#7f91aa";
  context.font = "500 22px Inter, system-ui, sans-serif";
  context.fillText(model.footer, 86, 548);
}

function drawBrand(context: CanvasRenderingContext2D, logo: CanvasImageSource | null) {
  if (logo) {
    context.drawImage(logo, 86, 76, 66, 66);
  } else {
    context.fillStyle = "#78f0b6";
    context.beginPath();
    context.arc(119, 109, 33, 0, Math.PI * 2);
    context.fill();
    context.fillStyle = "#0d1424";
    context.font = "800 24px Inter, system-ui, sans-serif";
    context.fillText("TS", 102, 117);
  }

  context.fillStyle = "#f6fbff";
  context.font = "700 40px Inter, system-ui, sans-serif";
  context.fillText("TokenStack", 174, 121);
}

function drawStats(context: CanvasRenderingContext2D, model: BadgeLayoutModel) {
  const compact = model.layout === "compact";
  const columns = compact ? 3 : 3;
  const startX = compact ? 86 : 90;
  const startY = compact ? 430 : 410;
  const columnWidth = compact ? 310 : 335;

  model.stats.forEach((stat, index) => {
    const x = startX + (index % columns) * columnWidth;
    const y = startY + Math.floor(index / columns) * 84;
    context.fillStyle = "#7f91aa";
    context.font = "600 19px Inter, system-ui, sans-serif";
    context.fillText(stat.label, x, y);
    context.fillStyle = "#f6fbff";
    context.font = "700 27px Inter, system-ui, sans-serif";
    context.fillText(stat.value, x, y + 36);
  });
}

function drawSparkline(context: CanvasRenderingContext2D, points: number[], x: number, y: number, width: number, height: number) {
  const step = width / Math.max(1, points.length - 1);

  context.strokeStyle = "#78f0b6";
  context.lineWidth = 5;
  context.beginPath();
  points.forEach((point, index) => {
    const px = x + index * step;
    const py = y + height - (point / 5) * height;
    if (index === 0) {
      context.moveTo(px, py);
    } else {
      context.lineTo(px, py);
    }
  });
  context.stroke();
}
