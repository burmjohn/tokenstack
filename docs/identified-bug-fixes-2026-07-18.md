# Identified bug fixes — 2026-07-18

This change resolves the defects found during the Windows build and test audit.

## Fixed behavior

- Combined and remote refreshes now persist a degraded account-connector run when Codex runtime discovery fails, while still returning imported local usage.
- The usage heatmap now returns the latest 112 New York calendar days, fills real date gaps with zeroes, labels actual weekdays and months, and provides working daily, weekly, and monthly views.
- Reset-credit expiration falls back to the earliest available detail expiration when the aggregate expiration is absent. Explicit zero-credit snapshots no longer display a zero-day expiration.
- Setup diagnostics derive metric freshness from coverage and connector freshness instead of presentation colors.
- Desktop CSV and PNG exports report success or failure. PNG bytes are persisted through Tauri, and repeated same-name exports receive a unique suffix instead of failing or overwriting.
- Runtime-picker operations participate in the shared pending state, preventing overlapping picker, clear, test, and selection actions.
- Persisted npm runtime diagnostics retain their complete display path, native executable, argument prefix, and source when current discovery differs or disappears.
- The Windows-only Clippy test target no longer compiles an unused Unix executable-fixture helper.

## Verification

- `pnpm build`
- `pnpm lint`
- `pnpm test` — 81 passed
- `pnpm test:browser` — 1 passed
- `pnpm secret:scan`
- `pnpm fixture:scan`
- `cargo test --manifest-path src-tauri/Cargo.toml` — 128 unit tests and 5 Windows runtime integration tests passed
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `pnpm exec tauri build --target x86_64-pc-windows-msvc` — release executable and NSIS installer produced
- `scripts/windows-smoke.ps1` against the freshly installed NSIS package — explicit and automatic runtime modes passed
