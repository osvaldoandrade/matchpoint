# PGMENC01 correction — local verification

2026-10-15. Tier T4 crypto-boundary fix.
[ADR-0016](adr/0016-pgmenc01-envelope.md), [plan/ledger](pgmenc01-plan.md),
[wire contract](contracts/encrypted-dat-gcm.md).
Receipts: `target/pgmenc01-verification/summary.json` and adjacent logs.

## Corrected behavior

The earlier implementation used a nonce at byte 0 and empty AAD. Independent
Python PGMENC01 fixtures reproduced its authentication failure (`before.log`).
The same test passes after the correction (`after.log`). The reader now:

1. Requires the 32-byte header with PGMENC01, big-endian version/algorithm=1,
   eight reserved zeros and checked plaintext length +92 equal to body length.
2. Verifies SHA-256 over all bytes preceding the final 32-byte checksum.
3. Opens AES-256-GCM with nonce [32..44], ciphertext/tag [44..len-32], and the
   exact header [0..32] as AAD. There is no empty-AAD/headerless retry.
4. Exposes plaintext to the existing raw/GZip Bloom decoder only after successful
   authentication. Failed framing/checksum/tag/key produces no filter/result.

The provider Bloom query already conforms to the supplied specification:
64-byte BE header, checked SHA-256 trailer, one digest of raw ASCII CPF input,
BE h1/h2, h2 OR 1, wrapping u64 multiply/add before modulo bits, LSB-first masks.
The query source is byte-for-byte unchanged; independent Python known positives,
negative controls and hash/probe vectors pass through decrypted provider data.
No CPF validation, automatic zero-padding or identifier normalization was added.
Raw 11-digit strings including zeros use SHA-256; pre-hashed hex uses None.

Login/key custody, job/pool code and animation files remain unchanged. Owned
download/key/decryption/Bloom buffers remain locked and zeroized on final drop;
the session key and per-job filter lifetimes retain their existing rules.
The extra SHA scan occurs once per download. No dependency or disk DAT cache
was added; Cargo.lock is unchanged.

## Commands and evidence

| Command | Result |
| --- | --- |
| `cargo test --offline --all-targets --all-features` | PASS: 214 tests, 0 failures |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | PASS |
| `cargo fmt --all -- --check` | PASS |
| `cargo build --offline --release --bin pgm-data --bin pgm-bench` | PASS |
| `cargo audit --no-fetch` | PASS using cached advisory DB |
| `gitleaks dir src --no-banner --redact` | PASS: no leaks |

Nine crypto tests include five independent Python AESGCM/struct/hashlib fixtures,
raw/GZip native/provider payloads, an authenticated empty envelope, invalid BE
header fields and reserved zeros, length mismatch/overflow, every truncation
and byte corruption, wrong key, empty/wrong AAD, obsolete headerless input and
1024 bounded malformed cases. Modified nonce/ciphertext/tag with recomputed
outer SHA still fails GCM. A separate HTTP regression confirms wrong header,
checksum and authenticated content cannot publish a snapshot or partial result.
All previous auth/resume/deletion/parallel-order tests remain green.

An owned release CLI PTY matched a synthetic raw-CPF CSV against a local HTTP
PGMENC01 provider envelope with SHA-256 and four workers. It produced exactly
true/false/true for known vectors, preserving the leading-zero key and duplicate
order, with one DAT GET, zero per-key HTTP calls, a plaintext fingerprint pin,
hidden key input and no downloaded DAT file. See `cli.json` / `cli.ansi`.
Seven protected animation-file hashes and login/query/pool source comparisons
match the task backup. No native Ghostty automation was used.

Focused coverage command:
`RUSTC_BOOTSTRAP=1 RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' CARGO_TARGET_DIR=target/aes-coverage LLVM_PROFILE_FILE='target/pgmenc01-verification/coverage-%p-%m.profraw' cargo test --offline --lib matching::crypto::tests`.
The clean crypto unit-test binary covers 20/20 instrumented production branch
outcomes. File totals including test bodies are 326/330 lines (98.79%) and
28/28 branch outcomes. This does not establish whole changed-package T4 floors,
mutation effectiveness, MC/DC, rare platform-allocation failures or race freedom.

## Scope, risk, rollback and DoD

Tests used synthetic public keys and temporary loopback state. The real endpoint
was not contacted, and the 169,768,248-byte provider file was not downloaded.
The supplied shortened whole-file/Bloom digests cannot identify that file fully;
they were not hardcoded or treated as validation evidence. Runtime verification
uses the full actual envelope trailer and authenticated plaintext Bloom trailer.
No claim of real-cohort membership certification or new speed benchmark is made.

Remaining memory limits are unchanged: HTTP/TLS/codec/crypto-library internals
are not all locked app mappings; privileged dumps, hibernation and application
wiping after abrupt termination are outside the guarantee.

- [x] T4 classification, ordered specialist review notes, plan and ADR.
- [x] Exact producer framing, SHA-256, header AAD and fail-closed decoding.
- [x] Reproduced-before/fixed-after regression, independent fixtures and HTTP/CLI.
- [x] Full tests/lint/build/audit, documentation and animation preservation.
- [ ] Whole-package T4 coverage/mutation and independent security review.
- [ ] PR/CI, signing, staging/canary/production acceptance.

No .git, PR, merge or deployment was created/requested. Local testing is ready;
this report does not assert publication. Rollback restores task source/docs from
`/tmp/pgm-pgmenc01-backup-path` and rebuilds without altering user jobs/settings.
The older decoder cannot read PGMENC01; never bypass AAD/SHA/header checks or
restore DAT disk caching to make rollback consume the encrypted source.
