# PGMBLOOM DAT v1

A DAT file is one GZip envelope (no optional filename/comment/extra header fields) containing exactly a 128-byte header and its bit array. All integers are unsigned little-endian. Unknown versions/algorithms/reserved values, malformed lengths, extra uncompressed data, invalid CRC or checksum are errors, never empty filters.

| Offset | Bytes | Meaning |
|---|---:|---|
| 0 | 8 | ASCII `PGMBLOOM` |
| 8 | 2 | version = 1 |
| 10 | 2 | header bytes = 128 |
| 12 | 4 | hash algorithm = 1 |
| 16 | 8 | bit count m, nonzero multiple of 64 |
| 24 | 4 | probe count k, 1..32 |
| 28 | 4 | key transform: 0 identity UTF-8, 1 SHA-256 lowercase hex ASCII, 2 SHA-256 raw 32 bytes |
| 32 | 8 | expected capacity, positive |
| 40 | 8 | inserted rows, at most capacity (duplicates count conservatively) |
| 48 | 8 | target FPP, IEEE-754 f64 little-endian; default 0.001 |
| 56 | 16 | seed (UUID random bytes for production builders) |
| 72 | 8 | payload byte count = m / 8 |
| 80 | 32 | SHA-256(header[0..80] + header[112..128] + payload) |
| 112 | 4 | extraction normalization flags: bit 0 trim, other bits zero |
| 116 | 12 | reserved, all zero |

Hash algorithm 1: compute SHA-256 over ASCII `PGMBLOOM/v1` followed by the 16 seed bytes followed by transformed key bytes. h1 is digest[0..8] little-endian; h2 is digest[8..16] little-endian OR 1. For i=0..k-1 probe `(h1 + i*h2) mod m`, using exact unsigned arithmetic (no u64 wrap; u128 suffices). Bit j is payload[j/8] bit `(j mod 8)`, least significant bit first. Membership is true only if every probed bit is set. Consumers must use the exact producer transformation; mismatch aborts matching.

Builder sizing uses `m = ceil(-n ln(p)/ln(2)^2)` rounded up to 64 bits, a safety margin for the target, and a rounded optimal probe count; integer parameters must satisfy the estimated FPP bound before export. Capacity overflow fails instead of silently exceeding the configured target. The default 0.001 is a probabilistic design target, not an exact membership guarantee; inserted keys must never produce false negatives with the same rule/snapshot.

CSV rules independently select a header name or a one-based `#index`, header presence, one ASCII delimiter and optional trimming. No implicit CPF validation, punctuation removal, numeric conversion or lowercasing occurs. Filter metadata pins identity/SHA-256 representation; the trim flag is embedded in the header and must match on both producer and consumer.

Output is UTF-8 without a header: `original_key: true` or `original_key: false`, one record per input row in original order, including duplicates. If a key contains control characters, a colon, backslash, quote or edge whitespace, serialize the key as a JSON string before `: boolean`; empty keys are errors. The output uses the original dataset key, not its transformed digest. Publish the final result only after complete matching/checksum verification; partial files/checkpoints are resumable, not completed output.

Sources: [Bloom semantics and serialization](https://guava.dev/releases/33.7.2-jre/api/docs/com/google/common/hash/BloomFilter.html), [Kirsch and Mitzenmacher](https://www.eecs.harvard.edu/~michaelm/postscripts/esa2006a.pdf), [flate2](https://docs.rs/flate2/1.1.10/flate2/).

The builder uses p=0.0008 for sizing against the declared 0.001 target, a minimum of 4,096 bits to avoid tiny-filter probe correlations, and optimal rounded k clamped to 1..32. Parquet uses exact top-level column names or #indexes, independent of CSV header/delimiter options; supported keys are UTF-8 strings and signed/unsigned integers in canonical decimal representation. Null/nested/floating-point keys are errors.
