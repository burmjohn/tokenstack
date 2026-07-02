# Evidence 08: Packaging And CI

Generated: 2026-07-02

## Implemented

- `src-tauri/tauri.conf.json` configures Tauri v2 app metadata, Vite dev/build commands, desktop window sizing, and NSIS bundle target.
- `.github/workflows/ci.yml` includes a Windows `x86_64-pc-windows-msvc` Tauri build smoke job.
- `.github/workflows/ci.yml` installs Tauri Linux prerequisites before default-feature Rust app compile checks on Ubuntu.
- `package.json` includes `pnpm tauri:dev` and `pnpm tauri:build`.
- `src-tauri/icons/` contains Tauri-generated desktop/mobile app icons, including `icon.png`, `icon.ico`, and `icon.icns`, generated from the TokenStack app icon source image.

## Fresh Verification

- `pnpm build`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features`: 39 Rust core tests passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` with the local Tauri GTK/WebKit sysroot: 41 Rust app/core tests passed plus the Tauri binary test target compiled and ran 0 tests.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --no-default-features -- -D warnings`: passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` with the local Tauri GTK/WebKit sysroot: passed.
- `pnpm tauri icon /home/jburmeister/.codex/generated_images/019f2444-a45b-7152-b733-e0a48baf4739/ig_0df4797ebb1dd0b2016a46c3ad18548190a2eb80ba6e24bd6d.png`: passed and generated Windows `.ico`, macOS `.icns`, PNG, iOS, and Android icon assets.
- `pnpm tauri:build` with the local Tauri GTK/WebKit sysroot: passed. The command completed the frontend production build, compiled the Tauri app, and produced `src-tauri/target/release/tokenstack` at 18,822,952 bytes.

## Packaging Notes

- The local build smoke used extracted Ubuntu packages under `/tmp/tokenstack-tauri-sysroot`; it did not install host packages or mutate system directories.
- This Linux host produced the Tauri release binary but did not emit a Windows NSIS installer artifact. Windows installer output remains delegated to the configured CI Windows runner because NSIS Windows installer smoke requires the `x86_64-pc-windows-msvc` target on Windows.
- Public release signing remains intentionally approval-gated and is not required for the unsigned development build smoke.

## Earlier Blocker Resolved

The first `pnpm tauri:build` attempts failed because this Linux host lacked Tauri GTK/WebKit prerequisites and the repository had no Tauri app icon. Fresh evidence now shows both issues resolved for local smoke: the required system libraries were provided by a local extracted sysroot, Tauri icon assets were generated, and the release binary build passed.
