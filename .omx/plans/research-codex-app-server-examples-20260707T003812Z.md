# Research notes: Codex app-server account examples

Created: 2026-07-07T00:38:12Z

## Sources inspected

- Official Codex manual, "Codex App Server" and "Authentication and sessions"
- Upstream `openai/codex` source at current `origin/main`
- `openai/codex/codex-rs/app-server/README.md`
- `openai/codex/codex-rs/app-server-client/README.md`
- `openai/codex/codex-rs/app-server-daemon/README.md`
- `openai/codex/codex-rs/tui/src/app/background_requests.rs`
- `openai/codex/codex-rs/app-server/src/request_processors/account_processor.rs`
- `heycarollan/codex-usage-status`
- `shanggqm/codexU`
- `nelsonjchen/codex-status-command`
- `steipete/CodexBar`

## Confirmed JSON-RPC method set

TokenStack needs:

- `initialize`
- `initialized` notification
- `account/read`
- `account/rateLimits/read`
- `account/usage/read`

TokenStack must observe but not require:

- `account/rateLimits/updated`

TokenStack must not call:

- `account/rateLimitResetCredit/consume`

## Launch shape

Preferred first attempt:

```text
codex app-server --listen stdio:// -c mcp_servers={}
```

Compatibility fallback:

```text
codex app-server
```

Use the fallback only when the first launch fails with an argument/support error. Do not silently hide recurring timeouts by retrying with multiple long-running children.

## Initialization

Send an `initialize` request with:

- stable protocol version if needed by the local client model
- `clientInfo.name = "tokenstack"`
- `clientInfo.version` from the app version
- `capabilities.experimentalApi = true`

After successful initialize response, send the `initialized` notification before account method calls.

## Request behavior

- Use monotonic request IDs.
- Track pending requests by ID.
- Ignore notifications and responses for unknown IDs.
- Return the response matching the requested ID.
- Capture JSON-RPC error objects with method context.
- Reject unsupported server requests with a JSON-RPC error if the app-server asks TokenStack to do something interactive.
- Drain stderr concurrently into a bounded buffer.
- Sanitize all diagnostic output before storage or export.

## Timeout behavior

Use separate timeout scopes:

- CLI discovery timeout: short and non-blocking.
- Process launch/initialize timeout: about 8 to 15 seconds.
- Per-request timeout: about 12 seconds.
- Whole refresh timeout: about 25 to 30 seconds.

On timeout:

- mark the current account source degraded or unavailable
- terminate the child process
- wait for the child process to exit
- use the last good snapshot if one exists
- emit a diagnostic event with sanitized stderr suffix and timeout stage

## Normalization rules

Rate limits:

- Prefer `rateLimitsByLimitId` when present.
- Fall back to `rateLimits` when the map is absent.
- Show the `codex` bucket first when present.
- Preserve additional bucket IDs and labels.
- Common window durations:
  - 300 minutes: 5-hour window
  - 10080 minutes: 7-day window
- Store unknown windows rather than dropping them.
- Remaining percent is `max(0, 100 - usedPercent)`.
- Do not invent absolute token quotas from percent-only windows.

Reset credits:

- Parse from `rateLimitResetCredits.availableCount`.
- Preserve expiry/reset-credit metadata when available.
- Display unavailable when the route fails, not zero.

Usage:

- Parse lifetime tokens from account usage summary.
- Parse daily buckets from `dailyUsageBuckets` or current schema equivalent.
- Treat account usage failures as partial failure if rate limits succeeded.
- Do not replace account lifetime usage with local imported JSONL totals.

## Windows-specific behavior

- Provide a configurable Codex executable path in setup/diagnostics.
- Support `TOKENSTACK_CODEX_BIN` for deterministic testing and user override.
- Search PATH using a platform-appropriate command or Rust path lookup.
- Do not assume Windows remote session data exists under local `%USERPROFILE%`.
- Report exact executable path selected, launch mode, and first failing stage.
- Avoid shell invocation when spawning Codex; pass arguments directly to `Command`.

## Logging and diagnostics

Diagnostics export should include:

- TokenStack version, OS, architecture, timezone
- selected data mode
- database path
- Codex executable candidates and selected candidate
- launch mode used
- whether MCP startup was disabled
- app-server initialize status
- account method success/failure by method
- HTTP/RPC error code and sanitized message
- timeout stage and duration
- child process exit status
- whether the child was killed on timeout
- whether a last-good snapshot was used
- account bucket IDs seen
- daily bucket count
- reset-credit count if available
- local folders scanned, file counts, event counts, warning counts
- redaction status and export timestamp

Diagnostics export must not include:

- auth tokens
- cookies
- raw prompt text
- raw tool output
- raw JSONL conversation bodies
- full stderr if it contains token-like values

## Implementation lesson from examples

The app-server route works when it is treated as a real child-process protocol with explicit lifecycle management. The broken behavior in TokenStack is not solved by adding more local folders; it is solved by making account snapshots a separate, hardened app-server connector with clear degradation and logs.
