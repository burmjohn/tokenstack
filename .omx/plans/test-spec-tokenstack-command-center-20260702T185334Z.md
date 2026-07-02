# Test Spec: TokenStack Command Center

Generated: 2026-07-02T18:53:34Z
Workflow: `$ralplan` deliberate consensus planning

## Test Strategy

Test from the safety boundary outward:

1. Prove unsafe network/auth behavior is impossible.
2. Prove local import, parsing, and persistence are deterministic and secret-safe.
3. Prove analytics and timezone transforms are correct.
4. Prove frontend data states, source coverage, and Command Center UI render correctly.
5. Prove Windows/Tauri packaging remains buildable.

No test may use real auth tokens, real full auth files, or private user history. All fixtures must be synthetic and redacted.

## Unit Tests

### Rust Safety Guard

- `rejects_any_path_containing_consume`
  - Given endpoint paths such as `/consume`, `/v1/consume`, `/wham/consume/reset`, and URL-encoded variants when normalized.
  - Expect validation fails before request construction.
- `rejects_non_readonly_methods`
  - Given POST, PUT, PATCH, DELETE, OPTIONS for authenticated connectors.
  - Expect validation fails.
- `rejects_request_body_for_authenticated_connectors`
  - Given a GET with a request body or mutation payload.
  - Expect validation fails.
- `allows_registered_get_reset_credit_endpoint`
  - Given registered `/wham/rate-limit-reset-credits` endpoint with method GET and no body.
  - Expect validation succeeds.
- `rejects_unregistered_undocumented_endpoint`
  - Given an arbitrary undocumented endpoint not in the audited registry.
  - Expect validation fails.
- `redacts_auth_values_in_errors`
  - Given an internal error containing token-like material.
  - Expect public error output removes/obscures secrets.
- `auth_handle_never_serializes_secret`
  - Given an auth handle or connector state.
  - Expect serde/IPC payload contains only availability/status metadata.

### Rust Auth-Adjacent Local Reads

- `auth_locator_reads_only_allowed_paths`
  - Given known auth-adjacent path candidates and unrelated paths.
  - Expect only allowlisted paths can be opened.
- `auth_parser_extracts_minimum_required_fields`
  - Given synthetic auth JSON shape.
  - Expect only opaque in-memory auth material and redacted account metadata are produced.
- `auth_file_contents_not_persisted`
  - Given an auth read and DB snapshot.
  - Expect no raw auth JSON or token-like values in persisted rows.

### Local History Import

- `imports_jsonl_token_count_events`
  - Given synthetic Codex JSONL with token count events.
  - Expect usage events and session aggregates.
- `skips_unknown_jsonl_shapes_with_warning`
  - Given unknown event shapes.
  - Expect warning and lower source coverage, not failed import.
- `deduplicates_reimported_events`
  - Given same file imported twice.
  - Expect stable event count and import run metadata.
- `tracks_source_document_offsets_or_hashes`
  - Given appended JSONL.
  - Expect only new events are imported or duplicate-safe reprocessing occurs.
- `never_fixtures_private_history`
  - Static fixture scan verifies no real-looking tokens, account IDs, or full auth documents.

### SQLite Persistence

- `migrations_create_schema_from_empty_db`
- `migrations_are_idempotent`
- `usage_events_roundtrip`
- `reset_credit_batches_roundtrip`
- `connector_runs_roundtrip_redacted_errors`
- `source_coverage_roundtrip`
- `derived_daily_usage_query_matches_raw_events`
- `foreign_keys_and_unique_constraints_prevent_duplicates`

### Analytics

- `computes_lifetime_tokens`
- `computes_today_in_america_new_york`
- `computes_month_to_date_in_america_new_york`
- `computes_peak_session`
- `computes_daily_heatmap_buckets`
- `computes_monthly_rollups`
- `computes_source_coverage_percentages`
- `coverage_formula_never_overstates_missing_sources`
- `coverage_formula_records_formula_version`
- `coverage_confidence_downgrades_on_unknown_source_shape`
- `coverage_explanation_names_required_missing_facets`
- `marks_derived_stats_partial_when_sources_missing`
- `handles_zero_data_without_nan_or_crash`

### Timezone Conversion

