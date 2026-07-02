# Connector Safety

Every authenticated connector request must pass `SafetyGuard::validate`.

The guard denies:

- Any normalized path containing `/consume`.
- POST, PUT, PATCH, DELETE, OPTIONS, and other non-read-only methods.
- Request bodies.
- Unregistered endpoints.
- Missing response schemas.
- Unsafe hosts.
- Missing read-only review metadata.

Denied requests fail before network execution.
