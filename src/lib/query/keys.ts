import type { DataMode } from "../schemas/dashboard";

export const queryKeys = {
  dashboard: {
    all: ["dashboard"] as const,
    summary: (dataMode: DataMode) => ["dashboard", "summary", dataMode] as const,
  },
  usage: {
    all: ["usage"] as const,
    daily: (range: string, dataMode: DataMode) => ["usage", "daily", range, dataMode] as const,
    monthly: (range: string, dataMode: DataMode) => ["usage", "monthly", range, dataMode] as const,
  },
  sessions: {
    recent: (dataMode: DataMode) => ["sessions", "recent", dataMode] as const,
  },
  resetCredits: {
    timeline: (dataMode: DataMode) => ["resetCredits", "timeline", dataMode] as const,
  },
  rateLimits: {
    windows: (dataMode: DataMode) => ["rateLimits", "windows", dataMode] as const,
  },
  sources: {
    coverage: (dataMode: DataMode) => ["sources", "coverage", dataMode] as const,
  },
  connectors: {
    status: () => ["connectors", "status"] as const,
  },
  diagnostics: {
    setup: () => ["diagnostics", "setup"] as const,
  },
  runtimes: {
    codex: () => ["runtimes", "codex"] as const,
  },
  refresh: {
    status: () => ["refresh", "status"] as const,
  },
};
