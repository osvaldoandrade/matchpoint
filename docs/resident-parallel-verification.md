# Resident DAT, job deletion and parallel matching — local verification

2026-10-15. Local implementation of the clarified per-job lifetime, tier T4.
Decision: [ADR-0014](adr/0014-resident-parallel-matching.md).
Plan and specialist ledger: [plan](resident-parallel-plan.md).
Artifacts: `target/resident-verification/summary.json` and adjacent logs.
No real API, user job, production credential, native Ghostty session or deployment
was used for these checks. The existing animations are unchanged (7/7 hashes).

## Delivered behavior

- Downloads have no DAT/partial/current cache on disk. Transfer staging and the
  decoded bit array use locked anonymous mappings; allocation fails closed when
  page locking is denied. The final owner zeroizes before unmapping.
- Pause retains the same job's Bloom in the running process. Completion, failure,
  deletion, sign-out and normal exit release it after workers join. Every next
  OAuth job obtains fresh client_credentials and a new DAT. An unused preload is
  claimed once. Refresh/cooldown and bounded 401 recovery remain intact.
- After restart the job re-downloads and verifies its exact pinned fingerprint.
  A changed DAT refuses resume without changing the committed output prefix.
  An already published, checksum-valid result recovers without any DAT request.
- 1–256 persistent matching threads share one immutable Bloom allocation.
  Default: available CPU parallelism capped at 32; legacy jobs read as 1.
  CSV/Parquet parsing and ordered writing remain on the coordinator. Waves have
  at most 8,192 real records; quoted multiline CSV is never split by raw newline.
- New Job and Settings > Rules expose threads; Jobs has T / X / Delete.
  Deletion defaults to Cancel. CLI: `job delete ID --yes` and
  `job threads ID --threads N`; match/run/resume also accept `--threads`.
  Stable and legacy locks reject active-job deletion. Atomic staging scopes
  cleanup to the selected stored job; originals/external exports survive.
  Recognized interrupted stages retry on launch; links and malformed IDs fail.

## Verification commands and results

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --offline --all-targets --all-features` | PASS: 199 tests, 0 failures |
| `cargo build --offline --release --bin pgm-data --bin pgm-bench` | PASS |
| `cargo audit --no-fetch` | PASS using local advisory DB, 393 dependencies |
| `gitleaks dir src --no-banner --redact` | PASS: no leaks |
| `cargo run --offline --example matching_review -- target/resident-verification/ui` | PASS: 4 terminal sizes, including threads/delete screens |

Behavioral evidence includes exact outputs at N=1/2/7/32/256, all three native
transformations, quoted semicolon/multiline records, duplicates, whitespace and
leading zeros; Parquet codecs/batch boundaries; checkpoint/prefix recovery;
worker join and shared Arc lifetime; error cleanup; denied mlock in an isolated
process; paused deletion; CLI confirmation/bounds; stable/legacy lock conflicts;
directory and child symlinks; legacy metadata/cache retirement; and interrupted
cleanup including arbitrary UTF-8 filenames. OAuth loopback tests prove fresh
token + DAT on each new job, no requests on same-process paused resume, and
fresh verified downloads on restart.

TestBackend renders cover 40x18, 60x22, 80x24 and 120x36; PNGs for deletion and
Jobs were visually inspected. An owned PTY exercised Home login, F1, manual
search, q return, Ctrl+C exit 0 and alternate-screen restoration. This does not
claim native Ghostty acceptance. The existing 20M CPF fixture remains 240000004
bytes with its three supplied CPFs first; it was not rewritten.

## Release performance sample

Synthetic 2,000,000-row CSV, native Bloom, no extra key transformation, 10%
inserted keys. Separate sequential runs of the previous release benchmark and
the new release; no competing CPU benchmark. This is one sample per mode, not a
statistical latency guarantee. Preparation and post-run verification are excluded
from matching time. All runs checked row count, checksums, zero false negatives
and zero HTTP requests; Bloom false positives remain possible.

| Mode | Matching seconds | Rows/second |
| --- | ---: | ---: |
| Previous sequential release | 0.780 | 2,564,284 |
| New, 1 worker | 0.796 | 2,511,597 |
| New, 4 workers | 0.712 | 2,808,202 |
| New, 8 workers | 0.649 | 3,080,197 |

Eight workers improved this sample by about 20% versus the previous release,
and 23% versus the new one-worker mode. One worker incurred about 2% overhead.
Parsing/writing are still serial; threads do not promise linear acceleration.
Raw measurements and private synthetic artifacts are under `bench/`.

## Measurement limits and publication gates

Coverage was measured with
`RUSTC_BOOTSTRAP=1 RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' CARGO_TARGET_DIR=target/resident-coverage LLVM_PROFILE_FILE='target/resident-verification/coverage-%p-%m.profraw' cargo test --offline --lib --test resident_parallel --test matching_auth`.
All selected tests passed. A separate clean unit-only profile avoids combining
incompatible cfg(test) mappings. Its memory module measured 91.67% lines / 90%
branches; parallel measured 92.37% / 75%. These are partial unit-only figures,
not a passed changed-package or T4 coverage floor. The full publication coverage
and mutation gates are **unmet/unverified**, not waived.

`RUSTFLAGS='-Zsanitizer=thread' CARGO_TARGET_DIR=target/resident-tsan cargo +nightly test --offline -Zbuild-std --target aarch64-apple-darwin --lib matching::parallel::tests`
built successfully, but the sanitizer process SIGSEGVs before running tests,
including `--list`. An isolated crate with only `assert_eq!(2+2,4)` also
SIGSEGVs under the same toolchain/runtime. Thus no successful sanitizer/race-free
claim is made. See `tsan.log`, `tsan-control.log`, `tsan-status.json`.

The memory guarantee covers owned mappings while the process is running.
HTTP/TLS/codec temporary buffers are not all locked mappings. Privileged dumps,
hibernation, forensic SSD erasure and application-level wiping after SIGKILL or
power loss are outside its scope. The kernel releases mappings on termination.
First-launch legacy cache removal is intentional; stop older caching binaries
before migration. Resume after restart needs the source to retain the exact DAT.

## DoD and delivery scope

- [x] Classification, design, invariants, ADR and rollback direction recorded.
- [x] Local requested functionality, focused failure tests, lint, build and audit.
- [x] English UI, CLI, shared 18-chapter manual, README and recovery runbook.
- [x] Local CPU performance and terminal evidence; animation preservation.
- [ ] T4 coverage/mutation floors and a working sanitizer environment.
- [ ] Independent security/two-person ADR review, signed SBOM and CI review.
- [ ] PR, merge, release signing, canary/production rollout or recovery drill.

This checkout has no `.git`; no commit/PR, merge or deployment was requested or
performed. Local testing is available. The remaining boxes are publication
follow-ups, and this report does not assert that the change is shipped.

Rollback: stop work, restore only task source files from the recorded source
backup and rebuild. Keep job inputs/results/metadata. Do not restore downloaded
disk caches as a fallback. Deleted jobs require an independent backup to recover.
