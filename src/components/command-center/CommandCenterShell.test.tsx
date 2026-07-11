import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { DesktopMenuCommand } from "../../features/desktop/commands";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { CommandCenterShell } from "./CommandCenterShell";

const desktopBridge = vi.hoisted(() => ({
  handler: undefined as ((command: DesktopMenuCommand) => void) | undefined,
}));

const runtimeApi = vi.hoisted(() => ({
  chooseCodexRuntime: vi.fn(),
  clearCodexRuntime: vi.fn(),
  getDashboardSummary: vi.fn(),
  listCodexRuntimes: vi.fn(),
  refreshAll: vi.fn(),
  selectCodexRuntime: vi.fn(),
  validateCodexRuntime: vi.fn(),
  exportSetupDiagnostics: vi.fn(),
}));

vi.mock("../../lib/api/tauri", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../lib/api/tauri")>()),
  ...runtimeApi,
}));

vi.mock("../../features/desktop/commands", () => ({
  listenForDesktopMenuCommands: vi.fn(async (handler) => {
    desktopBridge.handler = handler;
    return vi.fn();
  }),
}));

vi.mock("../../features/desktop/contextMenu", () => ({
  installDesktopContextMenu: vi.fn(() => vi.fn()),
}));

const originalCreateObjectURL = URL.createObjectURL;
const originalRevokeObjectURL = URL.revokeObjectURL;

beforeEach(async () => {
  runtimeApi.chooseCodexRuntime.mockReset().mockResolvedValue({ valid: false, version: null, error: "selection cancelled" });
  runtimeApi.clearCodexRuntime.mockReset().mockResolvedValue(undefined);
  const { createMockDashboardSummary } = await import("../../lib/api/mockData");
  runtimeApi.getDashboardSummary.mockReset().mockImplementation((mode) => Promise.resolve(createMockDashboardSummary(mode)));
  runtimeApi.listCodexRuntimes.mockReset().mockResolvedValue([]);
  runtimeApi.selectCodexRuntime.mockReset().mockResolvedValue({ valid: true, version: "codex 1.2.3", error: null });
  runtimeApi.validateCodexRuntime.mockReset().mockResolvedValue({ valid: true, version: "codex 1.2.3", error: null });
  runtimeApi.exportSetupDiagnostics.mockReset().mockResolvedValue({ status: "saved", path: "C:\\TokenStack\\diagnostics\\report.json" });
  runtimeApi.refreshAll.mockReset().mockImplementation(async (mode) => {
    await new Promise((resolve) => setTimeout(resolve, 120));
    return createMockDashboardSummary(mode);
  });
});

afterEach(() => {
  desktopBridge.handler = undefined;
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  restoreUrlApi("createObjectURL", originalCreateObjectURL);
  restoreUrlApi("revokeObjectURL", originalRevokeObjectURL);
});

function renderShell() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <CommandCenterShell />
    </QueryClientProvider>,
  );
}

