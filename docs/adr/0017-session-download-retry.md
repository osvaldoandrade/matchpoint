# ADR-0017: session credentials and stable job retries

Status: accepted for local implementation, 2026-10-15.

Context: DAT errors discard transport causes. OAuth and a large DAT share a
60-second total budget. Home login writes Keychain and every job reads it;
macOS can prompt again after rebuild. Failed DAT acquisition after durable job
creation returns New Job, so submitting again creates another UUID.

Decision: verify the real authorized flow before assigning the timeout cause;
use phase-specific safe errors and independently bounded DAT transfer settings.
Successful Home login keeps zeroizing credentials in profile-bound session RAM;
only explicit CLI saved-credential mode touches Keychain. Once prepared, failed
jobs return to Jobs and resume the existing UUID. Reject nested URL accidents.
No server/wire/crypto change, fallback to stale credentials, DAT cache or new pool.

Alternatives: extending all OAuth timeouts weakens token failure feedback;
changing Keychain ACLs changes OS security and does not fix session ownership;
deleting prepared jobs on failure loses retry identity and input snapshots.

Consequences: Home requires login again after process exit/sign-out. Existing
explicit CLI credentials remain compatible. Additive timeout fields default
on existing settings. No stored job migration or automatic duplicate deletion.

Threat delta: retain redaction/zeroization, exact profile binding, token response
validation, no redirects, HTTPS, bounded transfers, AEAD/checksums. No endpoints
or telemetry added. A session never reads/writes/deletes Keychain credentials.
STRIDE: no auth bypass; settings/profile changes drop credentials; external
responses remain bounded/untrusted; auth/transfer failures expose no payload.

Runbook: at minute 5 distinguish token, response headers, HTTP, body and decode;
at minute 15 correct external settings or wait for provider recovery and resume
the same UUID; at minute 60 preserve the failed job/checkpoint and safe timings
for investigation. Do not disable TLS/auth/checksums or copy secrets into logs.
Rollback source from task backup and rebuild; user files remain unchanged
(RPO zero for durable jobs, local rebuild RTO target 5 minutes). There is no
server rollout, on-call/page or production maintenance operation in this task.

Validation evidence and remaining publication gates: session-download-plan.md
and the accompanying verification report. Local functional approval is distinct
from independent T4 security review, signatures, CI or staged production rollout.

Backward compatibility: the current version reads old settings with defaults and
preserves a legacy timeout longer than 600 seconds. Prior versions reject new
settings fields (deny_unknown_fields). To roll back after saving new settings,
back up settings.json and remove only the newly introduced download_* keys;
retain max_download_mib, source/profile values and all job files. No secret
store migration or copied credentials are needed. Default single GET/empty form
keeps the historical CLI credential account fingerprint.
