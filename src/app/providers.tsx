import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { PropsWithChildren } from "react";
import { useMemo } from "react";

export function AppProviders({ children }: PropsWithChildren) {
  const queryClient = useMemo(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 30_000,
            retry: 1,
            refetchOnWindowFocus: false,
          },
        },
      }),
    [],
  );

  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
}
