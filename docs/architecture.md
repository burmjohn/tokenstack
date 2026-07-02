# Architecture

TokenStack is a local-first Tauri v2 app. React owns presentation and cached async state. Rust owns auth-adjacent reads, read-only connector execution, importer parsing, SQLite writes, redaction, and source coverage.

The Rust boundary exposes sanitized Tauri commands only:

- `get_dashboard_summary`
- `refresh_all`

Frontend code calls typed wrappers and Zod schemas. No React component calls authenticated HTTP or parses auth material.

## Runtime Boundaries

- Local history importer reads JSONL usage history and stores usage events with path hashes and redacted source labels.
- Safety guard validates every authenticated remote request before network construction.
- SQLite stores imported usage, connector run metadata, reset-credit snapshots, rate-limit windows, refresh snapshots, and coverage records.
- Coverage starts at zero and only increases when required facets are present.