describe("CommandCenterShell", () => {
  it("renders account and local history together in combined mode", async () => {
    const { createMockDashboardSummary } = await import("../../lib/api/mockData");
    const summary = createMockDashboardSummary("combined");
    summary.accountMetrics = [
      { ...summary.metrics[0], key: "account-lifetime", label: "Account lifetime", value: "999" },
      { ...summary.metrics[1], key: "account-today", label: "Account today", value: "30" },
      { ...summary.metrics[2], key: "account-month", label: "Account this month", value: "400" },
    ];
    summary.localMetrics = [
      { ...summary.metrics[0], key: "local-lifetime", label: "Local lifetime", value: "175" },
      { ...summary.metrics[1], key: "local-today", label: "Local today", value: "100" },
      { ...summary.metrics[2], key: "local-month", label: "Local this month", value: "150" },
      { ...summary.metrics[3], key: "local-peak", label: "Local peak session", value: "100" },
    ];
    runtimeApi.getDashboardSummary.mockResolvedValue(summary);

    renderShell();

    expect(await screen.findByRole("heading", { name: "Account usage" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Local history" })).toBeInTheDocument();
    expect(screen.getByText("999")).toBeInTheDocument();
    expect(screen.getByText("175")).toBeInTheDocument();
    expect(screen.getByText("Local token history from this device; not account-wide usage.")).toBeInTheDocument();
  });

  it("keeps local history visible when account usage is unavailable", async () => {
    const { createMockDashboardSummary } = await import("../../lib/api/mockData");
    const summary = createMockDashboardSummary("combined");
    summary.accountMetrics = [];
    summary.localMetrics = [{ ...summary.metrics[0], key: "local-lifetime", label: "Local lifetime", value: "175" }];
    const account = summary.connectors.find((connector) => connector.id === "account-usage");
    if (account) Object.assign(account, { status: "degraded", detail: "Account usage unavailable" });
    else summary.connectors.push({ ...summary.connectors[0], id: "account-usage", name: "Account usage", detail: "Account usage unavailable", status: "degraded" });
    runtimeApi.getDashboardSummary.mockResolvedValue(summary);

    renderShell();

    expect(await screen.findByRole("heading", { name: "Account usage unavailable" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Local history" })).toBeInTheDocument();
    expect(screen.getByText("175")).toBeInTheDocument();
  });

  it("marks last-good account usage stale without degrading local history", async () => {
    const { createMockDashboardSummary } = await import("../../lib/api/mockData");
    const summary = createMockDashboardSummary("combined");
    summary.accountMetrics = [{ ...summary.metrics[0], key: "account-lifetime", label: "Account lifetime", value: "999" }];
    summary.localMetrics = [{ ...summary.metrics[0], key: "local-lifetime", label: "Local lifetime", value: "175" }];
    summary.connectors.push({ ...summary.connectors[0], id: "account-usage", name: "Account usage", detail: "Last-good account snapshot", status: "degraded", freshness: "stale", ageSeconds: 60 });
    runtimeApi.getDashboardSummary.mockResolvedValue(summary);

    renderShell();

    expect(await screen.findByText("Stale account snapshot")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Local history" })).toBeInTheDocument();
  });

  it("shows no local history instead of zero metrics when no local evidence exists", async () => {
    const { createMockDashboardSummary } = await import("../../lib/api/mockData");
    const summary = createMockDashboardSummary("combined");
    summary.localMetrics = [];
    runtimeApi.getDashboardSummary.mockResolvedValue(summary);

    renderShell();

    expect(await screen.findByRole("heading", { name: "Local history unavailable" })).toBeInTheDocument();
    expect(screen.queryByLabelText("Local history metrics")).not.toBeInTheDocument();
  });

  it.each([
    ["local", false, true],
    ["remote", true, false],
  ] as const)("renders only requested source family in %s mode", async (mode, accountVisible, localVisible) => {
    const { createMockDashboardSummary } = await import("../../lib/api/mockData");
    const summary = createMockDashboardSummary(mode);
    summary.accountMetrics = [{ ...summary.metrics[0], key: "account-lifetime", label: "Account lifetime" }];
    runtimeApi.getDashboardSummary.mockResolvedValue(summary);
    renderShell();

    await waitFor(() => {
      expect(Boolean(screen.queryByRole("heading", { name: "Account usage" }))).toBe(accountVisible);
      expect(Boolean(screen.queryByRole("heading", { name: /Local history/ }))).toBe(localVisible);
    });
  });
  it("renders required Command Center concepts", async () => {
    renderShell();

    expect(await screen.findAllByText("TokenStack")).not.toHaveLength(0);
    expect(screen.getByRole("heading", { name: "Dashboard" })).toBeInTheDocument();
    expect(screen.getAllByText("Local + Remote")[0]).toBeInTheDocument();
    expect(screen.getByRole("toolbar", { name: "TokenStack controls" })).toBeInTheDocument();
    expect(screen.getByRole("status", { name: "TokenStack status" })).toBeInTheDocument();
    expect(await screen.findByText("Daily token usage")).toBeInTheDocument();
    expect(screen.getByText("Reset credit timeline")).toBeInTheDocument();
    expect(screen.getByText("Source coverage")).toBeInTheDocument();
    expect(screen.getByText("Active connectors")).toBeInTheDocument();
    expect(screen.getAllByText(/America\/New_York/)[0]).toBeInTheDocument();
  });

  it("renders desktop app chrome without web footer or defensive badges", async () => {
    const { container } = renderShell();
    await screen.findByText("Daily token usage");

    expect(screen.getByRole("toolbar", { name: "TokenStack controls" })).toBeInTheDocument();
    expect(screen.getByRole("status", { name: "TokenStack status" })).toBeInTheDocument();
    expect(screen.queryByText("No token display")).not.toBeInTheDocument();
    expect(screen.queryByText("Local app")).not.toBeInTheDocument();
    expect(screen.queryByText("MIT License")).not.toBeInTheDocument();
    expect(container).not.toHaveTextContent("Never /consume");
    expect(container).not.toHaveTextContent("/consume");
    expect(container).not.toHaveTextContent("Read-only");
  });

  it("does not render placeholder identity, fake repository stats, or internal safety copy", async () => {
    const { container } = renderShell();
    await screen.findByText("Daily token usage");

    expect(screen.queryByText("John B")).not.toBeInTheDocument();
    expect(screen.queryByText("@burmjohn")).not.toBeInTheDocument();
    expect(screen.queryByText("JB")).not.toBeInTheDocument();
    expect(screen.queryByText("1.2k")).not.toBeInTheDocument();
    expect(container).not.toHaveTextContent("Read-only");
    expect(container).not.toHaveTextContent("/consume");
    expect(container).not.toHaveTextContent("Undocumented (RO)");
    expect(container).not.toHaveTextContent("schema-gated");
  });

  it("uses sidebar navigation to open focused sections and setup controls", async () => {
    const user = userEvent.setup();
    renderShell();
    await screen.findByText("Daily token usage");

    await user.click(screen.getByRole("button", { name: "Usage" }));
    expect(screen.getByRole("heading", { name: "Usage" })).toBeInTheDocument();
    expect(screen.getByText("Recent sessions")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Setup" }));
    expect(screen.getByRole("heading", { name: "Setup" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Scan local data" })).toBeEnabled();
    expect(await screen.findByText("Diagnostics")).toBeInTheDocument();
    expect(screen.getByText("Local Codex folders")).toBeInTheDocument();
    expect(screen.getByText("No import run recorded")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Codex runtime" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Choose runtime" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Test connection" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Clear selection" })).toBeEnabled();
    expect(screen.getByText("No automatic Codex runtimes found")).toBeInTheDocument();
  });

  it("chooses and validates a runtime before reporting it connected", async () => {
    const user = userEvent.setup();
    runtimeApi.chooseCodexRuntime.mockResolvedValue({ valid: true, version: "codex 1.2.3", error: null });
    renderShell();
    await user.click(await screen.findByRole("button", { name: "Setup" }));

    await user.click(screen.getByRole("button", { name: "Choose runtime" }));

    expect(await screen.findByText("Connected — codex 1.2.3")).toBeInTheDocument();
    expect(runtimeApi.selectCodexRuntime).not.toHaveBeenCalled();
    expect(runtimeApi.refreshAll).toHaveBeenCalledWith("combined");
  });

  it("reports invalid picker selection without refreshing account data", async () => {
    const user = userEvent.setup();
    runtimeApi.chooseCodexRuntime.mockResolvedValue({ valid: false, version: null, error: "unsupported protocol" });
    renderShell();
    await user.click(await screen.findByRole("button", { name: "Setup" }));

    await user.click(screen.getByRole("button", { name: "Choose runtime" }));

    expect(await screen.findByText("Unsupported Codex CLI")).toBeInTheDocument();
    expect(runtimeApi.refreshAll).not.toHaveBeenCalled();
  });

  it("uses an automatic candidate, clears selection, and tests connection", async () => {
    const user = userEvent.setup();
    runtimeApi.listCodexRuntimes.mockResolvedValue([{
      displayPath: "C:\\Codex\\codex.exe",
      source: "path",
      exists: true,
      executable: true,
      version: "codex 1.2.3",
      validationError: null,
      configured: false,
      selected: true,
    }]);
    renderShell();
    await user.click(await screen.findByRole("button", { name: "Setup" }));

    expect(screen.getByText("selected")).toBeInTheDocument();
    await user.click(await screen.findByRole("button", { name: "Use PATH runtime C:\\Codex\\codex.exe" }));
    expect(runtimeApi.selectCodexRuntime).toHaveBeenCalledWith({ displayPath: "C:\\Codex\\codex.exe" });
    expect(await screen.findByText("Connected — codex 1.2.3")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Clear selection" }));
    expect(await screen.findByText("Selection cleared. Automatic discovery is active.")).toBeInTheDocument();
    expect(runtimeApi.clearCodexRuntime).toHaveBeenCalledOnce();

    await user.click(screen.getByRole("button", { name: "Test connection" }));
    expect(await screen.findByText("Connected — codex 1.2.3")).toBeInTheDocument();
    expect(runtimeApi.validateCodexRuntime).toHaveBeenCalledOnce();
  });

  it("renders backend-selected npm wrappers as configured", async () => {
    const user = userEvent.setup();
    runtimeApi.listCodexRuntimes.mockResolvedValue([{
      displayPath: "C:\\Users\\me\\AppData\\Roaming\\npm\\codex.cmd",
      source: "npm",
      exists: true,
      executable: true,
      version: "codex 1.2.3",
      validationError: null,
      configured: true,
      selected: true,
    }]);
    renderShell();
    await user.click(await screen.findByRole("button", { name: "Setup" }));

    expect(screen.getByText("selected")).toBeInTheDocument();
    expect(screen.getByText("configured")).toBeInTheDocument();
  });

  it.each([
    ["missing executable", "Codex runtime not found"],
    ["access denied", "Access denied"],
    ["user is logged out", "Codex is logged out"],
    ["initialize timeout", "App-server initialize timed out"],
    ["method partial failure: rate limits", "Connected with method partial failure — method partial failure: rate limits"],
  ])("stages runtime failure %s as an accessible alert", async (error, expected) => {
    const user = userEvent.setup();
    runtimeApi.chooseCodexRuntime.mockResolvedValue({ valid: false, version: null, error });
    renderShell();
    await user.click(await screen.findByRole("button", { name: "Setup" }));
    await user.click(screen.getByRole("button", { name: "Choose runtime" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(expected);
  });

  it("downloads setup diagnostics as JSON", async () => {
    const user = userEvent.setup();
    runtimeApi.exportSetupDiagnostics.mockResolvedValueOnce({ status: "downloaded" });
    renderShell();
    await screen.findByText("Daily token usage");

    await user.click(screen.getByRole("button", { name: "Setup" }));
    await user.click(await screen.findByRole("button", { name: "Export diagnostics" }));

    expect(await screen.findByText("Downloaded diagnostics JSON")).toBeInTheDocument();
  });

  it("shows the exact installed-app diagnostics path and an actionable failure", async () => {
    const user = userEvent.setup();
    renderShell();
    await user.click(await screen.findByRole("button", { name: "Setup" }));
    await user.click(await screen.findByRole("button", { name: "Export diagnostics" }));
    expect(await screen.findByText("Saved to C:\\TokenStack\\diagnostics\\report.json")).toBeInTheDocument();
    expect(runtimeApi.exportSetupDiagnostics).toHaveBeenCalledWith(expect.any(Object), "combined");

    runtimeApi.exportSetupDiagnostics.mockResolvedValueOnce({ status: "failed", error: "access denied writing diagnostics directory" });
    await user.click(screen.getByRole("button", { name: "Export diagnostics" }));
    expect(await screen.findByText("Export failed: access denied writing diagnostics directory")).toBeInTheDocument();
  });

  it("toggles theme and runs refresh without duplicate controls", async () => {
    const user = userEvent.setup();
    localStorage.setItem("tokenstack-theme", "dark");
    renderShell();

    const themeButton = await screen.findByRole("button", { name: /Switch to light theme/i });
    await user.click(themeButton);
    expect(document.documentElement.dataset.theme).toBe("light");

    const refreshButton = screen.getByRole("button", { name: "Refresh data" });
    await user.click(refreshButton);
    expect(refreshButton).toBeDisabled();
  });

  it("keeps export actions disabled until dashboard data is loaded", async () => {
    renderShell();

    const badgeButton = screen.getByRole("button", { name: "Export badge" });
    const csvButton = screen.getByRole("button", { name: "Export usage CSV" });
    expect(badgeButton).toBeDisabled();
    expect(csvButton).toBeDisabled();
    expect(screen.getByLabelText("Export badge requires loaded dashboard data")).toBeInTheDocument();
    expect(screen.getByLabelText("Export usage CSV requires loaded dashboard data")).toBeInTheDocument();

    await screen.findByText("Daily token usage");
    expect(badgeButton).toBeEnabled();
    expect(csvButton).toBeEnabled();
  });

  it("downloads the usage CSV bundle with browser object URLs", async () => {
    const user = userEvent.setup();
    renderShell();
    await screen.findByText("Daily token usage");

    const download = installDownloadMocks();
    await user.click(screen.getByRole("button", { name: "Export usage CSV" }));

    await waitFor(() => expect(download.anchorClick).toHaveBeenCalledTimes(1));
    expect(download.getAnchor()?.download).toMatch(/^tokenstack-usage-\d{4}-\d{2}-\d{2}\.csv$/);
    expect(download.revokeObjectURL).toHaveBeenCalledWith("blob:tokenstack-export");

    const blob = download.createObjectURL.mock.calls[0]?.[0];
    if (!blob) {
      throw new Error("CSV download did not create a blob.");
    }
    expect(blob.type).toBe("text/csv;charset=utf-8");
    await expect(readBlobText(blob)).resolves.toContain("source_coverage");
  });

  it("selects a badge layout and downloads a PNG from a mocked canvas", async () => {
    const user = userEvent.setup();
    installCanvasMocks();
    installImageMock();
    renderShell();
    await screen.findByText("Daily token usage");

    const download = installDownloadMocks();
    await user.click(screen.getByRole("button", { name: "Export badge" }));
    expect(await screen.findByRole("region", { name: "Badge export panel" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Usage badge layout" }));
    expect(screen.getByText("Local monthly output")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Download PNG for Usage badge" }));

    await waitFor(() => expect(download.anchorClick).toHaveBeenCalledTimes(1));
    expect(download.getAnchor()?.download).toMatch(/^tokenstack-badge-usage-\d{4}-\d{2}-\d{2}\.png$/);

    const blob = download.createObjectURL.mock.calls[0]?.[0];
    if (!blob) {
      throw new Error("PNG download did not create a blob.");
    }
    expect(blob.type).toBe("image/png");
    await waitFor(() => expect(screen.queryByRole("region", { name: "Badge export panel" })).not.toBeInTheDocument());
  });

  it("responds to native desktop menu commands", async () => {
    installDownloadMocks();
    localStorage.setItem("tokenstack-theme", "dark");
    renderShell();
    await screen.findByText("Daily token usage");
    await waitFor(() => expect(desktopBridge.handler).toBeTypeOf("function"));

    await act(async () => desktopBridge.handler?.("navigate-setup"));
    expect(screen.getByRole("heading", { name: "Setup" })).toBeInTheDocument();

    await act(async () => desktopBridge.handler?.("toggle-theme"));
    expect(document.documentElement.dataset.theme).toBe("light");

    await act(async () => desktopBridge.handler?.("export-badge"));
    expect(await screen.findByRole("region", { name: "Badge export panel" })).toBeInTheDocument();

    await act(async () => desktopBridge.handler?.("export-csv"));
    await waitFor(() => expect(URL.createObjectURL).toHaveBeenCalled());
  });
});

function restoreUrlApi(key: "createObjectURL", value: typeof URL.createObjectURL | undefined): void;
function restoreUrlApi(key: "revokeObjectURL", value: typeof URL.revokeObjectURL | undefined): void;
function restoreUrlApi(key: "createObjectURL" | "revokeObjectURL", value: typeof URL.createObjectURL | typeof URL.revokeObjectURL | undefined) {
  if (value) {
    Object.defineProperty(URL, key, { configurable: true, value });
    return;
  }
  delete (URL as unknown as Record<string, unknown>)[key];
}

function installDownloadMocks() {
  const createObjectURL = vi.fn<(blob: Blob) => string>(() => "blob:tokenstack-export");
  const revokeObjectURL = vi.fn<(url: string) => void>();
  const anchorClick = vi.fn<() => void>();
  let anchor: HTMLAnchorElement | undefined;
  const originalCreateElement = document.createElement.bind(document);

  Object.defineProperty(URL, "createObjectURL", { configurable: true, value: createObjectURL });
  Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: revokeObjectURL });

  vi.spyOn(document, "createElement").mockImplementation(((tagName: string, options?: ElementCreationOptions) => {
    const element = originalCreateElement(tagName, options);
    if (tagName.toLowerCase() === "a") {
      anchor = element as HTMLAnchorElement;
      vi.spyOn(anchor, "click").mockImplementation(anchorClick);
    }
    return element;
  }) as typeof document.createElement);

  return {
    anchorClick,
    createObjectURL,
    getAnchor: () => anchor,
    revokeObjectURL,
  };
}

function installCanvasMocks() {
  const context = {
    arc: vi.fn(),
    beginPath: vi.fn(),
    clearRect: vi.fn(),
    drawImage: vi.fn(),
    fill: vi.fn(),
    fillRect: vi.fn(),
    fillText: vi.fn(),
    lineTo: vi.fn(),
    moveTo: vi.fn(),
    stroke: vi.fn(),
    strokeRect: vi.fn(),
  } as unknown as CanvasRenderingContext2D;

  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(context);
  vi.spyOn(HTMLCanvasElement.prototype, "toBlob").mockImplementation((callback, type) => {
    callback(new Blob(["png"], { type: type ?? "image/png" }));
  });
}

function installImageMock() {
  class MockImage {
    onerror: (() => void) | null = null;
    onload: (() => void) | null = null;

    set src(_value: string) {
      queueMicrotask(() => this.onload?.());
    }
  }

  vi.stubGlobal("Image", MockImage as unknown as typeof Image);
}

function readBlobText(blob: Blob) {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsText(blob);
  });
}
