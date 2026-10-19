# Session and configurable DAT transport

2026-10-15. Delta to the existing authenticated PGMENC01 memory policy.

Home credentials and rotating refresh tokens belong to the active profile's
zeroizing RAM session. The normal UI never accesses Keychain. Explicit CLI
credential storage remains opt-in. Failed authentication installs no session;
profile changes and logout drop session credentials. Access generation guards,
expiry/cooldown, token response bounds and one 401 renewal remain intact.

Spoofing: retain HTTPS and certificate verification, no redirects or URL-embedded
credentials. Reject nested URL mistakes. Profile includes the request mode and
non-secret form metadata; default single GET preserves historical CLI profiles.
Tampering: parallel responses require strong ETag/If-Match plus exact ranges,
lengths and totals, followed by envelope SHA/AES-GCM/header AAD/Bloom SHA checks.
No partial output or plaintext downgrade. Ignore initial Range only by consuming
the same HTTP 200 response as one full stream.
Repudiation: safe events record mode, phase, HTTP status, bytes, retries and elapsed
time; never headers, field values, URLs, keys, tokens or raw error chains.
Disclosure: all owned DAT pages stay locked and wipe on drop, including canceled
parallel futures. Multipart fields are explicitly non-secret persisted metadata;
field names cannot inject headers and bounded values cannot contain controls.
Denial of service: total and body-idle deadlines, 10-second connect timeout,
download limit, 1-128 MiB parts, 1-16 in flight, 0-5 retries; one global transfer
deadline includes retries/parts/401 renewal. ADR-0018 extends the same shared
retry budget to interrupted GET bodies and individual ranges, with exact strong
ETag/If-Match and range/length checks. There is no transient-error retry of POST;
initial 401 recovery remains separately bounded to one. Unknown validators force
a prefix wipe/full GET restart; mixed or partial bytes never enter matching.
Retry-After beyond supported bounds aborts instead of retrying early.
Elevation: no authentication bypass, Keychain ACL/OS security change, new endpoint
or new dependency. Known memory limitations for TLS/codec internals, privileged
dumps, hibernation and abrupt kills remain documented in the shared manual.

Prepared-job failures preserve the UUID; repeat input while busy is refused.
Existing user jobs are not merged/deleted. There is no provider-side write,
production rollout or independent security review asserted by this local repair.
