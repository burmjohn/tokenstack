# Deep Interview Context Snapshot: Token Usage Dashboard

Generated: 2026-07-02T18:28:59Z
Workspace: /home/jburmeister/projects/tokenstack
Context type: greenfield

## Task Statement
Build a polished web app and Windows desktop app for checking Codex token usage, usage stats, and reset credit dates.

## Desired Outcome
A beautiful, clean dashboard that helps the user understand token usage over
time and see available reset credits, including when resets were given and when
they expire. The first usable version must include both a working local
dashboard and a polished packaged Tauri app with installer-ready Windows build
support, background refresh, and tested data-source coverage labels.

## Stated Solution
- React web app.
- Tailwind CSS for UI.
- TanStack tooling.
- SQLite-backed local data.
- Tauri Windows app packaging.
- Read-only use of local Codex/ChatGPT auth state.
- Prefer the read-only `/wham/rate-limit-reset-credits` endpoint.
- Avoid any endpoint containing `/consume`.

## Probable Intent Hypothesis
The user wants a local, privacy-conscious desktop dashboard that consolidates Codex/ChatGPT usage information and reset-credit visibility without exposing secrets or accidentally spending reset credits.

## Known Facts / Evidence
- The workspace currently has no application scaffold, README, package manifest, or source files; it contains only `.omx` state/log files.
- Codebase memory index was created for this project as `home-jburmeister-projects-tokenstack`; architecture shows only files/modules generated from existing `.omx` state.
- Visual reference image shows a dark, centered analytics dashboard with:
  - profile header/avatar
  - top metric strip
  - token activity heatmap with daily/weekly/cumulative tabs
  - activity insights
  - most-used plugins list
- Design SOT selection:
  - Selected direction: Command Center, option 1.
  - Dark reference: `.omx/specs/assets/tokenstack-command-center-dark-sot.png`.
  - Light reference: `.omx/specs/assets/tokenstack-command-center-light-sot.png`.
  - Written design SOT:
    `.omx/specs/design-sot-tokenstack-command-center.md`.
- Context7 docs checked:
  - Tauri v2: app permissions/capabilities, frontend `invoke`, SQL plugin install via `npm run tauri add sql`, Windows build via `tauri build`.
  - TanStack DB: reactive collections, `useLiveQuery`, optimistic mutations, collection persistence handlers.
  - Tailwind CSS v4: Vite plugin `@tailwindcss/vite`, CSS-first `@import "tailwindcss"` and `@theme` configuration.

## Constraints
- Must not print tokens, secrets, or full auth file contents.
- Must only perform read-only usage checks.
- Must avoid `/consume` endpoints.
- Must convert reset expiration timestamps to America/New_York.
- Must report evidence source for reset-credit reads.
- Must use Context7 for current package docs.
- Must produce a Windows-capable Tauri app.
- Must implement the selected Command Center design SOT with both light and
  dark modes.
- Must preserve local privacy and avoid external production side effects unless explicitly approved.
- Must be suitable for an open source project with very high code quality,
  maintainability, tests, documentation, and no committed private data.
- May probe undocumented Codex/ChatGPT read-only endpoints when discovered, but
  must never call endpoints containing `/consume` or perform any action that
  consumes or redeems reset credits.
- Undocumented read-only endpoint support must be enabled by default in the
  app, isolated in a clearly labeled connector, and guarded so unsafe endpoints
  cannot run.

## Unknowns / Open Questions
- Which exact data sources beyond reset credits are technically recoverable for
  token usage and stats.
- Whether SQLite should store snapshots/history only, or be the primary local
  analytics database.
- Whether the app should support only the current user/machine or multiple
  profiles.
- Which open source license and contribution rules should govern the repo.

## Decision-Boundary Unknowns
- Whether OMX/Codex may choose exact TanStack packages and database integration details.
- Whether OMX/Codex may inspect local auth metadata and log files during implementation, with secret redaction.
- Live API calls are allowed during app runtime if they are read-only and never
  consume or redeem reset credits.
- The screenshot is an example, not a strict design target.
- Whether optional durable docs/ADRs should be created.

## Likely Codebase Touchpoints
- New `package.json`, Vite/React/Tailwind configuration, and TypeScript source.
- New `src-tauri/` Rust project configuration.
- Tauri command layer for safe read-only auth/API interaction.
- Local SQLite schema for usage snapshots/reset-credit history.
- React dashboard views, charts/heatmap components, and data-access hooks.
- Tests for endpoint safety and timestamp conversion.

## Repo Docs / Rules / Context Inspected
- Top-level AGENTS instructions from the user prompt.
- `/home/jburmeister/.codex/skills/deep-interview/SKILL.md`.
- Workspace file scan under `/home/jburmeister/projects/tokenstack`.
- `.omx/state/deep-interview-state.json`.
- No repo-local `AGENTS.md`, `README`, `docs/`, `CONTEXT.md`, or package manifest found.

## Terminology / Conflicts
- "TanStack SQLite" is ambiguous. Possible meanings include TanStack DB with a local SQLite-backed adapter/command layer, TanStack Query over Tauri SQL commands, or another TanStack package plus SQLite persistence.
- "Token usage" is broader than the provided reset-credit endpoint. Reset credits are known, but lifetime tokens, peak tokens, streaks, plugin usage, and daily activity require separate data sources or derived local history.

## Prompt-Safe Initial Context Summary Status
not_needed
