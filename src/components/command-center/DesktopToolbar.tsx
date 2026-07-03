import { Database, Download, ImageDown, Moon, RefreshCcw, Sun } from "lucide-react";
import type { DataMode } from "../../lib/schemas/dashboard";
import { cn } from "../../lib/utils";
import { Button } from "../ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";
import { NAV_ITEMS, type NavSection } from "./sectionModel";

type Theme = "dark" | "light";

type DesktopToolbarProps = {
  activeSection: NavSection;
  dataMode: DataMode;
  hasSummary: boolean;
  isExportPanelOpen: boolean;
  isRefreshing: boolean;
  lastRefresh: string;
  theme: Theme;
  onDataModeChange: (mode: DataMode) => void;
  onExportBadge: () => void;
  onExportCsv: () => void;
  onRefresh: () => void;
  onSectionChange: (section: NavSection) => void;
  onToggleTheme: () => void;
};

export function DesktopToolbar({
  activeSection,
  dataMode,
  hasSummary,
  isExportPanelOpen,
  isRefreshing,
  lastRefresh,
  theme,
  onDataModeChange,
  onExportBadge,
  onExportCsv,
  onRefresh,
  onSectionChange,
  onToggleTheme,
}: DesktopToolbarProps) {
  return (
    <header className="desktop-toolbar" role="toolbar" aria-label="TokenStack controls">
      <div className="desktop-toolbar__brand">
        <span className="desktop-toolbar__icon" aria-hidden>
          <Database size={18} />
        </span>
        <span className="desktop-toolbar__title">TokenStack</span>
        <span className="desktop-toolbar__refresh">Updated {lastRefresh}</span>
      </div>

      <nav className="desktop-toolbar__nav" aria-label="Sections">
        {NAV_ITEMS.map((item) => {
          const Icon = item.icon;
          const isActive = activeSection === item.id;
          return (
            <button
              key={item.id}
              type="button"
              aria-current={isActive ? "page" : undefined}
              className={cn("desktop-toolbar__nav-button", isActive && "desktop-toolbar__nav-button--active")}
              onClick={() => onSectionChange(item.id)}
            >
              <Icon size={15} aria-hidden />
              <span>{item.label}</span>
            </button>
          );
        })}
      </nav>

      <div className="desktop-toolbar__actions">
        <label className="sr-only" htmlFor="desktop-data-mode">
          Data mode
        </label>
        <select
          id="desktop-data-mode"
          value={dataMode}
          onChange={(event) => onDataModeChange(event.target.value as DataMode)}
          className="desktop-toolbar__select"
          aria-label="Data mode"
        >
          <option value="combined">Local + Remote</option>
          <option value="local">Local</option>
          <option value="remote">Remote</option>
        </select>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon" aria-label="Refresh data" disabled={isRefreshing} onClick={onRefresh}>
              <RefreshCcw size={17} className={cn(isRefreshing && "animate-spin")} aria-hidden />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Refresh local imports and available snapshots.</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <span className="inline-flex" aria-label={!hasSummary ? "Export badge requires loaded dashboard data" : undefined} tabIndex={!hasSummary ? 0 : undefined}>
              <Button
                variant="secondary"
                size="icon"
                aria-label="Export badge"
                disabled={!hasSummary}
                onClick={onExportBadge}
                className={cn(isExportPanelOpen && "border-primary/60 bg-primary/15 text-primary")}
              >
                <ImageDown size={16} aria-hidden />
              </Button>
            </span>
          </TooltipTrigger>
          <TooltipContent>{hasSummary ? "Create a shareable TokenStack badge." : "Exports require loaded dashboard data."}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <span className="inline-flex" aria-label={!hasSummary ? "Export usage CSV requires loaded dashboard data" : undefined} tabIndex={!hasSummary ? 0 : undefined}>
              <Button variant="secondary" size="icon" aria-label="Export usage CSV" disabled={!hasSummary} onClick={onExportCsv}>
                <Download size={16} aria-hidden />
              </Button>
            </span>
          </TooltipTrigger>
          <TooltipContent>{hasSummary ? "Export dashboard usage bundle." : "Exports require loaded dashboard data."}</TooltipContent>
        </Tooltip>

        <Button variant="secondary" size="icon" aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} theme`} onClick={onToggleTheme}>
          {theme === "dark" ? <Sun size={16} aria-hidden /> : <Moon size={16} aria-hidden />}
        </Button>
      </div>
    </header>
  );
}
