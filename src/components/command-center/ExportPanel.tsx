import { Download, ImageDown } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { BADGE_HEIGHT, BADGE_LAYOUTS, BADGE_WIDTH, buildBadgeFilename, buildBadgeLayoutModel, type BadgeLayoutId, loadBadgeLogo, renderUsageBadge } from "../../features/exports/badges";
import { downloadCanvasPng } from "../../features/exports/download";
import type { DashboardSummary } from "../../lib/schemas/dashboard";
import { cn } from "../../lib/utils";
import { Button } from "../ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";

const tokenstackIconUrl = new URL("../../assets/tokenstack-icon.png", import.meta.url).href;

type LogoState = "loading" | "loaded" | "failed";

export function ExportPanel({ summary, onClose }: { summary: DashboardSummary; onClose: () => void }) {
  const [layout, setLayout] = useState<BadgeLayoutId>("compact");
  const [logo, setLogo] = useState<HTMLImageElement | null>(null);
  const [logoState, setLogoState] = useState<LogoState>("loading");
  const [warning, setWarning] = useState<string | null>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const model = useMemo(() => buildBadgeLayoutModel(summary, layout), [layout, summary]);
  const selectedLayout = BADGE_LAYOUTS.find((item) => item.id === layout)?.label ?? "Compact";

  useEffect(() => {
    let cancelled = false;

    loadBadgeLogo(tokenstackIconUrl)
      .then((image) => {
        if (cancelled) {
          return;
        }
        setLogo(image);
        setLogoState("loaded");
      })
      .catch(() => {
        if (cancelled) {
          return;
        }
        setLogo(null);
        setLogoState("failed");
        setWarning("Logo unavailable; using TS monogram.");
      });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (logoState === "loading" || !canvasRef.current) {
      return;
    }
    renderUsageBadge(canvasRef.current, summary, layout, logo);
  }, [layout, logo, logoState, summary]);

  const handleDownloadPng = async () => {
    const canvas = canvasRef.current;
    if (!canvas) {
      setWarning("PNG preview unavailable. Try again.");
      return;
    }

    renderUsageBadge(canvas, summary, layout, logo);
    const downloaded = await downloadCanvasPng(buildBadgeFilename(layout), canvas);
    if (!downloaded) {
      setWarning("PNG export did not produce an image. Try again.");
      return;
    }
    onClose();
  };

  return (
    <Card role="region" aria-label="Badge export panel" className="mb-6 overflow-hidden border-primary/30 bg-card/95">
      <CardHeader className="items-center gap-4 max-[780px]:items-start">
        <div>
          <CardTitle className="inline-flex items-center gap-2">
            <ImageDown size={17} aria-hidden />
            Export badge
          </CardTitle>
          <p className="mt-1 text-xs text-muted-foreground">1200x630 PNG · {selectedLayout}</p>
        </div>
        <div className="inline-flex rounded-[8px] border border-border bg-background p-1" aria-label="Badge layout">
          {BADGE_LAYOUTS.map((item) => (
            <button
              key={item.id}
              type="button"
              aria-label={`${item.label} badge layout`}
              aria-pressed={layout === item.id}
              className={cn(
                "h-8 rounded-[6px] px-3 text-xs font-medium text-muted-foreground outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring",
                layout === item.id && "bg-primary text-primary-foreground",
              )}
              onClick={() => setLayout(item.id)}
            >
              {item.label}
            </button>
          ))}
        </div>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-[minmax(280px,1fr)_minmax(240px,360px)] gap-5 max-[900px]:grid-cols-1">
          <div className="overflow-hidden rounded-[8px] border border-border bg-background">
            <canvas
              ref={canvasRef}
              width={BADGE_WIDTH}
              height={BADGE_HEIGHT}
              className="block aspect-[1200/630] w-full"
              role="img"
              aria-label={`${selectedLayout} TokenStack badge preview`}
            />
          </div>
          <div className="flex min-h-[220px] flex-col justify-between gap-4 rounded-[8px] border border-border bg-background p-4">
            <div>
              <div className="text-xs font-medium uppercase text-primary">{model.label}</div>
              <div className="mt-2 text-4xl font-semibold leading-none">{model.heroValue}</div>
              <div className="mt-2 text-sm text-muted-foreground">{model.heroLabel}</div>
              <dl className="mt-5 grid grid-cols-2 gap-3">
                {model.stats.slice(0, 4).map((stat) => (
                  <div key={stat.label}>
                    <dt className="text-[11px] text-muted-foreground">{stat.label}</dt>
                    <dd className="mt-1 text-sm font-medium">{stat.value}</dd>
                  </div>
                ))}
              </dl>
            </div>
            <div className="space-y-3">
              {warning ? <p className="rounded-[6px] border border-amber/40 bg-amber/10 px-3 py-2 text-xs text-amber">{warning}</p> : null}
              {logoState === "loading" ? <p className="text-xs text-muted-foreground">Preparing logo...</p> : null}
              <Button type="button" className="w-full" onClick={handleDownloadPng} disabled={logoState === "loading"} aria-label={`Download PNG for ${selectedLayout} badge`}>
                <Download size={15} aria-hidden />
                Download PNG
              </Button>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
