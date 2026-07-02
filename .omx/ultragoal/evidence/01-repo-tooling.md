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
- `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features`: 30 Rust core tests passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --no-default-features -- -D warnings`: passed.
- `pnpm secret:scan`: passed.
- `pnpm fixture:scan`: passed.

## Packaging Note

`cargo test --manifest-path src-tauri/Cargo.toml` with the default Tauri feature failed on this Linux host before app code compilation because system packages for `glib-2.0`, `gobject-2.0`, and `gio-2.0` are missing. `sudo -n true` failed with interactive authentication required, so this host cannot install the Tauri Linux prerequisites non-interactively. Core Rust verification runs with `--no-default-features`; Windows build smoke remains a later CI/Windows-runner gate.
