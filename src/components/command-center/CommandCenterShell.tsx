import {
  Activity,
  BarChart3,
  CalendarDays,
  Download,
  Info,
  RefreshCcw,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";
import { Progress } from "../ui/progress";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../ui/table";
import { Tabs, TabsList, TabsTrigger } from "../ui/tabs";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "../ui/tooltip";
import { useDashboardSummary, useRefreshAll, useSetupDiagnostics } from "../../features/dashboard/useDashboardSummary";
import { listenForDesktopMenuCommands } from "../../features/desktop/commands";
import { installDesktopContextMenu } from "../../features/desktop/contextMenu";
import { buildDashboardUsageCsv, buildUsageCsvFilename } from "../../features/exports/csv";
import { buildSetupDiagnosticsFilename, buildSetupDiagnosticsJson } from "../../features/exports/diagnostics";
import { downloadTextFile } from "../../features/exports/download";
import type { DataMode, DashboardSummary, MetricCoverage, SetupDiagnostics } from "../../lib/schemas/dashboard";
import { cn } from "../../lib/utils";
import { createDesktopShellActionHandler } from "./desktopShellActions";
import { DesktopStatusBar } from "./DesktopStatusBar";
import { DesktopToolbar } from "./DesktopToolbar";
import { ExportPanel } from "./ExportPanel";
import { SECTION_COPY, type NavSection } from "./sectionModel";

type Theme = "dark" | "light";

const metricIcons = [Activity, CalendarDays, BarChart3, Zap, RefreshCcw];
const AUTO_REFRESH_BASE_DELAY_MS = 60_000;
const AUTO_REFRESH_MAX_DELAY_MS = 300_000;

export function CommandCenterShell() {
  const [theme, setTheme] = useState<Theme>(() => (localStorage.getItem("tokenstack-theme") as Theme | null) ?? "dark");
  const [dataMode, setDataMode] = useState<DataMode>("combined");
  const [autoRefreshDelayMs, setAutoRefreshDelayMs] = useState(AUTO_REFRESH_BASE_DELAY_MS);
  const [activeSection, setActiveSection] = useState<NavSection>("dashboard");
  const [isExportPanelOpen, setIsExportPanelOpen] = useState(false);
  const query = useDashboardSummary(dataMode);
  const refresh = useRefreshAll(dataMode);
  const refreshPendingRef = useRef(false);
  const refreshMutationRef = useRef(refresh.mutateAsync);
  const handleDataModeChange = (mode: DataMode) => {
    setIsExportPanelOpen(false);
    setAutoRefreshDelayMs(AUTO_REFRESH_BASE_DELAY_MS);
    setDataMode(mode);
  };

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("tokenstack-theme", theme);
  }, [theme]);

  useEffect(() => {
    refreshPendingRef.current = refresh.isPending;
    refreshMutationRef.current = refresh.mutateAsync;
  }, [refresh.isPending, refresh.mutateAsync]);

  useEffect(() => {
    let cancelled = false;
    let nextDelay = AUTO_REFRESH_BASE_DELAY_MS;
    let timer: number | undefined;

    const schedule = (delay: number) => {
      timer = window.setTimeout(async () => {
        if (cancelled) {
          return;
        }
        if (refreshPendingRef.current) {
          schedule(nextDelay);
          return;
        }
        try {
          await refreshMutationRef.current();
          nextDelay = AUTO_REFRESH_BASE_DELAY_MS;
        } catch {
          nextDelay = Math.min(nextDelay * 2, AUTO_REFRESH_MAX_DELAY_MS);
        }
        if (!cancelled) {
          setAutoRefreshDelayMs(nextDelay);
          schedule(nextDelay);
        }
      }, delay);
    };

    schedule(AUTO_REFRESH_BASE_DELAY_MS);

    return () => {
      cancelled = true;
      if (timer) {
        window.clearTimeout(timer);
      }
    };
  }, [dataMode]);

  const summary = query.data;
  const handleCsvExport = useCallback(() => {
    if (!summary) {
      return;
    }
    void downloadTextFile(buildUsageCsvFilename(), buildDashboardUsageCsv(summary), "text/csv;charset=utf-8");
  }, [summary]);
  const openBadgeExport = useCallback(() => {
    if (summary) {
      setIsExportPanelOpen(true);
    }
  }, [summary]);
  const toggleBadgeExportPanel = useCallback(() => {
    if (summary) {
      setIsExportPanelOpen((open) => !open);
    }
  }, [summary]);
  const refreshNow = useCallback(() => refresh.mutate(), [refresh]);
  const toggleTheme = useCallback(() => setTheme((current) => (current === "dark" ? "light" : "dark")), []);
  const desktopActionHandler = useMemo(
    () =>
      createDesktopShellActionHandler({
        exportBadge: openBadgeExport,
        exportCsv: handleCsvExport,
        navigate: setActiveSection,
        refresh: refreshNow,
        toggleTheme,
      }),
    [handleCsvExport, openBadgeExport, refreshNow, toggleTheme],
  );

  useEffect(() => {
    let cleanup: (() => void) | null = null;
    let cancelled = false;

    void listenForDesktopMenuCommands(desktopActionHandler).then((unlisten) => {
      if (cancelled) {
        unlisten?.();
        return;
      }
      cleanup = unlisten;
    });

    const removeContextMenu = installDesktopContextMenu(desktopActionHandler);

    return () => {
      cancelled = true;
      cleanup?.();
      removeContextMenu();
    };
  }, [desktopActionHandler]);

  const sourceCount = summary?.connectors.length ?? 0;
  const connectedSourceCount = summary?.connectors.filter((connector) => connector.status === "connected").length ?? 0;

  return (
    <TooltipProvider delayDuration={150}>
      <div className="desktop-shell">
        <DesktopToolbar
          activeSection={activeSection}
          dataMode={dataMode}
          hasSummary={Boolean(summary)}
          isExportPanelOpen={isExportPanelOpen}
          isRefreshing={refresh.isPending}
          lastRefresh={summary?.lastRefreshLabel ?? "not yet"}
          theme={theme}
          onDataModeChange={handleDataModeChange}
          onExportBadge={toggleBadgeExportPanel}
          onExportCsv={handleCsvExport}
          onRefresh={refreshNow}
          onSectionChange={setActiveSection}
          onToggleTheme={toggleTheme}
        />
        <main className="desktop-main">
          <DashboardHeader activeSection={activeSection} />
          {isExportPanelOpen && summary ? <ExportPanel summary={summary} onClose={() => setIsExportPanelOpen(false)} /> : null}
          {summary ? (
            <CommandCenterContent
              activeSection={activeSection}
              isRefreshing={refresh.isPending}
              onRefresh={refreshNow}
              summary={summary}
            />
          ) : (
            <DashboardLoading hasError={query.isError} />
          )}
        </main>
        <DesktopStatusBar
          autoRefreshDelayMs={autoRefreshDelayMs}
          connectedSourceCount={connectedSourceCount}
          dataMode={dataMode}
          sourceCount={sourceCount}
          version="v0.1.0"
        />
      </div>
    </TooltipProvider>
  );
}

