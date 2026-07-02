# Evidence 06: Frontend Data Layer

Generated: 2026-07-02

## Implemented

- `src/lib/schemas/dashboard.ts` validates sanitized IPC/dashboard payloads with Zod.
- `src/lib/api/tauri.ts` routes browser preview to synthetic data and Tauri runtime to typed commands.
- `src/lib/query/keys.ts` defines stable query families for dashboard, usage, sessions, reset credits, rate limits, sources, connectors, and refresh status.
- `src/features/dashboard/useDashboardSummary.ts` uses TanStack Query v5 object-signature queries and invalidation.
- `src/components/command-center/CommandCenterShell.tsx` runs a real 60-second auto-refresh scheduler with capped backoff and duplicate-refresh protection.
- Tauri runtime calls use only typed `invoke` commands; the frontend SQL plugin permission was removed from `src-tauri/capabilities/main.json`.
- `src-tauri/tauri.conf.json` now defines a restrictive CSP instead of `csp: null`.

## Fresh Verification

- `pnpm typecheck`: passed.
- `pnpm test`: 3 files, 5 tests passed.
- `rg` over tracked source found no `sql:default`, `tauri-plugin-sql`, `sqlx`, or `csp: null`.
- `queryKeys` test passed for stable keys.
- Zod schema test passed for sanitized payloads and rejected invalid coverage.
