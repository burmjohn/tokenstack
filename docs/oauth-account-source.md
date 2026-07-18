# OAuth account source

TokenStack now follows the same core Codex OAuth path documented by CodexBar.
It reads the OAuth session created by Codex, requests the ChatGPT `wham/usage`
resource, and optionally requests reset-credit details. OAuth is the preferred
source for plan and quota-window state because it avoids scraping and reports
the server's current utilization and reset timestamps directly.

OAuth does **not** report complete token-consumption history. TokenStack keeps
the app-server `account/usage/read` method and local session importer for token
totals, daily buckets, and locally reconstructable history. Combined mode keeps
these source families separate so quota percentages are not mislabeled as token
counts and local history is not mislabeled as an account lifetime total.

## Fallback order

During a remote or combined refresh TokenStack:

1. Imports and persists available local history.
2. Requests app-server account/profile/token usage and retains any valid facets.
3. Requests OAuth plan, quota windows, and reset-credit data. Successful OAuth
   facets supersede app-server quota facets because they are persisted last.
4. Retains last-good facet snapshots when either source is unavailable or
   partially fails.

Missing OAuth credentials do not fail refresh. TokenStack records a sanitized
`not_configured` connector attempt and continues with app-server and local data.

## Credential handling

TokenStack reads `auth.json` into Rust memory only. Bearer and refresh tokens
are never included in errors, diagnostics, SQLite, frontend values, or logs.
Production endpoint hosts are constants rather than environment-controlled
URLs. When refresh-token rotation is needed, TokenStack verifies that the auth
file has not changed since it was read, writes a same-directory private
temporary file, flushes it, and atomically replaces the original.

## Compatibility note

The ChatGPT `wham` resources are not a public, versioned OpenAI API contract.
Schema validation and the app-server/local fallbacks are therefore required.
If the OAuth response changes, TokenStack fails that facet closed and continues
showing last-good or fallback evidence rather than guessing at new fields.
