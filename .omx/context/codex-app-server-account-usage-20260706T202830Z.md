# Context Snapshot: Codex App-Server Account Usage Integration

Generated: 2026-07-06T20:28:30Z
Workspace: /mnt/botsserver/projects/tokenstack
Context type: brownfield planning

## Task Statement
Create a detailed implementation plan to fix TokenStack account-scale Codex data by using the same authenticated Codex app-server surface that the open-source Codex app and CLI use.

## Desired Outcome
TokenStack should populate profile-scale lifetime usage, daily activity buckets, reset credits, and rate-limit windows by talking to `codex app-server` through JSON-RPC. Local Codex session scanning remains useful for local/per-thread detail, but it must not be treated as the authoritative source for account/profile totals when the user primarily runs remote Codex sessions.

## Known Facts / Evidence
- The installed Windows screenshot showed TokenStack reporting about 22.8M local lifetime tokens while the Codex profile screenshot showed about 38.5B lifetime tokens.
- The user primarily uses the Codex app to connect to remote sessions, so Windows local session files cannot contain most of the user's activity.
- A local smoke test against the user's signed-in Codex CLI app-server returned:
  - account type: `chatgpt`
  - plan: `pro`
  - lifetime tokens: `38,603,158,793`
  - daily bucket count: `136`
  - reset credits available: `4`
  - rate limit IDs: `codex`, `codex_bengalfox`
- Official Codex app-server documentation lists `account/read`, `account/usage/read`, `account/rateLimits/read`, and `account/rateLimits/updated`.
- Open-source Codex upstream maps:
  - `GetAccountTokenUsage` to `account/usage/read`
  - `GetAccountRateLimits` to `account/rateLimits/read`
  - `AccountRateLimitsUpdated` to `account/rateLimits/updated`
- Open-source Codex account processor requires ChatGPT/Codex backend auth for token usage and rate limits, constructs a backend client from Codex auth, and returns usage summary plus daily buckets.
- Generated app-server schema shows `AccountTokenUsageSummary` fields:
  - `lifetimeTokens`
  - `peakDailyTokens`
  - `longestRunningTurnSec`
  - `currentStreakDays`
  - `longestStreakDays`
- Generated app-server schema shows daily usage buckets as `{ startDate, tokens }`.
- Current TokenStack backend still reads raw Codex auth from disk and calls two private ChatGPT URLs:
  - `/wham/rate-limit-reset-credits`
  - `/backend-api/rate_limits`
- Those private endpoint calls returned 404s in the user's Windows diagnostics, while app-server calls worked locally.

## Constraints
- Do not read, log, export, or display raw `~/.codex/auth.json` contents.
- Do not call any reset-credit consume endpoint.
- Prefer documented/open-source Codex app-server JSON-RPC methods over direct private HTTP endpoints.
- Keep connector results and diagnostics redacted.
- Preserve current Tauri command boundary and local SQLite storage.
- Keep local history import available for local/per-session views and source coverage.
- Windows installed app must produce actionable diagnostics when `codex` is missing, not authenticated, incompatible, or app-server JSON-RPC fails.
- Planning mode only for this pass. Implementation must happen in a later execution lane.

## Unknowns / Open Questions
- Whether the Windows install environment has `codex` on the GUI app PATH.
- Whether `codex app-server` startup is stable enough for per-refresh spawning, or whether TokenStack should keep one short-lived process per refresh.
- Whether TokenStack should later offer a UI path picker for the Codex binary. This is out of scope for first pass unless binary discovery fails.
- Whether app-server schema changes should be tracked by vendored generated fixtures or permissive serde structs.

## Likely Codebase Touchpoints
- `src-tauri/src/commands.rs`
- `src-tauri/src/connectors.rs`
- `src-tauri/src/db.rs`
- `src-tauri/src/analytics.rs`
- `src-tauri/src/auth.rs` for deprecation/removal from connector refresh path
- `src-tauri/src/telemetry.rs`
- `src-tauri/src/lib.rs`
- `src/lib/schemas/dashboard.ts`
- `src/features/exports/diagnostics.ts`
- `src/components/command-center/CommandCenterShell.tsx`
- Tests under `src-tauri/src/*`, `src/lib/schemas/*`, and command-center/export tests

## Source References
- Official app-server documentation: https://developers.openai.com/codex/app-server
- Official auth warning: https://developers.openai.com/codex/auth
- Open-source protocol mappings: https://github.com/openai/codex/blob/main/codex-rs/app-server-protocol/src/protocol/common.rs
- Open-source account processor: https://github.com/openai/codex/blob/main/codex-rs/app-server/src/request_processors/account_processor.rs
- Open-source TUI background usage/rate-limit calls: https://github.com/openai/codex/blob/main/codex-rs/tui/src/app/background_requests.rs

## Planning Boundary
This snapshot supports a ralplan planning artifact. It is not an implementation handoff by itself.
