# Architect Review: Codex App-Server Account Usage Integration, Iteration 3

Generated: 2026-07-06T20:28:30Z
Verdict: ITERATE

## Summary
The direction and safety corrections are right. One more iteration is needed to make the handoff explicit about the app-server refresh aggregate boundary, exact diagnostics fields, and provenance-aware metric labels.

## Required Iterations
- Define one app-server refresh contract: one spawned process/session per remote refresh, one aggregate result, and one persistence transaction boundary.
- Spell out how `account/read`, `account/usage/read`, and `account/rateLimits/read` map to persisted records and connector statuses.
- Expand exact diagnostics fields for `appServer`, `accountProfile`, `accountUsage`, and `accountRateLimits`.
- Add a provenance-aware label rule so account-backed metrics do not retain local-only copy such as "Imported local history" or "Peak session".

## Steelman Antithesis
The plan may already be enough to implement well, and adding contract detail now can lock in DTO shape before fake app-server fixtures prove every field. This is outweighed by the need to prevent divergent implementations across backend, diagnostics, and UI.

## Tradeoff Tension
More explicit contracts reduce ambiguity and rework, but they make the plan larger. The compact appendix approach keeps the extra detail bounded.

## Synthesis
Keep the app-server-first architecture and add a narrow refresh/diagnostics/provenance contract appendix. That gives execution lanes a stable target without broadening scope.
