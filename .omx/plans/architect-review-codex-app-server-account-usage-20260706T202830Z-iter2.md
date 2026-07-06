# Architect Review: Codex App-Server Account Usage Integration, Iteration 2

Generated: 2026-07-06T20:28:30Z
Verdict: ITERATE

## Summary
The revised direction is correct: `codex app-server` is the authoritative account source, local history is separate, and the analytics seam is now explicit. The plan still needs two execution-critical details before handoff.

## Required Iterations
- Add `src-tauri/src/safety.rs` to the implementation plan and explicitly remove or legacy-flag the private endpoint specs in `EndpointRegistry::default_readonly()`.
- Make `account/read` persistence/DTO handling explicit for diagnostics/export instead of optional.
- Tighten source-to-coverage mapping for account usage, account rate limits, and local history.

## Steelman Antithesis
Keeping private HTTP fallback and raw auth parsing would make a first pass simpler because it avoids child process/PATH discovery issues. That fails the correctness and safety goals because the private path already 404s and preserves secret-handling risk.

## Tradeoff Tension
App-server integration adds Windows process discovery fragility, but that is a diagnosable dependency failure. Returning wrong profile totals and scraping private endpoints is a correctness failure.

## Synthesis
Make `codex app-server` the only default account connector, explicitly remove default private endpoint safety allowlists, persist sanitized account profile metadata from `account/read`, and keep local history as a separate local-detail source.
