# Implementation Plan: Codex App-Server Account Usage Integration

Generated: 2026-07-06T20:28:30Z
Status: draft for ralplan review

## Planning Decision
Implement the account connector by spawning `codex app-server` and using its read-only JSON-RPC methods. Do not read raw Codex auth in TokenStack for account usage/rate limits. Do not use private ChatGPT endpoint scraping as the default path.

## Work Breakdown

### 1. App-Server Protocol Client
Files:
- `src-tauri/src/codex_app_server.rs` new
- `src-tauri/src/telemetry.rs` if additional redaction helpers are needed

Tasks:
- Define request/response structs for the narrow methods TokenStack needs.
- Implement child process launch:
  - binary resolution via `TOKENSTACK_CODEX_BIN`, then `codex` on PATH.
  - `codex app-server` arguments.
  - piped stdin/stdout/stderr.
  - startup, request, and whole-refresh timeouts.
- Implement JSON-RPC request IDs and response matching.
- Send:
  - `initialize`
  - `initialized`
  - `account/read`
  - `account/usage/read`
  - `account/rateLimits/read`
- Reject or ignore any method outside the approved read-only allowlist.
- Kill the child process after refresh.
- Convert all errors into `PublicError` values.

Notes:
- Treat notifications as non-fatal; preserve `account/rateLimits/updated` support as optional.
- Avoid logging raw JSON responses until redaction is proven.

### 1A. App-Server Refresh Contract

Contract:
- One remote/account refresh spawns one `codex app-server` child process.
- One app-server session performs:
  - `initialize`
  - `initialized`
  - `account/read`
  - `account/usage/read`
  - `account/rateLimits/read`
- The connector returns one `CodexAccountRefreshResult` aggregate:
  - `started_at_utc`
  - `completed_at_utc`
  - `binary_diagnostics`
  - `process_diagnostics`
  - `profile_result`
  - `usage_result`
  - `rate_limits_result`
- Persistence uses one SQLite transaction for the aggregate result.
- Partial success is valid:
  - successful request payloads persist their sanitized snapshots.
  - failed request payloads persist only redacted diagnostics.
  - previous last-good snapshots remain available for dashboard display with degraded coverage.
- Connector/request rows may be separate, but they must be correlated by the aggregate refresh timestamp or a new `refresh_group_id`.
- The child process is always terminated after the aggregate reaches a terminal state.

### 2. Data Model and Migrations
Files:
- `src-tauri/src/db.rs`

Tasks:
- Add migration for `account_usage_snapshots`.
- Add migration for `account_usage_daily_buckets`.
- Add migration for sanitized account profile metadata, either:
  - `account_profile_snapshots`, or
  - explicit account profile columns on `account_usage_snapshots`.
- Add insert/load helpers:
  - `insert_account_profile_snapshot`
  - `load_latest_account_profile_snapshot`
  - `insert_account_usage_snapshot`
  - `load_latest_account_usage_snapshot`
  - `insert_account_usage_daily_bucket`
  - `load_latest_account_usage_daily_buckets`
- Add helpers for reset-credit available count if existing `reset_credit_batches` cannot represent a summary without expiration.
- Add an explicit account analytics seam:
  - `AccountUsageSnapshot`
  - `AccountUsageDailyBucket`
  - `load_latest_account_usage_snapshot`
  - `load_latest_account_usage_daily_buckets`
- Add deterministic storage for app-server rate-limit windows:
  - prefer a new `account_rate_limit_windows` table if provenance cannot be represented cleanly in `rate_limit_windows`.
  - otherwise add a source/limit-id convention and tests for `window_key`.
- Add deterministic storage for reset-credit summary:
  - prefer `account_rate_limit_reset_credit_snapshots` with `available_count` and optional detail JSON/redacted hash.
  - do not force app-server reset-credit summary into `reset_credit_batches` if expiration is unknown.

Required schema boundary:
- New usage tables for account profile data.
- Reuse `connector_runs`.
- Store `account/read` diagnostics fields only as sanitized metadata:
  - `account_kind`
  - `plan_type`
  - `email_present`
  - `requires_openai_auth`
- Do not store raw email addresses, backend account IDs, auth tokens, or raw `account/read` JSON.
- Account usage daily buckets must not be inserted into `usage_events`.
- Account reset-credit summary must not require per-credit expiration.
- Existing local `usage_events` remains the local/per-session source only.
- Add a refresh grouping field if separate connector rows are retained:
  - preferred: `refresh_group_id TEXT` on app-server-owned connector/profile/usage/rate-limit records.
  - fallback: shared `captured_at_utc` from the aggregate result, with tests proving correlation.

### 3. Connector Orchestration
Files:
- `src-tauri/src/connectors.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/safety.rs`

