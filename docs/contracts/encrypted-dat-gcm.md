# PGMENC01 encrypted DAT transport

Producer specification supplied by the user on 2026-10-15; supersedes the earlier
headerless/empty-AAD assumption. All integers are unsigned big-endian; offsets
are bytes from the start. No real key, endpoint URL or file-specific size/hash
prefix is compiled into the reader.

| Offset | Length | Content |
| --- | --- | --- |
| 0 | 32 | PGMENC01 header (also the GCM AAD) |
| 32 | 12 | GCM nonce/IV |
| 44 | declared plaintext length | Ciphertext |
| 44 + plaintext length | 16 | GCM authentication tag |
| body length - 32 | 32 | SHA-256 of every preceding byte |

| Header offset | Length | Value |
| --- | --- | --- |
| 0 | 8 | ASCII PGMENC01 |
| 8 | 4 | Version 1 |
| 12 | 4 | Algorithm 1: AES-256-GCM |
| 16 | 8 | Serialized plaintext Bloom length |
| 24 | 8 | Zero reserved bytes |

Validation and opening order:

1. Require at least 92 bytes, valid magic/version/algorithm/reserved fields,
   and checked declared_plaintext_length + 92 equal to the actual body length.
2. Compare SHA256(body[0..len-32]) with body[len-32..len]. This public checksum
   detects corruption; it does not replace authenticated encryption. The SHA-256
   of the *entire* file is a different digest and is not this trailer.
3. Open AES-256-GCM with decoded key[32], nonce body[32..44], ciphertext plus tag
   body[44..len-32], and AAD equal to the exact header body[0..32].
4. On any failure, expose no plaintext and publish no snapshot/result. Never
   retry empty AAD or the obsolete headerless layout as a fallback.
5. After successful authentication, validate the existing plaintext DAT header,
   bit array, SHA-256 trailer and configured memory limit. Native/provider raw
   or GZip plaintext codecs remain supported; the supplied provider Bloom is raw.

The whole HTTP body is binary. Only the manually entered key is padded standard
Base64 (44 characters encoding 32 bytes). No password derivation, CBC, variable
nonce/tag lengths or algorithm guessing. The producer must generate a unique
nonce for each encryption under a key. The key remains local session input,
never an HTTP field/header or persistent setting.

Plain provider Bloom: [PGMBLOOM v1](provider-bloom-v1.md), 64-byte BE header,
bits/8 payload, SHA256(header + bits) trailer. For an 11-digit raw CPF preserving
zeros, select SHA-256: hash its ASCII bytes once. h1=digest[0..8] BE;
h2=digest[8..16] BE OR 1; probe=((h1+i*h2) mod 2^64) mod bits. Check
payload[probe >> 3] & (1 << (probe & 7)); LSB-first. All k bits must be set for a
possible match. Already-hashed 64-hex input uses transformation None.

Synthetic independent vectors and full expected digests:
`tests/fixtures/bloom/aes-gcm-vectors.json`. Regenerate with
`python3 tests/fixtures/bloom/generate_encrypted_vectors.py` (Python cryptography
required for fixture generation only). These public test keys/nonces must never
be used operationally; no real provider bits or supplied key are in the fixtures.
