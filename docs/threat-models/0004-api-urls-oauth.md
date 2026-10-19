# Configured OAuth destinations — 2026-10-15

Assets: client secret, refresh/access tokens, source authorization, filter identity and ordered local output. Boundaries: external settings/UI → service; service → Keychain; service → configured OAuth/DAT endpoints; service → cache/job metadata. Only loopback fixtures are contacted in this change.

| Threat | Control / verification |
|---|---|
| Spoofed endpoint / credential forwarding | Empty URL defaults; explicit HTTPS or opt-in loopback; no redirect, embedded credentials, query or fragment; credentials bound to root + all three normalized URLs + client ID + scope. Profile-change tests verify no old credentials reach a new endpoint. |
| Tampered input/cache | Validated metadata; OAuth cache source identity includes the full auth profile; GUI drops loaded filters after profile edits. Manual bearer replacement acquires the source lock and invalidates the current cache pointer. Existing job pins remain immutable. |
| Repudiation / ambiguous errors | Safe structured oauth_ready/oauth_failed fields and existing job/source lifecycle events; HTTP status or bounded generic error only, never raw request/response bodies. No revocation operation is implemented. |
| Information disclosure | Hidden input only; client_secret and refresh token in profile-scoped Keychain items; generated access tokens RAM-only; Secret Debug redacted; token status exposes expiry only. Mask all three secret editor fields and zero editors on close/switch/save. Scan source/tests for accidental secrets. |
| Resource exhaustion / retry storm | 64 KiB token-body cap, configured overall/connect deadlines, one serialized auth flow with process lock, one invalid_grant fallback, one DAT 401 retry, auto-auth failure cooldown, no per-key calls. Cancellation drops guards and tasks are joined by the UI. |
| Privilege / tenant confusion | Full-profile credential/cache binding; no inferred URL paths; no client/default scope baked into the matching flow; no production endpoint fallback. Historical remote execution still requires explicit URLs and authorization, while disabled profiles may be saved as drafts. |

The existing provider JSON grant contract from ADR-0004 is preserved. [RFC 6749 section 6](https://www.rfc-editor.org/rfc/rfc6749.html#section-6) informs refresh-token replacement semantics; this client's JSON request encoding is the existing provider-specific contract, not a claim of generic form-encoded OAuth interoperability. Local tests do not establish real-provider compatibility, native Keychain UX, independent security review or production readiness.