Tasks:
- Add `CodexAppServerConnector`.
- Replace default `refresh_remote_connectors` behavior with app-server connector only.
- Remove `load_auth_handle` from the default remote/account refresh path.
- Demote existing `KnownResetCreditsConnector` and `UndocumentedRateLimitsConnector` to disabled legacy/test-only code:
  - no normal refresh call sites.
  - no default active connector status based on these IDs.
  - remove `/wham/rate-limit-reset-credits` and `/backend-api/rate_limits` from `EndpointRegistry::default_readonly()` in `src-tauri/src/safety.rs`.
  - if retained, place their endpoint specs behind `TOKENSTACK_ENABLE_LEGACY_CHATGPT_ENDPOINTS=1` and cover that with tests.
  - optional legacy path only behind `TOKENSTACK_ENABLE_LEGACY_CHATGPT_ENDPOINTS=1`.
- Persist connector run statuses for:
  - `codex-account-usage`
  - `codex-account-rate-limits`
- Persist `codex-account` for `account/read` as mandatory sanitized profile diagnostics.
- Persist `account/read` as a first-class profile result:
  - status maps to `accountProfile.status`.
  - parseable success writes a sanitized profile snapshot.
  - failure writes redacted account-profile diagnostics.
- Ensure refresh lock still covers local import plus app-server refresh.
- Preserve local import even when app-server refresh fails.

### 4. Analytics DTOs
Files:
- `src-tauri/src/analytics.rs`
- `src/lib/schemas/dashboard.ts`

Tasks:
- Add account-source coverage entries.
- Implement explicit coverage source mappings:
  - `account-usage` from latest account usage snapshot, summary field presence, and bucket count.
  - `account-rate-limits` from latest account rate-limit snapshot, limit IDs, and reset-credit summary.
  - `local-history` from latest import run and local source documents.
- Refactor summary construction into explicit source summaries:
  - `load_local_usage_summary(conn, now)`
  - `load_account_usage_summary(conn, now)`
  - `select_metric_source(data_mode, local_summary, account_summary)`
- Choose top-level metrics by data mode:
  - local: existing local `usage_events`.
  - remote: latest account snapshot.
  - combined: latest account snapshot for account-level cards plus local detail panels.
- Convert daily account buckets into heatmap days.
- Convert app-server rate-limit snapshots into `RateLimitWindowDto`.
- Convert reset-credit available count into the reset card.
- Load account rate-limit windows by app-server connector ID/limit ID, not by legacy `undocumented-rate-limits`.
- Load account reset credits by app-server reset-credit summary, not by legacy `known-reset-credit`.
- Update metric delta labels:
  - account: `Codex account profile`
  - local: `Imported local history`
- Update metric labels by provenance:
  - local-only peak metric label: `Peak session`
  - account-backed peak metric label: `Peak day`
  - account-backed today/month deltas: `Codex account activity`
  - local-backed today/month deltas: `America/New_York bucket` / `Month-to-date` as currently appropriate.
- Handle null account summary fields gracefully.
- Avoid JS precision bugs by sending display strings for large values or string-backed raw values if adding raw fields.
- Add a degraded remote-mode state when account snapshot is missing rather than presenting zeroes as success.

### 5. Diagnostics and Export
Files:
- `src-tauri/src/commands.rs`
- `src/features/exports/diagnostics.ts`
- `src/lib/schemas/dashboard.ts`
- `src/components/command-center/CommandCenterShell.tsx`

Tasks:
- Extend `SetupDiagnosticsDto` with:
  - `codexBinary`
  - `appServer`
  - `accountProfile`
  - `accountUsage`
  - `accountRateLimits`
- Include observed counts and statuses, not sensitive values.
- `accountProfile` may include only:
  - account kind
  - plan type
  - email-present boolean
  - requires-auth boolean
  - redacted error code/message
- Exact diagnostics fields:
  - `codexBinary.status`
  - `codexBinary.resolvedPathPresent`
  - `codexBinary.source`
  - `codexBinary.redactedErrorCode`
  - `codexBinary.redactedErrorMessage`
  - `appServer.status`
  - `appServer.startupMs`
  - `appServer.requestCount`
  - `appServer.completedRequestCount`
  - `appServer.timedOut`
  - `appServer.redactedErrorCode`
  - `appServer.redactedErrorMessage`
  - `accountProfile.status`
  - `accountProfile.accountKind`
  - `accountProfile.planType`
  - `accountProfile.emailPresent`
  - `accountProfile.requiresOpenaiAuth`
  - `accountProfile.capturedAtUtc`
  - `accountProfile.redactedErrorCode`
  - `accountProfile.redactedErrorMessage`
  - `accountUsage.status`
  - `accountUsage.summaryPresent`
  - `accountUsage.lifetimeTokensPresent`
  - `accountUsage.dailyBucketCount`
  - `accountUsage.firstBucketDate`
  - `accountUsage.lastBucketDate`
  - `accountUsage.capturedAtUtc`
  - `accountUsage.redactedErrorCode`
  - `accountUsage.redactedErrorMessage`
  - `accountRateLimits.status`
  - `accountRateLimits.observedLimitIds`
  - `accountRateLimits.primaryWindowPresent`
  - `accountRateLimits.secondaryWindowPresent`
  - `accountRateLimits.resetCreditAvailableCount`
  - `accountRateLimits.resetCreditDetailsPresent`
  - `accountRateLimits.capturedAtUtc`
  - `accountRateLimits.redactedErrorCode`
  - `accountRateLimits.redactedErrorMessage`
