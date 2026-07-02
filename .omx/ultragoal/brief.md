Complete TokenStack from the PRD, implementation plan, and test spec below. Do not mark complete until every PRD acceptance criterion, test-spec ledger item, documentation requirement, screenshot requirement, packaging smoke, and safety invariant has fresh evidence.

# PRD
# PRD: TokenStack Command Center

Generated: 2026-07-02T18:53:34Z
Workflow: `$ralplan` deliberate consensus planning
Repository target: https://github.com/burmjohn/tokenstack
Local workspace: `/home/jburmeister/projects/tokenstack`

## Binding Sources Of Truth

- `.omx/specs/deep-interview-token-usage-dashboard.md`
- `.omx/specs/design-sot-tokenstack-command-center.md`
- `.omx/specs/assets/tokenstack-command-center-dark-sot.png`
- `.omx/specs/assets/tokenstack-command-center-light-sot.png`
- GitHub repository: `burmjohn/tokenstack`, public, empty at planning time.

## Product Intent

TokenStack is a local, privacy-conscious Windows-capable desktop dashboard for Codex users. It imports local Codex usage history, refreshes read-only reset-credit data, computes daily and monthly token analytics, explains source coverage, and displays reset-credit expiration dates in `America/New_York`.

The first screen is the actual dashboard. No landing page, marketing hero, or empty onboarding shell is allowed.

## Non-Negotiable Safety Invariants

- Never call any endpoint whose path contains `/consume`.
- Never consume, redeem, claim, mutate, or spend reset credits.
- Never print, store, log, display, fixture, or commit auth tokens, account secrets, or full auth file contents.
- All auth-adjacent local reads are read-only, path-guarded, redacted, and performed in the Tauri/Rust safety boundary.
- Undocumented read-only endpoints are allowed and enabled by default, but only through audited connector code, endpoint registry entries, response schemas, and safety checks.
- Auth tokens never cross the Rust-to-frontend IPC boundary.
- Local SQLite never stores raw auth secrets or full auth files.

## Target Users

- Primary: local Codex users who need trustworthy token usage and reset-credit visibility without leaking secrets or triggering account mutations.
- Secondary: open source contributors who need clear architecture, tests, safety policy, and source documentation before adding connectors or analytics.

## Goals

1. Import local Codex history from session/archive JSONL and any non-secret local metadata stores.
2. Refresh reset-credit availability and expiration data through read-only remote connectors.
3. Compute token analytics: lifetime tokens, today, this month, peak session, daily heatmap, monthly trends, recent sessions, and rate-limit windows where source data supports them.
4. Display source coverage/confidence for every major metric and connector.
5. Convert reset-credit expiration timestamps to `America/New_York` with DST-safe tests.
6. Ship a Tauri v2 desktop app that can run locally and build for Windows installer output.
7. Implement the Command Center SOT in complete dark and light themes.
8. Preserve open source quality: docs, tests, CI, security policy, contributor guide, and frequent Lore-protocol commits.
9. Finish with a beautiful README that includes real dark/light screenshots captured from the finished app.

## Non-Goals

- Cloud backend or account sync.
- Browser extension.
- Pixel-perfect recreation of the SOT screenshots.
- Raw auth viewer, auth export, token debugger, or token copy feature.
- Mutating Codex/ChatGPT account state.
- Calling any unknown endpoint without a read-only safety classification.
- Selecting a final legal license if the SOT/user direction changes. Current plan assumes MIT because the design SOT footer says MIT License.

## Product Requirements

### Dashboard Shell

- Persistent left sidebar with TokenStack identity, navigation, data mode, refresh cadence, version, and GitHub affordance.
- Header with title, concise subtitle, last refresh status, user/profile affordance, read-only status, `Never /consume` safety badge, and data mode selector.
- Bottom footer with `All data is read-only`, `We never consume credits or tokens`, open source status, MIT license indicator, and repository link.
- Dark and light themes must share layout, spacing, hierarchy, and component structure.

### Required Visible Concepts

The first screen must visibly communicate:

- `TokenStack`
- `Dashboard`
- `Read-only`
- `Never /consume`
- `Local + Remote`
- `Daily token usage`
- `Reset credit timeline`
- `Source coverage`
- `Active connectors`
- `Undocumented (RO)`
- `America/New_York`
- `All data is read-only`

Exact copy may be shortened, but the concepts must remain visible.

### Analytics Modules

- Metric strip:
  - Lifetime tokens
  - Today
  - This month
  - Peak session
  - Reset credits
- Daily token usage heatmap with daily, weekly, and monthly controls.
- Reset credit timeline with credit counts, expiration dates, and days remaining.
- Source coverage module with per-source completeness labels.
- Active connectors module with local history, known read-only endpoint, and undocumented read-only endpoint states.
- Recent sessions table with start time, duration, total tokens, peak tokens, mode/model labels, and source labels.
- Rate-limit windows table when source data is available.
- Next reset-credit expiration panel in `America/New_York`.

### Data Modes

- Local: only imported local history and local non-secret metadata.
- Remote: only read-only remote connector snapshots.
- Combined: merged local and remote analytics with source coverage labels.

### Refresh Behavior

- Manual refresh button.
- Conservative auto-refresh cadence, default 60 seconds or slower for remote connectors.
- Background refresh while app is open.
- Visible last refresh, in-progress, stale, degraded, and failed states.
- Backoff after connector errors.
- Refresh must never bypass endpoint safety checks.

### Source Coverage

Every major stat must expose:

- Source kind: local history, local metadata, known read-only endpoint, undocumented read-only endpoint, derived aggregate.
- Completeness percent or qualitative status.
- Last evidence timestamp.
- Confidence level.
- Explanation text on hover or inspector.

Coverage percentages must be formula-backed, versioned, and conservative. A metric may show 100% only when all required source facets for that metric are available, parseable, fresh enough for the selected data mode, and schema-valid. Unknown or drifting source shapes lower coverage instead of being guessed.

### Open Source Requirements

- `README.md` with purpose, safety guarantees, setup, development, screenshots, data sources, privacy policy summary, and build instructions.
- Real screenshots captured after UI completion: dark dashboard and light dashboard.
- `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md` or documented decision not to include one.
- `docs/architecture.md`, `docs/data-sources.md`, `docs/connector-safety.md`, `docs/testing.md`.
- ADRs for connector safety, SQLite schema, and UI/theme architecture.
- License ADR. The planned license is MIT because the design SOT footer names MIT License; execution should record that decision explicitly and allow user override before release.
- CI for lint, typecheck, unit tests, Rust tests, security/static checks, and Windows build smoke.
- Frequent commits at each coherent milestone using the Lore Commit Protocol from the workspace instructions.

## Package Choices

### Core App

- Tauri v2: required desktop shell, Windows packaging, Rust safety boundary, capabilities.
- Vite: official frontend builder path for Tauri/React and Tailwind v4 plugin.
- React 19.x + TypeScript: current React docs and shadcn guidance target React 19-era behavior; use one `createRoot` and `StrictMode`.
- pnpm: deterministic lockfile and efficient open source dependency workflow.

### Styling And UI

- Tailwind CSS v4 with `@tailwindcss/vite`: current CSS-first theme token model and Vite setup.
- shadcn/ui-style generated local components: copied source components, not black-box component package.
- Radix UI primitives through shadcn components: accessible keyboard/focus/ARIA behavior.
- lucide-react: icon set matching shadcn conventions.
- class-variance-authority, clsx, tailwind-merge: standard shadcn composition utilities.
- Recharts: shadcn chart pattern compatibility for coverage/line/bar charts; custom SVG/HTML heatmap for daily token grid if Recharts is too heavy for that shape.

