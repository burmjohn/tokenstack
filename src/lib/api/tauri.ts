import { invoke } from "@tauri-apps/api/core";
import {
  dashboardSummarySchema,
  setupDiagnosticsSchema,
  type DashboardSummary,
  type DataMode,
  type SetupDiagnostics,
} from "../schemas/dashboard";
import { buildSetupDiagnosticsFilename, buildSetupDiagnosticsJson } from "../../features/exports/diagnostics";
import { downloadTextFile, type TextDownloadResult } from "../../features/exports/download";
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

export async function exportSetupDiagnostics(
  diagnostics: SetupDiagnostics,
): Promise<TextDownloadResult> {
  if (!isTauriRuntime()) {
    return downloadTextFile(
      buildSetupDiagnosticsFilename(),
      buildSetupDiagnosticsJson(diagnostics),
      "application/json;charset=utf-8",
    );
  }

  try {
    const path = await invoke<string>("export_diagnostics");
    return { status: "saved", path };
  } catch (error) {
    return { status: "failed", error: error instanceof Error ? error.message : String(error) };
  }
}
