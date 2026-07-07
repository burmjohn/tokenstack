# Test spec: Codex account usage through app-server, v2

Created: 2026-07-07T00:38:12Z

## Unit tests

### JSON-RPC client

Test:

- initialize request is sent with experimental API enabled
- initialized notification is sent after initialize response
- request IDs are monotonic
- response with matching ID resolves the correct request
- response with unknown/wrong ID is ignored
- notification does not fail pending request
- JSON-RPC error returns method-aware connector error
- malformed stdout returns parse error with sanitized context
- stderr is drained and bounded
- stderr diagnostics are redacted
- per-request timeout terminates child
- initialize timeout terminates child
- drop/cleanup terminates and waits for child
- unsupported server request receives a JSON-RPC error response or is safely ignored by explicit policy

### Launch resolution

Test:

- `TOKENSTACK_CODEX_BIN` is honored
- explicit configured path wins over PATH
- missing executable returns `missing_cli`
- first launch uses `app-server --listen stdio:// -c mcp_servers={}`
- old CLI argument failure falls back to plain `app-server`
- timeout does not trigger repeated fallback children
- spawn uses direct arguments, not shell command strings

### Account methods

Test:

- account refresh calls `account/read`
- account refresh calls `account/rateLimits/read`
- account refresh calls `account/usage/read`
- `account/usage/read` failure can produce partial success if rate limits succeed
- logged-out account maps to logged-out/degraded UI state
- no code path calls `account/rateLimitResetCredit/consume`

### Normalization

Test:

- `rateLimitsByLimitId` is preferred
- `rateLimits` fallback works
- `codex` bucket sorts first
- extra bucket IDs are preserved
- 300-minute window maps to 5-hour display metadata
- 10080-minute window maps to 7-day display metadata
- unknown window duration is preserved
- used percent clamps to 0..100
- remaining percent is derived from used percent
- reset credits parse from `availableCount`
- zero reset credits are distinguishable from unavailable reset credits
- lifetime tokens parse from account usage summary
- daily usage bucket dates and token values persist
- local imported totals cannot overwrite account lifetime totals

### Storage

Test:

- migrations are additive
- a refresh group records method-level statuses
- failed refresh can preserve and return last-good snapshot
- stale snapshot is marked stale/degraded
- local history source coverage and account source coverage are independent

### Diagnostics

Test:

- export writes a file
- export includes schema version, app version, OS, selected mode, DB path, selected Codex executable, launch mode, method statuses, timeout stage, and local scan counts
- export excludes token-like strings
- export excludes cookies
- export excludes prompt/message bodies
- export excludes raw JSONL conversation content
- export includes redaction summary
- failed export shows actionable UI state

## Integration tests

Use a fake app-server executable in tests.

Scenarios:

1. Happy path
   - fake server responds to all methods
   - dashboard model contains account lifetime, daily buckets, rate windows, reset credits

2. Rate-limit success, usage failure
   - rate-limit UI works
   - usage panel says account usage unavailable/degraded
   - source coverage reflects partial success

3. Logged out
   - account/read returns auth error
   - UI says Codex login required
   - local history still works

4. Missing CLI
   - resolver fails
   - setup diagnostics says missing CLI and shows override instructions
   - no panic and no misleading zero values

5. Hung app-server
   - fake process never responds
   - timeout fires
   - child is killed
   - diagnostics include timeout stage
   - UI uses last-good snapshot if present

6. Old CLI
   - first launch rejects `--listen`
   - fallback launch succeeds
   - diagnostics record fallback mode

7. Notifications and out-of-order messages
   - fake server emits `account/rateLimits/updated` during requests
   - pending calls still resolve correctly

8. Export diagnostics from Windows-like path data
   - paths are preserved as paths
   - secrets are redacted
   - file path is returned to frontend

## Frontend tests

Test:

- reset credits show unavailable when snapshot missing
- reset credits show zero only when explicit zero is reported
- rate-limit windows render bucket names and window durations
- local lifetime label does not masquerade as account lifetime
- account snapshot stale badge renders when last-good data is used
- export button shows success state with path
- export button shows failure state when backend command errors
- setup diagnostics shows selected Codex executable and first failing stage

## Manual Windows smoke test

On Windows installed app:

1. Confirm `codex --version` works in the same user context, or configure path in TokenStack.
2. Click setup refresh or scan local data.
3. Confirm setup diagnostics shows the selected Codex executable.
4. Confirm account connector status is connected or shows a precise logged-out/missing/unsupported error.
5. Confirm dashboard lifetime account tokens match Codex profile within expected refresh delay.
6. Confirm rate-limit windows populate from account snapshot.
7. Confirm reset-credit status is unavailable/degraded or populated, not silently zero.
8. Click export diagnostics.
9. Confirm a file is created and contains no token/cookie/prompt content.

## Required verification commands

Run the repo-appropriate equivalents:

```text
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
pnpm test
pnpm build
pnpm secret:scan
pnpm fixture:scan
```

If a command does not exist, document the exact failure and the closest successful substitute.
