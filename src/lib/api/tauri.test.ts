import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { chooseCodexRuntime, clearCodexRuntime, exportSetupDiagnostics, listCodexRuntimes, selectCodexRuntime, validateCodexRuntime } from "./tauri";
import { createMockSetupDiagnostics } from "./mockData";

describe("Codex runtime Tauri API", () => {
  beforeEach(() => {
    invoke.mockReset();
    Object.assign(window, { __TAURI_INTERNALS__: {} });
  });

  it("preserves the typed native executable and fixed npm argument prefix", async () => {
    const candidate = {
      displayPath: "C:\\Users\\Test\\AppData\\Roaming\\npm\\codex.cmd",
      source: "npm" as const,
      exists: true,
      executable: true,
      version: "codex 1.2.3",
      validationError: null,
      configured: true,
      selected: true,
    };
    invoke.mockResolvedValueOnce([candidate]);
    invoke.mockResolvedValueOnce({ valid: true, version: "codex 1.2.3", error: null });

    expect((await listCodexRuntimes())[0]).toMatchObject({ source: "npm", configured: true, selected: true });
    expect(await selectCodexRuntime({ displayPath: candidate.displayPath })).toEqual({ valid: true, version: "codex 1.2.3", error: null });
    expect(invoke).toHaveBeenLastCalledWith("select_codex_runtime", { selection: { displayPath: candidate.displayPath } });
  });

  it("asks Rust to pick and validate without exposing a filesystem path to the renderer", async () => {
    invoke.mockResolvedValue({ valid: true, version: "codex 1.2.3", error: null });
    await expect(chooseCodexRuntime()).resolves.toMatchObject({ valid: true });
    expect(invoke).toHaveBeenCalledWith("choose_codex_runtime");
  });

  it("uses dedicated commands to clear and test the configured connection", async () => {
    const { createMockDashboardSummary } = await import("./mockData");
    invoke.mockResolvedValueOnce(createMockDashboardSummary("combined"));
    invoke.mockResolvedValueOnce({ valid: false, version: null, error: "logged out" });

    await clearCodexRuntime("combined");
    await expect(validateCodexRuntime()).resolves.toEqual({ valid: false, version: null, error: "logged out" });
    expect(invoke.mock.calls).toEqual([
      ["clear_codex_runtime", { dataMode: "combined" }],
      ["validate_codex_runtime"],
    ]);
  });

  it("passes the active strict data mode to the backend diagnostics export", async () => {
    invoke.mockResolvedValue("C:\\TokenStack\\diagnostics\\report.json");
    await expect(exportSetupDiagnostics(createMockSetupDiagnostics(), "remote")).resolves.toEqual({
      status: "saved", path: "C:\\TokenStack\\diagnostics\\report.json",
    });
    expect(invoke).toHaveBeenCalledWith("export_diagnostics", { dataMode: "remote" });
  });
});