function DashboardHeader({ activeSection }: { activeSection: NavSection }) {
  const copy = SECTION_COPY[activeSection];

  return (
    <header className="mb-4" id={activeSection}>
      <h1 className="text-[24px] font-semibold leading-tight tracking-normal">{copy.heading}</h1>
      <p className="mt-1 text-sm text-muted-foreground">{copy.description}</p>
    </header>
  );
}

function DashboardLoading({ hasError }: { hasError: boolean }) {
  return (
    <Card className="p-8">
      <h2 className="text-lg font-semibold">{hasError ? "Dashboard data unavailable" : "Loading dashboard"}</h2>
      <p className="mt-2 text-sm text-muted-foreground">{hasError ? "The dashboard keeps existing local data and shows redacted errors only." : "Preparing local dashboard summary."}</p>
    </Card>
  );
}

function CommandCenterContent({
  activeSection,
  isRefreshing,
  onRefresh,
  summary,
}: {
  activeSection: NavSection;
  isRefreshing: boolean;
  onRefresh: () => void;
  summary: DashboardSummary;
}) {
  if (activeSection === "usage") {
    return (
      <div className="space-y-4">
        <MetricStrip summary={summary} />
        <div className="grid grid-cols-[minmax(0,1fr)_minmax(360px,0.9fr)] gap-4 max-[1100px]:grid-cols-1">
          <TokenHeatmap summary={summary} />
          <RateLimitWindows summary={summary} />
        </div>
        <RecentSessions summary={summary} />
      </div>
    );
  }

  if (activeSection === "resets") {
    return (
      <div className="grid grid-cols-[minmax(0,1fr)_minmax(320px,0.55fr)] items-start gap-4 max-[900px]:grid-cols-1">
        <ResetTimeline summary={summary} />
        <NextReset summary={summary} />
      </div>
    );
  }

  if (activeSection === "sources") {
    return (
      <div className="grid grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)] items-start gap-4 max-[900px]:grid-cols-1">
        <SourceCoverage summary={summary} />
        <ActiveConnectors summary={summary} />
      </div>
    );
  }

  if (activeSection === "setup") {
    return <SetupSection dataMode={summary.dataMode} isRefreshing={isRefreshing} onRefresh={onRefresh} summary={summary} />;
  }

  return <DashboardOverview summary={summary} />;
}

