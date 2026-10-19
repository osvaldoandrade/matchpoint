# Authenticated download, session and retry verification

2026-10-15. Local repair, tier T4 (session-secret custody). No service deployment,
credential rotation, Keychain ACL change, Git commit/PR or independent approval.
See ADR-0017, session-download-plan.md and threat-models/0017-session-download.md.

## Reproduction against the authorized QA API

The malformed download URL pasted in chat contains a duplicated origin. Existing
external settings already referenced the correct consumer-filter path. No QA URL,
client secret or AES key was added to source, fixtures, process arguments or env.
Credentials were supplied through an owned no-echo terminal into an isolated
Service using MemorySecrets. Tokens/DAT remained in process memory, and the
temporary service directory was removed on exit. No user jobs were created.

- Unauthenticated HEAD: HTTP 401, TLS 0.298 s, total 0.426 s. Basic reachability
  worked; this alone did not establish authenticated service health.
- Original client: token issued in 0.480 s; DAT failed after 60.485 s while waiting
  for response headers. OAuth and the large DAT shared a 60-second total timeout.
- An additional request with a longer client deadline returned HTTP 504 after
  60.888 s. That is a distinct provider/gateway timeout, not a GCM failure.
- Successful final single GET: fresh token 0.543 s; DAT headers HTTP 200 in 0.601 s;
  wire 169,768,248 bytes in 15.985 s, authenticated/decoded completion in 16.788 s.
  PGMENC01 framing, outer checksum, AES-256-GCM header AAD and provider Bloom SHA
  all passed. Bit array: 169,768,060 bytes, 70,846,959 entries, 13 hashes;
  declared FPP 0.0001. Plaintext Bloom fingerprint:
  `613d6b5ab243d972c08f9c85188c9508d33260f815b243cd97cc9ff9ec0ba62d`.
- Supplied raw identifiers, SHA-256 at lookup, in supplied order:
  `false`, `true`, `true`. No identifiers were transmitted to the API.
- Actual parallel check: initial GET/probe received 502 and 504, exercised both
  bounded jittered retries, then HTTP 206 in 111.778 s. A subsequent part returned
  HTTP 429; processing aborted at 112.607 s without publishing any snapshot.
  Strong validator/range acceptance was reached, but a full parallel download
  against QA was NOT completed. Default remains Single GET. No further calls
  were made after 429. Multipart POST interoperability was not claimed for QA;
  its request contract is covered by local HTTP tests and needs server support.

The repair cannot change provider/gateway rate limits or deadlines. Persistent
429/502/504 remains an external condition. The program now distinguishes the
phase/status and preserves the prepared job for R retry instead of duplicating it.

## Local checks

- `cargo test --offline --all-targets --all-features`: **228 passed**, no failures.
- `cargo clippy --offline --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo audit --no-fetch`: passed against the locally available advisory DB,
  393 locked dependencies; no dependency changes.
- `gitleaks dir src --no-banner --redact` and the same scan of `examples`: passed.
- `cargo build --offline --release --bin pgm-data --bin pgm-bench`: passed.
  Debug launcher binary is also built with `cargo build --offline --bin pgm-data`.

Regression evidence covers no SecretStore get/put/delete during Home login,
fresh-job tokens, initial DAT 401 recovery, refresh rotation, invalid_grant
fallback, missing refresh and sign-out. Explicit CLI credential tests still pass.
Authentication still requires a valid access token, Bearer type and positive
expiry. Default single-GET credential profile fingerprints remain compatible.

UI regressions verify one UUID through repeated failed DAT attempts, repeat input
while busy, New Job draft recovery only before preparation, and API settings mode,
metadata and numeric-limit persistence. Four terminal sizes verify focus/scroll
access to all new fields. TestBackend SVG/text artifacts are under
`target/session-download-review`, including parallel and multipart previews;
80x24 parallel and 40x18 multipart renders were visually inspected. Native
Ghostty acceptance was not performed. All seven protected animation files retain
their original fingerprints. The existing 20M CSV remains 240,000,004 bytes.

Transport tests verify ordered nonidentical parts, 1-MiB boundaries, bounded
parallelism, strong/weak/missing ETags, changed versions, exact ranges and body
lengths, 200 probe fallback without another GET, multipart framing/text metadata,
transient retries/exhaustion/disabled mode, Retry-After, no transient POST replay,
separate header/total/idle failures, interrupted/oversized bodies, cancellation
and source-lock release, URL mistakes/injection, legacy timeout defaults and
locked-page assembly rejection. A 4096-case deterministic mutation harness covers
range-probe parsing/overflow. Existing crypto/provider/pinned-resume/order/output
and DAT memory-release tests remain green.

## Final local review and delivery limits

Ordered self-applied consultations: forge GO implementation; refactory GO old
generic transfer removed and retry ownership clarified; sentinel GO functional
regressions; helmsman GO bounded waits/retries and cancellation; bastion GO local
custody/framing/audit review; beacon GO safe phase/status/size/timing events;
inquisitor GO local functionality; scribe GO contract/runbook/claim discipline;
conductor local binaries built, NOT PUBLISHED. These are not independent reviews.

T4 publication gates remain unclaimed: package branch/MC-DC and mutation-score
floors, independent security review, CI, signed SBOM/attestation, on-call approval
and staged production rollout. No production merge/release was requested.
Provider 429/5xx behavior is measured above, not declared fixed server-side.

Rollback: restore only changed source/docs from the task backup and rebuild.
After saving new settings with this version, prior binaries reject download_*
fields; back up settings.json and remove only those new fields if rolling back.
Keep original max_download_mib, URLs/profile values, user jobs, exports and
Keychain. Sign in again after restart. Never roll back AEAD/checksum validation,
enable DAT disk caching, or delete existing duplicate jobs automatically.
