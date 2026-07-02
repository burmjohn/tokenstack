# Evidence 03: Data Import And Analytics

Generated: 2026-07-02

## Implemented

- `src-tauri/src/db.rs` defines SQLite migrations for app metadata, import runs, source documents, usage events, sessions, connector runs, reset-credit batches, rate-limit windows, refresh snapshots, and source coverage.
- `src-tauri/src/importers.rs` imports local Codex JSONL from synthetic-safe roots, hashes source paths/content, deduplicates event ids, stores usage events, and records conservative source coverage.
- `src-tauri/fixtures/codex-history/history.jsonl` provides synthetic local history only.
- `src-tauri/src/analytics.rs` computes lifetime tokens, today, month-to-date, heatmap data, reset expiration display, and `America/New_York` timezone transforms.

## Fresh Verification

`cargo test --manifest-path src-tauri/Cargo.toml --no-default-features` passed all 30 Rust core tests, including:

- `migrations_create_schema_from_empty_db`
- `migrations_are_idempotent`
- `usage_events_roundtrip`
- `foreign_keys_and_unique_constraints_prevent_duplicates`
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

`pnpm fixture:scan` passed, confirming fixtures are synthetic and redacted.
