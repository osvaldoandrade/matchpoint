# Interrupted DAT recovery verification

2026-10-15. Local T3 reliability repair, ADR-0018. No auth/crypto changes,
dependency changes, deployment, user-job deletion or credential/Keychain writes.

## Diagnosis and before/after evidence

Saved external settings selected Single GET with 600-second total deadline,
60-second idle deadline and 2 retries. The latest safe event showed HTTP 200
headers but no completed transfer. The reported message was generated only by
an error from the response body stream; the previous code discarded its cause
and never retried a failed body, even though initial HTTP status retries existed.
The exact cause of the user's original network interruption was not recorded.

A direct isolated QA request completed successfully in 17.558 seconds; the
natural interruption did not occur in that attempt. An independent local HTTP
server then declared a full Content-Length, sent 33,013 bytes and closed the
socket. The pre-fix test failed with the exact reported message:
`DAT body interrupted before completion.` The same regression passes after the
repair, reconstructing every byte from a validated suffix at the unaligned offset.
Before receipt: `target/body-recovery-verification/before.log`.

## Real DAT with deliberate network interruption

`examples/interrupt_download.py` forwards only the configured DAT path to its
existing HTTPS origin, with certificate verification. A temporary loopback route
provides fault injection to the same Rust Service. OAuth remains on its configured
HTTPS token URL. The proxy never logs headers/tokens/URLs/payloads and streams
bounded encrypted chunks without saving the DAT. Both temporary service/settings
directories are removed on exit. Client secret/AES key were entered only through
the owned no-echo terminal, not arguments/environment/files. User jobs/settings
were not changed.

Observed on the authorized QA endpoint:

- Token issued in 0.845 s; initial DAT HTTP 200 headers in 0.438 s.
- The proxy deliberately closed the stream after **1,048,576 bytes** of the
  declared **169,768,248-byte** DAT.
- Client classified `unexpected_eof`, retained the prefix in locked RAM and used
  retry 1 with 197 ms jittered backoff. Range/If-Match suffix recovery started at
  exactly byte **1,048,576**, validated HTTP 206/version/range/length, and completed.
- Wire complete in 10.204 s; AES-GCM/envelope/Bloom validation complete in 10.922 s.
- Verified Bloom fingerprint:
  `613d6b5ab243d972c08f9c85188c9508d33260f815b243cd97cc9ff9ec0ba62d`.
- The three supplied raw identifiers, SHA-256 at lookup, again returned
  `false`, `true`, `true` in supplied order. No identifiers were sent to the API.
- Proxy receipt: **2 GETs, 1 Range request, 1 injected cut**. Session/DAT were
  released after the probes. No further remote calls were needed.

This proves recovery from a deliberate interruption against the real encrypted
representation. It does not establish the cause of the original intermittent
network failure or promise that persistent gateway/rate-limit errors disappear.

## Verification

- `cargo test --offline --all-targets --all-features`: **238 passed**, no failures.
- `cargo test --offline --lib matching::manual`: 4 passed after the manual update.
- `cargo clippy --offline --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo audit --no-fetch`: passed with the locally available advisory database.
- `gitleaks dir src --no-banner --redact` and the same scan of `examples`: passed.
- `cargo build --offline --release --bin pgm-data --bin pgm-bench`: passed.
- `cargo build --offline --bin pgm-data`: passed for the existing launcher.

The 20 focused transport tests include 10 new recovery regressions: exact suffix
recovery, weak/missing-validator full restart, ignored-Range full replacement,
changed tag/range/total rejection, repeated breaks, shared HTTP/body/parallel retry
budget, 429/503 handling, no multipart body replay, idle recovery, total deadline,
cancellation/source-lock release, ordered parallel assembly, full encrypted
matching and corrupt recovered-envelope rejection without result publication.
No duplicate job is created. Existing session/no-Keychain, provider/pinned-resume,
matching order/duplicates/leading-zero and crypto validation tests remain green.

Focused LLVM branch instrumentation used the cached Rust 1.94 coverage target:
`RUSTC_BOOTSTRAP=1 RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch'
LLVM_PROFILE_FILE=... CARGO_TARGET_DIR=target/aes-coverage cargo test --offline
--lib matching::download`, followed by llvm-profdata merge and llvm-cov report.
Both selected production files exclude test bodies:

| Production file | Lines covered | Branches covered |
| --- | --- | --- |
| download.rs | 190/227 (83.70%) | 40/58 (68.97%) |
| download/recovery.rs | 239/270 (88.52%) | 49/66 (74.24%) |
| Combined focused run | 429/497 (86.32%) | 89/124 (71.77%) |

These are focused transport-test measurements, not whole-package coverage or
proof of T3 publication floors. Reports/profiles: `target/body-recovery-coverage`.
No mutation-score, independent security review, race-runtime, CI, signed release,
on-call approval or production canary is claimed. Local behavior is verified;
publication gates remain unfulfilled and no merge/release was requested.

## Final local ledger

forge GO: scoped transport recovery with static diagnostics; refactory GO:
extracted recovery and removed the old single-shot body reader; sentinel GO
behavioral: failing before/passing after, 238 tests plus real fault injection;
helmsman GO: shared bounded jitter/deadline budget, no POST replay;
bastion GO local review: exact strong validator/ranges, RAM-only prefix, failclosed
AEAD/checksum/publication, no credential movement; beacon GO: bytes/expected/static
cause/protocol/counters only; pathfinder GO local: one atomic budget, no new
workers/locks, scoped cancellation and concurrent failure test;
inquisitor GO local functionality, publication NO-GO for unverified tier gates;
scribe GO: claims distinguish natural failure, synthetic and real fault injection;
conductor: debug/release binaries built, not published. These are self-applied
consultations, not independent approvals.

All seven protected animation files and Cargo.toml/Cargo.lock remain unchanged.
The existing 20M CSV is still 240,000,004 bytes. Rollback restores only task
source/docs from `/tmp/pgm-body-recovery-backup-path` and rebuilds; preserve user
jobs/settings/exports/Keychain. No schema migration is necessary. Reopen the app
to use the new binary, select the existing prepared job and R to retry it.
