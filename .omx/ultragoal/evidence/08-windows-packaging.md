# Evidence 08: Packaging And CI

Generated: 2026-07-02

## Implemented

- `src-tauri/tauri.conf.json` configures Tauri v2 app metadata, Vite dev/build commands, desktop window sizing, and NSIS bundle target.
- `.github/workflows/ci.yml` includes a Windows `x86_64-pc-windows-msvc` Tauri build smoke job.
- `package.json` includes `pnpm tauri:dev` and `pnpm tauri:build`.

## Fresh Verification

- `pnpm build`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features`: 30 Rust core tests passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --no-default-features -- -D warnings`: passed.

## Exact Local Packaging Blocker

`cargo test --manifest-path src-tauri/Cargo.toml` with the default Tauri app feature failed before app code compilation because this Linux host lacks `glib-2.0`, `gobject-2.0`, and `gio-2.0` pkg-config packages. `sudo -n true` failed with interactive authentication required, so the missing host prerequisites cannot be installed by this agent. Windows build smoke is configured in CI but has not been executed locally because the current host is Linux and lacks the required native Tauri desktop dependencies.
