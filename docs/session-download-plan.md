# Reliable authenticated downloads and job retry

2026-10-15. Classification T4: session-secret custody/authentication changes.
Local repair and verification only; no publication, server changes, credential
rotation, Keychain ACL changes, deletion of user jobs, or animation changes.
Backup pointer: `/tmp/pgm-session-download-backup-path`.

## Design and acceptance before implementation

Reproduce the authorized QA token/DAT flow with hidden interactive credentials,
an isolated temporary service and a RAM secret store. Never save credentials,
DAT bytes or returned tokens in diagnostic files. URLs come from external
settings; reject accidentally concatenated nested URLs. Record only timings,
status, sizes and verified Bloom metadata. Distinguish observed failures from
suspected causes. Preserve PGMENC01 header/AAD/checksum authentication.

Separate DAT total deadline from the OAuth request budget if the reproduction
confirms slow headers/download; bound idle body reads, size, cancellation and
one existing 401 recovery. Keep TLS checks and no redirects. Following measured
HTTP 504, bounded initial-GET 429/502/503/504 retries (default 2, max 5) use
exponential backoff/jitter within the same deadline. Honor bounded Retry-After;
never retry transient failures of multipart POST or individual partial-range requests; initial 401 recovery remains bounded to one. Expose safe phase
and deadline errors without including URLs, tokens or response bodies.

Successful Home login owns zeroizing client-secret/refresh credentials in RAM
for that source/profile/session. Login must still issue a valid token. Subsequent
fresh-job and refresh grants use this session without accessing Keychain. Keep
explicit CLI credential persistence working; never fall back from an active
session to stale saved credentials. Sign-out/profile changes drop the session.

After a job is prepared, failure must preserve/select its UUID on Jobs with
resume guidance. Only failures before preparation return the New Job draft.
Repeat input while busy cannot create another job. Existing user duplicates
are left for explicit user deletion. Preserve ordered output and per-job DAT
release, original keys, pinned resume and shared immutable Bloom workers.

Affected: matching auth/source/settings/UI and view/manual, auth/source/UI
regressions, generic interactive diagnostic example, README/ADR/runbook.
No dependencies. Additive settings use serde defaults; no job migration.

Scope extension authorized by user: configure both HTTP Range parallel download
and multipart/form-data requests in Settings. Use explicit modes: single GET,
parallel Range GET, multipart POST. Configure total/idle deadlines, part MiB,
parallelism and bounded non-secret text form fields. No file upload or custom
authentication headers. Range probe 0-0: require exact Content-Range, total under
limit, and strong ETag; send If-Match for every part and validate every response.
If probe returns 200, consume that response as a single stream (server ignores
Range), without another GET. Never concatenate inconsistent versions. Bound
in-flight parts, ordered assembly in locked RAM, one global deadline, cancellation
and no extra per-part auth retries. Multipart fields are external settings,
limited text metadata, never a place for secrets. No new dependency required.
herald/bastion/helmsman GO design: optional modes leave defaults compatible;
strict validator/range/body bounds protect against mixed snapshots and overflow.

## Ordered consultations (self-applied, no independent approvals)

architect GO: auth adapter owns session credentials; UI owns retry navigation.
curator GO: one prepared UUID per submission; a retry runs the existing UUID.
herald GO: OAuth JSON/provider DAT contracts unchanged; optional settings only.
migrator GO: default new limits on old settings; no credential/data backfill.
bastion GO design: no automatic Keychain calls in Home session, secrets zeroize;
profile bind and token validation remain mandatory, never log request errors raw.
sentry GO: distinguish headers/body/HTTP/decode; cancel then retry same job.
helmsman GO: bounded separate transfer budget, no blind retries or TLS downgrade.
beacon GO: safe stage/timing/byte diagnostics, no remote telemetry introduced.
scribe GO: document recovery and verification limits; no deployment claim.
Next: forge, refactory, sentinel, helmsman, bastion, beacon, inquisitor, scribe,
conductor. These consultations are local applications of specialist guidance.

## Verification and rollback

Regressions: no SecretStore calls through login/fresh job/refresh/sign-out;
rotation/invalid grant/profile isolation; failed creation vs failed DAT retry;
repeated Enter while busy; slow headers under independent DAT deadline; bounded
total/idle/cancellation/body errors; URL nesting rejected; no DAT on disk.
Run the authorized real endpoint through the same Service code and report
measured HTTP/decryption/matching results. Use synthetic identifiers/fixtures
for local tests, supplied identifiers only in isolated authorized live check.

Commands: `cargo test --offline --all-targets --all-features`,
`cargo clippy --offline --all-targets --all-features -- -D warnings`,
`cargo fmt --all -- --check`, `cargo audit --no-fetch`,
`gitleaks dir src --no-banner --redact`,
`cargo build --offline --release --bin pgm-data --bin pgm-bench`.
Record actual results. Full T4 publication gates (independent reviews, package
coverage/mutation floors, CI, signing and rollout) cannot be inferred from local
tests. No merge/release is requested or claimed.

Rollback: restore only affected source/docs from the task backup and rebuild.
Do not modify user jobs/settings/Keychain. Session loss requires another login.

## Final ledger

forge/refactory/sentinel/helmsman/bastion/beacon/inquisitor/scribe: local GO with
evidence and limits recorded in session-download-verification.md. conductor:
debug/release binaries built; not published. No independent sign-offs claimed.
228 full tests pass. Real single GET + GCM/Bloom check and raw-key probes passed;
real parallel mode reached 206 then stopped on provider 429. Multipart validated
with the local HTTP contract fixture, not asserted as a supported QA server API.
Publication gates remain unverified; no deployment/CI/production rollout implied.
Rollback of a newly saved config also requires backing up and removing the new
download_* fields before using an older deny_unknown_fields binary (ADR-0017).
