# PRD: Codex App-Server Account Usage Integration

Generated: 2026-07-06T20:28:30Z
Status: draft for ralplan review

## Problem
TokenStack currently reports misleading account-scale numbers because it treats locally imported Codex session history as the primary source for lifetime tokens and daily activity. That breaks for users who run most Codex work through remote sessions connected from the Codex desktop app. The same installed app also fails to retrieve reset credits and rate-limit windows because its connector path reads raw auth from disk and calls brittle private ChatGPT URLs that returned 404 in user diagnostics.

## Users
- Primary: TokenStack users who install the desktop app on Windows and use Codex desktop/CLI with ChatGPT auth.
- Secondary: users who run local-only Codex sessions and still need per-session local history import.
- Maintainers: open-source contributors who need clear diagnostics without handling secrets.

## Goals
- Use `codex app-server` as the account data bridge for TokenStack.
- Populate dashboard profile metrics from `account/usage/read`.
- Populate reset credits and rate-limit windows from `account/rateLimits/read`.
- Record account connector diagnostics in the existing diagnostics export without exposing tokens, raw account IDs, or raw auth.
- Keep local session import available for local-specific details, but clearly separate its provenance from account/profile totals.
- Provide a first-pass Windows-ready failure model when the Codex binary or auth is unavailable.

## Non-Goals
- No reset-credit redemption or any `/consume` call.
- No direct scraping of private ChatGPT HTTP endpoints for the account metrics covered by app-server.
- No raw `auth.json` parsing in the new account connector path.
- No manual Codex binary picker in the first pass unless implementation proves discovery cannot be made diagnosable.
- No attempt to reconstruct remote thread-level histories from the Windows machine.

## RALPLAN-DR Summary

### Principles
- Codex owns Codex auth. TokenStack should request account data through Codex app-server, not reimplement auth handling.
- Account/profile metrics and local session metrics must remain provenance-separated.
- Diagnostics must explain failure states without leaking secrets.
- Read-only account APIs are allowed; mutating account APIs are prohibited.
- Prefer a narrow compatibility layer over broad rewrites.

### Decision Drivers
- Correctness: profile-scale token totals must match the Codex app account surface.
- Safety: do not read or expose auth tokens and do not call consume endpoints.
- Maintainability: use documented/open-source app-server protocol rather than brittle private URLs.

### Viable Options
- Option A: Spawn `codex app-server` from Tauri and call JSON-RPC methods.
  - Pros: uses Codex's saved auth, refresh handling, backend client, and documented protocol; smoke test already matched profile numbers.
  - Cons: depends on Codex CLI being installed and discoverable by the Windows GUI app.
- Option B: Continue reading auth and call private ChatGPT endpoints directly.
  - Pros: fewer moving parts inside TokenStack.
  - Cons: already failed with 404; duplicates auth logic; higher secret handling risk; not aligned with open-source Codex app-server.
- Option C: Import only local/remote SQLite and JSONL files.
  - Pros: fully local and inspectable.
  - Cons: cannot solve Windows profile totals when work happens on remote machines; cannot reliably get account reset credits or rate limits.

Chosen direction: Option A. Options B and C remain useful only as fallbacks for local history and legacy diagnostics, not as the account source of truth.

## Functional Requirements

### Account Connector
- Add a new backend connector surface, tentatively `codex-app-server-account`.
- Make this connector the only default remote/account connector.
- Remove raw `auth.json` reading from the default remote/account refresh path.
- Remove direct private endpoint calls from the default remote/account refresh path.
- Resolve the Codex binary in this order:
  - `TOKENSTACK_CODEX_BIN` environment override for tests/support.
  - `codex` from PATH.
  - Common Windows npm/global install locations only if low-risk and testable.
- Spawn `codex app-server` as a child process and speak JSON-RPC over stdin/stdout.
- Send `initialize`, then `initialized`, then the read-only account requests.
- Required requests:
  - `account/read`
  - `account/usage/read`
  - `account/rateLimits/read`
- Optional follow-up after first pass:
  - Subscribe/merge `account/rateLimits/updated` notifications while a process is alive.
- Terminate the child process after refresh or timeout.
- Enforce timeouts:
  - startup/initialize timeout
  - per-request timeout
  - whole-refresh timeout
