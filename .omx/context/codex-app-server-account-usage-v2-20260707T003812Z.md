# TokenStack Codex account data route, v2

Created: 2026-07-07T00:38:12Z

## Decision

TokenStack should use the local Codex CLI app-server as the account data boundary:

1. Spawn the installed `codex` executable with `app-server` over stdio.
2. Initialize JSON-RPC with `capabilities.experimentalApi = true`.
3. Read account state through:
   - `account/read`
   - `account/rateLimits/read`
   - `account/usage/read`
4. Normalize those account snapshots into TokenStack account tables.
5. Keep local JSONL/session import as a separate local-history source.

This is still the right route after inspecting CodexBar, codexU, codex-status-command, codex-usage-status, the official Codex app-server docs, and the upstream Codex source.

## Why this route

The Windows installed TokenStack app cannot see remote Codex sessions by reading local Windows files. The Codex profile numbers and quota windows are account-level data that Codex obtains through authenticated Codex backend APIs behind the Codex CLI/app runtime. The supported local integration surface for external tools is `codex app-server`, not raw `auth.json` token parsing and not scraped ChatGPT private endpoints.

The account usage path must therefore be remote-account aware:

- Local history import answers "what local JSONL/session files are present on this machine?"
- App-server account snapshots answer "what does the logged-in Codex account report for lifetime/daily usage, rate-limit windows, and reset credits?"

Those two concepts must not be merged into one "local resource" model.

## Example-project evidence

### CodexBar

Relevant lessons:

- CodexBar uses Codex CLI RPC for account/rate-limit reads, but its primary OAuth/private-endpoint strategy is not appropriate for TokenStack.
- App-server RPC reads are bounded with separate startup and method timeouts.
- Hung child processes are terminated so background refresh cannot stay stuck.
- Automatic background refresh does not launch bare interactive Codex TUI fallback because that can open auth flows or browser tabs.
- Refreshes are coalesced to avoid repeated app-server spawns when multiple UI callers miss cache at the same time.
- CLI/status diagnostics are isolated and manually invoked, not part of normal usage refresh.

TokenStack should adopt the process discipline and fallback rules, not the private endpoint OAuth strategy.

### codex-usage-status

Relevant lessons:

- Spawns `codex app-server` over stdio and initializes JSON-RPC with experimental API enabled.
- Calls `account/rateLimits/read` and `account/usage/read`.
- Uses a per-request timeout and disposes the child process on failure.
- Parses `rateLimitsByLimitId` first and falls back to `rateLimits`.
- Treats `codex` as the primary bucket and keeps other bucket IDs available.
- Interprets 300-minute and 10080-minute windows as common 5-hour and 7-day windows.
- Documents that it should not read repo files, store Codex credentials, or send usage data to third parties.

TokenStack should use the same RPC shape, timeout discipline, and normalization fallback.

### codexU

Relevant lessons:

- Uses `codex app-server` as the stable local path.
- Sends `initialize`, then `initialized`, then `account/read`, `account/rateLimits/read`, and `account/usage/read`.
- Treats local SQLite/JSONL usage as helpful but not authoritative for account quota.
- Avoids reading raw `~/.codex/auth.json` token values and avoids private web endpoints.
- Parses `summary.lifetimeTokens` from `account/usage/read`.
- Parses reset credits from `rateLimitResetCredits.availableCount`.
- Notes that rate-limit windows expose percent used, not absolute token quota.

TokenStack should copy the separation between account snapshots and local logs.

### codex-status-command

Relevant lessons:

- Uses `codex app-server --listen stdio://` and disables MCP startup with `-c mcp_servers={}`.
- Calls `account/read`, `account/rateLimits/read`, and optional config reads.
- Drains stderr into a bounded buffer and includes useful suffixes in timeout errors.
- Ignores wrong JSON-RPC IDs until the matching response arrives.
- Kills and waits for the child in cleanup.
- Has a PTY fallback for manual status capture, but this should not be TokenStack's automatic refresh fallback.

TokenStack should consider `--listen stdio://` plus `-c mcp_servers={}` as the primary launch shape, with a compatibility fallback to plain `codex app-server` only if the installed CLI rejects `--listen`.

## Official source evidence

Official Codex app-server documentation identifies account routes for:

- `account/read`
- `account/rateLimits/read`
- `account/usage/read`
- `account/rateLimits/updated`
- `account/rateLimitResetCredit/consume`

The upstream Codex TUI uses typed background requests for rate limits and token usage. The app-server account processor requires authenticated Codex-backend auth, obtains detailed rate-limit/reset-credit data, and returns token usage profile summaries and daily buckets.

TokenStack must never call `account/rateLimitResetCredit/consume`; that route spends a reset credit.

## Rejected routes

### Raw auth.json parsing

Rejected because it is a credential handling risk and does not provide the account usage profile by itself. TokenStack should not store, export, parse, or display raw Codex auth tokens.

### Private ChatGPT or Codex web endpoints

Rejected because TokenStack already saw 404s from private endpoints, and those routes are undocumented and brittle. CodexBar can use provider-specific OAuth/private endpoint logic because that is its core design; TokenStack's safer route is the Codex app-server boundary.

### Windows local files as the source of profile totals

Rejected because the Windows Codex app connecting to remote machines will not have the remote JSONL/session files locally. Local logs only explain local history on that machine.

### Background interactive TUI or PTY fallback

Rejected for automatic refresh because it can trigger interactive auth/browser behavior and is fragile. Manual diagnostics can be considered later, but account refresh should not use it.

### App-server daemon as a Windows dependency

Rejected for the first implementation because upstream describes the daemon as experimental and Unix-oriented. TokenStack should spawn the installed CLI process directly on demand.

## Required product behavior

- Dashboard profile totals must come from account snapshots when available.
- Local imported events must be labeled as local history, not lifetime profile usage.
- Reset credits and rate-limit windows must show unavailable/degraded when the account snapshot fails; they must not show zero as if zero is confirmed.
- Source coverage must distinguish local history, account usage, reset credits, and rate-limit windows.
- Export diagnostics must show enough sanitized process/RPC detail for a Windows user to paste logs into a debugging session.
- The app must survive missing Codex CLI, logged-out Codex, old CLI without app-server support, partial account route failures, malformed JSON-RPC output, and hung child processes.
