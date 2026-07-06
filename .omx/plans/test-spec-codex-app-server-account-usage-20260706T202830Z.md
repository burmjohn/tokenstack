# Test Spec: Codex App-Server Account Usage Integration

Generated: 2026-07-06T20:28:30Z
Status: draft for ralplan review

## Test Strategy
Use fixtures and process-level fakes for deterministic tests. Do not require live Codex auth for CI. Add a manual smoke script or ignored test for signed-in local verification.

## Rust Unit Tests

### JSON-RPC Client
- Parses newline-delimited JSON-RPC responses by `id`.
- Ignores notifications that arrive between request responses.
- Captures `account/rateLimits/updated` notifications without corrupting request matching.
- Times out cleanly when initialize never completes.
- Times out cleanly when one request never responds.
- Returns redacted errors when the child exits early.
- Drains stderr and includes only redacted, bounded stderr snippets in diagnostics.

### App-Server Fixture Parsing
- Parses `account/read` ChatGPT account response with plan type and email present.
- Parses `account/usage/read` summary fields:
  - `lifetimeTokens`
  - `peakDailyTokens`
  - `longestRunningTurnSec`
  - `currentStreakDays`
  - `longestStreakDays`
- Parses `dailyUsageBuckets` with `{ startDate, tokens }`.
- Handles `summary` fields that are null.
- Handles `dailyUsageBuckets` null.
- Parses `account/rateLimits/read`:
  - legacy `rateLimits`
  - `rateLimitsByLimitId`
  - `rateLimitResetCredits.availableCount`
  - optional detailed reset credits when present in newer schema.
- Handles missing `rateLimitResetCredits` as unavailable, not zero.

### Storage
- Migration creates account snapshot tables idempotently.
- Migration creates sanitized account profile snapshot storage idempotently.
- Persisting a usage snapshot writes one `account_usage_snapshots` row and associated daily buckets.
- Persisting `account/read` writes account kind, plan type, email-present boolean, and requires-auth boolean without storing raw email or account IDs.
- Persisting a new account snapshot supersedes earlier snapshots in analytics without deleting historical snapshots.
- Persisting rate-limit snapshots maps multi-limit windows into deterministic `window_key` values.
- Reset-credit available count is stored without requiring an expiration timestamp.
- Big token counts round-trip as i64 in Rust and as safe display strings in frontend DTOs.

### Connector Results
- Successful app-server usage/rate-limit refresh stores connector run status `complete`.
- Missing Codex binary stores connector run status `failed` with code `codex_binary_unavailable`.
- Unauthenticated app-server stores connector run status `failed` or `degraded` with code `codex_auth_required`.
- App-server incompatible method error stores `app_server_method_unavailable`.
- Direct private HTTP connector failure no longer drives default reset/rate-limit status.
- Default refresh does not call `KnownResetCreditsConnector` or `UndocumentedRateLimitsConnector`.
- `load_auth_handle` is not used by the default remote/account refresh path.
- Private ChatGPT endpoint allowlist entries are unavailable unless the explicit legacy flag is enabled.
- `EndpointRegistry::default_readonly()` does not register `/wham/rate-limit-reset-credits` or `/backend-api/rate_limits` by default.
- Enabling `TOKENSTACK_ENABLE_LEGACY_CHATGPT_ENDPOINTS=1` is the only way to expose retained legacy private endpoint validation, if legacy code remains.
- One fake app-server refresh uses one spawned process/session and produces one aggregate result.
- The aggregate result persists in one SQLite transaction after all three account requests finish or reach terminal status.
- Partial success persists parseable successful records and redacted failure diagnostics for failed requests.
- Each persisted app-server request diagnostic has the same aggregate refresh timestamp or group id.

