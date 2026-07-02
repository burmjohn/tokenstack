# Execution spec: token usage dashboard

Generated: July 2, 2026, at 18:37:10 UTC

This spec defines the source of truth for building a greenfield open source
React, Tailwind CSS, TanStack, SQLite, and Tauri app that displays Codex token
usage and reset-credit analytics.

## Metadata

| Field | Value |
| --- | --- |
| Profile | Standard |
| Rounds | 4 |
| Final ambiguity | 8% |
| Threshold | 20% |
| Context type | Greenfield |
| Context snapshot | `.omx/context/token-usage-dashboard-20260702T182859Z.md` |
| Transcript | `.omx/interviews/token-usage-dashboard-20260702T183710Z.md` |
| Design SOT | `.omx/specs/design-sot-tokenstack-command-center.md` |

## Intent

Create a local, privacy-conscious dashboard that helps a Codex user understand
token usage, reset-credit availability, reset expiration dates, and historical
usage patterns without exposing account secrets or accidentally consuming reset
credits.

## Desired outcome

The completed first version must be both:

- A working local dashboard that imports existing Codex history and refreshes
  live reset-credit data.
- A polished Tauri Windows app with installer-ready build support, background
  refresh, tested source coverage labels, and a clean modern interface.

The screenshot provided by the user is a design reference only. It establishes
the desired quality level and analytics density, not a strict layout.

The selected app design source of truth is the Command Center direction in
`.omx/specs/design-sot-tokenstack-command-center.md`. The app must support both
dark and light modes with the same layout, hierarchy, and component structure.

## In scope

- Build a greenfield open source application in
  `/home/jburmeister/projects/tokenstack`.
- Use React and TypeScript for the frontend.
- Use Tailwind CSS v4 with the Vite plugin and CSS-first theming.
- Use shadcn/ui-style component composition for the dashboard and controls.
- Use TanStack tooling, with TanStack DB or TanStack Query-style data access for
  reactive dashboard state.
- Use SQLite for local persistent snapshots, imported history, and derived
  analytics.
- Package the app as a Tauri v2 Windows-capable desktop app.
- Read local Codex session and archive JSONL files to derive token usage,
  daily usage, monthly usage, peak usage, model/context metadata, and
  rate-limit events when available.
- Read local SQLite state if it contains useful non-secret usage metadata.
- Use local Codex/ChatGPT auth state only for read-only checks.
- Prefer the read-only `/wham/rate-limit-reset-credits` endpoint for reset
  credit information.
- Discover and use undocumented Codex/ChatGPT read-only endpoints when they are
  technically available.
- Enable undocumented read-only source support by default.
- Convert reset-credit `expires_at` values to America/New_York.
- Store refresh snapshots and source coverage metadata in SQLite.
- Show data confidence and source coverage labels, especially when a stat is
  derived from local history instead of an official endpoint.
- Implement the Command Center dashboard direction from the design SOT,
  including first-class dark and light themes.
- Include background refresh with conservative read-only behavior.
- Include tests for endpoint safety, parsing, time-zone conversion, database
  persistence, and major dashboard data transforms.
- Include open source project hygiene: README, contribution guidance, license
  placeholder or selected license, security notes, and secret-handling policy.

## Out of scope / non-goals

- Do not call any endpoint containing `/consume`.
- Do not consume or redeem any reset credit.
- Do not mutate account state.
- Do not print, log, display, or commit auth tokens, account secrets, or full
  auth file contents.
- Do not store raw auth secrets in SQLite.
- Do not require a cloud backend for the first version.
- Do not treat the provided screenshot as a pixel-perfect design target.

## Decision boundaries

Codex may decide without further confirmation:

- The exact React project scaffold and file layout.
- The exact TanStack package combination if it satisfies the reactive dashboard
  and local data requirements.
- The SQLite schema for snapshots, imported usage events, source coverage, and
  derived aggregates.
- The exact visual system, as long as it is polished, clean, responsive, and
  dashboard-first.
- The exact testing framework and test split.
- The exact read-only connector structure for local files, known endpoints, and
  undocumented endpoints.
- Whether to add ADRs or design docs if they make the open source architecture
  clearer and do not expose private data.

Codex must ask before:

