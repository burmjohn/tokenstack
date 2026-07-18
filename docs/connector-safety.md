# Account Connector Safety

TokenStack uses two authenticated account-data boundaries: the Codex OAuth
session already created by the installed Codex client, and the Codex CLI
app-server. OAuth is preferred for authoritative plan, quota-window, reset, and
credit data. The app-server remains the source for account token totals and the
fallback for account data that OAuth does not expose.

The OAuth connector:

- Reads OAuth tokens only from `CODEX_HOME/auth.json` or the default
  `~/.codex/auth.json`.
- Sends bearer credentials only to the fixed ChatGPT usage and reset-credit
  HTTPS endpoints, and sends refresh credentials only to the fixed OpenAI OAuth
  token endpoint. Runtime configuration cannot override these production hosts.
- Refreshes stale or rejected credentials using the public Codex OAuth client
  identifier, then atomically replaces `auth.json` while preserving unrelated
  fields and refusing to overwrite a concurrently changed file.
- Never stores tokens, raw authenticated responses, or auth-file contents in
  TokenStack SQLite, diagnostics, logs, or frontend state.
- Treats malformed values, out-of-range percentages, invalid timestamps, and
  negative credit counts as errors instead of displaying them.

The account connector:

- Spawns the selected `codex` executable directly, without a shell.
- Prefers `codex app-server --listen stdio:// -c mcp_servers={}`.
- Falls back to plain `codex app-server` only when the installed CLI rejects
  the preferred app-server arguments.
- Calls only `account/read`, `account/rateLimits/read`, and
  `account/usage/read`.
- Never calls the reset-credit consume route.
- Kills and waits for the child process on timeout, protocol error, or drop.
- Stores account snapshots separately from local `usage_events`.

Diagnostics exports are sanitized and exclude auth tokens, cookies, prompt
bodies, raw JSONL conversation content, and raw app-server output.
