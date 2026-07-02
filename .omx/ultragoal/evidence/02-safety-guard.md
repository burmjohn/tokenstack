# Evidence 02: Safety Guard

Generated: 2026-07-02

## Implemented

- `src-tauri/src/safety.rs` defines data-driven endpoint specs and `SafetyGuard::validate`.
- Guard rejects normalized paths containing `/consume`, non-GET/HEAD methods, request bodies, unregistered endpoints, unsafe hosts, plaintext transport for production endpoints, missing response schemas, and missing read-only review metadata.
- `src-tauri/src/auth.rs` keeps auth material in `SecretString` and exposes only redacted metadata.
- `src-tauri/src/telemetry.rs` redacts marker-based and token-shaped values before public error output.
- `src-tauri/src/connectors.rs` routes reset-credit and undocumented rate-limit connector requests through the guard before network execution.

## Fresh Verification

`cargo test --manifest-path src-tauri/Cargo.toml --no-default-features` passed all 39 Rust core tests, including:

- `rejects_any_path_containing_consume`
- `rejects_non_readonly_methods`
- `rejects_request_body_for_authenticated_connectors`
- `allows_registered_get_reset_credit_endpoint`
- `rejects_unregistered_undocumented_endpoint`
- `rejects_plaintext_transport_for_registered_auth_endpoint`
- `redacts_auth_values_in_errors`
- `auth_handle_never_serializes_secret`
- `consume_request_attempt_never_reaches_server`
- `allowed_endpoint_request_reaches_server_only_after_guard_approval`
- `undocumented_rate_limit_request_reaches_server_only_after_guard_approval`
- `connector_failure_does_not_expose_auth_values`

`pnpm secret:scan` passed with no auth-like token patterns found.
