# Critic Review: Codex App-Server Account Usage Integration

Generated: 2026-07-06T20:28:30Z
Verdict: APPROVE

## Justification
The plan is actionable and satisfies the ralplan gate. It targets the current raw-auth/private-endpoint refresh path in `src-tauri/src/commands.rs`, `src-tauri/src/connectors.rs`, and `src-tauri/src/safety.rs`, and it addresses the current analytics dependency on local `usage_events` for top-level cards.

## Gate Assessment
- Clarity: Pass. The plan defines files, connector contract, request order, persistence boundary, diagnostics DTOs, UI labels, and legacy gating.
- Verifiability: Pass. The test spec covers fake app-server process tests, missing binary/auth, no consume calls, partial success, transaction grouping, analytics modes, schema/export tests, and full Rust/frontend verification.
- Completeness: Pass. Prior architect gaps were closed: `safety.rs` removal/gating, mandatory `account/read`, coverage mapping, one process/session, one aggregate result, one SQLite transaction.
- Principle/option consistency: Pass. App-server matches "Codex owns auth" and provenance separation. Private HTTP and local-only alternatives are fairly rejected because private endpoints already 404 and local files cannot represent remote profile totals.
- Risk/verification rigor: Pass. Main risks are named and covered: Windows PATH, protocol drift, child-process hangs, partial failures, redaction, and JavaScript integer precision.

## Non-Blocking Hardening
Add `pnpm secret:scan` and `pnpm fixture:scan` to final verification if those scripts are available in the repository.

## Final Synthesis
Proceed. Executors should implement the app-server connector as the only default account refresh path, quarantine legacy private endpoints behind the explicit flag, persist sanitized account snapshots separately from local history, and prove the change with fake app-server plus diagnostics redaction tests.
