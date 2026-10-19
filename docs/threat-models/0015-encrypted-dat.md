# AES DAT trust boundary — STRIDE delta

Historical initial implementation: its nonce-first/empty-AAD transport is
superseded by ADR-0016 and the PGMENC01 contract. Prior test results below do
not establish compatibility with the actual PGMENC01 producer envelope.

2026-10-15, local T4 implementation. Assets: login AES key, authenticated Bloom
bits, OAuth credentials, pinned checkpoints and original datasets. Untrusted
input: typed/pasted key, HTTP ciphertext, DAT/GZip metadata. Adversary: network
tampering, malformed source content or accidental wrong key/profile; same-user
or privileged process-memory access is outside this mapping-level protection.

| Category | Control / evidence |
| --- | --- |
| Spoofing | AES key syntax never substitutes for OAuth success. Login requires an issued Bearer token with positive expiry; failed/cancelled login installs no key. |
| Tampering | Safe AWS-LC AES-256-GCM AEAD verifies full tag before decode/publication. Fixed nonce/tag framing and empty AAD; wrong key/nonce/tag, truncation, unexpected AAD and plaintext fail. DAT checksums/pins still apply afterward. |
| Repudiation | Existing local lifecycle diagnostics include encrypted state and safe load/failure status. No key, payload or authentication body is logged. |
| Information disclosure | Masked bounded login/prompt input; canonical 32-byte key directly decoded into locked memory. No key in Settings/Keychain/jobs/argv/environment/HTTP. Secret draft strings use zeroization; session key and owned decryption buffer zeroize on final drop. |
| Denial of service | Existing byte/time limits, bounded transfer chunks, checked framing before slicing and memory-lock failure with no fallback. AEAD needs a contiguous buffer; peak includes transfer coalescing, compressed plaintext and decoded bits. Decoder is always joined. |
| Elevation of privilege | No new endpoint, subprocess, API credential grant or logged-in key editor. Key is bound to source profile; logout/profile change/normal exit releases ownership. |

Residual: HTTP/TLS/codec/crypto-library internal buffers and expanded key contexts
are library-managed memory, not all locked mappings. No claim of whole-process
confidentiality, hibernation protection, forensic erasure or application wiping
after SIGKILL/power failure. A syntactically valid wrong key is detected only at
DAT authentication. Replay/freshness beyond existing exact DAT pins is not added;
nonce uniqueness is the encryption producer's responsibility.

Validation: independent Python native/provider raw/GZip encrypted vectors, NIST
empty-message known answer, wrong key and byte-by-byte corruption/truncation,
large chunk boundaries/AAD mismatch, local HTTP/session lifecycle tests. No real
provided key or endpoint data is committed or used as a fixture. Independent
security review/signing/deployment gates are not claimed by self-review.
