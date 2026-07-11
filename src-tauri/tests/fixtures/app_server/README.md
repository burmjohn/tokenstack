# Synthetic Codex app-server schema fixtures

These files are synthetic and contain no authentication material, account
identifiers, prompts, or response text.

- Upstream repository: `openai/codex`
- Upstream commit: `c4318c386de365bd0dd9595a08d55a30bb142d11`
- Generated schema directory: `codex-rs/app-server-protocol/schema/typescript/v2`
- Fixture generation date: `2026-07-10`
- Schema fingerprint: `codex-app-server-v2@c4318c386de365bd0dd9595a08d55a30bb142d11`

The fixtures were transcribed from the generated response and nested type
shapes, then populated only with invented values. Unknown fields intentionally
exercise forward-compatible parsing.

## Reproducing a refresh

1. Check out the exact upstream commit above.
2. Read these generated files under
   `codex-rs/app-server-protocol/schema/typescript/v2/`:
   `GetAccountRateLimitsResponse.ts`, `RateLimitSnapshot.ts`,
   `RateLimitWindow.ts`, `RateLimitResetCreditsSummary.ts`,
   `RateLimitResetCredit.ts`, `GetAccountTokenUsageResponse.ts`,
   `AccountTokenUsageSummary.ts`, and `AccountTokenUsageDailyBucket.ts`.
3. Transcribe each required property and nullable/array branch into synthetic
   JSON. Replace every identifier, label, timestamp, and count with invented
   values. Do not copy runtime responses or authentication files.
4. Compute the fingerprint as
   `codex-app-server-v2@<40-character-upstream-commit>`. The application and
   fixtures must use the identical string. A schema refresh therefore changes
   the fingerprint even when generated filenames remain stable.
5. Run the Rust golden tests and `pnpm fixture:scan` before accepting the
   refresh.

The fallback fixture represents the required legacy `rateLimits` snapshot;
the current fixture proves `rateLimitsByLimitId` precedence; the optional and
malformed fixtures lock nullable versus invalid required fields; and the
partial-method fixture proves one schema failure does not invalidate unrelated
facets.