- `formats_reset_expiration_in_america_new_york`
- `handles_dst_spring_forward`
- `handles_dst_fall_back`
- `stores_canonical_utc_and_displays_ny`
- `countdown_uses_timezone_safe_instant_math`

### Connector Response Validation

- `known_reset_credit_schema_accepts_expected_shape`
- `known_reset_credit_schema_rejects_missing_expiration`
- `undocumented_connector_schema_is_explicit_per_endpoint`
- `connector_failure_does_not_clear_last_good_snapshot`
- `connector_failure_sets_degraded_source_coverage`

### TypeScript Data Layer

- Query key factory returns stable keys for dashboard, usage, reset credits, connectors, and source coverage.
- Query functions call typed IPC/database adapters, not raw fetch for authenticated data.
- Zod schemas reject malformed IPC payloads.
- Data mode selector filters local, remote, and combined results.
- Refresh invalidates only the relevant query families.

## Component Tests

Use Vitest + React Testing Library for most components. Use Browser Mode for keyboard/focus behavior where needed.

### App Shell

- Renders TokenStack identity, Dashboard nav, data mode, auto refresh, version, and GitHub affordance.
- Theme toggle changes dark/light root class or data attribute and persists preference.
- Sidebar navigation has accessible names and selected state.

### Header And Safety Controls

- Shows last refresh, refresh button, `Read-only`, `Never /consume`, and data mode.
- Refresh pending state disables duplicate manual refresh.
- Error state shows redacted message and does not expose endpoint tokens or auth values.

### Metric Strip

- Renders lifetime tokens, today, this month, peak session, and reset credits.
- Shows source coverage/tooltip trigger for each metric.
- Handles loading, empty, stale, degraded, and error states.

### Token Heatmap

- Renders daily token usage with month/day labels and intensity legend.
- Daily, weekly, and monthly controls are keyboard reachable.
- Empty data renders a quiet, non-marketing empty state with source coverage explanation.

### Reset Credit Timeline

- Renders credit counts, expiration dates, days remaining, and `America/New_York` label.
- Sorts expirations by instant.
- Handles no credits and expired credits.

### Source Coverage

- Renders total coverage score and local history, rate limits, reset credits, and undocumented rows.
- Hover/inspector explains source evidence and incompleteness.

### Active Connectors

- Renders local history, known read-only endpoint, and undocumented read-only endpoint rows.
- Shows read-only status for each connector.
- Does not display auth token, secret, or full local file content.

### Tables

- Recent sessions table renders start time, duration, tokens, peak tokens, mode, and source labels.
- Rate-limit windows table renders window, limit, used, remaining, resets in, and overall progress.
- Tables have accessible headers and stable layout at desktop widths.

### Footer

- Renders `All data is read-only`, never-consume language, open source, license, and GitHub repository link.

## Integration Tests

### Import Pipeline

- Synthetic local history directory with multiple JSONL files imports into SQLite.
- Re-running import is idempotent.
- Partial corrupt files produce warnings and source coverage degradation.
- Derived dashboard summary matches raw imported events.

### Refresh Orchestrator

- Manual refresh runs local import, known read-only connector, undocumented read-only connector, persistence, and query invalidation in order.
- Remote connector failure keeps local analytics available.
- Background refresh obeys minimum cadence and backoff.
- Concurrent refresh requests coalesce or lock correctly.

### Connector Safety

- Mock HTTP server records requests.
- Allowed endpoint request reaches server only after guard approval.
- `/consume` request attempt never reaches server.
- Non-GET methods never reach server.
- Undocumented endpoint must be registered before use.

### Database And Query Layer

- App opens against an empty app-data SQLite database.
- Migrations run.
- Dashboard queries return expected summary.
- Source coverage updates after connector success/failure.

### Theme And Layout

- Dark and light themes render the same component structure.
- No nested page-section cards beyond repeated dashboard modules.
- Cards use radius 8px or less.
- Text does not overflow controls at 1280x800 and 1440x900.

## End-To-End And Visual Verification

### Web Preview E2E

- Launch Vite preview with mocked Tauri adapters.
- Load dashboard.
- Toggle dark/light themes.
- Trigger manual refresh with mocked data.
- Change data mode Local, Remote, Combined.
- Inspect source coverage hover/inspector.
- Verify no console errors.

### Tauri Smoke