- Redact all errors before storing or exporting.
- Refresh aggregate contract:
  - A remote/account refresh spawns exactly one `codex app-server` child process for that refresh attempt.
  - The process performs `initialize`, `initialized`, `account/read`, `account/usage/read`, and `account/rateLimits/read` in one app-server session.
  - The connector returns one aggregate result containing profile, usage, rate-limit, process diagnostics, and per-request status.
  - Persistence happens in one SQLite transaction after the aggregate result is assembled.
  - Partial app-server results are allowed: for example, a successful `account/read` may persist profile diagnostics even if `account/usage/read` fails.
  - Each request gets a distinct redacted status in diagnostics; connector run rows may be per request, but they must share the same aggregate refresh timestamp or group id.
  - The child process must be terminated after the refresh attempt, including timeout and error paths.
- Legacy connector policy:
  - `KnownResetCreditsConnector` and `UndocumentedRateLimitsConnector` must not run during normal refresh.
  - Any retained legacy connector code must be behind an explicit test/forensics-only flag such as `TOKENSTACK_ENABLE_LEGACY_CHATGPT_ENDPOINTS=1`.
  - `EndpointRegistry::default_readonly()` in `src-tauri/src/safety.rs` must not include private ChatGPT reset/rate-limit URLs unless that flag is enabled.

### Storage
- Add account-specific snapshot storage rather than mixing profile buckets into `usage_events`.
- Proposed tables:
  - `account_usage_snapshots`
    - `id`
    - `connector_run_id`
    - `captured_at_utc`
    - `account_kind`
    - `plan_type`
    - `email_present`
    - `lifetime_tokens`
    - `peak_daily_tokens`
    - `longest_running_turn_sec`
    - `current_streak_days`
    - `longest_streak_days`
    - `schema_version`
  - `account_profile_snapshots`
    - `id`
    - `connector_run_id`
    - `captured_at_utc`
    - `account_kind`
    - `plan_type`
    - `email_present`
    - `requires_openai_auth`
    - `schema_version`
  - `account_usage_daily_buckets`
    - `id`
    - `snapshot_id`
    - `start_date`
    - `tokens`
  - Extend or reuse `rate_limit_windows` for app-server windows with `window_key` values like `codex:primary`, `codex:secondary`, `codex_bengalfox:primary`.
  - Store reset-credit summary from `rateLimitResetCredits.availableCount`; store detailed credits only when available and only non-secret fields.
- Keep old local `usage_events` as the source for local-session views.
- Persist sanitized `account/read` output in `account_profile_snapshots` or the equivalent account-profile portion of `account_usage_snapshots`; do not store raw email addresses or account IDs.
- Persistence mapping:
  - `account/read` always attempts to persist an account profile snapshot when it returns a parseable response.
  - `account/usage/read` persists one account usage snapshot plus zero or more daily buckets.
  - `account/rateLimits/read` persists one account rate-limit snapshot group, zero or more window rows keyed by limit ID/window kind, and one reset-credit availability snapshot when present.
  - Request failures persist redacted connector/request diagnostics without deleting last-good snapshots.

### Analytics
- In `remote` mode:
  - dashboard lifetime/today/month/peak should use the latest successful account usage snapshot.
  - heatmap should use latest daily buckets.
  - reset credits should use latest app-server rate-limit snapshot.
  - local sessions list can be empty or explicitly unavailable.
- In `combined` mode:
  - account usage should drive top-level profile metrics.
  - local sessions/local import remain visible in sessions/source coverage.
  - UI copy should avoid implying account totals are the sum of local plus remote files.
- In `local` mode:
  - keep current local history behavior.
- Coverage should distinguish:
  - `account-usage`
  - `account-rate-limits`
  - `local-history`
- Source-to-coverage mapping:
  - `account-usage`: latest successful `account/usage/read` snapshot and daily bucket count.
  - `account-rate-limits`: latest successful `account/rateLimits/read` snapshot, rate-limit IDs, and reset-credit availability.
  - `local-history`: latest local import run, files/events/imported counts, and source documents.
- Account analytics seam:
  - add a `load_account_profile_summary` path that returns the latest account usage snapshot plus daily buckets.
  - add a `load_local_usage_summary` path for existing `usage_events`.
  - choose between those summaries before constructing top-level metric DTOs.
  - ensure account daily buckets power the heatmap in remote/combined mode when present.
  - do not insert account daily buckets into `usage_events`.

