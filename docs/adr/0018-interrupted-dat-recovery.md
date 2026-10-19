# ADR-0018: recover interrupted DAT bodies in RAM

Accepted for local implementation, 2026-10-15. Tier T3. Supplements ADR-0017.

Context: GET headers/status retries do not handle a reset or premature EOF after
HTTP 200/206. The current generic error loses the transport category and byte
offset, requiring the user to retry a potentially large DAT from the start.

Decision: recover idempotent GET body failures inside the existing total deadline
and shared download_retries budget. Resume a strong-ETag/known-size representation
with Range/If-Match and strict response checks; otherwise wipe the partial prefix
and restart the full GET. A full 200 response to single-transfer recovery replaces
the entire prefix; it is never appended. Parallel parts remain strict 206 ranges.
No automatic POST body replay or auth/crypto change. Keep partial bytes locked in
RAM, zeroized on drop, and inaccessible to matching until complete validation.

One AtomicU8 budget is shared across initial HTTP errors and existing part futures
so concurrency cannot multiply retry limits. Backoff/Retry-After remain bounded by
the total deadline. Log only static failure categories, offsets, expected bytes,
HTTP version/status and counters. The program does not claim to repair provider
gateway failures; it recovers eligible transient disruptions without mixing data.

Alternatives: unlimited retries amplify server rate limits; unconditional suffix
append without a strong validator can mix versions; always restarting wastes a
verified prefix; persisting it breaks the user's RAM-only requirement; replaying
arbitrary multipart POST may repeat server-side work.

Compatibility: existing Settings/CLI/UI/job schemas and default GET remain. The
retry count now covers all GET transport recovery, rather than initial status
responses only. Old jobs/pins are unchanged. No migration or dual-write needed.

Rollback: restore task source/docs and rebuild; no data/settings rollback needed.
Preserve user jobs/exports/Keychain. Local rebuild target RTO 5 minutes, RPO zero
for existing job checkpoints. No production ramp, on-call/page or service change.
Functional/security/coverage evidence and remaining publication gates are recorded
in body-recovery-verification.md; no independent approval is inferred.
