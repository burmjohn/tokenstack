import { invoke } from "@tauri-apps/api/core";
import {
  dashboardSummarySchema,
  setupDiagnosticsSchema,
  type DashboardSummary,
  type DataMode,
  type SetupDiagnostics,
} from "../schemas/dashboard";
import { createMockDashboardSummary, createMockSetupDiagnostics } from "./mockData";

type TauriWindow = Window & {
  __TAURI_INTERNALS__?: unknown;
};

export function isTauriRuntime() {
  return typeof window !== "undefined" && Boolean((window as TauriWindow).__TAURI_INTERNALS__);
}

export async function getDashboardSummary(dataMode: DataMode): Promise<DashboardSummary> {
  if (!isTauriRuntime()) {
    return createMockDashboardSummary(dataMode);
  }

  const payload = await invoke("get_dashboard_summary", { dataMode });
  return dashboardSummarySchema.parse(payload);
}

export async function refreshAll(dataMode: DataMode): Promise<DashboardSummary> {
  if (!isTauriRuntime()) {
    await new Promise((resolve) => setTimeout(resolve, 120));
    return createMockDashboardSummary(dataMode);
  }

  const payload = await invoke("refresh_all", { dataMode });
  return dashboardSummarySchema.parse(payload);
}

export async function getSetupDiagnostics(): Promise<SetupDiagnostics> {
  if (!isTauriRuntime()) {
    return createMockSetupDiagnostics();
  }

  const payload = await invoke("get_setup_diagnostics");
  return setupDiagnosticsSchema.parse(payload);
}