- Performing destructive file operations outside the project.
- Publishing, pushing, or releasing binaries.
- Choosing a legal license if no license preference is discoverable.
- Making authenticated network calls that are not demonstrably read-only.
- Using any endpoint whose behavior is uncertain or could consume/reset/redeem
  credits.

## Constraints

- Treat security and privacy as product requirements.
- Every authenticated request path must pass through a safety guard that rejects
  `/consume` and other known unsafe mutations.
- Endpoint support must be auditable in code. Keep undocumented endpoint logic
  isolated and clearly labeled.
- Do not include private user data in tests or fixtures.
- Use fixture redaction for any real-world shape captured during development.
- The app must work locally without requiring users to paste secrets into the
  UI.
- The project must be maintainable as open source code.

## Acceptance criteria

- The dashboard imports existing local Codex usage history from JSONL session
  and archive files.
- The dashboard computes daily and monthly token usage from imported history.
- The dashboard shows reset-credit availability and expiration times in
  America/New_York.
- The reset-credit connector uses the read-only
  `/wham/rate-limit-reset-credits` endpoint when available.
- The app has guarded support for undocumented read-only endpoints enabled by
  default.
- The guard rejects any endpoint path containing `/consume`.
- No token, account secret, or full auth file content appears in UI logs,
  test output, fixtures, or committed files.
- SQLite stores imported events, refresh snapshots, source coverage, and
  derived aggregates.
- The UI shows source coverage/confidence labels for each major stat.
- The UI implements the Command Center design SOT in both dark and light mode.
- The app includes background refresh with a visible last-refresh state and
  recoverable error states.
- The Tauri app can run locally in development.
- The Tauri configuration supports a Windows build and installer-ready output.
- Tests cover endpoint safety, timestamp conversion to America/New_York,
  importer parsing, SQLite persistence, and core aggregate calculations.
- The project includes README, contributor notes, security policy, and clear
  setup instructions suitable for open source use.

## Assumptions exposed and resolutions

| Assumption | Resolution |
| --- | --- |
| The screenshot was the requested design target. | It is only an example and quality reference. |
| The first version could be a smaller MVP. | Both the working dashboard and polished Tauri app are required. |
| Only documented endpoints are allowed. | Undocumented read-only endpoints are allowed and enabled by default. |
| Broad retrieval might include consuming reset credits. | Consuming or redeeming is explicitly prohibited. |

## Technical context findings

- The repository is effectively empty and greenfield.
- Local Codex archives contain JSONL `token_count` event shapes with token usage
  and rate-limit metadata.
- Local Codex state includes SQLite files that may contain useful metadata.
- The Codex auth file exists, but its contents must not be displayed or copied
  into app data.
- Context7 documentation confirmed current stack guidance for:
  - Tauri v2 capabilities, permissions, command invocation, SQL plugin setup,
    and Windows builds.
  - TanStack DB reactive collections and live queries.
  - Tailwind CSS v4 Vite plugin and CSS-first theme configuration.

## Docs and terminology ledger

- No repo-local README, `docs/`, `AGENTS.md`, `CONTEXT.md`, or package manifest
  existed during preflight.
- "TanStack SQLite" is treated as an implementation requirement to combine
  TanStack reactive frontend data patterns with a SQLite-backed local store.
- Use "source coverage" for labels that explain where a stat came from and how
  complete it is.
- Use "reset credit" for reset-credit grants. Do not call these "free tokens"
  unless a source uses that exact term.

## Optional public-safe documentation recommendations

- Add an ADR explaining the connector safety boundary.
- Add a security policy that describes secret redaction and responsible
  reporting.
- Add a data-source reference that documents local files, known endpoints, and
  undocumented read-only connectors without publishing secrets.

## Handoff recommendation

Use `$ralplan` next before implementation. The task is execution-ready from a
requirements standpoint, but the architecture and test shape are important
because this is an open source desktop app that reads local auth-adjacent state
and uses undocumented read-only endpoints.

The next planning stage must preserve:

- The no-consume safety invariant.
- The open source quality bar.
- The default-enabled undocumented read-only connector requirement.
- The Command Center design SOT, including dark and light themes.
- The Tauri Windows packaging requirement.
- The local SQLite analytics requirement.
- The source coverage/confidence label requirement.
