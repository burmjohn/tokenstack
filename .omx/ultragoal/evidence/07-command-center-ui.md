# Evidence 07: Command Center UI

Generated: 2026-07-02

## Implemented

- First screen is the dashboard, not a landing page.
- Dark and light themes use the same component structure and root `data-theme`.
- Visible dashboard concepts include TokenStack, Dashboard, Read-only, Never `/consume`, Local + Remote, Daily token usage, Reset credit timeline, Source coverage, Active connectors, Undocumented (RO), America/New_York, and All data is read-only.
- Dashboard includes sidebar, header, metric strip, heatmap, reset timeline, source coverage, active connectors, recent sessions, rate-limit windows, next reset expiration, and footer safety row.
- Sidebar auto-refresh reflects the active refresh cadence and the implementation backs off after refresh failures.

## Fresh Verification

- `pnpm test`: component test confirms required visible concepts and theme/refresh behavior.
- `pnpm test:browser`: 1 Chromium test passed with no console errors.
- Dark screenshot: `docs/screenshots/tokenstack-dashboard-dark.png`.
- Light screenshot: `docs/screenshots/tokenstack-dashboard-light.png`.
