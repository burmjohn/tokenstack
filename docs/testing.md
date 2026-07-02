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

Safety tests prove `/consume` and mutation methods are rejected before any network call. Import tests prove synthetic local history imports are deterministic and idempotent.
