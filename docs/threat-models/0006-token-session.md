# Token/session correction

2026-10-15. Assets: client secret, access/refresh tokens, authenticated UI session, destination identity and preserved local jobs. Trust boundaries remain terminal → worker/Keychain → issuer/DAT and settings → credential/cache profile. Only MemorySecrets/loopback fixtures are used for verification.

| Threat | Control |
|---|---|
| Spoofed authentication from a 2xx response | Require successful JSON grant with nonempty usable access_token, Bearer type, bounded positive expiry and successful secret-store commit. Explicit missing/empty/whitespace token regressions and UI 200-without-token test. |
| Credential forwarding to a second refresh URL | Both grants route only to token_url; deprecated persisted destination is inert and omitted on save. Hidden CLI compatibility rejects a distinct endpoint before saving. No redirects; existing HTTPS policy. |
| Identity tampering inside an authenticated session | Remove credential editors and credential persistence from Settings. Save guard rejects changed client ID/token URL. Change identity only via sign-out/Home login. Session metadata/refresh remain read-only. |
| Credential/cache profile confusion | Bind root/DAT/token/token/client/scope; retain old common-URL profile compatibility, require fresh login for distinct historical destinations. Do not migrate credentials automatically. Pinned jobs unchanged. |
| Disclosure | Secret input only on Home, fixed mask, cleared buffers, no secret arguments/history/status/logs, access RAM only and long-lived secrets Keychain. CSV is synthetic hash-only test data. |
| Denial/cancellation | Existing deadlines, body caps, serialized auth, process lock, single joined worker and bounded invalid_grant/401 fallback unchanged. Failure/cancel leaves UI locked. |

Self-applied design/security review covers authorized local changes only. No new ingress, parser, crypto primitive, dependency or concurrency topology. No independent review, real provider/native Keychain acceptance, signed supply-chain artifact or production deployment is claimed. Recovery and rollback: ADR-0011 and matching local-recovery runbook.
