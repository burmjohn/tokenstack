import {
  Activity,
  BarChart3,
  CalendarDays,
  ChevronDown,
  Database,
  ExternalLink,
  Gauge,
  Github,
  Info,
  LayoutDashboard,
  LockKeyhole,
  Moon,
  RefreshCcw,
  ServerCog,
  ShieldCheck,
  Sun,
  Zap,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";
import { Progress } from "../ui/progress";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../ui/table";
import { Tabs, TabsList, TabsTrigger } from "../ui/tabs";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "../ui/tooltip";
import { useDashboardSummary, useRefreshAll } from "../../features/dashboard/useDashboardSummary";
import type { DataMode, DashboardSummary, MetricCoverage } from "../../lib/schemas/dashboard";
import { cn } from "../../lib/utils";

type Theme = "dark" | "light";

const metricIcons = [Activity, CalendarDays, BarChart3, Zap, RefreshCcw];
const AUTO_REFRESH_BASE_DELAY_MS = 60_000;
const AUTO_REFRESH_MAX_DELAY_MS = 300_000;

export function CommandCenterShell() {
  const [theme, setTheme] = useState<Theme>(() => (localStorage.getItem("tokenstack-theme") as Theme | null) ?? "dark");
  const [dataMode, setDataMode] = useState<DataMode>("combined");
  const [autoRefreshDelayMs, setAutoRefreshDelayMs] = useState(AUTO_REFRESH_BASE_DELAY_MS);
  const query = useDashboardSummary(dataMode);
  const refresh = useRefreshAll(dataMode);
  const refreshPendingRef = useRef(false);
  const refreshMutationRef = useRef(refresh.mutateAsync);
  const handleDataModeChange = (mode: DataMode) => {
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

  return (
    <TooltipProvider delayDuration={150}>
      <div className="min-h-screen bg-background text-foreground">
        <div className="grid min-h-screen grid-cols-[196px_minmax(0,1fr)] grid-rows-[1fr_auto] max-[980px]:grid-cols-1">
          <Sidebar dataMode={dataMode} setDataMode={handleDataModeChange} autoRefreshDelayMs={autoRefreshDelayMs} />
          <main className="min-w-0 px-6 py-6 max-[980px]:px-4">
            <DashboardHeader
              dataMode={dataMode}
              setDataMode={handleDataModeChange}
              theme={theme}
              setTheme={setTheme}
              lastRefresh={summary?.lastRefreshLabel ?? "not yet"}
              isRefreshing={refresh.isPending}
              onRefresh={() => refresh.mutate()}
            />
            {summary ? <DashboardContent summary={summary} /> : <DashboardLoading hasError={query.isError} />}
          </main>
          <SafetyFooter />
        </div>
      </div>
    </TooltipProvider>
  );
}

function Sidebar({
  dataMode,
  setDataMode,
  autoRefreshDelayMs,
}: {
  dataMode: DataMode;
  setDataMode: (mode: DataMode) => void;
  autoRefreshDelayMs: number;
}) {
  return (
    <aside className="row-span-2 border-r border-border bg-sidebar p-3 max-[980px]:hidden" aria-label="Primary">
      <div className="mb-6 flex h-12 items-center gap-3 px-2">
        <div className="grid h-8 w-8 place-items-center rounded-[8px] bg-primary/15 text-primary">
          <Database aria-hidden size={20} />
        </div>
        <div className="text-lg font-semibold">TokenStack</div>
      </div>
      <nav className="space-y-1">
        {[
          ["Dashboard", LayoutDashboard],
          ["Usage", BarChart3],
          ["Reset Credits", RefreshCcw],
          ["Sources", Database],
          ["Settings", ServerCog],
        ].map(([label, Icon], index) => (
          <a
            key={label as string}
            className={cn("flex h-11 items-center gap-3 rounded-[8px] px-3 text-sm text-muted-foreground", index === 0 && "bg-primary/15 text-foreground")}
            href="#dashboard"
            aria-current={index === 0 ? "page" : undefined}
          >
            <Icon aria-hidden size={18} />
            {label as string}
          </a>
        ))}
      </nav>
      <div className="mt-auto flex min-h-[590px] flex-col justify-end gap-3">
        <label className="rounded-[8px] border border-border bg-card p-3 text-xs text-muted-foreground">
          <span className="mb-2 flex items-center justify-between">
            Data mode <span className="inline-flex items-center gap-1 text-mint">Live</span>
          </span>
          <select
            value={dataMode}
            onChange={(event) => setDataMode(event.target.value as DataMode)}
            className="w-full rounded-[6px] border border-border bg-background px-2 py-1.5 text-xs text-foreground"
            aria-label="Data mode"
          >
            <option value="combined">Local + Remote</option>
            <option value="local">Local</option>
            <option value="remote">Remote</option>
          </select>
        </label>
        <div className="rounded-[8px] border border-border bg-card p-3 text-xs text-muted-foreground">
          <div className="flex items-center justify-between">
            <span>Auto refresh</span>
            <span className="inline-flex items-center gap-1">{Math.round(autoRefreshDelayMs / 1000)}s <ChevronDown size={13} aria-hidden /></span>
          </div>
        </div>
        <div className="rounded-[8px] border border-border bg-card p-3 text-xs text-muted-foreground">
          <div className="flex items-center justify-between">
            <span>Version</span>
            <span>v0.1.0</span>
          </div>
        </div>
        <a className="flex h-10 items-center justify-between rounded-[8px] border border-border bg-card px-3 text-sm font-medium" href="https://github.com/burmjohn/tokenstack">
          <span className="inline-flex items-center gap-2"><Github size={17} aria-hidden /> Star</span>
          <span>1.2k</span>
        </a>
      </div>
    </aside>
  );
}

function DashboardHeader({
  dataMode,
  setDataMode,
  theme,
  setTheme,
  lastRefresh,
  isRefreshing,
  onRefresh,
}: {
  dataMode: DataMode;
  setDataMode: (mode: DataMode) => void;
  theme: Theme;
  setTheme: (theme: Theme) => void;
  lastRefresh: string;
  isRefreshing: boolean;
  onRefresh: () => void;
}) {
  return (
    <header className="mb-6 flex flex-wrap items-start justify-between gap-4" id="dashboard">
      <div>
        <h1 className="text-[26px] font-semibold leading-tight tracking-normal">Dashboard</h1>
        <p className="mt-2 text-sm text-muted-foreground">Local Codex usage, resets, and source intelligence. Always read-only.</p>
      </div>
      <div className="flex flex-wrap items-center justify-end gap-3">
        <span className="text-xs text-muted-foreground">Last refresh: {lastRefresh} <span className="text-mint">●</span></span>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon" aria-label="Refresh now" disabled={isRefreshing} onClick={onRefresh}>
              <RefreshCcw size={17} className={cn(isRefreshing && "animate-spin")} aria-hidden />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Refresh local import and read-only connector snapshots.</TooltipContent>
        </Tooltip>
        <Badge tone="success" className="h-9 gap-2 px-3"><ShieldCheck size={15} aria-hidden /> Read-only</Badge>
        <Badge className="h-9 gap-2 px-3"><LockKeyhole size={14} aria-hidden /> Never /consume</Badge>
        <label className="sr-only" htmlFor="header-data-mode">Data mode</label>
        <select
          id="header-data-mode"
          value={dataMode}
          onChange={(event) => setDataMode(event.target.value as DataMode)}
          className="h-9 rounded-[8px] border border-primary/40 bg-primary/10 px-3 text-sm text-primary"
        >
          <option value="combined">Local + Remote</option>
          <option value="local">Local</option>
          <option value="remote">Remote</option>
        </select>
        <Button variant="secondary" size="icon" aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} theme`} onClick={() => setTheme(theme === "dark" ? "light" : "dark")}>
          {theme === "dark" ? <Sun size={16} aria-hidden /> : <Moon size={16} aria-hidden />}
        </Button>
        <div className="flex h-10 items-center gap-3 rounded-[8px] border border-border bg-card px-3">
          <div className="grid h-7 w-7 place-items-center rounded-full bg-primary text-primary-foreground text-xs">JB</div>
          <div className="leading-tight">
            <div className="text-sm font-medium">John B</div>
            <div className="text-xs text-muted-foreground">@burmjohn</div>
          </div>
        </div>
      </div>
    </header>
  );
}

function DashboardLoading({ hasError }: { hasError: boolean }) {
  return (
    <Card className="p-8">
      <h2 className="text-lg font-semibold">{hasError ? "Dashboard data unavailable" : "Loading dashboard"}</h2>
      <p className="mt-2 text-sm text-muted-foreground">{hasError ? "The dashboard keeps existing local data and shows redacted errors only." : "Preparing local read-only summary."}</p>
    </Card>
  );
}

function DashboardContent({ summary }: { summary: DashboardSummary }) {
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
  const months = ["Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul"];
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
        <ol className="space-y-5">
          {summary.resetCredits.map((credit) => (
            <li key={credit.id} className="grid grid-cols-[26px_34px_minmax(0,1fr)_56px] items-start gap-2">
              <span className="mt-1 h-3 w-3 rounded-full border-2 border-mint" aria-hidden />
              <span className="text-xl font-semibold">{credit.creditCount}</span>
              <span className="text-sm">
                <span className="block text-muted-foreground">Expires {credit.expiresAtNy.split(", 2:")[0]}</span>
                <span className="text-xs text-muted-foreground">2:14 PM EDT</span>
              </span>
              <Badge tone="success" className="justify-center">{credit.daysRemaining} days</Badge>
            </li>
          ))}
        </ol>
        <p className="mt-6 text-xs text-muted-foreground">All times in {summary.timezone}</p>
      </CardContent>
    </Card>
  );
}

function SourceCoverage({ summary }: { summary: DashboardSummary }) {
  const average = Math.round(summary.coverage.reduce((total, item) => total + item.coveragePercent, 0) / summary.coverage.length);
  return (
    <Card>
      <CardHeader>
        <CardTitle className="inline-flex items-center gap-2">Source coverage <Info size={15} aria-label="Source coverage explanation" /></CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-center gap-5">
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
  return (
    <Card>
      <CardHeader>
        <CardTitle className="inline-flex items-center gap-2">Active connectors <Info size={15} aria-label="Connector status details" /></CardTitle>
        <Badge tone="muted">{summary.connectors.length}/3</Badge>
      </CardHeader>
      <CardContent>
        <ul className="divide-y divide-border">
          {summary.connectors.map((connector) => (
            <li key={connector.id} className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
              <div className="min-w-0">
                <div className="flex items-center gap-2 text-sm font-medium">
                  <span className="h-2 w-2 rounded-full bg-mint" aria-hidden />
                  {connector.name}
                </div>
                <div className="mt-1 truncate text-xs text-muted-foreground">{connector.detail}</div>
              </div>
              <Badge tone="source">{connector.safetyClass}</Badge>
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
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
        <div className="mt-3 text-xs text-muted-foreground">Showing 1 to 5 of 28 sessions</div>
      </CardContent>
    </Card>
  );
}

function RateLimitWindows({ summary }: { summary: DashboardSummary }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="inline-flex items-center gap-2">Rate-limit windows <Info size={15} aria-label="Rate-limit window coverage" /></CardTitle>
      </CardHeader>
      <CardContent>
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
          <span>Overall (30d)</span>
          <Progress value={18} className="max-w-[220px]" />
          <span>18%</span>
        </div>
      </CardContent>
    </Card>
  );
}

function CoverageTooltip({ coverage, children }: { coverage: MetricCoverage; children: React.ReactNode }) {
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

function SafetyFooter() {
  return (
    <footer className="col-span-2 flex h-16 items-center justify-around border-t border-border bg-sidebar px-6 text-sm text-muted-foreground max-[980px]:col-span-1 max-[760px]:h-auto max-[760px]:flex-col max-[760px]:items-start max-[760px]:gap-3 max-[760px]:py-4">
      <span className="inline-flex items-center gap-2"><LockKeyhole size={16} aria-hidden /> All data is read-only</span>
      <span className="inline-flex items-center gap-2"><ShieldCheck size={16} aria-hidden /> We never consume credits or tokens</span>
      <span className="inline-flex items-center gap-2"><Gauge size={16} aria-hidden /> Open source</span>
      <span className="inline-flex items-center gap-2"><Info size={16} aria-hidden /> MIT License</span>
      <a className="inline-flex items-center gap-2 text-primary" href="https://github.com/burmjohn/tokenstack">GitHub Repository <ExternalLink size={15} aria-hidden /></a>
    </footer>
  );
}
