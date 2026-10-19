# AES-256-GCM DAT decryption and login key

Historical initial implementation: its nonce-first/empty-AAD transport is
superseded by ADR-0016 and the PGMENC01 contract. Prior test results below do
not establish compatibility with the actual PGMENC01 producer envelope.

2026-10-15. Classification T4: crypto/secrets boundary and external DAT decoder.
No deployment or producer changes requested. Checkout has no .git; preserve all
existing work and animation files. Backup pointer: `/tmp/pgm-aes-backup-path`.

## Goal, constraints and design

Add a masked Base64 AES-256 key to Home login. Decode exactly 32 bytes into
locked RAM; no key in Settings, Keychain, arguments, environment, logs, HTTP
requests or job metadata. Authenticate with the existing OAuth contract; only
install the session key after successful token issuance. Clear failed/cancelled
drafts and clear the session key on logout/profile change/normal exit.

Download ciphertext under existing timeout/size/cancellation limits. Authenticate
and decrypt AES-256-GCM completely before GZip/Bloom decoding or publication.
Use aws-lc-rs =1.18.2 (already a transitive TLS dependency, ISC/Apache-2.0) and
base64 =0.23.1 (already resolved, MIT/Apache-2.0) through safe public APIs.
No handwritten cryptography, CBC guessing, authentication bypass, plaintext
fallback after a key is supplied, new endpoint or per-key network lookup.

User confirmed GCM, Base64 encoding and binary 12-byte nonce + ciphertext +
16-byte tag with empty AAD. Implement exactly that contract, authenticate before
decoding, and reject other layouts rather than guessing crypto parameters.

One locked contiguous decryption buffer plus locked download chunks may coexist;
release/wipe compressed plaintext after Bloom decode. Preserve per-job Bloom
lifetime, exact plaintext fingerprint, joined workers and ordered output.
Keep the login key for the authenticated session to support each new job's fresh
token/DAT; discard each job's Bloom at its existing terminal boundary.
CLI must offer hidden interactive key entry for encrypted downloads, no key flag.
Existing explicit local DAT build/inspect and trusted library plaintext fixtures
remain available. Application encrypted sessions never silently downgrade.

Affected: Cargo.toml/lock; matching/{crypto,memory,mod,source,cli,ui}.rs;
matching/ui/view.rs; manual; login/crypto/HTTP tests; render example; README,
ADR-0015, threat model and runbook. Settings/job wire schema stays unchanged.

## Ordered specialist ledger (self-applied)

architect GO: decrypt at source boundary, key as private session value, Bloom
domain/worker implementation unchanged. curator GO: no unauthenticated bytes can
become a snapshot/false result; original order/keys/checkpoints stay invariant.
herald GO: user confirmed transport layout/AAD; additive login/session APIs and masking.
migrator GO: no secret persistence or destructive migration; old job pins remain
plaintext snapshot hashes, old metadata and explicit local DAT tools unchanged.
bastion GO key custody/authenticated primitive and confirmed nonce/tag/AAD;
no plaintext downgrade. STRIDE recorded in ADR/threat note.
sentry GO: wrong key, damaged tag, truncation, memory/lock failure and cancellation
fail closed; retry requires corrected key/source, no output publication.
helmsman GO local: existing bounded transfer and always-joined decode; buffer
peak documented; no production rollout. beacon GO: safe phase/success/error
status only, no key/nonce/body logging. scribe GO: plan and draft ADR first.
quartermaster GO local reuse: two already-resolved, cached crypto/encoding crates;
no extra crypto backend. Security audit, dependency graph/license check required.
Next: forge -> refactory -> sentinel -> helmsman -> bastion -> beacon ->
inquisitor -> scribe -> conductor; record actual outcomes after verification.

## Acceptance, verification and rollback

Test masked input/navigation/paste/compact layouts; missing/malformed/wrong-size
key; successful, failed and cancelled login; no key serialization/network leak;
correct AES-GCM known vectors, wrong key/nonce/tag/AAD, malformed/truncated body,
bounded mutation harness; raw/GZip native/provider DAT; per-job fresh downloads,
pause/resume and plaintext pin across random encryption nonces; no DAT on disk.
Maintain legacy matching tests. Measure memory/drop behavior and render 4 sizes.

Commands: `cargo fmt --all -- --check`;
`cargo test --offline --all-targets --all-features`;
`cargo clippy --offline --all-targets --all-features -- -D warnings`;
`cargo audit --no-fetch`; `gitleaks dir src --no-banner --redact`;
`cargo build --offline --release --bin pgm-data --bin pgm-bench`;
`cargo run --offline --example matching_review -- target/aes-verification/ui`.
Targeted crypto coverage and runtime failure evidence; publication gates remain
explicit (independent review, CI, mutation, signed release, production acceptance).

Rollback: stop the process, restore only task source/docs from the backup and
rebuild; preserve user jobs/settings. Do not downgrade encrypted downloads to
plaintext automatically or restore DAT disk caching. Original producer format
and token endpoints are outside this change.

## Final ledger

forge GO local: safe AES-GCM adapter, locked Base64 key, source pipeline,
profile-bound session ownership, masked login and hidden CLI prompt implemented.
refactory GO local: reuse existing TLS crypto/backend and buffer abstractions;
no second backend, key schema, new pool or plaintext downgrade in keyed sessions.
sentinel GO behavioral: 210 tests pass, independent vectors, mutation/truncation
and negative secret-lifecycle/HTTP cases. Crypto production instrumented branches
8/8 covered; whole-package T4 coverage/mutation publication gates not claimed.
helmsman GO local: existing transfer limits and joined decoder preserved;
contiguous AEAD staging peak documented; release CLI encrypted-download PTY passes.
bastion GO local checks: canonical 256-bit key, fixed full-tag framing, authenticate
before decoding, no key persistence/transmission/argv, profile/logout lifecycle,
safe APIs and cached advisory/secrets scans pass. Independent review unperformed.
beacon GO local: safe encrypted lifecycle attribute and visible validation/error
messages; no key/body/nonce in new logs. No hosted metrics/traces/alerts deployed.
inquisitor GO local functionality: no unresolved functional must-fix; T4
publication remains NO-GO pending full tier gates. scribe GO: plan, ADR, contract,
STRIDE, README, shared manual and runbook updated; no actual supplied key retained.
conductor: local release built, NOT SHIPPED; no .git/PR/CI/review/signing/deploy.
These are self-applied specialist checks, not independent approvals.

Exact commands, limits and DoD: `aes-dat-verification.md`.
