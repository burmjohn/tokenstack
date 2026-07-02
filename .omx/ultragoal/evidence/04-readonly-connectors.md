# Evidence 04: Read-Only Connectors

Generated: 2026-07-02

## Implemented

- `KnownResetCreditsConnector` supports `/wham/rate-limit-reset-credits`.
- `UndocumentedRateLimitsConnector` supports `/backend-api/rate_limits`.
- Both read-only endpoints are represented in the endpoint registry as schema-gated, read-only reviewed metadata.
- `refresh_all` runs local import, the reset-credit connector, and the undocumented rate-limit connector in order; missing auth records degraded connector rows without network access.
- Connector run persistence records the actual connector id as endpoint provenance for both reset-credit and undocumented rate-limit lanes.
- Connector failures return redacted public errors.
- Mock HTTP tests prove allowed endpoints reach the server only after guard approval.
- Consume-path validation test proves denied `/consume` attempts fail before network execution.

## Fresh Verification

`cargo test --manifest-path src-tauri/Cargo.toml --no-default-features` passed:

- `known_reset_credit_schema_accepts_expected_shape`
- `known_reset_credit_schema_rejects_missing_expiration`
- `undocumented_rate_limit_schema_accepts_expected_shape`
- `undocumented_rate_limit_schema_rejects_missing_reset_timestamp`
- `allowed_endpoint_request_reaches_server_only_after_guard_approval`
- `undocumented_rate_limit_request_reaches_server_only_after_guard_approval`
- `consume_request_attempt_never_reaches_server`
- `connector_failure_does_not_expose_auth_values`
- `known_reset_credit_timeout_returns_failed_result`
- `refresh_persists_imported_history_for_later_summary_calls` proves missing auth degrades both remote lanes and persists `undocumented-rate-limits` provenance.
