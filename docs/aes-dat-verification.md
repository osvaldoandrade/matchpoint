# AES DAT — local verification

Historical initial implementation: its nonce-first/empty-AAD transport is
superseded by ADR-0016 and the PGMENC01 contract. Prior test results below do
not establish compatibility with the actual PGMENC01 producer envelope.

2026-10-15. Tier T4, crypto/secrets boundary.
[Plan](aes-dat-plan.md), [ADR-0015](adr/0015-aes-gcm-dat.md),
[wire contract](contracts/encrypted-dat-gcm.md),
[STRIDE](threat-models/0015-encrypted-dat.md).
Raw receipts: `target/aes-verification/`.

## Result

Home has four fields: Client ID, masked Client secret, Token URL and masked DAT
key (Base64). Tab/Enter advances through the first three; Enter on the key or
Sign in submits. The key must encode exactly 32 bytes. Local validation happens
before a token request; only successful OAuth installs the key. A syntactically
valid wrong key is detected when the DAT is authenticated, not during OAuth.

The source accepts nonce[12] || ciphertext || tag[16], AES-256-GCM with empty AAD,
as confirmed by the user. The entire response authenticates before raw/GZip
native/provider Bloom decoding. Wrong key/tamper/truncation and plaintext
downgrade fail without publishing a filter/result. The existing plaintext DAT
checksum/pin, memory limit, ordered output and worker lifecycle remain intact.
Fresh nonces for the same plaintext preserve the same job pin across resume.

The decoded key stays in locked session RAM. It is not in Settings, Keychain,
jobs, logs, arguments, environment or HTTP. It survives between jobs to decrypt
each new download; the Bloom still releases per job. Logout/profile change/exit
releases the key; secret drafts are cleared on submit/failure/cancellation.
CLI download/match/run/resume offers a hidden interactive key prompt. Key flags
are rejected before opening storage, without echoing their values. Explicit
local build/inspect and trusted library no-key plaintext use remain supported;
authenticated application sessions never fall back to plaintext.

## Checks

| Command | Result |
| --- | --- |
| `cargo test --offline --all-targets --all-features` | PASS: 210 tests, 0 failures |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | PASS |
| `cargo fmt --all -- --check` | PASS |
| `cargo build --offline --release --bin pgm-data --bin pgm-bench` | PASS |
| `cargo audit --no-fetch` | PASS: cached DB, 393 dependencies |
| `gitleaks dir src --no-banner --redact` | PASS: no leaks |
| `cargo run --offline --example matching_review -- target/aes-verification/ui` | PASS: four terminal sizes, including the new masked field |

Six crypto-focused tests cover canonical key size/encoding, NIST empty-message
known answer, independent Python AESGCM vectors for native/provider raw/GZip,
wrong key and all-byte mutation/truncation, unexpected AAD, >64KiB chunk
boundaries, profile binding and key-owner release. Two HTTP integration tests
cover encrypted local matching, key exclusion from requests/settings, rotated
nonces on resume, new token/download calls and no files/results on wrong key.
Login tests exercise missing/malformed key, masking, paste bounds, success,
failure, cancellation, storage errors, navigation and logout. Existing matching,
Parquet, auth, deletion and parallel-order tests continue to pass.

An owned PTY exercised CLI hidden key input and a synthetic encrypted loopback
download through the release binary: exit 0, one GET, no key echo/storage, no DAT
cache. TestBackend snapshots cover 40x18, 60x22, 80x24 and 120x36. Login at 80x24
and key focus at 40x18 were visually inspected as PNGs. The seven animation-file
hashes match the task backup. Native Ghostty was not automated.

Crypto coverage command:
`RUSTC_BOOTSTRAP=1 RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' CARGO_TARGET_DIR=target/aes-coverage LLVM_PROFILE_FILE='target/aes-verification/coverage-%p-%m.profraw' cargo test --offline --lib matching::crypto::tests`.
The clean single-binary report has 188/192 lines (97.92%) and 12/12 branches
(100%) for crypto.rs **including test bodies**. Filtering branch locations to
production code leaves 8/8 instrumented branch outcomes covered. This does not
prove whole changed-package T4 coverage, platform allocation failures, mutex
poisoning or mutation effectiveness. Full publication coverage/mutation gates
remain unmet/unverified. No fresh sanitizer pass is claimed.

aws-lc-rs 1.18.2 and base64 0.23.1 were already in Cargo.lock; promoting them
to direct dependencies added **zero package versions**. Licenses were checked
from their cached manifests: ISC AND (Apache-2.0 OR ISC), and MIT OR Apache-2.0.
The project retains forbid(unsafe_code) and uses safe AEAD APIs, with no shell
crypto pipeline. Independent security review, signed SBOM/release and CI are
not claimed by these local checks.

## Limits, rollback and DoD

Owned key/transfer/decryption/Bloom mappings lock before use and zeroize on final
drop. Transfer coalescing temporarily needs both chunked and contiguous buffers;
decode needs compressed plaintext plus decoded Bloom. Existing configured byte
limits and OS lock failure apply. HTTP/TLS/codec/crypto-library internal buffers
and expanded key contexts are library-managed, not all locked app mappings.
No whole-process memory confidentiality, privileged-dump/hibernation protection,
forensic erasure or application wiping after SIGKILL/power loss is promised.

Tests use only public synthetic keys/vectors and temporary loopback state. The
actual key supplied in chat was not written to code/files or used as a fixture.
No real endpoint, production credential, user job or original dataset was used.
No performance gain for encryption or real endpoint interoperability is claimed.

- [x] T4 classification, ordered specialist notes, plan, ADR and threat model.
- [x] Required hidden login key and authenticated in-memory DAT decryption.
- [x] Negative tests, independent vectors, HTTP/PTY/UI evidence, lint and release.
- [x] English shared manual (19 chapters), README, contract and recovery runbook.
- [ ] Full changed-package T4 coverage/mutation, independent security review.
- [ ] CI, reviewers, signed SBOM/release, staging/canary/production acceptance.

Local testing is available. This checkout has no .git; no commit/PR, merge or
deployment was requested/performed. It is not a published or production-approved
release. Rollback restores task source from `/tmp/pgm-aes-backup-path` and rebuilds;
preserve user jobs/settings. Old code cannot decode encrypted source bytes, so
rollback must not silently downgrade transport or restore a downloaded DAT cache.
