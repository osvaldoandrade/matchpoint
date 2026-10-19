# Large DAT body interruption — HTTP/1.1 verification

2026-10-16 UTC. The DAT client now uses HTTP/1.1 over verified TLS, isolated from
the unchanged OAuth client. The live Single GET received all 169,768,248 bytes
and authenticated/decoded the encrypted Bloom in 7.254s without recovery. This
mitigates the observed HTTP/2 body resets; it does not establish the provider's
internal root cause or prove a rate-limit policy. Scope: local client repair.

## Observed failure

The user's recent logs differ from the earlier 504-before-headers failures:

- 11:55:59 UTC: Single GET received HTTP 200 using HTTP/2.0, with byte ranges
  and ETag available. Body stream errors occurred at 3,796,313 and 18,175,616
  bytes, followed by exact offset resumes. The final stream failed at
  26,563,549/169,768,248 bytes after the two-retry budget was consumed.
- 12:13:12 UTC: Parallel GET received HTTP 206 over HTTP/2.0. Part bodies also
  failed with http2_stream_error and consumed the shared recovery budget.
- No HTTP 429 was recorded in these attempts. A slowdown/reset alone does not
  distinguish a gateway policy, HTTP/2 implementation/flow-control problem or
  another network interruption. No raw error chains, tokens or payloads were
  exposed to infer a more specific internal reset cause.

## Change and real API check

Only DAT client construction changed: HTTP/1.1, existing TLS verification,
redirect refusal, 10s connect bound, request timeout and total/idle budgets.
Bearer authorization, configured request modes, part size/parallelism, retry
limits, exact Range/strong-ETag checks, locked/wiped memory and GCM/checksum
validation remain intact. No new dependency, auth/check/schema/setting change.

The first isolated comparison failed during token acquisition, before any DAT
request; it provides no HTTP/1.1 download evidence. One subsequent fresh session
completed the following, using temporary non-secret settings, MemorySecrets,
verified no-echo credential input and no real user job or identifier probes:

| Observation | Measured outcome |
| --- | --- |
| Login token, 12:19:08.417 UTC | 0.488s; valid issued token |
| Fresh-job token, 12:19:08.869 UTC | Succeeded before DAT request |
| DAT headers, 12:19:09.315 UTC | HTTP/1.1 200; 0.446s; ranges and ETag available |
| Complete encrypted DAT | 169,768,248 bytes; 6.530s transfer, after fresh-job acquisition |
| GCM/envelope/Bloom validation | 7.254s total; success |
| Retry/parallel load during this check | Single GET, one stream, zero transient retries |
| Plaintext Bloom | 169,768,060 bit-array bytes; 70,846,959 entries; 13 hashes |

Verified plaintext fingerprint:
`613d6b5ab243d972c08f9c85188c9508d33260f815b243cd97cc9ff9ec0ba62d`.
The provider header declares FPP 0.0001; no Bloom parameter was modified.
The diagnostic skipped personal-key probes, released session/DAT memory and
exited successfully. Temporary metadata was removed. Saved settings and the one
prepared-job metadata file matched their before/after SHA-256 hashes.

## Local checks

| Command | Result |
| --- | --- |
| `cargo test --offline --lib matching::download` | 21 passed |
| `cargo test --offline --all-targets --all-features` | 246 passed; 0 failed |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo audit --no-fetch` | No vulnerabilities reported using cached advisory database |
| `gitleaks dir src --no-banner --redact` | No leaks found |
| `cargo build --offline --release --example matching_diagnose` | Passed; used for the live check |
| `cargo build --offline --release --bin pgm-data --bin pgm-bench` | Passed |
| `cargo build --offline --bin pgm-data` | Passed; launcher binary updated |

Existing transport tests cover ordered/bounded parts, exact suffixes and full
restart, changing validators/ranges, shared HTTP/body retry budget, 429 and
Retry-After, deadlines, idle stalls, cancellation and corrupt-envelope rejection.
Existing OAuth tests verify the actual received Bearer header and session/no-
Keychain behavior. No implementation-mirroring constructor test was added; the
real response protocol and complete authenticated transfer exercise the policy.
Logs: `/tmp/pgm-http-body-{transport-tests,all-tests,clippy,fmt,audit,secrets,diagnostic-build,release,debug}.log`.

## Local T3 Definition of Done and limits

- [x] Classification, ordered consultations, written plan and ADR-0021 before source changes.
- [x] Source-only backup and rollback path recorded.
- [x] Actual HTTP/2 body-failure evidence distinguished from earlier header 504 and unconfirmed rate limiting.
- [x] Scoped DAT-only HTTP/1.1 policy validated through a complete current real encrypted DAT.
- [x] Existing transport/auth/memory/crypto/job tests, lint, audit and local binaries verified.
- [x] Shared manual, README and recovery runbook updated; no animation/UI redesign.
- [x] User process, preferences, job, datasets, exports and credentials preserved; diagnostic metadata removed.

Actual remote Parallel mode under HTTP/1.1 was not load-tested; its assembly and
bounds are covered locally and the user's strategy/parallelism are unchanged.
No independent review, coverage/mutation floors, race-runtime, CI, signed release,
PR/merge, production SLO/canary or staging rollback drill is claimed. This is a
local build/functional verification, not publication approval. No new parser,
pool or migration requires a new fuzz/schema/concurrency rollout artifact.

All seven protected animation/font/palette files and Cargo.toml/Cargo.lock match
the source backup. Rollback restores only task source/docs from the location in
`/tmp/pgm-http-body-backup-path` and rebuilds debug/release. No state migration.
The existing user terminal was not restarted: reopen the app to load the new
binary, select the existing prepared job and press R to retry the same UUID.
