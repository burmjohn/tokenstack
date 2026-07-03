import type { DataMode } from "../../lib/schemas/dashboard";

type DesktopStatusBarProps = {
  autoRefreshDelayMs: number;
  connectedSourceCount: number;
  dataMode: DataMode;
  sourceCount: number;
  version: string;
};

export function DesktopStatusBar({
  autoRefreshDelayMs,
  connectedSourceCount,
  dataMode,
  sourceCount,
  version,
}: DesktopStatusBarProps) {
  return (
    <footer className="desktop-statusbar" role="status" aria-label="TokenStack status">
      <span>Mode: {dataModeLabel(dataMode)}</span>
      <span>Auto refresh: {Math.round(autoRefreshDelayMs / 1000)}s</span>
      <span>
        Sources: {connectedSourceCount}/{sourceCount}
      </span>
      <span>Version {version}</span>
    </footer>
  );
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
