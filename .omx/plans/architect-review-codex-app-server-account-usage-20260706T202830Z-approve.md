# Architect Review: Codex App-Server Account Usage Integration, Approval

Generated: 2026-07-06T20:28:30Z
Verdict: APPROVE
Architectural status: CLEAR

## Summary
The latest revision closes the prior architectural gaps. The plan has an explicit one-process/one-transaction app-server refresh contract, concrete diagnostics DTOs, default-path removal of raw auth/private endpoints, and provenance-aware label rules.

## Strongest Antithesis
Retaining private HTTP connectors and raw-auth parsing would make the first implementation simpler because it avoids child-process management, PATH discovery, and new aggregate persistence. This is the wrong trade because it preserves already-observed 404-prone private endpoints and the secret-handling risk the task is meant to remove.

## Tradeoff Tension
Spawning `codex app-server` adds Windows binary-discovery and child-process failure modes. Those failures are diagnosable and redacted. Continuing with private endpoints keeps the code simpler while silently preserving the wrong source of truth and secret exposure risk.

## Required Changes Before Execution
None architecturally required. The plan is specific enough to hand to execution. One wording polish was recommended: make `account/read` persistence mandatory rather than optional.

## Principle Violations
None material remain. The plan preserves provenance separation, avoids raw auth in the default path, keeps local history separate from account totals, and redacts diagnostics.

## Synthesis
Proceed with `codex app-server` as the authoritative account connector, persist sanitized snapshots in a single refresh group, keep local import for local-detail views only, and quarantine any legacy private endpoint code behind the explicit forensics flag.
