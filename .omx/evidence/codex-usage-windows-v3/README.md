# Codex usage reliability Windows evidence

This ledger separates automated installed-package evidence from authenticated
Windows acceptance evidence. Add only sanitized screenshots, diagnostics, and
command summaries to this directory.

<!-- prettier-ignore -->
> [!CAUTION]
> Never add authentication tokens, cookies, prompts, response bodies, raw JSONL,
> SQLite databases, session transcripts, or account-identifying labels.

## Automated installed-package evidence

GitHub Actions proved the distributable package and connector harness on
Windows. This evidence does not replace testing against an authenticated Codex
App or Codex CLI installation.

- TokenStack commit: `0d12fc67058f3921fa49d3a80c7ea0b64d16b4f4`
- Windows workflow:
  [run 29137616409](https://github.com/burmjohn/tokenstack/actions/runs/29137616409)
- Result: `frontend-and-rust` and `windows-build-smoke` passed.
- Package: `tokenstack-windows-x64-setup` artifact uploaded.
- Diagnostics: `tokenstack-windows-packaged-smoke-diagnostics` artifact
  uploaded with explicit-selection and automatic-discovery reports.
- Runtime launch: native fake Codex executable launched directly from a path
  containing spaces, without a shell or PTY.
- Discovery modes: persisted user selection and thin mixed-case Windows `Path`.
- Protocol result: `account/read`, `account/rateLimits/read`, and
  `account/usage/read` succeeded; child cleanup was persisted and exported.

## Authenticated installed-Windows environment

Record the environment before running the manual matrix. Leave unknown values
as `Pending`; don't infer them from hosted CI.

- Status: `Pending — no authenticated Windows host is connected to this task`
- Windows version: `Pending`
- TokenStack commit: `0d12fc67058f3921fa49d3a80c7ea0b64d16b4f4`
- TokenStack installer artifact or build: `Pending`
- Codex App version: `Pending`
- Standalone Codex CLI version: `Pending`
- npm Codex package version: `Pending`
- Test timestamp with time zone: `Pending`
- Tester or host identifier, sanitized: `Pending`

## Authenticated acceptance matrix

For each row, record `Pass` or `Fail`, a sanitized evidence filename, and a
short observation. `Not run` is not a pass.

- [ ] Codex App installed with no standalone CLI on `Path`: discover or select
  `%LOCALAPPDATA%\OpenAI\Codex\bin\codex.exe` or an accessible bundled runtime,
  and complete the connection test.
- [ ] Standalone Codex CLI installed: discover, select, persist, restart, and
  complete the connection test.
- [ ] npm Codex installed: confirm diagnostics identify native `node.exe` plus
  the fixed Codex JavaScript argument prefix, with no `.cmd` shell launch.
- [ ] Codex App and CLI installed together: confirm deterministic precedence,
  switch runtimes, restart, and confirm the selected runtime persists.
- [ ] Logged out: confirm TokenStack shows login required without starting a
  login flow, interactive TUI, shell, or PTY.
- [ ] Runtime missing or inaccessible: confirm local history stays visible and
  **Setup** can select and validate a replacement runtime.
- [ ] Authenticated account data: compare lifetime usage, daily buckets,
  rate-limit windows, and reset-credit count with Codex at the same refresh
  time.
- [ ] Local Codex App and CLI sessions: confirm local totals increase without
  changing their local-history label or double-counting shared events.
- [ ] Diagnostics export: reopen the file, parse schema version 2, confirm the
  selected runtime and method stages, and run the repository secret scanner
  against the copied sanitized artifact.

## Evidence inventory

Add one line for every retained artifact. Use descriptive lowercase filenames,
and record why the artifact is safe to retain.

- `Pending`

## Completion decision

Keep the goal active until every authenticated matrix row passes and the
environment metadata and evidence inventory are complete. A compiled package,
fake-runtime smoke, or green hosted workflow alone is insufficient.
