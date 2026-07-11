import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  clearCodexRuntime,
  getDashboardSummary,
  getSetupDiagnostics,
  listCodexRuntimes,
  refreshAll,
  selectCodexRuntime,
  validateCodexRuntime,
} from "../../lib/api/tauri";
import { queryKeys } from "../../lib/query/keys";
import type { DataMode } from "../../lib/schemas/dashboard";
import type { CodexRuntimeSelection } from "../../lib/schemas/dashboard";

export function useDashboardSummary(dataMode: DataMode) {
  return useQuery({
    queryKey: queryKeys.dashboard.summary(dataMode),
    queryFn: () => getDashboardSummary(dataMode),
    staleTime: 45_000,
  });
}

export function useRefreshAll(dataMode: DataMode) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => refreshAll(dataMode),
    onSuccess: (summary) => {
      queryClient.setQueryData(queryKeys.dashboard.summary(dataMode), summary);
      void queryClient.invalidateQueries({ queryKey: queryKeys.usage.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.sources.coverage(dataMode) });
      void queryClient.invalidateQueries({ queryKey: queryKeys.connectors.status() });
      void queryClient.invalidateQueries({ queryKey: queryKeys.diagnostics.setup() });
    },
  });
}

export function useSetupDiagnostics() {
  return useQuery({
    queryKey: queryKeys.diagnostics.setup(),
    queryFn: () => getSetupDiagnostics(),
    staleTime: 15_000,
  });
}

export function useCodexRuntimes() {
  return useQuery({
    queryKey: queryKeys.runtimes.codex(),
    queryFn: listCodexRuntimes,
    staleTime: 15_000,
  });
}

export function useCodexRuntimeActions(dataMode: DataMode) {
  const queryClient = useQueryClient();
  const refreshRuntimeState = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.runtimes.codex() }),
      queryClient.invalidateQueries({ queryKey: queryKeys.diagnostics.setup() }),
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard.all }),
    ]);
  };
  const refreshAccount = async () => {
    const summary = await refreshAll(dataMode);
    queryClient.setQueryData(queryKeys.dashboard.summary(dataMode), summary);
    await refreshRuntimeState();
  };

  const select = useMutation({
    mutationFn: async (selection: CodexRuntimeSelection) => {
      const validation = await selectCodexRuntime(selection);
      if (validation.valid) {
        await refreshAccount();
      } else {
        await refreshRuntimeState();
      }
      return validation;
    },
  });
  const clear = useMutation({
    mutationFn: async () => {
      const summary = await clearCodexRuntime(dataMode);
      queryClient.setQueryData(queryKeys.dashboard.summary(dataMode), summary);
      await refreshRuntimeState();
    },
  });
  const test = useMutation({
    mutationFn: async () => {
      const validation = await validateCodexRuntime();
      if (validation.valid) {
        await refreshAccount();
      } else {
        await refreshRuntimeState();
      }
      return validation;
    },
  });

  return { clear, refreshAccount, select, test };
}