### Data And State

- SQLite: local persistent snapshots, imports, derived aggregates, source coverage.
- tauri-plugin-sql with SQLite feature: Tauri-supported SQLite integration and migrations, capability-gated.
- TanStack Query v5: frontend server-state/cache layer over typed Tauri commands, background refetch, stale/error states.
- TanStack Router: type-safe local navigation for Dashboard, Usage, Reset Credits, Sources, Settings.
- TanStack Table: headless tables for recent sessions and rate-limit windows.
- Zod: boundary validation for IPC payloads, connector responses, local parser outputs, and settings.
- date-fns + date-fns-tz or equivalent tested timezone helper: frontend display of `America/New_York` dates; Rust side stores canonical UTC.

### Rust Backend

- serde/serde_json: structured parsing and IPC payloads.
- time or chrono + chrono-tz: UTC storage and `America/New_York` conversion tests.
- reqwest: Rust-owned authenticated read-only HTTP connector so auth tokens never enter frontend code.
- secrecy + zeroize: auth material held only in short-lived redacted secret types.
- thiserror/anyhow: internal errors with redacted public error conversion.
- tracing: structured logs with redaction layer; no token/full-auth logging.
- sqlx only if needed beyond `tauri-plugin-sql`; otherwise keep database access on the Tauri SQL path.

### Testing And Quality

- Vitest: Vite-native TypeScript unit tests.
- React Testing Library: component tests with user-centric queries.
- Vitest Browser Mode: targeted focus/keyboard/theme/component behavior where JSDOM is insufficient.
- Playwright: end-to-end browser preview and screenshot capture; Tauri-specific smoke when practical.
- cargo test: Rust safety guard, parser, connector, timezone, and database tests.
- ESLint + TypeScript strict mode: static correctness.
- Prettier or Biome: formatting; choose one and document it.
- gitleaks or equivalent secret scan in CI.

## Acceptance Criteria

1. The app imports local Codex JSONL history without reading or storing auth secrets.
2. Daily and monthly usage aggregates are computed from imported local history and stored or cached with deterministic recalculation.
3. Reset-credit availability and expiration data are refreshed through read-only connector code.
4. The known reset-credit connector supports `/wham/rate-limit-reset-credits`.
5. Undocumented read-only endpoint support is enabled by default, isolated in an audited connector module, and user-visible as `Undocumented (RO)`.
6. Endpoint guard rejects any endpoint path containing `/consume` before any network call can occur.
7. No connector supports POST, PUT, PATCH, DELETE, request bodies, or mutation-like endpoint specs for authenticated remote calls.
8. Auth tokens never appear in frontend state, SQLite, logs, screenshots, fixtures, or test output.
9. Reset-credit expiration dates display in `America/New_York` with timezone label.
10. Source coverage labels are present for lifetime tokens, today, this month, peak session, reset credits, heatmap, sessions, and rate-limit windows.
11. The dashboard implements the Command Center SOT in complete dark and light modes.
12. The dashboard is keyboard usable and screen-reader labeled for controls, tables, badges, charts, and safety status.
13. Manual refresh and auto-refresh expose last-refresh, pending, stale, degraded, and error states.
14. SQLite migrations create the full local schema from an empty app data directory.
15. Tests cover endpoint safety, local parser behavior, auth redaction, SQLite persistence, aggregate calculations, timezone conversion, connector response validation, and core UI rendering.
16. CI runs lint, typecheck, frontend tests, Rust tests, secret scan, and Windows build smoke.
17. Documentation includes README, screenshots, contributing guide, security policy, data-source guide, connector-safety guide, and architecture ADRs.
18. The final README includes real dark and light screenshots captured after implementation.

## Success Metrics

- Safety: zero network calls can be made to paths containing `/consume`; all safety tests pass.
- Privacy: secret scan passes; redaction tests prove auth material is not emitted.
- Reliability: repeat local imports are idempotent with no duplicate usage events.
- Accuracy: source coverage labels explain missing/partial data instead of inventing certainty.
- UX: first viewport matches Command Center structure in dark and light themes with no overlapping text at 1280x800 and 1440x900.
- Packaging: Windows build job produces installer-ready Tauri output or documents the exact missing signing/release requirement.
  - Target: `x86_64-pc-windows-msvc`.
  - Installer: NSIS output from Tauri unless current Tauri defaults change during implementation.
  - Code signing and public release are separate approval-gated steps.

## Release Gate

Do not publish binaries or push releases without explicit user approval. Local commits and PR preparation are in scope for execution, but release distribution is a separate approval gate.

## Ultragoal Completion Contract

This PRD is intended to hand off to `$ultragoal` for full completion. Ultragoal must not mark the goal complete until all acceptance criteria are either passed with evidence or explicitly marked out of scope by the user in a later instruction.

Completion requires:

- Working Tauri desktop app.
- Local history import.
- Read-only reset-credit refresh.
- Undocumented read-only connector support enabled by default.
- SQLite persistence and migrations.
- Daily/monthly analytics.
- Source coverage labels.
- `America/New_York` expiration display.
- Complete Command Center dark and light themes.
- Safety guard proof for `/consume` and mutation rejection.
- Secret/auth redaction proof.
- Tests and CI gates.
- Windows packaging smoke.
- Open source docs.
- Beautiful README with real screenshots.
- Coherent Lore-protocol commits.

# Implementation Plan
# Implementation Plan: TokenStack Command Center

Generated: 2026-07-02T18:53:34Z
Workflow: `$ralplan` deliberate consensus planning
Status: Consensus approved by Architect and Critic

## Scope

Plan a greenfield, open source, production-quality Tauri Windows desktop app with React, TypeScript, Tailwind v4, shadcn/ui-style components, TanStack data patterns, and SQLite persistence. The app imports local Codex usage history, refreshes live reset-credit data, shows daily/monthly token analytics, source coverage, reset-credit expiration dates in `America/New_York`, and supports Command Center dark/light themes.

This is a planning artifact only. Do not implement source code during `$ralplan`.

## Evidence Used

### Local Source Of Truth

- `.omx/specs/deep-interview-token-usage-dashboard.md`
- `.omx/specs/design-sot-tokenstack-command-center.md`
- `.omx/specs/assets/tokenstack-command-center-dark-sot.png`
- `.omx/specs/assets/tokenstack-command-center-light-sot.png`
- `.omx/context/token-usage-dashboard-20260702T182859Z.md`
- GitHub repository metadata: `burmjohn/tokenstack` is public and empty at planning time.

### Current Docs And Research

- Context7: Tauri v2 docs for Windows build, SQL plugin, HTTP plugin, capabilities/permissions.
- Context7: Tailwind CSS v4 docs for `@tailwindcss/vite`, CSS-first `@import "tailwindcss"`, and `@theme`.
- Context7: TanStack Query v5 docs for `QueryClientProvider`, `useQuery`, query keys, error states, and invalidation/refetch patterns.
- Researcher official-doc pass:
  - React `createRoot`, `StrictMode`, and TypeScript guidance for React 19.2 docs.
  - shadcn/ui Vite, Tailwind v4, theming, `components.json`, package imports.
  - Radix Primitives accessibility and composition.
  - Tauri SQL plugin, JS reference, and capabilities.
  - TanStack Query v5 QueryClient, queries, invalidation, errors.
  - Vitest, React Testing Library, Zod.

## RALPLAN-DR Summary

### Principles

1. Safety is an invariant, not a UI promise: endpoint and auth guards must make unsafe behavior unrepresentable.
2. Privacy by construction: auth material stays in Rust memory, is redacted in errors/logs, and is never persisted or returned to the frontend.
3. Evidence over certainty: every metric carries source coverage and confidence instead of guessing from incomplete data.
4. Local-first open source quality: SQLite persistence, deterministic imports, reproducible tests, clear docs, and frequent reviewable commits.
5. Command Center fidelity: first viewport is a dense, useful desktop dashboard in complete dark and light themes.

