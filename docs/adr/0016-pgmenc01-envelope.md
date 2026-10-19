# ADR-0016: consume the producer's PGMENC01 envelope

Accepted for local correction, 2026-10-15. T4 crypto boundary.
Supersedes the nonce-first/empty-AAD transport assumption in ADR-0015; session
key custody and all plaintext Bloom semantics remain in force.

The user's complete producer contract adds a 32-byte header and 32-byte SHA-256
trailer, shifts the nonce to bytes 32..43, and authenticates the header as GCM
AAD. The earlier implementation cannot authenticate this body. Use the supplied
format exactly, not heuristic parsing or retrying multiple AAD choices.

Require PGMENC01, BE version/algorithm=1, zero reserved bytes, checked declared
plaintext length +92 matching the body, and SHA-256(body without trailer) equal
to the trailer. Then open ciphertext/tag at [44..len-32] using nonce [32..44]
and AAD [0..32]. A checksum is a framing/corruption check, not a substitute for
GCM authentication. Expose zero plaintext on any failure.

Keep safe AWS-LC AEAD calls and locked/zeroized key/plaintext mappings. Existing
PGMBLOOM header, checksum, SHA-256 digest/uint64-wrapping probes and LSB-first bit
order already satisfy the producer rule. Raw identifiers use SHA-256 exactly
once; existing hex identifiers use None. Pins refer to plaintext Bloom checksum.

Rejected alternatives: keep empty AAD (incompatible), accept either framing or
guess parameters (ambiguous downgrade), hardcode observed size/checksum prefix
(breaks legitimate refreshed filters). No new dependency, endpoint or persistent
secret/data artifact. New tests/fixtures are synthetic and independently built.

Risk: one additional checksum pass and existing contiguous-buffer memory peak.
Bad framing/checksum/key/tag stops download before snapshot/result publication.
Rollback source only; do not change source encryption or silently weaken checks.
Exact test commands, specialist ledger and scope: [plan](../pgmenc01-plan.md).
No PR, CI, independent review, signed release or production rollout is asserted.
