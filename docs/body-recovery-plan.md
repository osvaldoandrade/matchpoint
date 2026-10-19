# Interrupted DAT body recovery

2026-10-15, T3: external read/transport recovery, shared retry budget and bounded
concurrency. No auth/crypto/check changes, new dependency, settings migration,
UI/animation redesign, user-job deletion, secret persistence or service rollout.
Backup pointer `/tmp/pgm-body-recovery-backup-path`.

The reported error is emitted only by a failed response-body stream. The current
GET retry logic covers transient response statuses, not interrupted bodies.
Read-only settings confirm Single GET, DAT deadline 600 s, idle 60 s, 2 retries.
The latest event shows HTTP 200 headers and no completed dat_received event.
The exact transport cause remains unknown because it was erased. Reproduce via
the existing isolated hidden-input diagnostic; classify causes safely without
logging raw reqwest chains, request headers, tokens, URLs or body content.

Decision before source edits: keep an interrupted GET prefix only in locked RAM.
When total length and a strong ETag are known, recover with exact Range and
If-Match, checking 206, ETag, Content-Range/total and body length before append.
Without a safe validator/length, discard the prefix and restart the GET. If a
server returns a full 200 instead of a single-download recovery range, discard
the prefix before consuming the new full response. A strict parallel part may
never consume an unexpected full response. Changed version/range fails closed.
Multipart POST bodies are not automatically replayed. AEAD/checksum/parse must
still succeed after the complete transfer; no partial bytes reach matching.

Use the existing download_retries as one shared atomic budget for initial HTTP
errors, part HTTP errors and body recovery. This prevents N independent retry
loops from amplifying requests. Keep existing parallelism and one total deadline;
body-idle recovery is permitted only within those bounds. No pool/task/lock added.
Cancellation drops all scoped futures and locked prefixes. Safe diagnostics show
received/expected byte counts and static cause codes, never raw error strings.

Affected: matching/download.rs plus a focused recovery module/tests; shared manual,
README, runbook, ADR-0018 and verification report. No wire/settings/job schema change.

## Ordered consultations (self-applied)

architect GO: transport adapter only; retain Service/auth/crypto/domain boundaries.
curator GO: one UUID, complete verified DAT only, no cross-version concatenation.
herald GO: existing GET/Range/If-Match contracts, no new endpoint or public schema.
migrator GO: no persistence changes; interrupted bytes never written to disk.
bastion GO design: strong-validator resume, full restart without one, no plaintext
downgrade/partial publication, keep memory lock/wipe and redaction guarantees.
scribe GO: ADR/plan/runbook before implementation, distinguish local/server proof.
helmsman GO: shared attempt budget, jitter, no POST replay, bounded deadline/idle.
beacon GO: static error class, transfer offsets/expected sizes and retry counters.
pathfinder GO: AtomicU8 budget needed by existing concurrent part futures; no
lock across await, no spawned recovery workers, same bounded lifetime/cancellation.
Next: forge -> refactory -> sentinel -> helmsman -> bastion -> beacon -> inquisitor
-> scribe -> conductor. No independent reviews or deployment approvals implied.

## Acceptance and verification

An independent HTTP server closes after a non-aligned prefix, then serves the
requested suffix. The old client must fail; the fixed client must reconstruct the
exact original bytes and pass full encrypted Bloom/matching validation. Cover
multiple breaks, restart without strong ETag, changed ETag/total/range, ignored
Range, budget exhaustion, concurrent failures, 429/5xx with Retry-After, idle and
global deadline, cancellation, and POST no replay. Confirm no disk DAT and no
duplicate jobs. Preserve all existing provider/crypto/order/pin tests.

Commands: cargo test --offline --all-targets --all-features;
cargo clippy --offline --all-targets --all-features -- -D warnings;
cargo fmt --all -- --check; cargo audit --no-fetch;
gitleaks dir src --no-banner --redact;
cargo build --offline --release --bin pgm-data --bin pgm-bench;
cargo build --offline --bin pgm-data.
Record focused coverage and actual endpoint observations where feasible; no
unmeasured package/mutation/independent review/CI/production claims.

Rollback: restore only task source/docs from backup and rebuild. Settings, jobs,
exports, credentials, animations and user datasets remain unchanged. The prior
binary still fails on an interrupted stream; it must never use partial DAT data.

## Final ledger

Implementation, fault injection, 238-test run, local reviews and unfulfilled
publication gates are recorded in body-recovery-verification.md. The old client
fails the independent cut-stream test; the fixed client passes. Actual QA DAT
recovery after a deliberately injected 1-MiB cut completed authentication and
decoding in 10.922 seconds using two GETs, one carrying the exact suffix range.
Default source/settings/user jobs and all animation fingerprints were preserved.