### Decision Drivers

1. Hard safety constraints around `/consume`, reset-credit mutation, and auth-adjacent local state.
2. Data-source uncertainty: local history and undocumented endpoints may drift or be incomplete.
3. Production-quality open source delivery: tests, docs, CI, packaging, contributor experience, and screenshot-backed README.

### Viable Options

#### Option A: Rust-owned safety/data core with React/TanStack dashboard

Approach: Tauri/Rust owns auth-adjacent reads, remote connector safety, local imports, SQLite writes, and redacted logs; React uses TanStack Query over typed commands and shadcn UI for the dashboard.

Pros:

- Strongest boundary for secrets and endpoint safety.
- Clear place to test `/consume` rejection before network calls.
- Frontend never sees auth tokens.
- SQLite schema and import logic can be tested with `cargo test`.
- Fits Tauri production packaging.

Cons:

- More Rust surface area.
- Requires careful IPC schema design.
- Frontend data mocks need adapter layer for tests.

Decision: Chosen.

#### Option B: Frontend-first Tauri SQL and HTTP plugins

Approach: React/TanStack directly calls Tauri plugin APIs for SQL, filesystem, and HTTP.

Pros:

- Faster initial scaffold.
- Simpler frontend iteration.
- Less custom Rust service code.

Cons:

- Higher risk of auth material crossing into frontend code.
- Harder to centrally prove endpoint safety.
- Easier for future contributors to bypass guards with raw HTTP/SQL calls.
- Weaker match for hard privacy invariants.

Decision: Rejected for authenticated and auth-adjacent paths. Limited Tauri SQL use may remain, capability-gated, but components must not own raw connector/auth behavior.

#### Option C: Web-only app with later Tauri wrapper

Approach: Build a browser app first and add Tauri packaging later.

Pros:

- Fast UI iteration.
- Easier browser testing.

Cons:

- Violates Windows/Tauri desktop requirement as a first-class deliverable.
- Auth/local filesystem access would be designed too late.
- Risks rework in safety and persistence layers.

Decision: Rejected.

#### Option D: Electron desktop app

Approach: Build Electron/React app with Node file and network access.

Pros:

- Familiar JS-only stack.
- Many desktop examples.

Cons:

- Does not satisfy Tauri requirement.
- Larger runtime and weaker Rust safety boundary.

Decision: Rejected.

### Pre-Mortem

1. Secret leak through logs, fixture, IPC, or SQLite.
   - Mitigation: Rust-only auth handles, redacted error types, no auth data in IPC schemas, fixture/secret scans in CI, synthetic tests only.
2. Undocumented endpoint changes or has side effects.
   - Mitigation: audited endpoint registry, GET-only/no-body guard, schema validation, last-good snapshots, degraded source coverage, kill switch setting, no unregistered endpoint execution.
3. UI looks polished in one theme but breaks in light mode or real desktop window sizes.
   - Mitigation: shared CSS tokens, SOT checklist, Playwright screenshots for dark/light, text-overflow checks, fixed component dimensions, accessibility checks.

## Architecture

### High-Level Structure

```text
src/
  app/
    main.tsx
    providers.tsx
    router.tsx
  components/
    ui/                  # shadcn-generated local components
    command-center/      # dashboard-specific composed modules
    charts/
  features/
    dashboard/
    usage/
    reset-credits/
    sources/
    settings/
  lib/
    api/                 # typed Tauri command wrappers
    query/               # query keys and hooks
    schemas/             # Zod IPC/data schemas
    format/              # tokens, dates, timezone display
    theme/
  test/
    fixtures/
src-tauri/
  src/
    main.rs
    commands/
    safety/
    auth/
    connectors/
    importers/
    db/
    analytics/
    telemetry/
  migrations/
  capabilities/
docs/
  architecture.md
  data-sources.md
  connector-safety.md
  testing.md
  adr/
```

### Runtime Boundaries

- Frontend owns presentation, data-mode selection, refresh controls, and cached async state.
- Rust owns all auth-adjacent reads, remote connector requests, local history scanning, SQLite writes, and redaction.
- SQLite stores local application data only: imported usage events, connector snapshots, derived aggregates, coverage metadata, and redacted connector run status.
- Tauri IPC exposes typed commands that return sanitized domain DTOs.
- No component may call authenticated HTTP directly.
- No frontend module may parse or hold auth tokens.

## Connector Boundary Design

### Connector Types

```text
Connector
  id
  display_name
  source_kind
  safety_class
  enabled_by_default
  run(context) -> ConnectorRunResult
```

- `LocalCodexHistoryConnector`
  - Reads session/archive JSONL and non-secret local metadata.
  - No auth token needed.
  - Produces usage events, sessions, rate-limit metadata when present.
- `KnownResetCreditsConnector`
  - Uses registered read-only endpoint `/wham/rate-limit-reset-credits`.
  - Auth material remains in Rust memory.
  - Produces reset-credit batches and connector snapshot metadata.
- `UndocumentedReadonlyConnector`
  - Enabled by default.
  - Only uses endpoint registry entries reviewed as read-only.
  - Each endpoint has a response schema and test fixture.
  - Any uncertainty downgrades source coverage or disables that endpoint, not the whole app.

### Endpoint Registry

Each remote endpoint must be represented as data, not ad hoc string construction:

```text
EndpointSpec
  id
  method: GET | HEAD
  host_policy
  path
  query_schema
  body_allowed: false
  documented_status: documented | undocumented
  readonly_review: required
  response_schema
  redaction_policy
  reviewed_at
```

### Safety Guard

All authenticated remote requests pass through `SafetyGuard::validate`.

Rules:

- Reject normalized path containing `/consume`.
- Reject method other than GET or HEAD.
- Reject request body.
- Reject unregistered endpoint.
- Reject missing response schema.
- Reject unsafe host.
- Reject connector code that attempts raw URL execution.
- Emit redacted audit event on allow or deny.
- Deny by default.

The guard is tested independently with a mock HTTP server proving denied requests never reach the server.

### Auth Handling

- `AuthLocator` finds only known local auth-adjacent paths.
- Reads are read-only and minimal.
- `AuthParser` extracts only fields required for in-memory authenticated request construction.
- Auth values are held in `SecretString`/zeroizable types.
- Public DTOs expose only `available`, `connector_status`, and redacted account label if safe.
- Full auth file contents are never logged, persisted, printed, displayed, committed, or sent to frontend.

## Data Model

SQLite lives in the app data directory, never inside the source repo. Store canonical timestamps as UTC instants. Store display timezone separately when useful, but derive `America/New_York` display from UTC.

### Tables

#### `app_meta`

- `key`
- `value`
- `updated_at_utc`

#### `import_runs`

- `id`
- `source_kind`
- `started_at_utc`
- `completed_at_utc`
- `status`
- `files_seen`
- `events_seen`
- `events_imported`
- `warnings_json`

#### `source_documents`

- `id`
- `source_kind`
- `path_hash`
- `safe_label`
- `first_seen_at_utc`
- `last_seen_at_utc`
- `content_hash`
- `last_offset`
- `redaction_level`

#### `usage_events`

- `id`
- `event_uid`
- `source_document_id`
- `session_uid`
- `occurred_at_utc`
- `model`
- `mode`
- `input_tokens`
- `output_tokens`
- `cache_read_tokens`
- `cache_write_tokens`
- `total_tokens`
- `raw_event_kind`
- `confidence`
- `metadata_json_redacted`

