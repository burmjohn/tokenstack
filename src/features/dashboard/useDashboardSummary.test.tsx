import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook } from "@testing-library/react";
import type { PropsWithChildren } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { queryKeys } from "../../lib/query/keys";
import { createMockDashboardSummary } from "../../lib/api/mockData";

const api = vi.hoisted(() => ({
  clearCodexRuntime: vi.fn(),
  refreshAll: vi.fn(),
  selectCodexRuntime: vi.fn(),
  validateCodexRuntime: vi.fn(),
}));

vi.mock("../../lib/api/tauri", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../lib/api/tauri")>()),
  ...api,
}));

import { useCodexRuntimeActions } from "./useDashboardSummary";

function createWrapper(client: QueryClient) {
  return function Wrapper({ children }: PropsWithChildren) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  };
}

describe("useCodexRuntimeActions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("refreshes account data and all setup views after a successful selection", async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const invalidate = vi.spyOn(client, "invalidateQueries");
    const summary = createMockDashboardSummary("combined");
    api.selectCodexRuntime.mockResolvedValue({ valid: true, version: "codex 1.2.3", error: null });
    api.refreshAll.mockResolvedValue(summary);
    const { result } = renderHook(() => useCodexRuntimeActions("combined"), { wrapper: createWrapper(client) });

    await act(() => result.current.select.mutateAsync({ displayPath: "C:\\Program Files\\Codex\\codex.exe" }));

    expect(api.refreshAll).toHaveBeenCalledWith("combined");
    expect(client.getQueryData(queryKeys.dashboard.summary("combined"))).toEqual(summary);
    expect(invalidate).toHaveBeenCalledWith({ queryKey: queryKeys.runtimes.codex() });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: queryKeys.diagnostics.setup() });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: queryKeys.dashboard.all });
  });

  it("does not refresh account data when validation rejects a selection", async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    api.selectCodexRuntime.mockResolvedValue({ valid: false, version: null, error: "unsupported CLI" });
    const { result } = renderHook(() => useCodexRuntimeActions("combined"), { wrapper: createWrapper(client) });

    await expect(act(() => result.current.select.mutateAsync({ displayPath: "C:\\bad.exe" }))).resolves.toMatchObject({ valid: false });
    expect(api.refreshAll).not.toHaveBeenCalled();
  });

  it("clears the persisted selection and re-runs automatic discovery", async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const invalidate = vi.spyOn(client, "invalidateQueries");
    api.clearCodexRuntime.mockResolvedValue(createMockDashboardSummary("combined"));
    const { result } = renderHook(() => useCodexRuntimeActions("combined"), { wrapper: createWrapper(client) });

    await act(() => result.current.clear.mutateAsync());

    expect(api.clearCodexRuntime).toHaveBeenCalledWith("combined");
    expect(invalidate).toHaveBeenCalledWith({ queryKey: queryKeys.runtimes.codex() });
  });
});
