# Route plan: Codex account usage through app-server, v2

Created: 2026-07-07T00:38:12Z

## Goal

Make TokenStack work for a Windows desktop install where the user's Codex work mostly happens through remote Codex sessions. The app must show account-level Codex profile usage, reset credits, and rate-limit windows through the authenticated Codex app-server, while keeping local JSONL/session import as local history only.

## Success criteria

- Clicking "Scan local data" imports local history and also refreshes available account snapshots when the selected mode includes remote/account data.
- Dashboard lifetime/daily profile totals match Codex account usage from `account/usage/read` when available.
- Reset-credit and rate-limit panels populate from `account/rateLimits/read`.
- Local-only data is clearly labeled local history and never presented as full account lifetime usage.
- Failed connectors show actionable degraded/unavailable states, not misleading zeros.
- Export diagnostics produces a sanitized file that explains why account snapshots failed on Windows.
- No raw auth tokens, cookies, prompt bodies, or private endpoint output are read or exported.
- Tests prove protocol behavior, normalization, failure handling, and no reset-credit consumption.

## Non-goals

- Do not scrape ChatGPT private endpoints.
- Do not parse or export raw `auth.json` token values.
- Do not launch interactive Codex TUI/PTTY flows during automatic refresh.
- Do not depend on the experimental app-server daemon.
- Do not try to reconstruct remote session totals from Windows local files.
- Do not spend reset credits.

## Architecture

### Data-source model

TokenStack should have two explicit source families:

1. Local history source
   - Reads local Codex JSONL/session folders.
   - Populates local `usage_events` or equivalent existing tables.
   - Provides local-only token/event summaries.

2. Account snapshot source
   - Spawns `codex app-server` and calls account JSON-RPC methods.
   - Populates account usage, rate-limit, and reset-credit snapshot tables.
   - Provides Codex profile totals and current quota windows.

The dashboard can combine these in "Local + Remote" mode, but the data model must preserve provenance.

### Rust module boundaries

Add or refactor toward these modules:

- `codex_app_server`
  - Child process launch
  - JSON-RPC framing
  - initialize/initialized handshake
  - typed account method calls
  - timeout and cleanup

- `account_snapshots`
  - Normalized structs for account usage, rate limits, reset credits
  - Storage and last-good snapshot retrieval
  - Conversion from app-server response JSON

- `diagnostics`
  - Sanitized event recording
  - Diagnostics export command
  - Redaction scanner

Keep the existing local import path intact except where labels/provenance need correction.

## Implementation stages

### Stage 1: fixture spike and contract lock

Create fake app-server fixtures before the production connector:

- happy path with `initialize`, `initialized`, `account/read`, `account/rateLimits/read`, `account/usage/read`
- `rateLimitsByLimitId` path
- `rateLimits` fallback path
- notification before/after responses
- out-of-order/wrong IDs
- JSON-RPC error
- hung initialize
- hung method request
- malformed non-JSON stdout
- stderr-only failure
- old CLI rejecting `--listen stdio://`
- logged-out account error

These fixtures define the behavior the Rust client must satisfy.

### Stage 2: app-server client

Implement the child-process client:

- Resolve Codex executable from:
  - explicit UI setting if present
  - `TOKENSTACK_CODEX_BIN`
  - PATH lookup
  - discovered candidate list for diagnostics only
- Try `codex app-server --listen stdio:// -c mcp_servers={}` first.
- Fall back to `codex app-server` only on argument/support failure.
- Use direct process spawn, not shell invocation.
- Pipe stdin/stdout/stderr.
- Read stdout by line as JSON-RPC.
- Drain stderr concurrently into a bounded sanitized buffer.
- Send initialize with experimental API enabled.
- Send initialized notification after initialize success.
- Use per-request and whole-refresh timeouts.
- Kill and wait for the child on timeout, error, drop, or cancellation.
- Reject or ignore unsupported app-server initiated requests.

### Stage 3: account snapshot connector

Implement the account refresh:

- Call `account/read` first for connection/auth status.
- Call `account/rateLimits/read` and `account/usage/read`.
- Let account usage fail independently when rate limits succeed.
- Persist a refresh group that records method-level status.
- Use last-good snapshots for UI display when current refresh partially fails.
- Mark stale/degraded clearly.
- Never call `account/rateLimitResetCredit/consume`.

### Stage 4: normalization

Normalize account data:

- `rateLimitsByLimitId` first, `rateLimits` fallback.
- Primary `codex` bucket first.
- Extra buckets retained and visible.
- Primary/secondary windows normalized by `windowDurationMins`, preserving unknown windows.
- `remainingPercent = clamp(100 - usedPercent, 0, 100)`.
- Reset credits from `rateLimitResetCredits.availableCount`.
- Usage lifetime and daily buckets from account usage summary.
- No account totals sourced from local JSONL.

### Stage 5: storage and migrations

Add account snapshot tables if not already present:

- account refresh runs
- account identity snapshot with non-sensitive fields only
- account usage summary snapshot
- account daily usage buckets
- account rate-limit buckets
- account rate-limit windows
- account reset-credit snapshot
- connector diagnostics/events

Use additive migrations. Do not mutate existing local history semantics except for labels/provenance corrections.

### Stage 6: UI and IPC

Update commands/state:

- `scan_local_data` should report local import status plus account snapshot status when mode includes remote/account.
- Add or update `refresh_account_snapshots` if separate refresh is cleaner.
- `export_diagnostics` must return a saved file path or a user-visible error.
- UI should show:
  - connected, degraded, unavailable, logged out, missing CLI, unsupported CLI
  - last successful account refresh time
  - source family labels
  - bucket IDs/names
  - stale snapshot warning when using last-good data

Do not display reset credits as `0` unless the app-server explicitly reports zero.

### Stage 7: diagnostics export

Make export deterministic:

- On click, open a native save dialog if supported or write to the app data diagnostics directory.
- Show success/failure toast/status with the exported path.
- Include sanitized JSON with schema version.
- Include redaction summary.
- Include connector method history.
- Include app-server launch details and timeout/error stages.

If export cannot open a dialog on Windows, fallback to a known app-data path and copy/open-folder affordance.

### Stage 8: refresh coalescing

Prevent stampedes:

- Only one account refresh process may run at a time.
- Concurrent UI calls await the in-flight refresh or receive the same result.
- Background auto-refresh must not launch repeated failing app-server children.
- Use cooldown after missing CLI, logged-out, unsupported CLI, or repeated timeout.
- Manual refresh can bypass cooldown but still cannot run concurrently.

### Stage 9: documentation

Update project docs with:

- Data-source distinction: local history vs account snapshot.
- How to set Codex executable path on Windows.
- How to run diagnostics export.
- What logs are safe to paste.
- Known limitations: app-server experimental API, percent-only rate windows, no private endpoints.

## Test plan

See `test-spec-codex-app-server-account-usage-v2-20260707T003812Z.md`.

## Rollout sequence

1. Land fixtures and protocol tests.
2. Land app-server client behind account connector.
3. Land storage migrations and normalization.
4. Land UI labels/status changes.
5. Land diagnostics export.
6. Run Windows build/smoke on the installed app.
7. Ask the user to export diagnostics only if the installed app still fails.

## Acceptance gate

Do not mark the implementation complete until:

- Rust unit tests pass.
- Frontend tests/build pass.
- Tauri build or check passes.
- Secret/fixture scan passes.
- Manual or scripted fake app-server smoke proves export diagnostics creates a file.
- There is evidence that app-server failures render degraded/unavailable, not zero.