#### `sessions`

- `id`
- `session_uid`
- `started_at_utc`
- `ended_at_utc`
- `duration_seconds`
- `total_tokens`
- `peak_tokens`
- `model_mix_json`
- `mode_labels_json`
- `source_summary_json`

#### `connector_runs`

- `id`
- `connector_id`
- `started_at_utc`
- `completed_at_utc`
- `status`
- `endpoint_id`
- `http_status`
- `redacted_error_code`
- `redacted_error_message`

#### `reset_credit_batches`

- `id`
- `connector_run_id`
- `captured_at_utc`
- `credit_count`
- `expires_at_utc`
- `source_connector_id`
- `confidence`
- `raw_batch_hash`

#### `rate_limit_windows`

- `id`
- `connector_run_id`
- `captured_at_utc`
- `window_key`
- `limit_tokens`
- `used_tokens`
- `remaining_tokens`
- `resets_at_utc`
- `confidence`

#### `refresh_snapshots`

- `id`
- `trigger`
- `started_at_utc`
- `completed_at_utc`
- `status`
- `connector_summary_json`
- `dashboard_summary_json`

#### `source_coverage`

- `id`
- `snapshot_id`
- `metric_key`
- `source_kind`
- `coverage_percent`
- `confidence`
- `last_evidence_at_utc`
- `formula_version`
- `required_facets_json`
- `missing_facets_json`
- `explanation`

### Derived Views

- `daily_usage_ny`
- `monthly_usage_ny`
- `session_summary`
- `reset_credit_timeline`
- `dashboard_summary`
- `coverage_summary`

Views can be SQL views or Rust/TypeScript selectors; choose based on testability and performance.

## Source Coverage Scoring Contract

Coverage is a product contract, not decorative telemetry. Each metric must have a formula-backed coverage record.

### Coverage Formula Shape

```text
MetricCoverage
  metric_key
  formula_version
  required_facets[]
  observed_facets[]
  missing_facets[]
  coverage_percent
  confidence: high | medium | low | unavailable
  explanation
```

### Conservative Rules

- Coverage starts at 0 and only increases for verified source facets.
- Unknown event shapes never count as full coverage.
- Stale remote snapshots lower freshness coverage.
- Derived metrics inherit the weakest required source facet unless the formula explicitly weights sources.
- A metric may show 100% only when all required facets are present, schema-valid, parseable, and fresh for the selected data mode.
- Coverage explanations must name missing facets in user-safe language.

### Initial Metric Facets

- Lifetime/today/month tokens:
  - required: local usage events, parseable token fields, dedupe key, selected date range.
  - optional: remote usage endpoint if later available.
- Peak session:
  - required: session grouping, token totals, timestamps.
- Daily heatmap:
  - required: timestamp UTC, `America/New_York` date bucket, token totals.
- Reset credits:
  - required: reset-credit connector success, schema-valid credit count, schema-valid expiration timestamp.
- Rate-limit windows:
  - required: rate-limit source event or endpoint, window key, limit/used/remaining, reset timestamp.
- Active connectors:
  - required: connector run status and last attempted timestamp.

### Tests Required Before UI Trusts Coverage

- Missing local history lowers local metric coverage.
- Unknown JSONL shapes lower local coverage.
- Remote connector failure keeps last-good reset-credit data but lowers freshness/confidence.
- Undocumented source data never appears as high confidence unless endpoint schema and recency pass.
- Formula version is stored and surfaced in developer/debug docs.

## UI And Component Structure

### App Shell

- `CommandCenterShell`
- `SidebarNav`
- `DashboardHeader`
- `SafetyControlGroup`
- `SafetyFooter`

### Dashboard Modules

- `MetricStrip`
- `MetricCard`
- `TokenUsageHeatmap`
- `ResetCreditTimeline`
- `SourceCoveragePanel`
- `ActiveConnectorsPanel`
- `RecentSessionsTable`
- `RateLimitWindowsTable`
- `NextResetExpirationPanel`

### shadcn Components To Generate Early

- Button
- Badge
- Card
- Sidebar
- Separator
- Tooltip
- HoverCard
- Tabs
- ToggleGroup
- Table
- Progress
- ScrollArea
- Avatar
- DropdownMenu
- Select
- Switch
- Skeleton
- Alert

### Theme Tokens

Use Tailwind v4 CSS-first tokens and CSS variables:

- Dark base: graphite black.
- Dark foreground: warm off-white.
- Dark selected/usage: muted blue.
- Dark read-only/positive: mint green.
- Dark warning/expiration: amber.
- Light base: warm white.
- Light cards: white/near-white.
- Light foreground: ink.
- Light borders: cool gray.
- Light selected/usage: cobalt blue.
- Shared radius: 8px or less.

Avoid decorative gradients, orbs, nested cards, fake placeholder blocks, and one-note purple/blue palettes.

## TanStack Data Patterns

### Query Families

- `dashboard.summary(dataMode)`
- `usage.daily(range, dataMode)`
- `usage.monthly(range, dataMode)`
- `sessions.recent(filters)`
- `resetCredits.timeline(dataMode)`
- `rateLimits.windows(dataMode)`
- `sources.coverage(dataMode)`
- `connectors.status()`
- `refresh.status()`

### Query Rules

- One app-level `QueryClientProvider`.
- Query functions call typed Tauri command wrappers.
- Use `staleTime` to prevent noisy refetching.
- Use `refetchInterval` only for refresh/status queries.
- Manual refresh calls a typed `refresh_all` command, then invalidates dashboard query families.
- Use TanStack mutations only for local app state writes such as settings or manual refresh commands, never for authenticated account mutation.
- Render loading, stale, degraded, and error states explicitly.

## Background Refresh Design

### Orchestrator

`RefreshOrchestrator` runs:

1. Acquire refresh lock.
2. Import local history.
3. Run known read-only remote connector if auth is available.
4. Run undocumented read-only connector registry if enabled.
5. Persist connector runs and snapshots.
6. Recompute source coverage.
7. Recompute or invalidate derived analytics.
8. Emit redacted refresh status.
9. Release lock.

### Cadence

- Manual refresh: user initiated, still safety-guarded.
- Auto refresh: default 60 seconds or slower for remote connectors.
- Backoff: exponential per connector after failures.
- Local import may run more often than remote calls if cheap.
- No refresh runs while another refresh is active unless explicitly coalesced.

### Failure Model

- Connector failure does not delete last good snapshot.
- Failed remote connector downgrades source coverage.
- Local import failure keeps existing persisted analytics available.
- Error display uses redacted codes/messages only.

## Staged Implementation Plan

### Stage 0: Repository Bootstrap And Planning Commit

- Reconcile local workspace with `burmjohn/tokenstack`.
  - If workspace remains non-git, initialize or clone the empty GitHub repo into the workspace during execution.
  - Add `origin` as `https://github.com/burmjohn/tokenstack.git`.
- Preserve `.omx` planning artifacts.
- Commit planning artifacts first if desired, using Lore protocol.
- Document commit cadence in `docs/development.md`.
- Add `docs/adr/0000-license.md` recording MIT as the planned license because the SOT footer names MIT License, with a note that public release can still be user-overridden.

Commit intent: establish a reviewed implementation contract before code.

### Stage 1: Scaffold And Tooling

- Create Tauri v2 + Vite + React + TypeScript project.
- Configure pnpm, Node version, Rust toolchain notes.
- Add Tailwind v4 with `@tailwindcss/vite`.
- Add shadcn setup with `components.json`.
- Add strict TypeScript, ESLint, formatter, Vitest, cargo fmt/clippy.
- Add base CI skeleton.

Commit intent: create reproducible app foundation.

