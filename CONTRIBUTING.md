# Contributing

TokenStack is safety-first. Before adding connectors or analytics:

- Add tests first.
- Use synthetic fixtures only.
- Do not commit auth files, private history, or raw private endpoint responses.
- Keep authenticated HTTP inside the Rust connector boundary.
- Run lint, typecheck, Rust tests, frontend tests, secret scan, and fixture scan before opening changes.

Commit messages should follow the Lore Commit Protocol from the workspace instructions.
