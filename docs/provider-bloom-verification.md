# Provider Bloom interoperability — local verification

2026-10-15, T4. [Plan](provider-bloom-plan.md), [ADR-0012](adr/0012-provider-bloom.md), [wire contract](contracts/provider-bloom-v1.md), [threat model](threat-models/0007-provider-bloom.md). Local patch/build only; no release or external data mutation.

User-authorized discovery: hidden credential prompts, one issuer POST and one authenticated DAT GET succeeded. Token stayed in RAM; no credentials were put in shell arguments/source/logs/settings/Keychain. The 169,768,156-byte response was raw PGMBLOOM, not GZip, with a different header, byte order, digest probes and trailing checksum. Actual bytes remained private outside the repository. No per-key API was used.

Implemented: explicit native/provider discriminator, raw/plain-GZip reading, bounded allocation and parameter checks, big-endian 64-byte provider header, SHA-256 trailer verification, exact LSB/wrapping digest probes, direct hex decoding or explicit raw-key hashing. Provider digests are fallible input; malformed data stops the job instead of producing false. Native encoder/golden files/fingerprints/strict transform checks remain compatible. Provider snapshots cannot be inserted into or re-encoded as native filters. Inspect reports wire_format; FPP display has sufficient decimal precision.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo test --offline --all-targets --all-features` | PASS, 178 tests |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | PASS |
| `cargo build --offline --release --bin pgm-data` | PASS |
| `cargo audit --no-fetch` | PASS against installed advisory DB |
| `gitleaks detect --no-git --redact --source src` | PASS |
| Independent Python synthetic raw/GZip vectors | PASS, native and provider formats |
| Provider bounded mutation fuzz harness | PASS, 4,000 structured mutations plus GZip/random inputs, 4 KiB decode budget |
| Protected animation/art/motion/splash/palette/runtime hashes | All seven unchanged |

New regressions: unknown version/algorithm/reserved fields, byte alignment versus native 64-bit alignment, impossible count/probe/FPP/size, memory limits, truncated header/payload/checksum, corrupt SHA/CRC, trailing bytes/members, immutable foreign writer, invalid digest text, uppercase digest encoding, explicit UTF-8 SHA transforms and trim. Raw/GZip source downloads verify/cache correctly; corrupt filters do not publish a new current pointer. Provider jobs preserve original keys/order/duplicates/boolean counts and resume from verified pins; bad keys never publish a completed result. Existing native matching/OAuth/UI/recovery tests remain green.

Private real-file release validation: bloom inspect verified the exact downloaded response, including its complete SHA-256 trailer and 169,768,060-byte bit array. A local streaming fixture served those bytes once to the real release executable; it matched the existing million-row synthetic hash CSV. All 1,000,000 output lines matched an independent Python mmap/struct verifier: 70,692 true, 929,308 false, original order/keys preserved. One local GET, zero per-key HTTP. Measured reference query 1.231 s; application download/decode/import/match 1.120 s in this local run, not a production SLO. The synthetic CSV was not modified.

Evidence limits: the probe algorithm was established by exact layout/checksum observations and differential query evidence, then independently implemented and compared. No producer source or separately labelled real-cohort positive/negative controls were supplied. Agreement is interoperability evidence, not a certified false-negative/FPP audit of the real cohort. The provider nominal p=0.0001 has estimated discrete p=0.0001001346028606502; its rounding fits the documented 1% relative tolerance and absolute 0.001 ceiling. No claim that true is confirmed membership/identity.

Artifacts: target/provider-verification/{tests,clippy,build,audit,secrets-scan}.log, real-filter-match.json and source/release hash receipt. Repository fixtures are synthetic only. Private response/cache/results and diagnostic helpers are task-owned temporary data outside the checkout; retain only safe aggregate evidence and remove unneeded temporary payloads after verification. No credentials, actual filter bits or individual results are copied into repository evidence.

Ordered self-review ledger: sentinel GO for measured behavior/vectors/fuzz smoke, algorithmist GO for bounded O(bytes) decode and O(key bytes+k) provider query without per-key allocations, helmsman/pathfinder GO for unchanged bounded network/worker/lock/cancellation, bastion GO for secret isolation/integrity/input validation, beacon GO for safe metadata/events, inquisitor GO for authorized local scope, scribe GO for current contract/ADR/README/recovery, conductor records no publication. These are self-applied reviews, not independent approvals.

Local DoD: [x] classification/plan/ADR/threat/contract, [x] actual-file compatibility, [x] independent synthetic/query evidence, [x] 178 tests/lint/build/scans, [x] million-row real-file comparison, [x] native compatibility/animation preservation/rollback. T4 publication gates remain unverified: coverage/mutation floors, long fuzz campaign, independent security review, CI/PR, signed SBOM, staging rollback, production ramp/observability and human sign-offs. No publishing is performed. Rollback restores only snapshotted task source/docs, preserving user settings/Keychain/jobs/native pins/CSV; provider-pinned jobs require the compatible reader and original snapshot. See ADR-0012.