### Stage 2: Safety Guard First

- Implement endpoint registry, request type, and safety guard.
- Implement redacted error/log types.
- Add Rust tests for `/consume`, methods, bodies, unregistered endpoints, and redaction.
- Add docs ADR for connector safety.

Commit intent: make unsafe connector behavior impossible before adding connectors.

### Stage 3: SQLite Schema And Persistence

- Add Tauri SQL/SQLite configuration and migrations.
- Implement database repositories or typed command layer.
- Add migration/idempotency tests.
- Add synthetic fixture policy and secret scan.

Commit intent: establish local persistence without secrets.

### Stage 4: Local History Importer

- Implement local Codex history locator and JSONL parser using synthetic fixtures.
- Import usage events, sessions, rate-limit metadata when available.
- Add idempotency and partial-file tests.
- Add source coverage output for local sources.

Commit intent: turn local history into trustworthy analytics inputs.

### Stage 5: Remote Read-Only Connectors

- Implement auth locator/parser with in-memory secret handling.
- Implement known reset-credit connector for `/wham/rate-limit-reset-credits`.
- Implement undocumented read-only connector registry enabled by default.
- Add mock HTTP tests proving denied requests never leave process.
- Persist reset-credit snapshots and connector runs.

Commit intent: refresh remote data without exposing or mutating auth/account state.

### Stage 6: Analytics Layer

- Implement daily/monthly aggregates, peak session, reset timeline, source coverage, and dashboard summary selectors.
- Add timezone conversion to `America/New_York`.
- Add DST and zero-data tests.

Commit intent: produce tested dashboard facts from persisted evidence.

### Stage 7: Frontend Data Layer

- Add typed Tauri command wrappers, Zod schemas, query key factory, and TanStack Query hooks.
- Add data mode filtering.
- Add refresh state and invalidation flow.
- Add mock adapter for UI tests.

Commit intent: connect UI to safe typed data without exposing unsafe primitives.

### Stage 8: Command Center UI Dark Theme

- Build shell, sidebar, header safety controls, metric strip, heatmap, reset timeline, coverage, connectors, tables, and footer.
- Use shadcn/Radix components and lucide icons.
- Match dark SOT density and hierarchy.
- Add component tests.

Commit intent: deliver the primary dashboard experience.

### Stage 9: Light Theme, Accessibility, And Polish

- Complete light theme tokens with same structure.
- Add keyboard/focus/tooltip/a11y tests.
- Run screenshot checks at desktop viewports.
- Fix text overflow and layout density issues.

Commit intent: make the Command Center complete and accessible in both themes.

### Stage 10: Packaging And CI

- Configure Tauri capabilities.
- Configure Windows build job targeting `x86_64-pc-windows-msvc`.
- Prefer Tauri NSIS installer output for initial Windows artifact.
- Distinguish three packaging states in docs and CI:
  - dev smoke: app launches locally.
  - installer smoke: Windows installer artifact is produced.
  - release-ready: signing/notarization or release distribution prerequisites are satisfied.
- Add Tauri dev/build smoke.
- Document unsigned installer/signing status. Signing/public release remains approval-gated.

Commit intent: make the app installer-ready for Windows development builds.

### Stage 11: Documentation, Screenshots, And Open Source Finish

- Write README with real dark/light screenshots, safety guarantees, setup, usage, data sources, and build instructions.
- Add `CONTRIBUTING.md`, `SECURITY.md`, architecture/data-source/connector docs, and ADRs.
- Add screenshot capture script or documented manual capture.
- Run final secret scan and test suite.

Commit intent: make the project legible, safe, and contributor-ready.

## Verification Strategy

- Safety before connectors: guard tests must pass before any remote connector is merged.
- Data before UI: importer/schema/analytics tests must pass before UI claims real metrics.
- UI after data mocks: component tests use synthetic typed data.
- Visual verification after UI: capture dark/light screenshots and inspect against SOT checklist.
- Packaging after app behavior: Windows build smoke after core tests pass.
- Documentation after screenshots: README screenshot section waits until real app screenshots exist.

## Risk Register

### Auth-Adjacent Local State

Risk: auth tokens or full auth files leak through logs, IPC, SQLite, fixtures, or screenshots.

Mitigations:

- Rust-only auth handles.
- `secrecy`/zeroize for in-memory secrets.
- Redacted errors and tracing layer.
- No auth DTO fields in frontend schemas.
- Secret scan in CI.
- Synthetic fixtures only.
- Tests for serialization/logging/persistence redaction.

Residual risk: platform-specific auth file formats may change. Treat parser failures as unavailable auth with degraded remote coverage.

### Undocumented Endpoints

Risk: endpoint behavior changes, returns unexpected data, or proves not read-only.

Mitigations:

- Endpoint registry with GET/HEAD only and no body.
- `/consume` path deny rule.
- Response schema per endpoint.
- Mock HTTP safety tests.
- Last-good snapshot retention.
- Source coverage degradation on failure.
- User-visible connector status and optional kill switch.

Residual risk: undocumented semantics cannot be fully guaranteed. Plan labels these sources as `Undocumented (RO)` and preserves local-only value when unavailable.

### Windows/Tauri Packaging

Risk: app builds on Linux dev but fails on Windows, or installer requires signing/release configuration.

Mitigations:

- Windows CI job.
- Tauri v2 Windows build docs followed.
- Document prerequisites.
- Treat code signing/release publishing as separate approval gate.
- Keep native dependencies minimal.

Residual risk: signing and SmartScreen reputation require distribution decisions outside initial implementation.

### Source Coverage Accuracy

Risk: imported local history is incomplete, duplicate, malformed, or changes format.

Mitigations:

- Idempotent import with source hashes/offsets.
- Parser version tests.
- Unknown event warnings.
- Coverage labels for each metric.
- No guessed precision when source is partial.
- Synthetic fixture matrix for known shapes.

Residual risk: Codex local history format can drift. App should degrade gracefully and invite issue reports with redacted fixture guidance.

### Timezone And Expiration Accuracy

Risk: reset expiration display is wrong across DST or local machine timezone differences.

Mitigations:

- Store UTC instants.
- Convert explicitly to `America/New_York`.
- DST boundary tests.
- UI labels timezone everywhere reset expiration is shown.

Residual risk: source endpoint semantics might provide date-only or ambiguous timestamps. Schema should classify ambiguity and lower confidence.

## ADR

### Decision

Build TokenStack as a Tauri v2 app with a Rust-owned safety/data core, React 19 + TypeScript frontend, Tailwind v4 CSS-first theme tokens, shadcn/Radix local components, TanStack Query/Router/Table data patterns, and SQLite persistence through Tauri SQL/SQLite.

### Drivers

- Prevent unsafe endpoint calls and reset-credit mutation.
- Keep auth material out of frontend and persisted state.
- Support local-first analytics with source coverage.
- Match the Command Center SOT in both themes.
- Preserve open source maintainability and testability.

### Alternatives Considered

- Frontend-first Tauri plugin calls: rejected for auth/safety risk.
- Web-only first: rejected because Tauri/Windows is first-version scope.
- Electron: rejected because Tauri is required and better supports Rust safety boundary.
- TanStack DB as primary persistence layer: considered but deferred unless it materially improves reactive local collections; TanStack Query + SQLite is simpler and sufficient for first version.

### Why Chosen

The chosen architecture makes safety and privacy enforceable in Rust before any UI or connector code can bypass them, while keeping the React dashboard ergonomic and testable.

### Consequences

- More upfront Rust and IPC design.
- Stronger tests are required before visible data claims.
- Frontend contributors must use typed adapters instead of raw authenticated fetches.
- SQLite schema and migration discipline become part of the public contract.

