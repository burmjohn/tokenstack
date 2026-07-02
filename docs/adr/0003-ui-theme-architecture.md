# ADR 0003: Command Center Theme Architecture

## Status

Accepted.

## Context

The app must ship complete dark and light Command Center themes with matching layout and hierarchy.

## Decision

Use Tailwind v4 CSS-first tokens and a root `data-theme` attribute. Components use shared semantic tokens rather than separate component trees per theme.

## Consequences

Dark and light screenshots can verify the same dashboard structure with different color tokens.
