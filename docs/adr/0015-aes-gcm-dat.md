# ADR-0015: authenticate and decrypt DAT before decoding

Historical initial implementation: its nonce-first/empty-AAD transport is
superseded by ADR-0016 and the PGMENC01 contract. Prior test results below do
not establish compatibility with the actual PGMENC01 producer envelope.

Status: accepted for local implementation, 2026-10-15.
Tier T4, crypto/secrets boundary. [Plan](../aes-dat-plan.md).

The endpoint now returns AES-256-GCM ciphertext; the user supplies a Base64
256-bit key at login. Mode and key encoding are confirmed. The key is local
session material, distinct from OAuth client credentials, never sent to either
endpoint. The user confirmed nonce[12] || ciphertext || tag[16], with empty AAD. No endpoint URL or real key belongs in code or fixtures.

Put a concrete decryption adapter between bounded download and Bloom::decode.
Authenticate the full ciphertext before exposing compressed plaintext to the
decoder. Keep owned plaintext/key mappings locked and zeroized on final drop.
Existing HTTP/TLS/crypto-library internals can hold ordinary temporary buffers;
no whole-process confidentiality, forensic erasure, SIGKILL wipe or hibernation
protection is claimed. Sign-out/profile change/exit clears session ownership.

Reuse aws-lc-rs 1.18.2 already present for TLS and base64 0.23.1 already in the
lockfile. Rust's standard library has no AEAD. Rejected: handwritten GCM,
shell/OpenSSL process pipelines (secret/buffer exposure), another crypto backend
without a demonstrated need. Use safe APIs with the project unsafe-code ban.
Library owner: matching maintainers; review/update with the existing TLS backend.

Keep old local native/provider plaintext codecs and job metadata unchanged.
Per-job pins identify authenticated plaintext Bloom bytes, not randomized
ciphertext. A supplied session key requires authenticated ciphertext; no fallback
to plaintext or unauthenticated mode is permitted. Login still requires an issued
Bearer token and positive expiry, not merely a valid AES key.

Roll back task source and rebuild without touching stored user jobs. Old code
cannot read encrypted downloads; rollback requires fixing/versioning the source,
never silent downgrade. No production deployment, merge or reviewer approval is
part of local implementation. Verification/DoD will distinguish local checks from
unmet publication gates.

Sources: [AWS-LC AEAD](https://docs.rs/aws-lc-rs/1.18.2/aws_lc_rs/aead/index.html),
[Base64 decoding](https://docs.rs/base64/0.23.1/base64/engine/trait.Engine.html).