### Follow-Ups

- Confirm exact pinned package versions during implementation using current docs.
- Confirm MIT license assumption before public release if the user changes the design SOT.
- Add ADRs for connector safety, SQLite schema, and theme architecture.
- Capture real README screenshots only after the dashboard is implemented.

## Available Agent Types Roster

- `explore`: repo/file discovery and implementation mapping.
- `researcher`: official/upstream docs and version-aware guidance.
- `dependency-expert`: dependency comparison or upgrade/replacement decisions.
- `planner`: sequencing and risk flags.
- `architect`: architecture review.
- `critic`: plan/design critique.
- `executor`: implementation/refactoring.
- `test-engineer`: test strategy and coverage.
- `designer`: UX/UI architecture.
- `verifier`: completion evidence and test adequacy.
- `code-reviewer`: comprehensive review before merge.
- `git-master`: commit strategy and history hygiene.
- `writer`: documentation and README polish.

## Follow-Up Staffing Guidance

### Recommended Default: `$ultragoal` + `$team`

Use `$ultragoal` as durable goal ledger and `$team` for parallel implementation lanes. Team returns checkpoint evidence; Ultragoal records durable completion.

Suggested lanes:

- Safety/backend lane: `executor`, high attention to Rust guard, auth, connector, and DB boundaries.
- Importer/analytics lane: `executor` or `test-engineer`, focused on JSONL import, SQLite schema, source coverage, timezone tests.
- Frontend UI lane: `executor` plus `designer`, focused on Command Center shell and themes.
- QA/verification lane: `test-engineer` and `verifier`, focused on tests, screenshots, CI, packaging.
- Docs lane: `writer`, focused on README, docs, ADRs, contributor/security policy.

Suggested reasoning levels:

- Safety/backend: high.
- Importer/analytics: high.
- Frontend UI: medium-high.
- QA/verification: high.
- Docs: medium-high.

## Ultragoal Durable Completion Structure

The execution handoff should be framed as one durable goal:

```text
$ultragoal complete TokenStack from .omx/plans/implementation-plan-tokenstack-command-center-20260702T185334Z.md and .omx/plans/test-spec-tokenstack-command-center-20260702T185334Z.md. Do not mark complete until every PRD acceptance criterion, test-spec ledger item, documentation requirement, screenshot requirement, packaging smoke, and safety invariant has fresh evidence.
```

### Ultragoal Milestones

1. Repository and tooling foundation
   - GitHub remote connected.
   - Tauri/React/Tailwind/shadcn/TanStack/SQLite scaffold complete.
   - Initial CI and quality tools present.
   - Lore commit made.
   - Evidence file: `.omx/ultragoal/evidence/01-repo-tooling.md`.
2. Safety invariant foundation
   - Endpoint registry and guard implemented.
   - `/consume` rejection proved.
   - Auth redaction proved.
   - No remote connector can bypass guard.
   - Lore commit made.
   - Evidence file: `.omx/ultragoal/evidence/02-safety-guard.md`.
3. Persistence and local import
   - SQLite migrations complete.
   - Local history importer complete.
   - Import idempotency proved.
   - Source coverage emitted for local data.
   - Lore commit made.
   - Evidence file: `.omx/ultragoal/evidence/03-data-import-analytics.md`.
4. Remote read-only connectors
   - Known reset-credit connector complete.
   - Undocumented read-only connector registry enabled by default.
   - Mock HTTP safety tests pass.
   - Last-good snapshot/degraded failure behavior implemented.
   - Lore commit made.
   - Evidence file: `.omx/ultragoal/evidence/04-readonly-connectors.md`.
5. Analytics and timezone
   - Daily/monthly/lifetime/peak/session/reset timeline calculations complete.
   - `America/New_York` conversion and DST tests pass.
   - Coverage labels attached to metrics.
   - Lore commit made.
   - Evidence file: `.omx/ultragoal/evidence/05-analytics-timezone.md`.
6. Frontend data layer
   - Typed IPC wrappers and Zod schemas complete.
   - TanStack Query hooks and data-mode filtering complete.
   - Refresh state/invalidation complete.
   - Lore commit made.
   - Evidence file: `.omx/ultragoal/evidence/06-frontend-data-layer.md`.
7. Command Center UI
   - Dark theme dashboard complete.
   - Light theme dashboard complete.
   - Required first-screen concepts visible.
   - Accessibility and layout tests pass.
   - Lore commit made.
   - Evidence file: `.omx/ultragoal/evidence/07-command-center-ui.md`.
8. Packaging and CI
   - Tauri dev smoke complete.
   - Windows build smoke complete or exact blocker documented.
   - CI runs lint/typecheck/frontend tests/Rust tests/secret scan/build smoke.
   - Lore commit made.
   - Evidence file: `.omx/ultragoal/evidence/08-windows-packaging.md`.
9. Open source finish
   - README complete with real dark/light screenshots.
   - `CONTRIBUTING.md`, `SECURITY.md`, docs, and ADRs complete.
   - Screenshot artifacts committed.
   - Final verification pass recorded.
   - Lore commit made.
   - Evidence file: `.omx/ultragoal/evidence/09-open-source-docs.md`.

10. Final review and git history
   - Final verifier/code-reviewer pass complete.
   - Commit history reviewed for Lore protocol.
   - No blocking TODOs remain in the PRD/test-spec acceptance matrix.
   - Evidence file: `.omx/ultragoal/evidence/10-git-history.md`.

### First Vertical Slice

Ultragoal should make the first implementation slice narrow but end-to-end:

- Repository/tooling initialized.
- Safety guard with `/consume` rejection tests.
- Empty SQLite migration.
- One synthetic local-history fixture imported into SQLite.
- One read-only reset-credit connector behind mock HTTP.
- One dashboard summary command returning sanitized synthetic data.
- One minimal Command Center dashboard screen rendering that data.
- Evidence files `01-repo-tooling.md` and `02-safety-guard.md` started immediately, not deferred.

This slice proves the architecture under real app wiring before adding analytics breadth, undocumented endpoint coverage, or full UI polish.

### Ultragoal Definition Of Done

Ultragoal must keep the goal active until all of these are true:

- Every hard invariant is enforced by code and tests.
- Every PRD acceptance criterion has implementation evidence.
- Every test-spec category has passing or explicitly explained evidence.
- No known secret/auth leakage exists in logs, fixtures, SQLite, screenshots, or commits.
- The app can be run locally as a Tauri desktop app.
- The dashboard is the first screen and matches the Command Center SOT in dark and light mode.
- The app imports local history and refreshes reset-credit data through read-only connectors.
- Source coverage is visible and accurate enough to explain partial data.
- Windows packaging has been smoke-tested or blocked by a documented external prerequisite.
- README includes real screenshots, not design SOT mockups.
- Documentation is complete enough for open source contributors.
- Git history uses coherent Lore-protocol commits.
- A final verifier or code-reviewer pass finds no blocking issues.

### `$team` Launch Hint

```text
$team implement TokenStack using .omx/plans/implementation-plan-tokenstack-command-center-20260702T185334Z.md with lanes for safety/backend, importer/analytics, frontend Command Center UI, QA/verification, and docs. Preserve the hard invariants: never call /consume, never mutate reset credits, never expose auth secrets.
```

### `omx team` Launch Hint

```text
omx team "Implement TokenStack from .omx/plans/implementation-plan-tokenstack-command-center-20260702T185334Z.md with parallel lanes: safety/backend, importer/analytics, frontend UI/theme, QA/verification, docs. Return checkpoint evidence for Ultragoal."
```

### Team Verification Path

Before Team shuts down, it must prove:

