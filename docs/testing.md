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
