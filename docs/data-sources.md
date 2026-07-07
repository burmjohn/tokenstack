# Data Sources

TokenStack supports two source families:

- Local Codex history JSONL.
- Codex account snapshots read through `codex app-server` over stdio.

Local history is local-only evidence. It can explain imported sessions on the
current machine, but it must not be labeled as Codex account lifetime totals. In
combined mode, TokenStack can show local history as a labeled fallback when no
account snapshot is available.

Account snapshots provide Codex profile usage, daily buckets, rate-limit
windows, and reset-credit availability. TokenStack launches the installed Codex
CLI directly, initializes the app-server JSON-RPC session with the experimental
API enabled, and calls `account/read`, `account/rateLimits/read`, and
`account/usage/read`.

On Windows, TokenStack resolves the Codex executable from an explicit configured
path when supplied, then `TOKENSTACK_CODEX_BIN`, then `PATH`, then common desktop
and npm-global install locations. Setup diagnostics show the selected
executable, launch mode, first failing account stage, and last successful
account refresh.

TokenStack must not parse raw Codex auth tokens, call private ChatGPT/Codex web
endpoints, launch automatic interactive TUI/PTTY fallbacks, or call the reset
credit consume route.

Fixtures must be synthetic. Real auth files, private user histories, prompt
bodies, cookies, tokens, and raw JSONL conversation content must not be
committed or exported.

Unknown local history shapes produce warnings and lower source coverage instead of failing the whole import.
