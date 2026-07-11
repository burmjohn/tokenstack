# Make Codex App and CLI usage reliable on Windows

> **For goal execution:** Treat this file as the durable source of truth. Use
> test-driven development, update the checkboxes as evidence is produced, and
> do not mark the goal complete until the installed-Windows acceptance gate
> passes with a sanitized diagnostics artifact.

**Goal:** Make TokenStack reliably show Codex account usage, rate-limit windows,
reset credits, and local token history for Windows users who work through either
Codex App or Codex CLI.

**Architecture:** Use the authenticated Codex runtime's `app-server` JSON-RPC
surface for account-wide telemetry. Import local Codex App and CLI session data
as a separate local-history source. Preserve provenance through storage, IPC,
and UI so account snapshots and local history can be displayed together without
being conflated.

**Tech stack:** Tauri 2, Rust, SQLite, React, TypeScript, Vitest, GitHub Actions,
and a cross-platform fake Codex executable.

## Outcome and stop condition

The goal is complete only when a Windows packaged build proves all of the
following with the same executable artifact users install:

- TokenStack discovers or accepts a user-selected Codex runtime and validates
  it by completing the app-server handshake.
- `account/read`, `account/rateLimits/read`, and `account/usage/read` produce
  independently stored, independently degraded account snapshots.
- Codex App and CLI `token_count` records produce local history without being
  labeled as account-wide usage.
- Combined mode displays available account and local data at the same time.
- A failed account connector never hides valid local history.
- Diagnostics identify every executable candidate, the selected runtime,
  launch arguments, method-level result, timeout/exit stage, and last-good use.
- Diagnostics export writes a sanitized JSON file and reports its path.
- No code reads raw authentication tokens, calls private ChatGPT/Codex web
  endpoints, starts an interactive TUI/PTTY fallback, or calls
  `account/rateLimitResetCredit/consume`.
- Linux and Windows CI run the fake app-server protocol suite. A signed or
  unsigned Windows package is installed and manually validated against a real
  Codex App runtime and a real standalone/npm CLI runtime.

## Why the previous fix was insufficient

The July 7 implementation added the correct app-server methods and repaired
local-history display, but it did not close the installed-Windows execution
loop:

- `refresh_all` always constructs `CodexAppServerConfig::default()` and has no
  persisted configured path to pass into the connector
  (`src-tauri/src/commands.rs:333-357`).
- Windows discovery checks npm and several guessed `Programs` directories, but
  omits the Codex App's observed per-user runtime path
  `%LOCALAPPDATA%\OpenAI\Codex\bin\codex.exe`
  (`src-tauri/src/codex_app_server.rs:571-593`).
- Failed executable resolution stores no candidate list because
  `insert_account_refresh_error` writes `selected_codex_executable = NULL` and
  an empty candidate array (`src-tauri/src/db.rs:611-635`).
- The fake app-server is a generated POSIX shell script and imports
  `std::os::unix`, so Windows CI does not execute the protocol contract
  (`src-tauri/src/codex_app_server.rs:1183-1277`).
- GitHub's Windows job proved package compilation, not an installed GUI process
  locating and launching a real Codex runtime.

The plan therefore treats runtime discovery, configuration, process launch,
protocol compatibility, and installed-package validation as one connector
contract rather than separate best-effort patches.

## Non-negotiable constraints

- Do not parse, store, display, or export raw `auth.json` token values.
- Do not call private ChatGPT or Codex HTTP endpoints.
- Do not launch Codex TUI, `/status`, PTY, or browser authentication as an
  automatic fallback.
- Never call `account/rateLimitResetCredit/consume`.
- Spawn executables directly with argument arrays; do not use a shell.
- Treat local SQLite/JSONL/session data as local history only.
- Treat `account/usage/read` as account activity, not local session history.
- Preserve last-good account snapshots and visibly mark stale data.
- Do not show zero when a value is unavailable.
- Do not add dependencies unless existing Rust, Tauri, and frontend facilities
  cannot meet the requirement.

## Upstream and open-source evidence

Use these references as implementation evidence, not as code to copy blindly.
Pin fixture shapes to generated upstream schemas, and record the upstream commit
used when fixtures are refreshed.

### Official Codex protocol

