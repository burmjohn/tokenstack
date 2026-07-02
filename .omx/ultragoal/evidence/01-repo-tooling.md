# Evidence 01: Repository And Tooling

Generated: 2026-07-02

## Implemented

- Initialized git repository in `/home/jburmeister/projects/tokenstack`.
- Configured origin as `https://github.com/burmjohn/tokenstack.git`.
- Added Tauri v2, Vite, React 19, TypeScript, Tailwind v4, shadcn-style local UI primitives, TanStack Query, Vitest, Playwright, ESLint, and pnpm lockfile.
- Added CI workflow for frontend gates, Rust core gates, secret scan, fixture scan, and Windows Tauri build smoke.
- Added MIT license and initial open source docs.

## Fresh Verification

- `pnpm lint`: passed.
- `pnpm typecheck`: passed.
- `pnpm test`: 3 files, 5 tests passed.
- `pnpm build`: Vite production build passed.
- `pnpm test:browser`: 1 Chromium dashboard screenshot test passed.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features`: 39 Rust core tests passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --no-default-features -- -D warnings`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` with the local Tauri GTK/WebKit sysroot: 41 Rust app/core tests passed plus the Tauri binary test target compiled and ran 0 tests.
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` with the local Tauri GTK/WebKit sysroot: passed.
- `pnpm secret:scan`: passed.
- `pnpm fixture:scan`: passed.
- `pnpm tauri:build` with the local Tauri GTK/WebKit sysroot: passed and produced `src-tauri/target/release/tokenstack`.

## Packaging Note

The local default-feature and Tauri build smoke used extracted Ubuntu packages under `/tmp/tokenstack-tauri-sysroot`; it did not install host packages or mutate system directories. Windows installer output remains delegated to the configured CI Windows runner.
