# Evidence 06: Frontend Data Layer

Generated: 2026-07-02

## Implemented

- `src/lib/schemas/dashboard.ts` validates sanitized IPC/dashboard payloads with Zod.
- `src/lib/api/tauri.ts` routes browser preview to synthetic data and Tauri runtime to typed commands.
- `src/lib/query/keys.ts` defines stable query families for dashboard, usage, sessions, reset credits, rate limits, sources, connectors, and refresh status.
- `src/features/dashboard/useDashboardSummary.ts` uses TanStack Query v5 object-signature queries and invalidation.

## Fresh Verification

- `pnpm typecheck`: passed.
- `pnpm test`: 3 files, 5 tests passed.
- `queryKeys` test passed for stable keys.
- Zod schema test passed for sanitized payloads and rejected invalid coverage.
