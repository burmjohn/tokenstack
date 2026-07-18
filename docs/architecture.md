# Architecture

TokenStack is a local-first Tauri v2 app. React owns presentation and cached async state. Rust owns local importer parsing, Codex OAuth and app-server account refresh, SQLite writes, redaction, diagnostics export, and source coverage.

The Rust boundary exposes sanitized Tauri commands only:

- `get_dashboard_summary`
- `get_setup_diagnostics`
- `export_diagnostics`
- `refresh_all`

Frontend code calls typed wrappers and Zod schemas. No React component calls authenticated HTTP or parses auth material.

## Runtime Boundaries

- Local history importer reads JSONL usage history and stores usage events with path hashes and redacted source labels.
- OAuth account refresh reads the existing Codex credential file only inside Rust, obtains authoritative quota/reset data from fixed HTTPS hosts, and never sends credentials or raw responses across the Tauri IPC boundary.
- Account refresh spawns `codex app-server` over stdio and reads account usage, rate limits, and reset credits through JSON-RPC.
- SQLite stores imported local usage separately from account refresh runs, account usage snapshots, account reset-credit snapshots, account rate-limit buckets/windows, refresh snapshots, and coverage records.
- Coverage starts at zero and only increases when required facets are present.
