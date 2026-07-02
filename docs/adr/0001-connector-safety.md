# ADR 0001: Rust-Owned Connector Safety

## Status

Accepted.

## Context

TokenStack must refresh reset-credit visibility without consuming credits, mutating account state, or exposing auth material.

## Decision

All authenticated connector requests are built and validated in Rust. Endpoint metadata is data-driven and every request passes the safety guard before network execution.

## Consequences

Frontend contributors use typed Tauri commands rather than raw authenticated fetches.
