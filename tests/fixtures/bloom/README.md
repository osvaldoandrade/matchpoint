Independent Python stdlib vectors. Seed bytes 00..0f; m=4096, k=16, capacity=4, inserted=4, target=0.001; no normalization. Keys: `00012345678`, `partner-abc`, `a:b`, `ç`. Files 0/1/2 use identity/SHA-256 lowercase hex/SHA-256 raw. Generated with hashlib, struct and gzip independently of the Rust codec.

Provider vectors: provider-v1.dat and provider-v1.dat.gz contain the same synthetic keys hashed with Python hashlib.sha256. m=19176 (byte aligned, not a multiple of 64), k=13, capacity=1000, inserted=4, nominal FPP=0.0001. Python struct packs >8sIIIIQQQdQ (64 bytes); LSB-first bit array; SHA-256(header+bits) trailer. Query halves are big-endian, h2 odd, wrapping u64 probes. provider-v1-vectors.json records independently generated digests, probe positions and negative controls. No actual provider data is included.

PGMENC01 vectors: `aes-gcm-vectors.json` and `*-gcm*.enc` are produced independently
by `generate_encrypted_vectors.py` using Python cryptography AESGCM + struct/hashlib.
Header fields are BE, AAD is the full 32-byte header, nonce follows at offset 32,
and an outer SHA-256 trailer closes each envelope. Metadata includes trailer and
whole-file digests separately. The public synthetic key is not a live credential.
The empty vector tests authenticated framing only; it is not a valid Bloom.
