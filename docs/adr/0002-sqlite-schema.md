# ADR 0002: SQLite Persistence

## Status

Accepted.

## Context

TokenStack needs deterministic local imports, last-good connector snapshots, derived analytics, and source coverage.

## Decision

Use a local SQLite schema with usage events, source documents, import runs, connector runs, reset-credit batches, rate-limit windows, refresh snapshots, and source coverage.

## Consequences

Migrations are part of the public contract and must be idempotent.