- Keep app-server process diagnostics separate from legacy `connector_runs` diagnostics.
- Include disabled-legacy status only if the legacy flag is present.
- Update setup UI to render these diagnostics.
- Update diagnostics export tests for redaction and new fields.

### 6. UI Adjustments
Files:
- `src/components/command-center/CommandCenterShell.tsx`
- `src/components/command-center/DesktopStatusBar.tsx` if needed
- `src/components/command-center/sectionModel.ts` if copy changes are centralized

Tasks:
- Add active connector labels for app-server account usage/rate limits.
- Ensure combined mode makes account-vs-local provenance clear.
- Keep controls stable and avoid layout shifts with longer connector names.
- Preserve existing dashboard density and command-center style.

### 7. Tests
Files:
- Rust unit tests near new module and existing backend modules.
- Frontend tests near existing schema/export/component tests.

Tasks:
- Build fake Codex app-server process fixture.
- Cover happy path, missing binary, unauthenticated, malformed response, timeout, notification interleaving, and no consume calls.
- Update dashboard schema tests for account fields.
- Update diagnostics export tests for redaction.
- Run full required verification.

### 8. Documentation and Release Notes
Files:
- `docs/data-sources.md`
- `docs/architecture.md`
- `docs/connector-safety.md`
- possibly `README.md`

Tasks:
- Document Codex app-server as the authoritative account/profile source.
- Document local history as local-detail source.
- Document no-consume/no-raw-auth safety policy.
- Document troubleshooting for missing Codex CLI or signed-out CLI.

## ADR

### Decision
Use `codex app-server` JSON-RPC as TokenStack's account data source for usage, reset credits, and rate limits.

### Drivers
- It matched the user's profile-scale token count in a live smoke test.
- It uses Codex's own auth and backend client.
- It is documented and implemented in the open-source Codex repository.
- It avoids brittle private endpoint scraping.

### Alternatives Considered
- Direct private ChatGPT HTTP endpoints: rejected because they returned 404 and duplicate auth behavior.
- Local/remote file import only: rejected because Windows local files do not include most remote-session account activity.
- Reading Codex SQLite state only: rejected as insufficient for account/profile totals and not portable to Windows remote-session usage.

### Consequences
- TokenStack depends on Codex CLI availability for account data.
- Diagnostics must make binary/auth/protocol failures clear.
- Data model must separate account snapshots from local session import.
- Tests need fake app-server fixtures.

### Follow-Ups
- Consider app-server notification support after snapshot refresh is stable.
- Consider optional Codex binary path selection if PATH discovery is not reliable on Windows.
- Consider enterprise/admin analytics separately if workspace-wide reporting is requested later.

## Available Agent Types
- `executor`: implementation across Rust/Tauri and React.
- `test-engineer`: fake app-server fixture and verification matrix.
- `architect`: app-server client/process/data-model review.
- `critic`: final plan and implementation risk review.
- `code-reviewer`: pre-merge review.
- `verifier`: final evidence pass.
- `git-master`: commit, push, and merge hygiene.

## Suggested Staffing
- `$ultragoal` default: one durable goal with checkpoints for protocol client, storage, analytics, UI diagnostics, tests, docs, and git landing.
- `$team` option: parallelize into three lanes:
  - Rust backend app-server and storage lane.
  - Frontend diagnostics/dashboard lane.
  - Test/docs/verification lane.
- `$ralph` fallback: use only if a single-owner persistent verification loop is explicitly desired.

## Team Verification Path
- Backend lane proves fake app-server tests and cargo checks.
- Frontend lane proves schema/component/export tests and pnpm checks.
- Verification lane runs full command list and inspects diagnostics export for secret leakage.
- Final owner verifies manual smoke instructions and commit hygiene.

## Goal-Mode Follow-Up Suggestions
- `$ultragoal`: recommended default for durable sequential implementation.
- `$team`: recommended with `$ultragoal` if parallel delivery is desired.
- `$ralph`: explicit fallback only for single-owner persistence.
