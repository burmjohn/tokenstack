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
