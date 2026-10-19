# ADR-0009 — Independent externally configured API URLs

Accepted for local configuration implementation, 2026-10-15.

Context: replacing per-key processing with local Bloom matching does not remove the provider's token acquisition/reset endpoints. The application must retain three independently configurable destinations. Provider URLs must not be runtime constants.

Decision: keep all URL values in external settings, initially empty. Matching settings expose token_url, reset_token_url and dat_url; accept the previous url field as a read alias. The API tab and CLI edit all three independently. No destination is derived from another. URL validation is shared, with HTTPS by default and explicit loopback HTTP for tests. No embedded credentials/query/fragment, no raw URL in validation diagnostics. Preserve the existing DAT-scoped bearer binding and job/filter pins. Historical OAuth defaults also become empty and require explicit configuration before remote operation; existing persisted profiles are unchanged.

Reset is an independent refresh-token renewal destination, as clarified by the user; it does not revoke access. Merely saving configuration never contacts an endpoint or triggers token rotation. Existing local matching remains usable with its configured DAT and optional bearer.

Alternatives rejected: deriving /token or /reset from the DAT URL (provider contracts need not share a host/path); retaining a fixed provider fallback (contradicts explicit configuration); inventing reset payload semantics (could revoke valid access).

Migration: additive matching fields plus serde read alias for old url. Old settings deserialize; new writes use canonical dat_url. No job/checkpoint/filter/data migration or Keychain rewrite. Rollback restores task source and translates dat_url back to url in a copy only if an old binary needs that settings file; preserve the user's original. No production rollout, destructive operation or release is performed.

Clarified decision: reset is refresh-token renewal, not revocation. The active application supports explicit OAuth2 mode (default OFF), configurable client_id/scope, hidden client_secret/optional refresh token inputs, initial JSON client_credentials grant at token_url and JSON refresh_token grant at reset_token_url. Preserve ADR-0004's established provider JSON payload rather than silently changing to form encoding. Both responses require Bearer/access_token/expires_in; a returned refresh token replaces the stored one. Scope is omitted when empty. Access tokens live only in memory, shared by Service clones; credentials/refresh tokens remain in Keychain, isolated by exact endpoint/client/scope profile. Public/manual bearer modes remain compatible.

A mutex serializes auth for one Service; a nonblocking process lock protects cross-process refresh rotation. Cancellation drops guards without a sticky refresh flag or detached task. The source GET may renew/retry once on 401. Refresh invalid_grant falls back once to the acquisition endpoint; transient POST failures are not retried. Failure cooldown bounds repeated auto-auth attempts. All calls have configured deadlines and token bodies are capped at 64 KiB. Configuration changes cannot reuse another profile's credentials/access token. Local tests cover these decisions; actual provider acceptance remains unverified.

Cache/rotation invariant: OAuth DAT cache source identity includes the complete credential profile, not only the DAT URL. Manual bearer replacement holds the source lock and invalidates the current cache pointer. Existing pinned jobs remain reproducible.
