# Testing

Run the standard local gates:

```sh
pnpm lint
pnpm typecheck
pnpm test
pnpm test:browser
pnpm secret:scan
pnpm fixture:scan
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```

App-server protocol tests use a fake Codex CLI process to prove TokenStack only
calls the account read methods, handles fallback launch, timeouts, logged-out
states, partial method failures, notifications, and out-of-order responses.
Import tests prove synthetic local history imports are deterministic and
idempotent.

## Validate the packaged Windows connector

The Windows CI job installs the NSIS artifact and runs the same packaged
executable users receive. The smoke gate compiles a native fake Codex runtime
under a path that contains spaces, launches it without a shell, completes the
app-server account reads, persists the selected runtime and snapshots, and
reopens a sanitized diagnostics export.

The harness runs the installed executable twice with isolated application data.
The first run supplies and persists a user-selected runtime. The second removes
the runtime override and uses a thin Windows `PATH` containing the fake
`codex.exe` directory plus essential operating-system directories. Both runs
must report the selected native executable, runtime source, launch mode, method
results, and child cleanup in separate diagnostics files.

Run the gate from PowerShell after building or installing TokenStack:

```powershell
pnpm exec tauri build --target x86_64-pc-windows-msvc
pwsh ./scripts/windows-smoke.ps1
```

The smoke entrypoint is inert during normal app launches. It activates only
when the process receives `--tokenstack-packaged-smoke` and the harness sets
`TOKENSTACK_ENABLE_PACKAGED_SMOKE=1`.

## Complete the real Windows release check

Hosted CI has no authenticated Codex account. Before release, use an installed
TokenStack artifact and record sanitized evidence for this matrix:

- Launch with a thin GUI `PATH`, select the Codex App runtime under
  `%LOCALAPPDATA%\OpenAI\Codex\bin\codex.exe`, and test the connection.
- Select a standalone Codex CLI runtime and confirm account reads succeed.
- Select an npm Codex installation and confirm diagnostics show a native Node
  executable plus a fixed script argument prefix.
- Remove the configured selection, restart TokenStack, and confirm automatic
  discovery is deterministic.
- Test logged-out and missing-runtime states and confirm local history remains
  visible without an automatic login or interactive TUI.
- Export diagnostics, reopen the JSON file, and confirm it contains method,
  cleanup, persistence, and selected-runtime evidence without tokens, cookies,
  prompts, response bodies, or raw transcripts.

Record the Windows version, TokenStack commit, Codex App version, Codex CLI
version, test time, and diagnostics filename. A compiled package or fake-server
smoke does not replace this authenticated installed-Windows check.
