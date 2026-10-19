# Correct PGMENC01 transport decoding

2026-10-15. Tier T4: correction to authenticated crypto checks and external wire
contract. Owner: matching maintainers. Scope: local implementation/verification;
no deployment, service change, credential rotation, user dataset/job migration
or animation/login redesign. Source backup: `/tmp/pgm-pgmenc01-backup-path`.

## Decision before implementation

The earlier nonce-first/empty-AAD assumption is incompatible with the user's
producer specification. Replace that decoder, without fallback, with:
32-byte PGMENC01 header, 12-byte nonce, ciphertext with 16-byte GCM tag, 32-byte
SHA-256 trailer. Integers are big-endian. Require magic, version=1, algorithm=1,
reserved zeros and checked plaintext_length+92 equal to actual body length.
Verify SHA-256 over all bytes excluding the trailer before AEAD. Pass the exact
32 header bytes as AAD; decrypt only [44..len-32]. Expose plaintext only after
authentication; keep the current locked/wiped buffer and joined decoder.

The plaintext PGMBLOOM reader already matches the supplied contract: 64-byte
big-endian header/checksum, digest h1/h2 big-endian, h2|1, wrapping uint64 probes,
LSB-first bit indexing. Characterize it with an independent producer/reference
and raw 11-digit strings preserving zeros; do not introduce CPF validation or
change generic identifier rules. SHA-256 transform hashes raw input once.

Keep masked Base64 login key, RAM-only session custody, per-job fresh token/DAT,
pin identity based on plaintext Bloom, pause/resume, ordered multithread output
and explicit local raw/GZip build/inspect. GZip remains optional plaintext codec,
not an extra layer in the PGMENC01 envelope. No new dependency or network path.
Never hardcode a single file length or truncated example checksum prefix.

Files: matching/crypto.rs; independent fixtures/generator under tests/fixtures;
HTTP tests as needed; README/shared manual/contract/runbook/threat note; ADR-0016
superseding the incorrect transport part of ADR-0015. Historical verification
reports get supersession notices, not rewritten historical results.

## Ordered specialist consultations (self-applied)

architect GO: boundary adapter only, retain matching domain/worker layout.
curator GO: authenticate before parse/publication, preserve keys/order/pins.
herald GO: supplied producer bytes are authoritative; replace incompatible
assumption, reject old framing; preserve public Rust/login/job schemas.
migrator GO: no persistence migration; old completed jobs remain readable and
pins still identify plaintext; no dual-write/cache path to introduce.
bastion GO design: fixed validated framing, checked addition, public SHA before
AEAD, header AAD, full tag, no partial plaintext/fallback/key logging.
sentry GO: short/wrong header/size/checksum/tag/key fails closed; preserve
checkpoints, correct source/key then retry; no blind crypto-parameter guessing.
helmsman GO: bounded existing transfer plus joined in-memory decrypt/decode;
extra checksum scan is once per download; no pool or unbounded allocation added.
beacon GO: existing safe encrypted load/errors, no body/key/nonce diagnostics.
scribe GO: record this correction explicitly in ADR/contract and recovery docs.
Next: forge -> refactory -> sentinel -> helmsman -> bastion -> beacon ->
inquisitor -> scribe -> conductor. No independent reviewer sign-off implied.

## Verification and rollback

Before/after regression using independent Python AESGCM envelope fixture.
Test raw/GZip native/provider payloads; prefix/BE integers/reserved zeros,
length boundaries/overflow, outer checksum, header as AAD, wrong key/nonce/tag,
same-length header changes with recomputed SHA, empty-AAD ciphertext wrapped in
a valid envelope, truncation/extra bytes, obsolete headerless framing and bounded
malformed-input mutation. Confirm matching against Python probe vectors including
uint64 wrap and LSB order; HTTP keyed download/resume/no disk cache stay covered.

Commands: `cargo test --offline --all-targets --all-features`;
`cargo clippy --offline --all-targets --all-features -- -D warnings`;
`cargo fmt --all -- --check`; `cargo audit --no-fetch`;
`gitleaks dir src --no-banner --redact`;
`cargo build --offline --release --bin pgm-data --bin pgm-bench`.
Measure focused crypto branch coverage; do not claim whole-package mutation,
independent security review, CI/signing/deployment or actual endpoint acceptance
without evidence. Tests use synthetic keys/local fixtures, not the supplied key.

Rollback: restore only task source/docs from backup and rebuild, preserving user
jobs/settings. The old decoder cannot consume PGMENC01; rollback does not justify
dropping AAD/checksum/header validation or restoring DAT disk caching.

## Final specialist ledger

forge GO local: validated fixed envelope before safe AES-GCM open with header AAD.
refactory GO: replaced the incompatible framing, no guessing/downgrade path;
login/key custody, Bloom query and worker pool are byte-for-byte unchanged.
sentinel GO behavioral: before-test fails and after-test passes; 214 full tests,
independent Python vectors, 1024 malformed cases, recalculated SHA/AAD attacks,
HTTP no-publication and release CLI checks. 20/20 crypto production branch
outcomes covered; whole-package T4/mutation/MC-DC publication gates unverified.
helmsman GO local: transfer limits and joined decoder unchanged; one additional
bounded SHA pass, no new pool, no real-service load/production claim.
bastion GO local review/audit/secrets checks: fixed magic/version/algorithm,
checked sizes, full outer SHA, exact header AAD/full GCM tag before any exposure,
no new dependency or credential movement. Independent security review unperformed.
beacon GO local: existing safe load/error diagnostics cover distinct header,
size, checksum and authentication failures; no bytes/key/nonce logged.
inquisitor GO local functionality: no unresolved functional must-fix identified;
publication remains NO-GO for unverified tier gates. scribe GO: current contract,
README/manual/runbook and supersession notices distinguish prior tests from
producer-compatible evidence. conductor: release built locally, NOT SHIPPED;
no PR, CI, independent approvals, signing or rollout asserted.

These are self-applied consultations, not independent reviewer approvals.
Exact results, scope and final DoD: `pgmenc01-verification.md`.