- Guard tests reject `/consume` and non-read-only remote calls.
- Secret scan passes.
- Local importer and SQLite tests pass.
- Analytics/timezone tests pass.
- Dashboard renders dark and light SOT structure.
- Windows/Tauri build smoke is documented or passing.
- README contains real screenshots after UI completion.
- Git history has coherent Lore-protocol commits.

## Goal-Mode Follow-Up Suggestions

- `$ultragoal`: default durable follow-up for this implementation plan. Use it to track sequential completion and checkpoint evidence.
- `$team`: recommended alongside Ultragoal because work splits naturally into safety/backend, data, UI, QA, and docs lanes.
- `$performance-goal`: not primary; use only later if startup time, import speed, or query latency becomes a measured optimization project.
- `$autoresearch-goal`: not primary; docs research is already sufficient for planning, and this is an implementation project.
- `$ralph`: fallback only if the user explicitly wants a persistent single-owner implementation/verification loop instead of the default durable goal ledger.

## Ralplan Consensus Gate

Consensus gate status: complete.

- Architect review: APPROVE.
  - Strongest antithesis: a frontend-first/plugin-heavy Tauri design could ship faster with less Rust overhead.
  - Synthesis: keep Rust-owned safety/data core, but make the first implementation slice narrow and end-to-end.
  - Improvements applied: source coverage scoring contract, Windows packaging target/signing distinction, explicit MIT license ADR, concrete Ultragoal evidence files, first vertical slice.
- Critic review: APPROVE.
  - Confirmed clarity, verifiability, completeness, principle/option consistency, fair alternatives, risk/verification rigor, deliberate-mode pre-mortem/test planning, and Ultragoal handoff completeness.
  - Found no blocking issues.

Durable handoff record:

- `.omx/plans/ralplan-handoff-tokenstack-command-center-20260702T185334Z.json`

## Commit And Documentation Cadence

- Commit after every coherent stage or safety milestone.
- Use Lore Commit Protocol for every commit.
- Keep commits small enough to review: scaffold, safety guard, schema, importer, connectors, analytics, frontend shell, themes, packaging, docs.
- Update docs in the same commit as the behavior they explain when possible.
- Add or update ADRs when architecture boundaries change.
- Keep README screenshot work until real app screens exist; do not use the SOT mockups as final README screenshots.

## Planner Changelog

- Created deliberate-mode RALPLAN summary.
- Selected Rust-owned safety/data core over frontend-first connectors.
- Added package choices and rationale.
- Added data model, connector boundary, background refresh design, UI structure, risk register, verification strategy, staffing guidance, and commit/docs cadence.
- Applied Architect improvements: source-coverage scoring contract, Windows target/signing distinction, explicit MIT license ADR, concrete Ultragoal evidence files, and first vertical slice.
- Recorded Architect APPROVE and Critic APPROVE consensus gate for Ultragoal handoff.

# Test Spec
# Test Spec: TokenStack Command Center

Generated: 2026-07-02T18:53:34Z
Workflow: `$ralplan` deliberate consensus planning

## Test Strategy

Test from the safety boundary outward:

1. Prove unsafe network/auth behavior is impossible.
2. Prove local import, parsing, and persistence are deterministic and secret-safe.
3. Prove analytics and timezone transforms are correct.
4. Prove frontend data states, source coverage, and Command Center UI render correctly.
5. Prove Windows/Tauri packaging remains buildable.

No test may use real auth tokens, real full auth files, or private user history. All fixtures must be synthetic and redacted.

## Unit Tests

### Rust Safety Guard

- `rejects_any_path_containing_consume`
  - Given endpoint paths such as `/consume`, `/v1/consume`, `/wham/consume/reset`, and URL-encoded variants when normalized.
  - Expect validation fails before request construction.
- `rejects_non_readonly_methods`
  - Given POST, PUT, PATCH, DELETE, OPTIONS for authenticated connectors.
  - Expect validation fails.
- `rejects_request_body_for_authenticated_connectors`
  - Given a GET with a request body or mutation payload.
  - Expect validation fails.
- `allows_registered_get_reset_credit_endpoint`
  - Given registered `/wham/rate-limit-reset-credits` endpoint with method GET and no body.
  - Expect validation succeeds.
- `rejects_unregistered_undocumented_endpoint`
  - Given an arbitrary undocumented endpoint not in the audited registry.
  - Expect validation fails.
- `redacts_auth_values_in_errors`
  - Given an internal error containing token-like material.
  - Expect public error output removes/obscures secrets.
- `auth_handle_never_serializes_secret`
  - Given an auth handle or connector state.
  - Expect serde/IPC payload contains only availability/status metadata.

### Rust Auth-Adjacent Local Reads

- `auth_locator_reads_only_allowed_paths`
  - Given known auth-adjacent path candidates and unrelated paths.
  - Expect only allowlisted paths can be opened.
- `auth_parser_extracts_minimum_required_fields`
  - Given synthetic auth JSON shape.
  - Expect only opaque in-memory auth material and redacted account metadata are produced.
- `auth_file_contents_not_persisted`
  - Given an auth read and DB snapshot.
  - Expect no raw auth JSON or token-like values in persisted rows.

### Local History Import

- `imports_jsonl_token_count_events`
  - Given synthetic Codex JSONL with token count events.
  - Expect usage events and session aggregates.
- `skips_unknown_jsonl_shapes_with_warning`
  - Given unknown event shapes.
  - Expect warning and lower source coverage, not failed import.
- `deduplicates_reimported_events`
  - Given same file imported twice.
  - Expect stable event count and import run metadata.
- `tracks_source_document_offsets_or_hashes`
  - Given appended JSONL.
  - Expect only new events are imported or duplicate-safe reprocessing occurs.
- `never_fixtures_private_history`
  - Static fixture scan verifies no real-looking tokens, account IDs, or full auth documents.

### SQLite Persistence

- `migrations_create_schema_from_empty_db`
- `migrations_are_idempotent`
- `usage_events_roundtrip`
- `reset_credit_batches_roundtrip`
- `connector_runs_roundtrip_redacted_errors`
- `source_coverage_roundtrip`
- `derived_daily_usage_query_matches_raw_events`
- `foreign_keys_and_unique_constraints_prevent_duplicates`

### Analytics

- `computes_lifetime_tokens`
- `computes_today_in_america_new_york`
- `computes_month_to_date_in_america_new_york`
- `computes_peak_session`
- `computes_daily_heatmap_buckets`
- `computes_monthly_rollups`
- `computes_source_coverage_percentages`
- `coverage_formula_never_overstates_missing_sources`
- `coverage_formula_records_formula_version`
- `coverage_confidence_downgrades_on_unknown_source_shape`
- `coverage_explanation_names_required_missing_facets`
- `marks_derived_stats_partial_when_sources_missing`
- `handles_zero_data_without_nan_or_crash`

### Timezone Conversion

- `formats_reset_expiration_in_america_new_york`
- `handles_dst_spring_forward`
- `handles_dst_fall_back`
- `stores_canonical_utc_and_displays_ny`
- `countdown_uses_timezone_safe_instant_math`

### Connector Response Validation

- `known_reset_credit_schema_accepts_expected_shape`
- `known_reset_credit_schema_rejects_missing_expiration`
- `undocumented_connector_schema_is_explicit_per_endpoint`
- `connector_failure_does_not_clear_last_good_snapshot`
- `connector_failure_sets_degraded_source_coverage`

### TypeScript Data Layer

- Query key factory returns stable keys for dashboard, usage, reset credits, connectors, and source coverage.
- Query functions call typed IPC/database adapters, not raw fetch for authenticated data.
- Zod schemas reject malformed IPC payloads.
- Data mode selector filters local, remote, and combined results.
- Refresh invalidates only the relevant query families.

