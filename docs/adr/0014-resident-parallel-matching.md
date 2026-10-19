# ADR-0014: locked resident snapshots and ordered parallel matching

Status: accepted for requested local implementation, 2026-10-15. Tier T4.

## Context and decision

The user requires the downloaded DAT to remain in memory, explicit job deletion,
and N-thread CSV matching against the same Bloom. The current reader persists
the download and reloads it per run; the matcher is sequential. Replace the disk
cache with a per-job resident registry of immutable Arc<Bloom> snapshots in
anonymous page-locked memory. Lock before filling; zeroize before release. Use
memmap2 0.9.11's safe map_anon/lock APIs, MIT OR Apache-2.0, without unsafe code
in this project. Supported Unix kernels must accept mlock or loading fails.
The user clarified lifetime: release on each job completion, with fresh token
acquisition and download for the next job. Pause retains only that job's mapping;
completion/failure/deletion and normal shutdown wipe after all its workers join.
Never deduplicate allocations across different jobs, even for identical DAT bits.

Parse CSV/Parquet records with the existing parser, distribute bounded waves to
persistent CPU workers and commit completed chunks in input order. No worker
copies the Bloom. Preserve byte/row positions and checksums at commit boundaries.
Thread count changes scheduling only, never lookup semantics or order.

Expose deletion by opaque job ID with explicit UI/CLI confirmation. Stable locks
outside the directory prevent deletion/run races; retain legacy run.lock checks.
Stage a validated owned directory with an atomic rename, then remove its files.
Reject symlinks and mismatched metadata. Do not delete original inputs/exports.

## Alternatives and consequences

An ordinary Vec stays logically live but may swap; a file-backed mmap persists
the DAT; mlockall locks unrelated application memory. Anonymous, individually
locked mappings constrain the guarantee and fail visibly on resource limits.
Raw byte-range CSV splitting mishandles quoted newlines. Unbounded completion
queues or one full Bloom copy per worker violate the memory goal. Serial mode
remains selectable with 1 thread as a baseline and troubleshooting path.

Memory-only cache semantics intentionally supersede disk recovery. Old job
metadata reads with 1 thread. Session initialization retires only recognized
owned legacy DAT/current artifacts; user dataset/job files do not migrate.
After process exit, resume re-downloads and checks the exact old fingerprint;
a changed endpoint snapshot causes a visible refusal, never mixed results.
The default thread count uses available CPU parallelism capped at 32; limits
and explicit 1-256 overrides bound oversubscription and queued record memory.

## Threat and failure model

Assets: filter bits, original inputs, output and pinned checkpoints. Prevent
disk persistence/swap of application DAT buffers, path traversal, deletion of
foreign data, concurrent deletion of a running job and corrupt partial output.
Fail closed on page-lock/allocation, checksum, parser, pool or pin errors. Join
workers before releasing snapshots; zeroize mappings on normal drop/unwind.
No claim covers privileged dumps, hibernation images, forensic erasure of old
SSD blocks or running both old/new binaries concurrently during cache retirement.
No new endpoint, credential movement or hash algorithm is introduced.
The locked guarantee covers the decoded bit array and owned transfer staging;
HTTP/TLS/codec temporary buffers are not all locked mappings. This is not a
whole-process memory-confidentiality mechanism.

## Rollback and delivery

Task-only source backup and verification commands are in
`docs/resident-parallel-plan.md`. Stop new work and restore the prior binary on
functional regression; retained job input/output metadata remains compatible.
Do not restore a disk cache as an implicit fallback. Deleted job contents need
an independently owned backup to recover. Implementation/testing is local;
production, CI, independent security review and staged-release gates are separate.

Primary references: [memmap2 anonymous mappings and lock](https://docs.rs/memmap2/0.9.11/memmap2/struct.MmapMut.html),
[Apple mlock contract](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/mlock.2.html).
