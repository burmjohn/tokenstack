# Evidence 04: Read-Only Connectors

Generated: 2026-07-02

## Implemented

- `KnownResetCreditsConnector` supports `/wham/rate-limit-reset-credits`.
- Undocumented read-only endpoint support is represented in the endpoint registry as schema-gated, read-only reviewed metadata.
- Connector failures return redacted public errors.
- Mock HTTP test proves an allowed endpoint reaches the server only after guard approval.
- Consume-path validation test proves denied `/consume` attempts fail before network execution.

## Fresh Verification

`cargo test --manifest-path src-tauri/Cargo.toml --no-default-features` passed:

- `known_reset_credit_schema_accepts_expected_shape`
- `known_reset_credit_schema_rejects_missing_expiration`
- `allowed_endpoint_request_reaches_server_only_after_guard_approval`
- `consume_request_attempt_never_reaches_server`
- `connector_failure_does_not_expose_auth_values`

The default full Tauri build path remains blocked on missing Linux GTK/GLib prerequisites in this host, documented in `08-windows-packaging.md` when that packaging gate is reached.
