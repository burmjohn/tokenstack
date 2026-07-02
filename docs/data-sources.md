# Data Sources

TokenStack supports three source families:

- Local Codex history JSONL.
- Known read-only reset-credit endpoint: `/wham/rate-limit-reset-credits`.
- Undocumented read-only endpoints that are explicitly registered, schema-validated, and shown as `Undocumented (RO)`.

Fixtures must be synthetic. Real auth files, private user histories, and full private endpoint responses must not be committed.

Unknown local history shapes produce warnings and lower source coverage instead of failing the whole import.