- Launch Tauri dev app with synthetic local data path.
- App initializes SQLite and renders dashboard.
- Manual refresh succeeds with mocked remote connector.
- Close/reopen preserves snapshots and theme preference.

### Screenshot Verification

- Capture dark dashboard screenshot after data fixture load.
- Capture light dashboard screenshot after data fixture load.
- Compare screenshots against Command Center SOT checklist:
  - Persistent sidebar.
  - Header safety controls.
  - Metric strip.
  - Heatmap.
  - Reset timeline.
  - Coverage/connectors.
  - Footer safety row.
  - No marketing hero.
  - No decorative gradients/orbs.
  - No one-note purple/blue palette.

### Accessibility

- Keyboard navigation reaches sidebar, header controls, view toggles, tables, source coverage inspector, and footer links.
- Axe or equivalent catches no serious/critical issues in dashboard states.
- Icon-only buttons have accessible names and tooltips.
- Color contrast passes for text, badges, charts, focus rings, and status chips in both themes.

## CI Gates

- `pnpm lint`
- `pnpm typecheck`
- `pnpm test`
- `pnpm test:browser` for targeted Browser Mode tests
- `cargo test`
- `cargo clippy -- -D warnings`
- `cargo fmt --check`
- Secret scan against repository
- Fixture scan for auth-like/token-like values
- Build frontend
- Tauri dev/build smoke
- Windows build job with `pnpm tauri build` or `cargo tauri build` on Windows runner

## Manual Verification Checklist

- App starts with no local data and shows safe empty dashboard.
- App imports synthetic Codex history and shows daily/monthly analytics.
- App refreshes mocked reset-credit data and shows expiration in `America/New_York`.
- Unsafe endpoint attempts are visibly impossible from UI and rejected in guard tests.
- Logs contain no secrets.
- SQLite contains no secrets.
- Dark and light themes match Command Center density and hierarchy.
- README screenshots are captured from the real app, not mockups.

## Ultragoal Evidence Ledger

When handed off to `$ultragoal`, each checkpoint must attach or cite fresh evidence:

- Safety evidence: `.omx/ultragoal/evidence/02-safety-guard.md` with Rust guard test output, mock HTTP proof that denied endpoints are never called, and secret scan output.
- Data evidence: `.omx/ultragoal/evidence/03-data-import-analytics.md` with migration/importer/analytics/timezone test output and synthetic fixture coverage.
- Connector evidence: `.omx/ultragoal/evidence/04-readonly-connectors.md` with known reset-credit mock test, undocumented read-only registry test, and redacted connector failure test.
- UI evidence: `.omx/ultragoal/evidence/07-command-center-ui.md` with component tests, accessibility checks, dark screenshot, and light screenshot.
- Packaging evidence: `.omx/ultragoal/evidence/08-windows-packaging.md` with Tauri dev smoke and Windows `x86_64-pc-windows-msvc` build smoke result or exact blocker.
- Documentation evidence: `.omx/ultragoal/evidence/09-open-source-docs.md` with README, screenshots, security policy, contributor guide, data-source docs, connector-safety docs, and ADRs.
- Git evidence: `.omx/ultragoal/evidence/10-git-history.md` with commit list showing coherent Lore-protocol commits by stage.

Ultragoal should keep the goal open if any ledger item is missing, stale, or not tied to a concrete verification result.

## Test Data Policy

- Synthetic fixtures only.
- No real auth files.
- No real local user histories.
- No full raw endpoint responses from private accounts.
- Any real-world shape used during development must be manually minimized and redacted before becoming a fixture.

## Expanded Deliberate-Mode Test Plan

### Unit

Rust guard, auth locator/parser, endpoint registry, JSONL importer, SQLite repositories, analytics transforms, timezone conversion, Zod schemas, query key factories, UI pure components.

### Integration

Import pipeline, refresh orchestrator, connector safety with mock HTTP server, SQLite migrations, source coverage lifecycle, data mode filtering.

### E2E

Vite preview with mocked Tauri, Tauri dev smoke with synthetic data, dark/light screenshots, keyboard navigation, manual refresh, data mode switching.

### Observability

Structured redacted connector events, refresh spans, source coverage history, app-visible last refresh/degraded state, CI artifacts for screenshots and test logs.
