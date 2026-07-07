# Ultra Code handoff: make TokenStack account usage work

Use this as the full prompt for the implementation session.

```text
You are working in /mnt/botsserver/projects/tokenstack.

Goal:
Make TokenStack work for a Windows desktop install where Codex usage mostly happens through remote Codex sessions. The app must get account-level Codex profile usage, rate-limit windows, and reset credits through the authenticated Codex CLI app-server. Local JSONL/session import must remain local history only.

Read first:
- /mnt/botsserver/projects/tokenstack/.omx/context/codex-app-server-account-usage-v2-20260707T003812Z.md
- /mnt/botsserver/projects/tokenstack/.omx/plans/research-codex-app-server-examples-20260707T003812Z.md
- /mnt/botsserver/projects/tokenstack/.omx/plans/route-plan-codex-app-server-account-usage-v2-20260707T003812Z.md
- /mnt/botsserver/projects/tokenstack/.omx/plans/test-spec-codex-app-server-account-usage-v2-20260707T003812Z.md
- /mnt/botsserver/projects/tokenstack/AGENTS.md

External examples already inspected:
- CodexBar
- codexU
- codex-status-command
- codex-usage-status
- upstream openai/codex app-server docs and source

Core route:
1. Spawn installed Codex CLI with app-server over stdio.
2. Initialize JSON-RPC with experimental API enabled.
3. Send initialized notification.
4. Call account/read, account/rateLimits/read, and account/usage/read.
5. Persist normalized account snapshots separately from local usage_events.
6. Update dashboard/setup/export diagnostics so failures are actionable and not misleading.

Launch requirements:
- Prefer direct process spawn:
  codex app-server --listen stdio:// -c mcp_servers={}
- Fall back to:
  codex app-server
  only when the installed CLI rejects the listen/config args.
- Do not invoke through a shell.
- Support explicit configured Codex path and TOKENSTACK_CODEX_BIN.
- Include selected executable and launch mode in diagnostics.
- Kill and wait for the child on timeout/error/drop.

Timeout and refresh requirements:
- Separate initialize, per-request, and whole-refresh timeouts.
- Coalesce concurrent account refreshes so multiple UI callers do not spawn multiple app-server children.
- Use cooldown for repeated missing CLI, logged-out, unsupported CLI, or timeout failures.
- Keep last-good account snapshot and mark it stale/degraded when current refresh fails.

Security requirements:
- Do not parse, store, export, or display raw auth.json tokens.
- Do not use private ChatGPT/Codex web endpoints.
- Do not launch interactive Codex TUI/PTTY flows during automatic refresh.
- Do not call account/rateLimitResetCredit/consume.
- Do not export prompt bodies, cookies, tokens, or raw JSONL conversation content.

Normalization requirements:
- Prefer rateLimitsByLimitId; fallback to rateLimits.
- Show codex bucket first and preserve all extra bucket IDs.
- Treat 300-minute and 10080-minute windows as common 5-hour and 7-day windows, but preserve unknown windows.
- Derive remaining percent from usedPercent; do not invent absolute token quotas.
- Parse reset credits from rateLimitResetCredits.availableCount.
- Parse lifetime and daily buckets from account/usage/read.
- Never let local imported JSONL totals overwrite account lifetime totals.

UI requirements:
- Replace misleading zeros with unavailable/degraded states when account snapshots are missing.
- Label local history separately from account usage.
- Show selected Codex executable, first failing stage, and last successful account refresh in setup diagnostics.
- Make Export diagnostics actually write a sanitized file and return/show the path.

Testing requirements:
- Start with fake app-server fixtures and protocol tests.
- Cover happy path, partial method failure, logged out, missing CLI, hung app-server, old CLI fallback, notifications/out-of-order messages, and diagnostics export.
- Include tests that prove no consume-reset-credit call exists.
- Include redaction/secret tests for exported diagnostics.

Verification to run:
- cargo test --manifest-path src-tauri/Cargo.toml
- cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
- pnpm test
- pnpm build
- pnpm secret:scan
- pnpm fixture:scan

If any command is missing, record the exact output and run the closest available substitute.

Delivery:
- Keep diffs scoped.
- Do not stage unrelated dirty files.
- Document changed files and verification.
- Commit in Lore format.
- Push the branch.
- Merge only if the repository branch/PR workflow is clear and checks are safe.

Acceptance:
TokenStack no longer treats Windows local files as the source of remote Codex profile data. Account totals, rate limits, and reset credits come from the app-server snapshot path. Local-only mode still works. All connector failures are diagnosable from the exported logs.
```
