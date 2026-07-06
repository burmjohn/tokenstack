# Architect Review: Codex App-Server Account Usage Integration, Iteration 1

Generated: 2026-07-06T20:28:30Z
Verdict: ITERATE

## Summary
The direction is architecturally correct: move TokenStack off raw `auth.json` parsing and brittle private ChatGPT endpoints toward `codex app-server`. The draft is not yet execution-ready because it must explicitly sever the legacy private-endpoint path from the default connector graph and specify how account snapshot data feeds the analytics layer.

## Required Iterations
- Remove or hard-disable `KnownResetCreditsConnector` and `UndocumentedRateLimitsConnector` from default refresh orchestration.
- Remove the default allowlist entries in `safety.rs` or isolate them behind an explicit legacy flag.
- Retire `load_auth_handle` from the default account refresh path.
- Add a concrete account-snapshot analytics seam:
  - insert/load helpers for account usage snapshots and daily buckets.
  - update `build_dashboard_summary` so remote/combined modes read account snapshots.
  - keep local `usage_events` for local/per-session views.
- Split process/app-server diagnostics from legacy connector diagnostics.

## Steelman Antithesis
Keeping private endpoint fallback could reduce short-term implementation risk because it avoids process spawning, PATH discovery, and schema changes. This fails the task's correctness and safety goals because it preserves raw auth parsing, 404-prone URLs, and the wrong source of truth.

## Tradeoff Tension
App-server integration adds a child-process dependency and Windows binary discovery risk. That is a better risk than continuing with a path that already fails and duplicates Codex auth handling.

## Synthesis
Use `codex app-server` as the only authoritative account connector path. Keep local file scanning strictly for local/per-thread history. Preserve legacy endpoint code only as disabled test/forensics code if needed, not as a normal fallback.
