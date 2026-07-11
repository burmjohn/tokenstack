import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import {
  dashboardSummarySchema,
  setupDiagnosticsSchema,
  type DashboardSummary,
  type DataMode,
  type SetupDiagnostics,
  codexRuntimeCandidateSchema,
  codexRuntimeValidationSchema,
  type CodexRuntimeCandidate,
  type CodexRuntimeSelection,
  type CodexRuntimeValidation,
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
  dataMode: DataMode,
): Promise<TextDownloadResult> {
  if (!isTauriRuntime()) {
    return downloadTextFile(
      buildSetupDiagnosticsFilename(),
      buildSetupDiagnosticsJson(diagnostics),
      "application/json;charset=utf-8",
    );
  }

  try {
    const path = await invoke<string>("export_diagnostics", { dataMode });
    return { status: "saved", path };
  } catch (error) {
    return { status: "failed", error: error instanceof Error ? error.message : String(error) };
  }
}

export async function listCodexRuntimes(): Promise<CodexRuntimeCandidate[]> {
  if (!isTauriRuntime()) {
    return [];
  }
  const payload = await invoke("list_codex_runtimes");
  return z.array(codexRuntimeCandidateSchema).parse(payload);
}

export async function chooseCodexRuntime(): Promise<CodexRuntimeValidation> {
  return codexRuntimeValidationSchema.parse(await invoke("choose_codex_runtime"));
}

export async function selectCodexRuntime(selection: CodexRuntimeSelection): Promise<CodexRuntimeValidation> {
  return codexRuntimeValidationSchema.parse(await invoke("select_codex_runtime", { selection }));
}

export async function clearCodexRuntime(dataMode: DataMode): Promise<DashboardSummary> {
  return dashboardSummarySchema.parse(await invoke("clear_codex_runtime", { dataMode }));
}

export async function validateCodexRuntime(): Promise<CodexRuntimeValidation> {
  return codexRuntimeValidationSchema.parse(await invoke("validate_codex_runtime"));
}
