# Evidence 03: Data Import And Analytics

Generated: 2026-07-02

## Implemented

- `src-tauri/src/db.rs` defines SQLite migrations for app metadata, import runs, source documents, usage events, sessions, connector runs, reset-credit batches, rate-limit windows, refresh snapshots, and source coverage.
- `src-tauri/src/importers.rs` imports local Codex JSONL from synthetic-safe roots, hashes source paths/content, deduplicates event ids, stores usage events, and records conservative source coverage.
- `src-tauri/fixtures/codex-history/history.jsonl` provides synthetic local history only.
- `src-tauri/src/commands.rs` opens a persistent app-data SQLite database for Tauri commands, imports configured local Codex roots, refreshes both remote connector lanes through an explicit auth-home boundary, and serializes backend refreshes with a process-local mutex.
- `src-tauri/src/analytics.rs` computes lifetime tokens, today, month-to-date, heatmap data, reset expiration display, backend local/remote/combined data-mode filtering, source coverage, connector status, sessions, and `America/New_York` timezone transforms from SQLite rows.
- Reset-credit and rate-limit dashboard reads use the latest successful connector run only, preserving last-good snapshots without double-counting repeated refreshes while lowering coverage/confidence after a newer failed connector run.

## Fresh Verification

`cargo test --manifest-path src-tauri/Cargo.toml --no-default-features` passed all 39 Rust core tests, including:

- `migrations_create_schema_from_empty_db`
- `migrations_are_idempotent`
- `usage_events_roundtrip`
- `foreign_keys_and_unique_constraints_prevent_duplicates`
- `connector_runs_and_reset_credit_batches_roundtrip`
- `imports_jsonl_token_count_events`
- `skips_unknown_jsonl_shapes_with_warning`
- `deduplicates_reimported_events`
- `tracks_source_document_offsets_or_hashes`
- `computes_lifetime_tokens`
- `computes_today_in_america_new_york`
- `computes_month_to_date_in_america_new_york`
- `formats_reset_expiration_in_america_new_york`
- `handles_dst_spring_forward`
- `handles_dst_fall_back`
- `handles_zero_data_without_nan_or_crash`
- `missing_reset_connector_evidence_does_not_overstate_coverage`
- `data_mode_filters_local_and_remote_sources`
- `repeated_remote_refreshes_use_latest_successful_snapshot_only`
- `refresh_persists_imported_history_for_later_summary_calls`
- `backend_refresh_lock_blocks_concurrent_refreshes` in the default-feature Tauri test run.

`pnpm fixture:scan` passed, confirming fixtures are synthetic and redacted.
