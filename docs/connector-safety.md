# Account Connector Safety

TokenStack treats the Codex CLI app-server as the authenticated account data
boundary. The app does not construct authenticated HTTP requests for Codex
account data.

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
