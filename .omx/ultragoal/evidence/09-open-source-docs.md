# Evidence 09: Open Source Docs

Generated: 2026-07-02

## Implemented

- `README.md` includes purpose, safety guarantees, setup, verification commands, data sources, privacy summary, build instructions, and real screenshots.
- `CONTRIBUTING.md` documents safety-first contribution rules.
- `SECURITY.md` documents security reporting and high-priority safety findings.
- `CODE_OF_CONDUCT.md` records the project conduct decision.
- `docs/architecture.md`, `docs/data-sources.md`, `docs/connector-safety.md`, and `docs/testing.md` are present.
- ADRs exist for MIT license, connector safety, SQLite schema, and UI/theme architecture.

## Fresh Verification

- `pnpm secret:scan`: passed.
- `pnpm fixture:scan`: passed.
- `pnpm test:browser`: refreshed `docs/screenshots/tokenstack-dashboard-dark.png` and `docs/screenshots/tokenstack-dashboard-light.png` from the real app preview.