function DashboardOverview({ summary }: { summary: DashboardSummary }) {
  return (
    <div className="space-y-4">
      <MetricStrip summary={summary} />
      <div className="grid grid-cols-[minmax(0,1.55fr)_minmax(300px,0.85fr)_minmax(320px,1fr)] items-start gap-4 max-[1280px]:grid-cols-2 max-[900px]:grid-cols-1">
        <TokenHeatmap summary={summary} />
        <ResetTimeline summary={summary} />
        <div className="space-y-4">
          <SourceCoverage summary={summary} />
          <ActiveConnectors summary={summary} />
          <NextReset summary={summary} />
        </div>
      </div>
      <div className="grid grid-cols-[minmax(0,1.1fr)_minmax(360px,0.9fr)] gap-4 max-[1100px]:grid-cols-1">
        <RecentSessions summary={summary} />
        <RateLimitWindows summary={summary} />
      </div>
    </div>
  );
}

function SetupSection({
  dataMode,
  isRefreshing,
  onRefresh,
  summary,
}: {
  dataMode: DataMode;
  isRefreshing: boolean;
  onRefresh: () => void;
  summary: DashboardSummary;
}) {
  const diagnostics = useSetupDiagnostics();

  return (
    <div className="grid grid-cols-[minmax(0,0.95fr)_minmax(320px,0.65fr)] items-start gap-4 max-[900px]:grid-cols-1">
      <div className="space-y-4">
        <Card>
          <CardHeader>
            <CardTitle>Local data</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="text-sm text-muted-foreground">Scan local history and refresh available account snapshots for the selected data mode.</p>
            <Button type="button" onClick={onRefresh} disabled={isRefreshing} aria-label="Scan local data">
              <RefreshCcw size={16} className={cn(isRefreshing && "animate-spin")} aria-hidden />
              Scan local data
            </Button>
          </CardContent>
        </Card>
        <SetupDiagnosticsCard diagnostics={diagnostics.data} isLoading={diagnostics.isLoading} />
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Current configuration</CardTitle>
          <Badge tone="muted">{dataModeLabel(dataMode)}</Badge>
        </CardHeader>
        <CardContent>
          <ul className="divide-y divide-border">
            {summary.connectors.map((connector) => (
              <li key={connector.id} className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
                <div className="min-w-0">
                  <div className="text-sm font-medium">{connector.name}</div>
                  <div className="mt-1 truncate text-xs text-muted-foreground">{connector.detail}</div>
                </div>
                <ConnectorStatusBadge status={connector.status} />
              </li>
            ))}
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}

function SetupDiagnosticsCard({
  diagnostics,
  isLoading,
}: {
  diagnostics?: SetupDiagnostics;
  isLoading: boolean;
}) {
  const latestImport = diagnostics?.latestImportRun;
  const localRoots = diagnostics?.localRoots ?? [];
  const connectorRuns = diagnostics?.connectorRuns ?? [];
  const [exportStatus, setExportStatus] = useState<{ tone: "success" | "warning" | "muted"; message: string } | null>(null);
  const exportDiagnostics = async () => {
    if (!diagnostics) {
      return;
    }
    setExportStatus({ tone: "muted", message: "Saving diagnostics..." });
    const result = await downloadTextFile(
      buildSetupDiagnosticsFilename(),
      buildSetupDiagnosticsJson(diagnostics),
      "application/json;charset=utf-8",
    );
    if (result.status === "saved") {
      setExportStatus({ tone: "success", message: `Saved to ${result.path}` });
      return;
    }
    if (result.status === "downloaded") {
      setExportStatus({ tone: "success", message: "Downloaded diagnostics JSON" });
      return;
    }
    setExportStatus({ tone: "warning", message: `Export failed: ${result.error}` });
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Diagnostics</CardTitle>
        <div className="flex items-center gap-2">
          <Button type="button" variant="secondary" size="sm" onClick={exportDiagnostics} disabled={!diagnostics} aria-label="Export diagnostics">
            <Download size={14} aria-hidden />
            Export diagnostics
          </Button>
          <Badge tone={latestImport ? "success" : "muted"}>{latestImport ? "checked" : "waiting"}</Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {isLoading && !diagnostics ? (
          <p className="text-sm text-muted-foreground">Loading diagnostics</p>
        ) : (
          <>
            <div className="grid grid-cols-2 gap-3 max-[560px]:grid-cols-1">
              <DiagnosticPath label="Database" value={diagnostics?.databasePath ?? "Unavailable"} />
              <DiagnosticPath label="Auth home" value={diagnostics?.authHome ?? "Unavailable"} />
            </div>
            <div>
              <div className="mb-2 text-xs font-medium uppercase tracking-normal text-muted-foreground">Local Codex folders</div>
              <ul className="space-y-2">
                {localRoots.length > 0 ? (
                  localRoots.map((root) => (
                    <li key={root.path} className="flex min-w-0 items-center justify-between gap-3 text-xs">
                      <span className="inline-flex min-w-0 items-center gap-2">
                        <span className={cn("h-2 w-2 shrink-0 rounded-full", root.exists && root.isDirectory ? "bg-mint" : "bg-muted-foreground")} aria-hidden />
                        <span className="truncate font-mono" title={root.path}>{root.path}</span>
                      </span>
                      <Badge tone={root.exists && root.isDirectory ? "success" : "muted"}>{root.exists && root.isDirectory ? "found" : "missing"}</Badge>
                    </li>
                  ))
                ) : (
                  <li className="text-xs text-muted-foreground">No local roots configured</li>
                )}
              </ul>
            </div>
            {latestImport ? (
              <dl className="grid grid-cols-4 gap-2 text-xs max-[560px]:grid-cols-2">
                <DiagnosticCount label="Files" value={latestImport.filesSeen} />
                <DiagnosticCount label="Events" value={latestImport.eventsSeen} />
                <DiagnosticCount label="Imported" value={latestImport.eventsImported} />
                <DiagnosticCount label="Warnings" value={latestImport.warningCount} />
              </dl>
            ) : (
              <p className="text-xs text-muted-foreground">No import run recorded</p>
            )}
            <dl className="grid grid-cols-3 gap-2 text-xs max-[560px]:grid-cols-1">
              <DiagnosticCount label="Stored events" value={diagnostics?.usageEventCount ?? 0} />
              <DiagnosticCount label="Stored tokens" value={diagnostics?.usageTotalTokens ?? 0} />
              <DiagnosticCount label="Source files" value={diagnostics?.sourceDocumentCount ?? 0} />
            </dl>
            {exportStatus ? (
              <div
                className={cn(
                  "rounded-[6px] border px-3 py-2 text-xs",
                  exportStatus.tone === "success" && "border-mint/35 bg-mint/10 text-mint",
                  exportStatus.tone === "warning" && "border-amber/35 bg-amber/10 text-amber",
                  exportStatus.tone === "muted" && "border-border bg-secondary/35 text-muted-foreground",
                )}
                role="status"
              >
                <span className="block truncate" title={exportStatus.message}>
                  {exportStatus.message}
                </span>
              </div>
            ) : null}
            {latestImport?.warningSamples.length ? (
              <div className="space-y-2 border-t border-border pt-3">
                <div className="text-xs font-medium uppercase tracking-normal text-muted-foreground">Warning samples</div>
                <ul className="space-y-1">
                  {latestImport.warningSamples.slice(0, 3).map((warning) => (
                    <li key={warning} className="truncate font-mono text-xs text-muted-foreground" title={warning}>{warning}</li>
                  ))}
                </ul>
              </div>
            ) : null}
            {connectorRuns.length > 0 ? (
              <ul className="space-y-2 border-t border-border pt-3">
                {connectorRuns.map((run) => (
                  <li key={`${run.connectorId}-${run.completedAtUtc}`} className="flex min-w-0 items-center justify-between gap-3 text-xs">
                    <span className="truncate">{run.connectorId}</span>
                    <span className="inline-flex min-w-0 items-center gap-2">
                      <Badge tone={run.status === "complete" || run.status === "connected" ? "success" : "warning"}>{run.status}</Badge>
                      {run.redactedErrorMessage ? <span className="truncate text-muted-foreground" title={run.redactedErrorMessage}>{run.redactedErrorMessage}</span> : null}
                      {run.redactedErrorCode ? <span className="truncate text-muted-foreground">{run.redactedErrorCode}</span> : null}
                    </span>
                  </li>
                ))}
              </ul>
            ) : null}
          </>
        )}
      </CardContent>
    </Card>
  );
}

function DiagnosticPath({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 truncate font-mono text-xs" title={value}>{value}</div>
    </div>
  );
}

function DiagnosticCount({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-[6px] border border-border bg-secondary/35 px-3 py-2">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="mt-1 text-sm font-semibold">{value.toLocaleString()}</dd>
    </div>
  );
}

function MetricStrip({ summary }: { summary: DashboardSummary }) {
  return (
    <section className="grid grid-cols-5 gap-3 max-[1200px]:grid-cols-3 max-[760px]:grid-cols-1" aria-label="Token metrics">
      {summary.metrics.map((metric, index) => {
        const Icon = metricIcons[index] ?? Activity;
        return (
          <Card key={metric.key} className="min-h-[104px]">
            <CardContent className="flex h-full items-center justify-between gap-4 pt-4">
              <div>
                <div className="text-sm text-muted-foreground">{metric.label}</div>
                <div className="mt-2 text-[26px] font-semibold leading-none">{metric.value}</div>
                <div className={cn("mt-2 text-xs", metric.status === "positive" ? "text-mint" : "text-muted-foreground")}>{metric.delta}</div>
              </div>
              <CoverageTooltip coverage={metric.coverage}>
                <div className="grid h-10 w-10 shrink-0 place-items-center rounded-full bg-primary/15 text-primary">
                  <Icon size={19} aria-hidden />
                </div>
              </CoverageTooltip>
            </CardContent>
          </Card>
        );
      })}
    </section>
  );
}

function TokenHeatmap({ summary }: { summary: DashboardSummary }) {
  const months = ["Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb", "Mar", "May", "Jun", "Jul"];
  return (
    <Card className="min-h-[330px]">
      <CardHeader>
        <CardTitle className="inline-flex items-center gap-2">Daily token usage <Info size={15} aria-label="Coverage details available" /></CardTitle>
        <Tabs defaultValue="daily" aria-label="Usage range">
          <TabsList>
            <TabsTrigger value="daily">Daily</TabsTrigger>
            <TabsTrigger value="weekly">Weekly</TabsTrigger>
            <TabsTrigger value="monthly">Monthly</TabsTrigger>
          </TabsList>
        </Tabs>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-[32px_minmax(0,1fr)] gap-3">
          <div className="grid grid-rows-7 gap-1 pt-1 text-xs text-muted-foreground">
            {["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map((day) => <span key={day}>{day}</span>)}
          </div>
          <div className="grid grid-flow-col grid-rows-7 gap-1 overflow-hidden" role="img" aria-label="Daily token usage heatmap">
            {summary.heatmap.map((day) => (
              <Tooltip key={day.date}>
                <TooltipTrigger asChild>
                  <span
                    className={cn("h-3.5 min-w-3 rounded-[3px]", heatmapIntensity(day.intensity))}
                    aria-label={`${day.date}: ${day.tokens.toLocaleString()} tokens`}
                  />
                </TooltipTrigger>
                <TooltipContent>{day.date}: {day.tokens.toLocaleString()} tokens</TooltipContent>
              </Tooltip>
            ))}
          </div>
        </div>
        <div className="ml-11 mt-4 flex justify-between text-xs text-muted-foreground">
          {months.map((month) => <span key={month}>{month}</span>)}
        </div>
        <div className="mt-8 flex items-center justify-between text-xs text-muted-foreground">
          <div className="flex items-center gap-2">
            <span>Less</span>
            {[0, 1, 2, 3, 4, 5].map((level) => <span key={level} className={cn("h-3 w-3 rounded-[3px]", heatmapIntensity(level))} />)}
            <span>More</span>
          </div>
          <span>Timezone: {summary.timezone}</span>
        </div>
      </CardContent>
    </Card>
  );
}

function heatmapIntensity(level: number) {
  return ["bg-muted", "bg-primary/25", "bg-primary/40", "bg-primary/60", "bg-primary/80", "bg-cyan"][level] ?? "bg-muted";
}

function ResetTimeline({ summary }: { summary: DashboardSummary }) {
  return (
    <Card className="min-h-[330px]">
      <CardHeader>
        <CardTitle className="inline-flex items-center gap-2">Reset credit timeline <Info size={15} aria-label="Reset credit coverage" /></CardTitle>
      </CardHeader>
      <CardContent>
        {summary.resetCredits.length > 0 ? (
          <ol className="space-y-5">
            {summary.resetCredits.map((credit) => {
              const { date, time } = splitResetDate(credit.expiresAtNy);
              return (
                <li key={credit.id} className="grid grid-cols-[26px_34px_minmax(0,1fr)_56px] items-start gap-2">
                  <span className="mt-1 h-3 w-3 rounded-full border-2 border-mint" aria-hidden />
                  <span className="text-xl font-semibold">{credit.creditCount}</span>
                  <span className="text-sm">
                    <span className="block text-muted-foreground">Expires {date}</span>
                    <span className="text-xs text-muted-foreground">{time}</span>
                  </span>
                  <Badge tone="success" className="justify-center">{credit.daysRemaining} days</Badge>
                </li>
              );
            })}
          </ol>
        ) : (
          <p className="text-sm text-muted-foreground">No reset-credit snapshot yet.</p>
        )}
        <p className="mt-6 text-xs text-muted-foreground">All times in {summary.timezone}</p>
      </CardContent>
    </Card>
  );
}

function SourceCoverage({ summary }: { summary: DashboardSummary }) {
  const average = summary.coverage.length > 0 ? Math.round(summary.coverage.reduce((total, item) => total + item.coveragePercent, 0) / summary.coverage.length) : 0;
  return (
    <Card>
      <CardHeader>
        <CardTitle className="inline-flex items-center gap-2">Source coverage <Info size={15} aria-label="Source coverage explanation" /></CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-center gap-5 max-[520px]:items-start">
          <div className="grid h-28 w-28 shrink-0 place-items-center rounded-full border-[10px] border-mint/80">
            <div className="text-center">
              <div className="text-2xl font-semibold">{average}%</div>
              <div className="text-[11px] text-muted-foreground">Coverage score</div>
            </div>
          </div>
          <ul className="w-full space-y-2">
            {summary.coverage.map((item) => (
              <li key={item.metricKey} className="flex items-center justify-between gap-3 text-sm">
                <CoverageTooltip coverage={item}>
                  <span className="inline-flex min-w-0 items-center gap-2">
                    <span className="h-2 w-2 rounded-full bg-mint" aria-hidden />
                    <span className="truncate">{item.sourceKind}</span>
                  </span>
                </CoverageTooltip>
                <span className="text-muted-foreground">{item.coveragePercent}%</span>
              </li>
            ))}
          </ul>
        </div>
      </CardContent>
    </Card>
  );
}

function ActiveConnectors({ summary }: { summary: DashboardSummary }) {
  const connected = summary.connectors.filter((connector) => connector.status === "connected").length;
  return (
    <Card>
      <CardHeader>
        <CardTitle className="inline-flex items-center gap-2">Active connectors <Info size={15} aria-label="Connector status details" /></CardTitle>
        <Badge tone="muted">{connected}/{summary.connectors.length}</Badge>
      </CardHeader>
      <CardContent>
        <ul className="divide-y divide-border">
          {summary.connectors.map((connector) => (
            <li key={connector.id} className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
              <div className="min-w-0">
                <div className="flex items-center gap-2 text-sm font-medium">
                  <span className={cn("h-2 w-2 rounded-full", connector.status === "connected" ? "bg-mint" : connector.status === "degraded" ? "bg-amber" : "bg-muted-foreground")} aria-hidden />
                  {connector.name}
                </div>
                <div className="mt-1 truncate text-xs text-muted-foreground">{connector.detail}</div>
              </div>
              <ConnectorStatusBadge status={connector.status} />
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
}

function ConnectorStatusBadge({ status }: { status: DashboardSummary["connectors"][number]["status"] }) {
  if (status === "connected") {
    return <Badge tone="success" className="whitespace-nowrap">Connected</Badge>;
  }
  if (status === "degraded") {
    return <Badge tone="warning" className="whitespace-nowrap">Needs attention</Badge>;
  }
  return <Badge tone="muted" className="whitespace-nowrap">Needs setup</Badge>;
}

function NextReset({ summary }: { summary: DashboardSummary }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="inline-flex items-center gap-2">Next reset credit expires <Info size={15} aria-label="Next reset expiration timezone" /></CardTitle>
      </CardHeader>
      <CardContent className="text-center">
        <div className="text-3xl font-semibold text-mint">{summary.nextReset.label}</div>
        <p className="mt-2 text-sm text-muted-foreground">{summary.nextReset.expiresAtNy}</p>
        <p className="mt-6 text-xs text-muted-foreground">Timezone: {summary.nextReset.timezone}</p>
      </CardContent>
    </Card>
  );
}

function RecentSessions({ summary }: { summary: DashboardSummary }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="inline-flex items-center gap-2">Recent sessions <Info size={15} aria-label="Recent session source labels" /></CardTitle>
      </CardHeader>
      <CardContent>
        {summary.sessions.length > 0 ? (
          <>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Start time</TableHead>
                  <TableHead>Duration</TableHead>
                  <TableHead>Tokens</TableHead>
                  <TableHead>Peak tokens</TableHead>
                  <TableHead>Mode</TableHead>
                  <TableHead>Source</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {summary.sessions.map((session) => (
                  <TableRow key={session.id}>
                    <TableCell>{session.startTime}</TableCell>
                    <TableCell>{session.duration}</TableCell>
                    <TableCell>{session.tokens}</TableCell>
                    <TableCell>{session.peakTokens}</TableCell>
                    <TableCell><Badge tone="source">{session.mode}</Badge></TableCell>
                    <TableCell className="space-x-1">{session.sources.map((source) => <Badge key={source} tone={source === "CLI" ? "success" : "source"}>{source}</Badge>)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            <div className="mt-3 text-xs text-muted-foreground">Showing {summary.sessions.length} {summary.sessions.length === 1 ? "session" : "sessions"}</div>
          </>
        ) : (
          <p className="text-sm text-muted-foreground">No sessions imported yet.</p>
        )}
      </CardContent>
    </Card>
  );
}

function RateLimitWindows({ summary }: { summary: DashboardSummary }) {
  const progress = summary.rateLimitWindows.length > 0
    ? Math.round(summary.rateLimitWindows.reduce((total, window) => total + window.progressPercent, 0) / summary.rateLimitWindows.length)
    : 0;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="inline-flex items-center gap-2">Rate-limit windows <Info size={15} aria-label="Rate-limit window coverage" /></CardTitle>
      </CardHeader>
      <CardContent>
        {summary.rateLimitWindows.length > 0 ? (
          <>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Window</TableHead>
                  <TableHead>Limit</TableHead>
                  <TableHead>Used</TableHead>
                  <TableHead>Remaining</TableHead>
                  <TableHead>Resets in</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {summary.rateLimitWindows.map((window) => (
                  <TableRow key={window.id}>
                    <TableCell>{window.window}</TableCell>
                    <TableCell>{window.limit}</TableCell>
                    <TableCell>{window.used}</TableCell>
                    <TableCell>{window.remaining}</TableCell>
                    <TableCell>{window.resetsIn}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            <div className="mt-4 flex items-center gap-3 text-xs text-muted-foreground">
              <span>Overall</span>
              <Progress value={progress} className="max-w-[220px]" />
              <span>{progress}%</span>
            </div>
          </>
        ) : (
          <p className="text-sm text-muted-foreground">No rate-limit window snapshots yet.</p>
        )}
      </CardContent>
    </Card>
  );
}

function CoverageTooltip({ coverage, children }: { coverage: MetricCoverage; children: ReactNode }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent>
        <div className="font-medium">{coverage.sourceKind}: {coverage.coveragePercent}%</div>
        <div className="mt-1 text-muted-foreground">{coverage.explanation}</div>
        <div className="mt-2 text-muted-foreground">Formula: {coverage.formulaVersion}</div>
      </TooltipContent>
    </Tooltip>
  );
}

function splitResetDate(value: string) {
  const parts = value.split(", ");
  if (parts.length >= 3) {
    return {
      date: `${parts[0]}, ${parts[1]}`,
      time: parts.slice(2).join(", "),
    };
  }
  return { date: value, time: "" };
}

function dataModeLabel(dataMode: DataMode) {
  if (dataMode === "local") {
    return "Local";
  }
  if (dataMode === "remote") {
    return "Remote";
  }
  return "Local + Remote";
}
