import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getDashboardSummary, refreshAll } from "../../lib/api/tauri";
import { queryKeys } from "../../lib/query/keys";
import type { DataMode } from "../../lib/schemas/dashboard";

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
    },
  });
}
