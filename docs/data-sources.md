# Data Sources

TokenStack supports three source families:

- Local Codex history JSONL.
- Codex OAuth account snapshots from the existing Codex login.
- Codex account snapshots read through `codex app-server` over stdio.

Local history is local-only evidence. It can explain imported sessions on the
current machine, but it must not be labeled as Codex account lifetime totals. In
combined mode, TokenStack can show local history as a labeled fallback when no
account snapshot is available.

OAuth snapshots provide the account plan, authoritative session/weekly quota
utilization, reset times, and reset-credit availability. This endpoint does not
provide lifetime or daily token totals. TokenStack therefore still launches the
installed Codex CLI, initializes the app-server JSON-RPC session with the
experimental API enabled, and calls `account/read`, `account/rateLimits/read`,
and `account/usage/read`. Local history remains a separately labeled token-count
source when account token totals are unavailable.

On Windows, TokenStack resolves the Codex executable from an explicit configured
path when supplied, then `TOKENSTACK_CODEX_BIN`, then `PATH`, then common desktop
and npm-global install locations. Setup diagnostics show the selected
executable, launch mode, first failing account stage, and last successful
account refresh.

The Windows release smoke uses the packaged TokenStack executable and a
synthetic native app-server runtime. This proves process launch, protocol reads,
runtime-setting persistence, child cleanup, and diagnostics export without
using an authenticated account. A manual installed-Windows release check still
verifies Codex App, standalone CLI, and npm CLI discovery and account behavior.

TokenStack must not expose or persist raw Codex auth tokens, accept configurable
authenticated endpoint hosts, launch automatic interactive TUI/PTTY fallbacks,
or call the reset-credit consume route.

Fixtures must be synthetic. Real auth files, private user histories, prompt
bodies, cookies, tokens, and raw JSONL conversation content must not be
committed or exported.

Unknown local history shapes produce warnings and lower source coverage instead of failing the whole import.
