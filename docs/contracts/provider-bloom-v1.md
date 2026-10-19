# Provider PGMBLOOM v1

This reader supports the binary format observed at the user-authorized provider, separate from [native PGMBLOOM](bloom-dat-v1.md). The shared magic/version label does not imply identical bytes or membership probes. No provider URL or credentials are compiled into the application.

Inside the authenticated [PGMENC01 transport](encrypted-dat-gcm.md), the plaintext is raw bytes or the existing plain single-member GZip envelope. The decoded content is exactly a 64-byte header, m/8 bytes of bits and a 32-byte SHA-256 trailer. No optional GZip headers, extra members, trailing bytes or failed CRC/checksum are accepted. All integers are unsigned big-endian; the FPP is a big-endian IEEE-754 f64.

| Offset | Bytes | Meaning |
|---|---:|---|
| 0 | 8 | ASCII PGMBLOOM |
| 8 | 4 | version 1 |
| 12 | 4 | digest-probe algorithm 1 |
| 16 | 4 | k, 1..32 probes |
| 20 | 4 | reserved, zero |
| 24 | 8 | m bits, positive multiple of 8 |
| 32 | 8 | positive capacity |
| 40 | 8 | inserted count, at most capacity |
| 48 | 8 | nominal target FPP, finite and 0 < p <= 0.001 |
| 56 | 8 | reserved, zero |
| 64 | m/8 | LSB-first bit array |
| 64 + m/8 | 32 | SHA-256(header + bit array) |

Validate parameters and configured memory budget before allocating one bit array. k is integer-rounded: permit at most 1% relative difference between the estimated design FPP and nominal target, while retaining the absolute application ceiling 0.001. Native parameters retain their previous strict validation. Unknown versions, algorithms and reserved values fail closed.

Query: obtain a 32-byte SHA-256 digest. For already-hashed CSV/Parquet keys select None: require exactly 64 ASCII hexadecimal characters and decode them; letter case does not change digest bytes. For original identifiers, SHA-256 hex or SHA-256 raw computes the digest once from the selected UTF-8 key. Trim is explicit. The original key remains unchanged in output. Invalid digest text or an empty normalized key is an error, never a false result.

h1 = digest[0..8] interpreted big-endian; h2 = digest[8..16] big-endian OR 1. For i=0..k-1, bit = ((h1 + i*h2) mod 2^64) mod m. Test payload[bit/8] bit (bit mod 8), least significant bit first. Every probed bit must be set. The u64 wrap is significant and differs from native u128 arithmetic/seed/domain hashing. Do not rehash digest text or reinterpret native bits using provider probes.

Fingerprint: the verified trailer hash identifies the decoded provider header/payload. Keep the verified Bloom in per-job locked RAM under that identity, without a downloaded DAT disk cache; pinned jobs retain their original extraction/transformation rule. The native encoder cannot insert into or rewrite provider snapshots; keep the provider's original DAT.

Evidence: exact downloaded length/header/trailer SHA verification, independently implemented query comparison over the synthetic million-row file, and independent Python-generated synthetic raw/GZip vectors. The user subsequently supplied the producer header/probe specification, confirming the BE uint64 wrap and LSB-first rules. No externally labelled real-cohort membership controls or certified false-negative statistics are claimed. No actual provider bits/keys are included in repository fixtures.

Rust query API is fallible: contains(key) returns Result<bool, Error>; contains_with_rule(key, rule) applies explicit job preprocessing. Application jobs propagate errors and do not publish completed false results for malformed input. Native producer, golden vectors and historical pinned fingerprints remain compatible.