- [Codex app-server README](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)
  defines stdio JSONL transport, initialization, `account/read`,
  `account/rateLimits/read`, `account/usage/read`, and rate-limit update
  notifications.
- [GetAccountTokenUsageResponse](https://github.com/openai/codex/blob/main/codex-rs/app-server-protocol/schema/typescript/v2/GetAccountTokenUsageResponse.ts)
  defines `summary` plus nullable `dailyUsageBuckets`.
- [GetAccountRateLimitsResponse](https://github.com/openai/codex/blob/main/codex-rs/app-server-protocol/schema/typescript/v2/GetAccountRateLimitsResponse.ts)
  defines the backward-compatible `rateLimits`, optional
  `rateLimitsByLimitId`, and optional `rateLimitResetCredits`.
- [Account protocol source](https://github.com/openai/codex/blob/main/codex-rs/app-server-protocol/src/protocol/v2/account.rs)
  is the canonical source for reset-credit and usage response fields.

### Open-source consumers

- [codex-usage-status app-server client](https://github.com/heycarollan/codex-usage-status/blob/main/src/codexAppServerClient.ts)
  launches `codex app-server`, enables `experimentalApi`, sends `initialized`,
  tracks request IDs, listens for updates, and exposes both rate-limit and token
  usage reads. It also exposes a consume method; TokenStack must explicitly
  reject that part of the example.
- [codexU](https://github.com/shanggqm/codexU/blob/main/Sources/CodexUsageWidget/main.swift)
  keeps cloud lifetime usage and local usage in separate fields, calls all three
  account methods, and parses local `token_count` events from `payload.info`.
- [codex-status-command](https://github.com/nelsonjchen/codex-status-command/blob/main/src/main.rs)
  launches `app-server --listen stdio:// -c mcp_servers={}`, reads account and
  rate limits, prefers the `codex` bucket, and preserves additional buckets. Its
  PTY fallback is out of scope for TokenStack.
- [CodexBar UsageFetcher](https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/UsageFetcher.swift)
  resolves Codex outside the GUI's thin environment, applies separate startup
  and request timeouts, serializes reads from one stdout stream, drains stderr,
  and shuts down the child deterministically. TokenStack must not adopt its raw
  token/JWT or private-web paths.

### Windows runtime evidence

- [OpenAI Codex issue 20872](https://github.com/openai/codex/issues/20872)
  documents `%LOCALAPPDATA%\OpenAI\Codex\bin\codex.exe` and the packaged
  `WindowsApps\...\app\resources\codex.exe` runtime.
- [OpenAI Codex issue 14364](https://github.com/openai/codex/issues/14364)
  shows Codex App launching its bundled `app\resources\codex.exe` as the
  app-server and demonstrates that existence alone does not prove executability.
- [OpenAI Codex issue 20864](https://github.com/openai/codex/issues/20864)
  documents that Codex App and CLI share `~/.codex/state_5.sqlite` and session
  indexes for local history.

## Source-of-truth model

The implementation must expose four explicit source facets:

1. **Local history:** additive token events parsed from Codex App and CLI local
   JSONL/session files. This powers local daily, monthly, project, session, and
   heatmap views.
2. **Account usage:** lifetime and account daily buckets from
   `account/usage/read`.
3. **Rate-limit windows:** every bucket and window from
   `account/rateLimits/read`, preferring `rateLimitsByLimitId` and falling back
   to `rateLimits`.
4. **Reset credits:** `rateLimitResetCredits.availableCount` and available
   metadata from the same read-only rate-limit response.

Combined mode is a presentation union, not a data merge. Local history must
never overwrite an account snapshot, and an account failure must never suppress
local history.

## Required interfaces

The goal may adjust names to match repository conventions, but it must preserve
these responsibilities and typed boundaries:

```rust
pub struct CodexRuntimeCandidate {
    pub display_path: PathBuf,
    pub launch: CodexLaunchSpec,
    pub source: CodexRuntimeSource,
    pub exists: bool,
    pub executable: Option<bool>,
    pub version: Option<String>,
    pub validation_error: Option<String>,
}

pub struct CodexLaunchSpec {
    pub executable_path: PathBuf,
    pub argv_prefix: Vec<String>,
}

pub struct CodexRuntimeSettings {
    pub configured_runtime: Option<CodexLaunchSpec>,
}

pub fn discover_codex_runtimes(settings: &CodexRuntimeSettings)
    -> Vec<CodexRuntimeCandidate>;

pub fn validate_codex_runtime(spec: &CodexLaunchSpec)
    -> Result<CodexRuntimeValidation, AccountConnectorError>;

pub fn refresh_account_snapshot(config: CodexAppServerConfig)
    -> Result<AccountSnapshot, AccountConnectorError>;
```

The error type must carry launch diagnostics even when resolution or spawn
fails:

```rust
pub struct AccountConnectorError {
    pub kind: AccountConnectorErrorKind,
    pub stage: String,
    pub public_message: String,
    pub launch: AccountLaunchDiagnostics,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub child_terminated: bool,
}
```

IPC must support read, select, clear, and validate operations for the configured
runtime. The frontend must never receive credentials or raw child output.

## Execution plan

### Task 1: Lock the failure with cross-platform tests

**Files:**

- Create: `src-tauri/tests/support/fake_codex.rs`
- Create: `src-tauri/tests/codex_runtime_windows.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/codex_app_server.rs`
- Modify: `.github/workflows/ci.yml`

**Deliverable:** The same native fake Codex executable runs on Linux and Windows
without a shell, and Windows CI executes the protocol tests instead of only
building the package.

- [x] Replace the POSIX-shell fixture with a compiled Rust test helper that can
  emit happy, partial, logged-out, malformed, wrong-ID, notification, hung,
  early-exit, and unsupported-argument scenarios.
- [x] Write failing Windows-safe tests for runtime discovery, direct spawn,
  initialization, method reads, timeout termination, and exit-code capture.
- [x] Add a static allow-list test that records every outbound account method
  and fails if any method other than `account/read`,
  `account/rateLimits/read`, or `account/usage/read` is sent after initialize.
- [x] Add a repository scan test that fails on the literal
  `account/rateLimitResetCredit/consume` outside test assertions and plan/docs
  allow-lists.
- [x] Run the suite on `windows-latest` and Linux before production changes.

**Proof:** Both operating systems show the expected red tests for missing
runtime settings/discovery and green existing protocol behavior.

### Task 2: Make runtime discovery deterministic and observable

**Files:**

- Create: `src-tauri/src/codex_runtime.rs`
- Modify: `src-tauri/src/codex_app_server.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/codex_runtime_windows.rs`

**Candidate precedence:**

1. Persisted user-selected path.
2. `TOKENSTACK_CODEX_BIN`.
3. Current process `PATH`.
4. `%LOCALAPPDATA%\OpenAI\Codex\bin\codex.exe`.
5. `%APPDATA%\npm\codex.cmd` and its resolved Node.js entrypoint.
6. Native standalone install paths under `%LOCALAPPDATA%\Programs`.
7. Read-only discovery of installed MSIX package resources.

- [x] Deduplicate candidates by normalized/canonical path while preserving
  source and precedence.
- [x] Validate candidates by direct process spawn with a short version/help
  timeout; a file that exists but returns access denied is not valid.
- [x] Model every candidate as a typed launch specification containing a display
  path, native executable path, and fixed argument prefix. For an npm shim,
  parse only the standard launcher structure to resolve `node.exe` plus the
  Codex JavaScript entrypoint; never execute `.cmd`/`.bat` through `cmd.exe`.
- [x] Reject an npm shim whose target cannot be resolved unambiguously. Do not
  accept arbitrary command text or persist shell fragments.
- [x] Record all candidates and validation outcomes in sanitized diagnostics,
  including access denied, file not found, timeout, and nonzero exit.
- [x] Select the first validated candidate, not the first existing path.

**Proof:** Fixture tests cover thin GUI `PATH`, per-user Codex App runtime,
standalone CLI, npm install, inaccessible WindowsApps runtime, stale configured
path, and no runtime found.

### Task 3: Persist and validate the user's Codex runtime selection

**Files:**

- Create: `src-tauri/src/settings.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/api/tauri.ts`
- Modify: `src/lib/schemas/dashboard.ts`
- Test: Rust command and migration tests; frontend API schema tests

**Deliverable:** A selected executable survives app restart and is used by every
account refresh.

- [x] Add an additive settings migration for the typed configured runtime:
  display path, native executable path, fixed argument prefix, and source.
- [x] Add Tauri commands to list candidates, choose/clear a path, and validate
  the current selection.
- [x] Pass the complete configured `CodexLaunchSpec` into
  `CodexAppServerConfig.explicit_runtime` from `refresh_all`; remove the current
  unconfigurable default-only path without dropping an npm runtime's argument
  prefix.
- [x] Validate before persisting. Return a structured failure without replacing
  the last working selection.
- [x] Store the typed launch specification and validation metadata only. Never
  copy the executable, persist arbitrary shell commands, or read authentication
  files.

**Proof:** A restart-oriented database test selects fixture A, reopens the
database, refreshes through fixture A, switches to fixture B, and clears back to
automatic discovery.

### Task 4: Harden app-server lifecycle and protocol compatibility

**Files:**

- Modify: `src-tauri/src/codex_app_server.rs`
- Modify: `src-tauri/src/telemetry.rs`
- Test: cross-platform fake app-server suite

**Deliverable:** A bounded read-only JSON-RPC client that leaves no child
processes behind and explains exactly where it failed.

- [x] Keep direct argument-array launch and newline-delimited JSON framing. Build
  the final arguments as `launch_spec.argv_prefix + ["app-server", ...]` and
  pass them directly to `Command`.
- [x] Use an ordered, bounded launch strategy based on current official CLI
  support: canonical stdio mode first, one compatibility fallback only after a
  proven argument rejection, and no fallback after timeout/auth/protocol errors.
- [x] Include `capabilities.experimentalApi = true`, then send `initialized`.
- [x] Match responses by ID, accept notifications at any point, reject
  unsupported server requests, and preserve method context on JSON-RPC errors.
- [x] Drain stderr concurrently into a bounded redacted suffix.
- [x] Separate validation, initialize, request, and whole-refresh timeouts.
- [x] On every timeout/error/drop path, close stdin, terminate the child, wait
  for exit, and record whether termination succeeded.
- [x] Keep account, rate-limit, and usage reads independently degradable after
  authentication succeeds.

**Proof:** Tests assert no orphan process, no retry on timeout, one fallback on
argument rejection, notification tolerance, wrong-ID tolerance, bounded stderr,
and redaction of token-like content.

### Task 5: Normalize against generated upstream schemas

**Files:**

- Create: `src-tauri/tests/fixtures/app_server/README.md`
- Create: synthetic JSON fixtures under
  `src-tauri/tests/fixtures/app_server/`
- Modify: `src-tauri/src/codex_app_server.rs`
- Modify: `src-tauri/src/db.rs`

**Deliverable:** Schema drift produces a visible partial failure, not silently
empty data.

- [x] Generate synthetic fixtures from the upstream TypeScript/JSON schemas and
  record the upstream commit SHA and generation date.
- [x] Parse `rateLimitsByLimitId` first, with `rateLimits` fallback.
- [x] Sort `codex` first while preserving every additional bucket and unknown
  window duration.
- [x] Parse optional reset-credit summary and detail rows. Distinguish `null`,
  explicit zero, and a positive count.
- [x] Parse account usage summary and nullable daily buckets. Distinguish absent
  fields from explicit zero.
- [x] Store the method status and raw schema version/fingerprint needed for
  diagnosis, but do not store raw response bodies.
- [x] Preserve the last-good facet separately when one method fails.

**Proof:** Golden fixture tests cover current schema, backward-compatible rate
limits, missing optional fields, unknown extra fields, malformed required
fields, explicit zero, and partial method success.

### Task 6: Import both Codex App and CLI local history safely

**Files:**

- Modify: `src-tauri/src/discovery.rs`
- Modify: `src-tauri/src/importers.rs`
- Modify: `src-tauri/src/db.rs`
- Test: importer fixtures derived from sanitized Codex App and CLI shapes

**Deliverable:** Local history includes every parseable local `token_count`
event once, regardless of whether Codex App or CLI wrote it.

- [x] Discover the shared default `CODEX_HOME`, explicit `CODEX_HOME`, session
  and archived-session locations, and bounded state/index sources without
  scanning unrelated directories.
- [x] Parse the known `payload.info.total_token_usage`, `last_token_usage`, and
  direct `payload.info` variants with snake_case and camelCase aliases.
- [x] Treat cumulative counters as snapshots and derive nonnegative deltas per
  session where needed; do not sum repeated cumulative totals as independent
  usage.
- [x] Deduplicate by stable source identity, session/turn identity, timestamp,
  and token counters so App and CLI views of the same event do not double count.
- [x] Keep warning samples shape-only and bounded; never include prompt or tool
  bodies.
- [x] Compute local coverage from parseable evidence, not only newly inserted
  rows.

**Proof:** Tests cover App-written JSONL, CLI-written JSONL, shared files,
duplicates, cumulative updates, archives, malformed lines, explicit
`CODEX_HOME`, and re-import stability.

### Task 7: Preserve provenance and last-good facets in storage

**Files:**

- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/analytics.rs`
- Test: migration, query, and dashboard model tests

**Deliverable:** Each UI value has one explicit source, freshness, and status.

- [x] Make account refresh runs record candidate diagnostics even when
  resolution/spawn fails.
- [x] Associate usage, rate limits, and reset credits with method-level status
  and capture time.
- [x] Query the newest successful snapshot per facet, not only the newest run.
- [x] Mark last-good data stale/degraded when the latest attempt for that facet
  failed.
- [x] Keep local-history rows and account-snapshot rows in separate tables and
  formulas.
- [x] Collapse source coverage to one latest row per metric.

**Proof:** A sequence of success, partial failure, total failure, and recovery
returns the correct current/last-good value and status for every facet.

### Task 8: Make Setup repair the connector in-app

**Files:**

- Modify: `src/components/command-center/CommandCenterShell.tsx`
- Modify: `src/features/dashboard/useDashboardSummary.ts`
- Modify: `src/lib/api/tauri.ts`
- Modify: `src/lib/schemas/dashboard.ts`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `package.json`
- Modify: `src-tauri/capabilities/main.json`
- Test: component and API tests

**Deliverable:** A Windows user can resolve `missing_cli` without setting an
environment variable or opening a terminal.

- [x] Show automatic candidates, source, version, validation result, and selected
  status.
- [x] Add the Tauri dialog plugin and least-privilege capability needed for a
  native executable picker, plus **Use**, **Clear**, and **Test connection**
  actions. Scope the picker to files and validate the selected file in Rust.
- [x] After selection, validate immediately, persist only on success, refresh
  account facets, and invalidate dashboard/setup queries.
- [x] Show exact staged outcomes: runtime not found, access denied, unsupported
  CLI, logged out, initialize timeout, method partial failure, and connected.
- [x] Show the last successful account refresh and whether displayed data is
  stale.

**Proof:** Frontend tests cover missing to connected, invalid selection,
selection persistence, clear to auto-discovery, and partial-method status.

### Task 9: Present account and local data together without hiding either

**Files:**

- Modify: `src-tauri/src/analytics.rs`
- Modify: `src/components/command-center/CommandCenterShell.tsx`
- Modify: `src/lib/schemas/dashboard.ts`
- Test: Rust summary tests and React component tests

**Deliverable:** Combined mode visibly contains both source families.

- [x] Render account lifetime/today/month when available, with stale state when
  last-good is used.
- [x] Render local lifetime/today/month/session/heatmap as local history in a
  distinct section or clearly distinct metric labels.
- [x] When account usage is unavailable, keep local history visible and show an
  account-specific failure alongside it.
- [x] Keep rate limits and reset credits unavailable unless explicitly returned;
  explicit zero remains distinguishable from unavailable.
- [x] Give every connector and coverage row a stable unique key and one latest
  entry.

**Proof:** Snapshot/component tests cover local-only, remote-only, combined
success, combined account failure, last-good account data, zero credits, and no
local history.

### Task 10: Upgrade diagnostics into a reproducible support artifact

**Files:**

- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/telemetry.rs`
- Modify: `src/components/command-center/CommandCenterShell.tsx`
- Modify: `src/lib/schemas/dashboard.ts`
- Test: Rust export tests and frontend export-state tests

**Deliverable:** One exported file explains discovery, launch, protocol, local
import, storage, and UI source selection without exposing sensitive content.

- [x] Bump the diagnostics schema and include candidate paths/sources/results,
  selected path, version, launch arguments as a safe enum, first failing stage,
  exit code, timeout, child cleanup, method statuses, last-good use, schema
  fingerprint, local roots, parse counts, duplicate counts, and warning counts.
- [x] Include selected data mode and the source/freshness/status behind each
  displayed dashboard metric.
- [x] Redact tokens, cookies, prompts, response bodies, raw JSONL content, and
  account-identifying labels.
- [x] Write atomically to the diagnostics directory, return the final path, and
  show success/failure in the UI.
- [x] Add a test that reopens and parses the written file from disk.

**Proof:** Secret/fixture scans pass, a token-seeded test remains redacted, and
the frontend displays the returned saved path.

### Task 11: Add installed-Windows release gates

**Files:**

- Modify: `.github/workflows/ci.yml`
- Create: `scripts/windows-smoke.ps1`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `docs/testing.md`
- Modify: `docs/data-sources.md`

**Deliverable:** A green build means more than compilation.

- [x] Run Rust unit/integration tests on `windows-latest`, including the native
  fake app-server and runtime discovery fixtures.
- [x] Add an argument-gated packaged smoke entrypoint that reuses production
  refresh and diagnostics code, accepts only a test runtime launch spec from the
  CI harness, exports diagnostics, verifies expected method results and child
  cleanup, then exits with a meaningful process code. Compile the entrypoint
  into release artifacts but make it inert unless the explicit smoke argument
  is present.
- [x] Build the Tauri Windows artifact, install/unpack it in CI, invoke the
  packaged smoke entrypoint, and prove it can start the fake Codex executable
  from a path containing spaces.
- [x] Export diagnostics during the smoke and upload the sanitized file as a CI
  artifact.
- [x] Keep a manual release checklist for real Codex App and standalone/npm CLI
  because hosted CI has no authenticated Codex account.
- [x] Test both a thin GUI `PATH` and a user-selected executable.

**Proof:** CI logs show protocol tests on Windows, packaged smoke success, child
cleanup, diagnostics file creation, and artifact upload.

### Task 12: Validate on a real Windows install and close the goal

**Files:**

- Create: `.omx/evidence/codex-usage-windows-v3/README.md`
- Store only sanitized screenshots, diagnostics, and command/check summaries
  under the evidence directory.

Run the final matrix on the installed application:

- [ ] Codex App installed, no standalone CLI on `PATH`: auto-discover or select
  the per-user/bundled runtime and connect.
- [ ] Standalone or npm Codex CLI installed: discover and connect.
- [ ] Both installed: show deterministic selected runtime and permit switching.
- [ ] Logged out: show login required without launching login automatically.
- [ ] Runtime missing/inaccessible: local history remains visible and Setup can
  repair the path.
- [ ] Happy account response: compare account lifetime, daily buckets,
  rate-limit windows, and reset-credit count with Codex's own visible data at
  the same refresh time.
- [ ] Local App and CLI sessions: confirm local totals increase without changing
  provenance or double counting shared events.
- [ ] Export diagnostics: confirm the file exists, parses, explains the selected
  runtime and methods, and contains no token, cookie, prompt, or raw transcript.

The goal owner must attach the sanitized evidence and record the exact Codex App
version, CLI version, TokenStack commit, Windows version, and test timestamp.
Do not close the goal based only on fake-server tests or GitHub build success.

## Verification commands

Run targeted tests during each task, then run the full gate from the repository
root:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
pnpm lint
pnpm test
pnpm build
pnpm secret:scan
pnpm fixture:scan
pnpm tauri:build
git diff --check
```

Run the Windows-specific gate in GitHub Actions and locally on Windows:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
pnpm test
pnpm exec tauri build --target x86_64-pc-windows-msvc
pwsh ./scripts/windows-smoke.ps1
```

Record exact failures and the closest valid substitute if a command is not
available. Never convert an unavailable real-Windows test into a pass.

## Acceptance criteria

- [ ] The installed app can discover, select, persist, validate, and use a Codex
  App or CLI runtime on Windows.
- [x] Account usage comes only from `account/usage/read`.
- [x] Rate limits and reset credits come only from
  `account/rateLimits/read`.
- [x] Local App and CLI token history remains local and is visible in combined
  mode.
- [x] Every missing or stale facet is explicit; unavailable is never rendered as
  zero.
- [x] Protocol and discovery tests execute on Windows and Linux.
- [x] The packaged Windows smoke launches a fake runtime from a path with spaces.
- [ ] Real installed-Windows evidence covers Codex App and standalone/npm CLI.
- [x] Diagnostics writes and reopens a sanitized file that identifies the actual
  failing stage.
- [x] Static and runtime tests prove no consume call, token parsing, private web
  endpoint, shell launch, or interactive fallback exists.
- [x] Full verification passes, the change is committed in Lore format, pushed,
  and GitHub checks are green.

## Execution evidence — 2026-07-10

Local Linux implementation and release verification is complete on branch
`codex/codex-app-cli-usage-reliability-v3`:

- Rust on Linux: 126 unit tests and 2 cross-platform integration tests passed.
- Rust on Windows: 122 unit tests and 5 integration tests passed, including
  mixed-case `Path` discovery and direct native launch from a path with spaces.
- Frontend: 74 tests passed; ESLint and the production Vite build passed.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` passed.
- Secret and synthetic-fixture scans passed.
- `pnpm tauri:build` produced
  `src-tauri/target/release/tokenstack` from the production configuration.
- `git diff --check` passed.
- Diagnostics tests write atomically, reopen and parse schema v2, exercise
  concurrent filenames, and verify backend plus browser-fallback redaction.
- The Windows workflow now installs the NSIS artifact and invokes the installed
  executable twice: an explicit runtime and automatic discovery under a thin
  `PATH`, both using a native fake runtime from a path containing spaces.
- GitHub Actions run
  [29137283843](https://github.com/burmjohn/tokenstack/actions/runs/29137283843)
  passed for commit `50f4c37e39b3065041f1e2bbb9940601e4fafdd8`.
  Its installed-package log contains `PACKAGED_SMOKE_OK` for both `explicit`
  and `automatic` modes, and it uploaded
  `tokenstack-windows-packaged-smoke-diagnostics` plus
  `tokenstack-windows-x64-setup`.

Pending evidence that must not be treated as passed:

- A real installed Windows environment with authenticated Codex App and
  standalone/npm CLI must complete the manual checklist in `docs/testing.md`.
  Record the sanitized results in
  `.omx/evidence/codex-usage-windows-v3/README.md`; the ledger currently marks
  every authenticated row as pending because no such host is connected.

## Risks and mitigations

- **MSIX runtime ACLs:** A bundled path may exist but be inaccessible. Validate
  executability and keep a user-selected per-user runtime path as the supported
  repair route.
- **Protocol drift:** Pin synthetic fixture provenance to upstream generated
  schemas and treat malformed required fields as visible partial failure.
- **Cumulative local counters:** Derive per-session deltas and deduplicate before
  aggregation; never blindly sum snapshots.
- **Multiple Codex installs:** Use explicit precedence, expose the selected
  runtime, and let the user override it.
- **Stale account data:** Store per-facet success and latest attempt separately,
  display stale age, and never silently present stale data as current.
- **Child leaks:** Test process termination and waiting on every failure path on
  Windows and Linux.

## Goal execution guidance

Use `$ultragoal` as the durable ledger owner for this plan. Parallel execution
is useful only after Task 1 locks the cross-platform contract:

- `test-engineer`: Tasks 1 and 11, including Windows CI evidence.
- `executor` (Rust connector): Tasks 2, 4, and 5.
- `executor` (storage/import): Tasks 3, 6, and 7.
- `executor` (frontend/IPC): Tasks 8, 9, and 10.
- `verifier`: Task 12 and final acceptance audit.

Recommended sequencing is Task 1, then Tasks 2/4 and 6 in parallel, followed by
Tasks 3/5/7, then Tasks 8/9/10, then Tasks 11/12. The goal owner must checkpoint
test output, diagnostics artifacts, and commit SHAs after every task. Keep no
more than one writer on a shared file at a time.

## Delivery protocol

- Stage only task-related files.
- Use small Lore-format commits at independently reviewable task boundaries.
- Push the branch after full local verification.
- Watch GitHub checks through completion and fix failures before claiming done.
- Merge only when the repository workflow and target branch are unambiguous.
- Preserve unrelated worktree changes.
