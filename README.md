# TokenStack

TokenStack is a local, read-only command center for Codex usage, reset-credit visibility, and source coverage. It is a Tauri desktop app with a Rust safety boundary, React dashboard, SQLite persistence, and synthetic test fixtures.

![TokenStack dark dashboard](docs/screenshots/tokenstack-dashboard-dark.png)

![TokenStack light dashboard](docs/screenshots/tokenstack-dashboard-light.png)

## Safety Guarantees

- Never calls any endpoint whose path contains `/consume`.
- Never consumes, redeems, claims, mutates, or spends reset credits.
- Keeps auth material in the Rust boundary and never sends auth tokens to React.
- Stores local analytics, connector status, and coverage metadata in SQLite without raw auth files or tokens.
- Shows source coverage and confidence instead of inventing certainty from incomplete data.

## Development

```sh
pnpm install
pnpm dev
pnpm tauri:dev
```

## Verification

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

Windows packaging is configured for Tauri NSIS output. A Windows runner should execute:

```sh
pnpm install
pnpm exec tauri build --target x86_64-pc-windows-msvc
```

## Data Sources

TokenStack imports synthetic-safe Codex history shapes from local JSONL files and refreshes reset-credit snapshots only through audited read-only connector code. Undocumented read-only support is enabled by default but isolated behind endpoint registry entries, response schemas, and the same safety guard as known endpoints.

## Privacy Summary

TokenStack runs locally and summarizes usage without exposing auth tokens or raw credential data. Test fixtures are synthetic, and connector errors are shown as safe status messages.

## License

MIT. See [LICENSE](LICENSE) and [docs/adr/0000-license.md](docs/adr/0000-license.md).