### Analytics Seam
- `build_dashboard_summary("remote")` reads latest `account_usage_snapshots` and returns nonzero lifetime/today/month/peak values from account data.
- `build_dashboard_summary("combined")` prefers account profile metrics for cards and preserves local sessions from `usage_events`.
- `build_dashboard_summary("local")` ignores account snapshots and preserves existing local-only behavior.
- Account daily buckets render heatmap data without being inserted into `usage_events`.
- Missing account snapshot in remote mode produces degraded/unavailable coverage, not misleading zero-success metrics.
- Account-backed metric labels use `Peak day` and account-profile/account-activity deltas.
- Local-only metric labels keep `Peak session` and local-history deltas.
- Combined mode account-backed cards never display `Imported local history`.
- Coverage mappings are tested independently:
  - account usage coverage from account usage snapshot presence and bucket count.
  - account rate-limit coverage from app-server rate-limit snapshot and reset-credit summary.
  - local history coverage from import run/source document counts.

## Rust Integration Tests
- Use a fake `codex` executable placed first in PATH that implements enough `app-server` JSON-RPC for tests.
- Verify `refresh_all_with_auth_home` or its successor can refresh local import and app-server snapshots in one call.
- Verify the fake app-server receives no request with `rateLimitResetCredit/consume`.
- Verify child process is terminated after refresh.
- Verify concurrent refresh lock still prevents simultaneous child app-server processes.
- Verify a fake app-server happy path persists account snapshots, daily buckets, reset-credit summary, and multi-limit rate windows in one refresh.

## Frontend Schema Tests
- `dashboardSummarySchema` accepts account-source metrics and account connector entries.
- `setupDiagnosticsSchema` accepts app-server diagnostics fields.
- Big account totals are represented as strings or already formatted display values where JavaScript precision matters.

## Frontend Component Tests
- Setup diagnostics renders Codex app-server status and failure codes.
- Active connectors shows:
  - Codex account usage
  - Codex account rate limits
  - Local Codex history
- Combined mode dashboard copy does not call account totals "Imported local history".
- Export diagnostics includes app-server diagnostics.
- Export diagnostics redaction test rejects token-shaped values.

## Manual Smoke Tests

### Signed-In Happy Path
1. Ensure Codex CLI is signed in with ChatGPT auth.
2. Run TokenStack installed/dev desktop app.
3. Click `Scan local data`.
4. Expected:
   - lifetime tokens matches Codex profile order of magnitude.
   - reset credits available count appears.
   - rate-limit windows appear.
   - diagnostics show app-server connected.

### Codex Missing
1. Launch TokenStack with PATH excluding Codex CLI.
2. Click `Scan local data`.
3. Expected:
   - app does not crash.
   - diagnostics say Codex binary unavailable.
   - local history still imports if present.

### Codex Not Authenticated
1. Launch with a fake Codex home or signed-out Codex CLI.
2. Click `Scan local data`.
3. Expected:
   - account usage/rate limits degraded.
   - diagnostics say ChatGPT authentication required.
   - no raw auth content in logs/export.

### Diagnostics Redaction
- Exported diagnostics may contain `accountKind`, `planType`, `emailPresent`, and `requiresOpenaiAuth`.
- Exported diagnostics must not contain a raw email address, backend account ID, access token, refresh token, or authorization header.
- Exported diagnostics include exact app-server status groups:
  - `codexBinary`
  - `appServer`
  - `accountProfile`
  - `accountUsage`
  - `accountRateLimits`
- `accountUsage` reports summary presence, lifetime-token presence, daily bucket count, first bucket date, and last bucket date.
- `accountRateLimits` reports observed limit IDs, primary/secondary window presence, reset-credit available count, and reset-credit detail presence.

## Required Verification Commands
- `pnpm typecheck`
- `pnpm lint`
- `pnpm test`
- `pnpm build`
- `pnpm secret:scan` if available
- `pnpm fixture:scan` if available
- `cargo fmt --check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`

## Completion Evidence
- Test output from all required commands.
- Sanitized diagnostics export from a fake app-server fixture.
- Manual smoke output from a signed-in developer machine, with personal fields redacted.