### Diagnostics
- Add app-server diagnostics to `get_setup_diagnostics`:
  - Codex binary resolution status.
  - Codex app-server startup status.
  - account auth type, plan type, and email presence only.
  - `requiresOpenaiAuth` from `account/read` when available.
  - usage summary presence and daily bucket count.
  - rate-limit IDs observed.
  - reset-credit available count.
  - redacted error code and message.
- Diagnostics export must never contain:
  - access tokens
  - refresh tokens
  - full auth documents
  - raw email address
  - raw backend account IDs
- If `codex` is missing, show a connector status that points to installing/signing into Codex CLI rather than saying "not connected" generically.
- Exact diagnostics DTO contract:
  - `codexBinary`: `{ status, resolvedPathPresent, source, redactedErrorCode, redactedErrorMessage }`
  - `appServer`: `{ status, startupMs, requestCount, completedRequestCount, timedOut, redactedErrorCode, redactedErrorMessage }`
  - `accountProfile`: `{ status, accountKind, planType, emailPresent, requiresOpenaiAuth, capturedAtUtc, redactedErrorCode, redactedErrorMessage }`
  - `accountUsage`: `{ status, summaryPresent, lifetimeTokensPresent, dailyBucketCount, firstBucketDate, lastBucketDate, capturedAtUtc, redactedErrorCode, redactedErrorMessage }`
  - `accountRateLimits`: `{ status, observedLimitIds, primaryWindowPresent, secondaryWindowPresent, resetCreditAvailableCount, resetCreditDetailsPresent, capturedAtUtc, redactedErrorCode, redactedErrorMessage }`
  - `status` values should be `connected`, `degraded`, or `unavailable` to match the UI connector vocabulary.

### UI
- Dashboard cards:
  - Lifetime tokens should use account snapshot in remote/combined mode.
  - Today/month should use account daily buckets when account usage is available.
  - Peak should become "Peak day" for account mode unless local-only.
  - Reset credits should use app-server `rateLimitResetCredits.availableCount`.
- Provenance-aware label rule:
  - local metric source uses "Lifetime tokens", "Today", "This month", and "Peak session" with local-history deltas.
  - account metric source uses "Lifetime tokens", "Today", "This month", and "Peak day" with account-profile/account-activity deltas.
  - combined mode must display account-backed top cards with account deltas while local session panels retain local wording.
  - Never display "Imported local history" on a value sourced from account snapshots.
- Source coverage and active connectors:
  - Show "Codex account usage" connected/degraded/unavailable.
  - Show "Codex account rate limits" connected/degraded/unavailable.
  - Keep "Local Codex history" separate.
- Setup section:
  - Add app-server diagnostic block near current diagnostics.
  - Export diagnostics button should include all new redacted fields.

## Acceptance Criteria
- With a signed-in Codex CLI, TokenStack refresh persists an account usage snapshot and displays lifetime tokens in the same order as the Codex profile.
- With a signed-in Codex CLI, TokenStack refresh displays reset credits from `account/rateLimits/read`.
- With a signed-in Codex CLI, TokenStack refresh displays rate-limit windows from `rateLimitsByLimitId` when present.
- With no Codex CLI on PATH, refresh does not crash and diagnostics say Codex binary unavailable.
- With Codex CLI installed but not authenticated, refresh does not crash and diagnostics say ChatGPT/Codex auth is required.
- Diagnostics export contains no token-shaped secret or raw `auth.json` content.
- Existing local history import tests continue passing.
- Direct private endpoint connectors are removed from the default refresh path or explicitly demoted to disabled legacy fallback.
- `build_dashboard_summary("remote")` shows account snapshot metrics instead of zero local metrics.
- `build_dashboard_summary("combined")` uses account snapshot metrics for profile cards while preserving local sessions/source coverage.

## Risks
- Windows GUI process PATH may not include the Codex CLI install path.
- App-server protocol can evolve. TokenStack must parse permissively and include schema-versioned storage.
- Long-running child process management can hang if stdout/stderr are not drained correctly.
- Big integer fields may exceed frontend JavaScript safe integer range if treated as numbers. Backend should format summary strings or serialize large counts as strings where needed.

## Release Notes Target
TokenStack now reads Codex account usage, reset credits, and rate limits through Codex's app-server protocol, matching the profile-scale data shown by Codex while keeping local session import separate for local history detail.