## Component Tests

Use Vitest + React Testing Library for most components. Use Browser Mode for keyboard/focus behavior where needed.

### App Shell

- Renders TokenStack identity, Dashboard nav, data mode, auto refresh, version, and GitHub affordance.
- Theme toggle changes dark/light root class or data attribute and persists preference.
- Sidebar navigation has accessible names and selected state.

### Header And Safety Controls

- Shows last refresh, refresh button, `Read-only`, `Never /consume`, and data mode.
- Refresh pending state disables duplicate manual refresh.
- Error state shows redacted message and does not expose endpoint tokens or auth values.

### Metric Strip

- Renders lifetime tokens, today, this month, peak session, and reset credits.
- Shows source coverage/tooltip trigger for each metric.
- Handles loading, empty, stale, degraded, and error states.

### Token Heatmap

- Renders daily token usage with month/day labels and intensity legend.
- Daily, weekly, and monthly controls are keyboard reachable.
- Empty data renders a quiet, non-marketing empty state with source coverage explanation.

### Reset Credit Timeline

- Renders credit counts, expiration dates, days remaining, and `America/New_York` label.
- Sorts expirations by instant.
- Handles no credits and expired credits.

### Source Coverage

- Renders total coverage score and local history, rate limits, reset credits, and undocumented rows.
- Hover/inspector explains source evidence and incompleteness.

### Active Connectors

- Renders local history, known read-only endpoint, and undocumented read-only endpoint rows.
- Shows read-only status for each connector.
- Does not display auth token, secret, or full local file content.

### Tables

- Recent sessions table renders start time, duration, tokens, peak tokens, mode, and source labels.
- Rate-limit windows table renders window, limit, used, remaining, resets in, and overall progress.
- Tables have accessible headers and stable layout at desktop widths.

### Footer

- Renders `All data is read-only`, never-consume language, open source, license, and GitHub repository link.

## Integration Tests

### Import Pipeline

- Synthetic local history directory with multiple JSONL files imports into SQLite.
- Re-running import is idempotent.
- Partial corrupt files produce warnings and source coverage degradation.
- Derived dashboard summary matches raw imported events.

### Refresh Orchestrator

- Manual refresh runs local import, known read-only connector, undocumented read-only connector, persistence, and query invalidation in order.
- Remote connector failure keeps local analytics available.
- Background refresh obeys minimum cadence and backoff.
- Concurrent refresh requests coalesce or lock correctly.

### Connector Safety

- Mock HTTP server records requests.
- Allowed endpoint request reaches server only after guard approval.
- `/consume` request attempt never reaches server.
- Non-GET methods never reach server.
- Undocumented endpoint must be registered before use.

### Database And Query Layer

- App opens against an empty app-data SQLite database.
- Migrations run.
- Dashboard queries return expected summary.
- Source coverage updates after connector success/failure.

### Theme And Layout

- Dark and light themes render the same component structure.
- No nested page-section cards beyond repeated dashboard modules.
- Cards use radius 8px or less.
- Text does not overflow controls at 1280x800 and 1440x900.

## End-To-End And Visual Verification

### Web Preview E2E

- Launch Vite preview with mocked Tauri adapters.
- Load dashboard.
- Toggle dark/light themes.
- Trigger manual refresh with mocked data.
- Change data mode Local, Remote, Combined.
- Inspect source coverage hover/inspector.
- Verify no console errors.

### Tauri Smoke

- Launch Tauri dev app with synthetic local data path.
- App initializes SQLite and renders dashboard.
- Manual refresh succeeds with mocked remote connector.
- Close/reopen preserves snapshots and theme preference.

### Screenshot Verification

- Capture dark dashboard screenshot after data fixture load.
- Capture light dashboard screenshot after data fixture load.
- Compare screenshots against Command Center SOT checklist:
  - Persistent sidebar.
  - Header safety controls.
  - Metric strip.
  - Heatmap.
  - Reset timeline.
  - Coverage/connectors.
  - Footer safety row.
  - No marketing hero.
  - No decorative gradients/orbs.
  - No one-note purple/blue palette.

### Accessibility

- Keyboard navigation reaches sidebar, header controls, view toggles, tables, source coverage inspector, and footer links.
- Axe or equivalent catches no serious/critical issues in dashboard states.
- Icon-only buttons have accessible names and tooltips.
- Color contrast passes for text, badges, charts, focus rings, and status chips in both themes.

## CI Gates

- `pnpm lint`
- `pnpm typecheck`
- `pnpm test`
- `pnpm test:browser` for targeted Browser Mode tests
- `cargo test`
- `cargo clippy -- -D warnings`
- `cargo fmt --check`
- Secret scan against repository
- Fixture scan for auth-like/token-like values
- Build frontend
- Tauri dev/build smoke
- Windows build job with `pnpm tauri build` or `cargo tauri build` on Windows runner

## Manual Verification Checklist

- App starts with no local data and shows safe empty dashboard.
- App imports synthetic Codex history and shows daily/monthly analytics.
- App refreshes mocked reset-credit data and shows expiration in `America/New_York`.
- Unsafe endpoint attempts are visibly impossible from UI and rejected in guard tests.
- Logs contain no secrets.
- SQLite contains no secrets.
- Dark and light themes match Command Center density and hierarchy.
- README screenshots are captured from the real app, not mockups.

## Ultragoal Evidence Ledger

When handed off to `$ultragoal`, each checkpoint must attach or cite fresh evidence:

- Safety evidence: `.omx/ultragoal/evidence/02-safety-guard.md` with Rust guard test output, mock HTTP proof that denied endpoints are never called, and secret scan output.
- Data evidence: `.omx/ultragoal/evidence/03-data-import-analytics.md` with migration/importer/analytics/timezone test output and synthetic fixture coverage.
- Connector evidence: `.omx/ultragoal/evidence/04-readonly-connectors.md` with known reset-credit mock test, undocumented read-only registry test, and redacted connector failure test.
- UI evidence: `.omx/ultragoal/evidence/07-command-center-ui.md` with component tests, accessibility checks, dark screenshot, and light screenshot.
- Packaging evidence: `.omx/ultragoal/evidence/08-windows-packaging.md` with Tauri dev smoke and Windows `x86_64-pc-windows-msvc` build smoke result or exact blocker.
- Documentation evidence: `.omx/ultragoal/evidence/09-open-source-docs.md` with README, screenshots, security policy, contributor guide, data-source docs, connector-safety docs, and ADRs.
- Git evidence: `.omx/ultragoal/evidence/10-git-history.md` with commit list showing coherent Lore-protocol commits by stage.

Ultragoal should keep the goal open if any ledger item is missing, stale, or not tied to a concrete verification result.

## Test Data Policy

- Synthetic fixtures only.
- No real auth files.
- No real local user histories.
- No full raw endpoint responses from private accounts.
- Any real-world shape used during development must be manually minimized and redacted before becoming a fixture.

## Expanded Deliberate-Mode Test Plan

### Unit

Rust guard, auth locator/parser, endpoint registry, JSONL importer, SQLite repositories, analytics transforms, timezone conversion, Zod schemas, query key factories, UI pure components.

### Integration

Import pipeline, refresh orchestrator, connector safety with mock HTTP server, SQLite migrations, source coverage lifecycle, data mode filtering.

### E2E

Vite preview with mocked Tauri, Tauri dev smoke with synthetic data, dark/light screenshots, keyboard navigation, manual refresh, data mode switching.

### Observability

Structured redacted connector events, refresh spans, source coverage history, app-visible last refresh/degraded state, CI artifacts for screenshots and test logs.
