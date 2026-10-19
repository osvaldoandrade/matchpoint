# Resident DAT, deletion and parallel matching

Implement the three requested changes locally: retain downloaded Bloom snapshots
in locked anonymous RAM until each job ends, delete a selected job explicitly, and
match record batches in N CPU threads sharing one immutable Bloom. Preserve the
ordered original-key output and exact checkpoint/filter identity. The scope is
T4: worker pool on the full matching path and irreversible user-selected deletion.
No production service or user job is modified during implementation/verification.

## Placement, constraints and design

`matching/memory.rs` wraps safe anonymous memmap2 0.9.11 allocation + mlock;
allocation fails closed when pages cannot be locked, and Drop zeroizes before
unmapping. Keep `forbid(unsafe_code)`. Transfer uses bounded locked chunks;
`source.rs` publishes only an Arc in the Service's shared resident registry.
No DAT stage, disk cache or current.json is written. Each job owns its allocation;
all its threads share that one allocation. Pause retains that job's filter only.
Completion/failure/deletion releases it after workers join. A subsequent job
obtains a fresh OAuth token and download, even with an identical fingerprint.
Normal exit/sign-out drops remaining snapshots after workers join. Existing owned legacy
cache artifacts are retired under source.lock at session initialization.
No forensic erasure of old SSD sectors or protection against privileged process
dumps/hibernation is claimed. The operating-system mlock guarantee applies while
the mapping is live. CLI processes end their own memory session on exit.

`matching/parallel.rs` owns N persistent workers, one capacity-1 queue per worker
and a capacity-N result queue. The coordinator parses valid CSV/Parquet records,
dispatches bounded record batches and commits results in original order. It
never splits raw CSV at arbitrary newlines. All workers share Arc<Bloom> and
exit on channel close/error; all handles join before cleanup. No per-key Bloom
mutex. Thread count 1-256; default hardware parallelism capped at 32. Bound each
wave to 8192 records, including checkpoints and exact pause row budgets.

`jobs.rs` adds persisted thread count (legacy default 1), runtime override and
selected-job deletion. Use stable per-job locks outside the deleted directory,
also honoring legacy run.lock. Validate UUID/directory identity and refuse links
or running jobs. Rename to a private deletion stage before recursive removal;
never remove the original dataset or external export. UI requires an explicit
confirmation; CLI requires `job delete ID --yes`. Tests delete synthetic data only.

UI/CLI, Settings, New Job, Jobs and the shared manual expose threads, deletion
and resident state. Animations, URLs, credential checks and wire hashes stay
unchanged. New memory-only policy changes cross-process recovery: re-download
and verify the identical pinned fingerprint, otherwise refuse resume. Never
silently substitute the current filter for an old pin.

## Ordered specialist ledger (self-applied, not independent approvals)

architect GO: local matching context; concrete memory/pool/lifecycle adapters.
curator GO: ordered key/boolean semantics and snapshot identity remain invariants.
herald GO: additive thread/delete CLI APIs; document memory-only cache semantics
and old-job reads. migrator GO: read old metadata with 1 thread; no job backfill;
retire only recognized legacy cache files, fail closed on filesystem surprises.
bastion GO design: locked/wiped buffers, no new auth/crypto, no path-based deletion.
sentry GO design: cancellation/failed allocation/decode/thread/delete paths must
leave prior checkpoints valid. helmsman GO design: bounded buffers, join/drain,
local rollback; no deployment is requested. beacon GO design: safe lifecycle,
thread/row/time metrics and resident bytes; no identifiers/credentials in logs.
quartermaster GO: memmap2 0.9.11, MIT OR Apache-2.0, cached source, libc backend;
anonymous safe API avoids weakening unsafe-code prohibition. scribe GO: ADR-0014.
pathfinder/algorithmist GO design: bounded persistent CPU workers, immutable Arc,
single ordered writer; benchmark against 1 thread rather than assume speedup.
forge -> refactory -> sentinel -> helmsman -> bastion -> beacon -> inquisitor ->
scribe -> conductor will record implementation and validation evidence.

## Verification and rollback

Before/after exact-output comparisons at 1/2/N threads; quoted multiline CSV,
Parquet, original keys/duplicates/zeros, transform failure, row budget, restart,
out-of-order completion, worker cleanup, running/delete races, symlink traversal,
memory sharing/zeroization/lock failure, absent DAT artifacts, old metadata.
Commands: `cargo fmt --all -- --check`;
`cargo clippy --all-targets --all-features -- -D warnings`;
`cargo test --all-targets --all-features`;
`cargo audit --no-fetch`; `gitleaks dir src --no-banner --redact`;
`cargo build --release --bin pgm-data --bin pgm-bench`;
synthetic release benchmarks at 1/2/N threads and UI/CLI/PTY checks.
Attempt targeted coverage/sanitizer checks with the installed toolchains; report
unavailable/unmet publication gates explicitly. No CI, merge, production rollout
or independent review is implied by local evidence.

Restore only task source/docs from `/tmp/pgm-resident-parallel-backup-path` and
rebuild to roll back code. Do not roll back by restoring deleted user jobs or
inventing a missing filter: deletion is irreversible without a separate backup,
and a pinned job needs the exact DAT. Retiring the disk cache is intentional under
the requested privacy policy; never reintroduce a disk fallback on failure.

## Final implementation / verification ledger

forge GO local: locked buffers, per-job registry and FreshJob OAuth grant,
bounded joined CPU pool, scoped atomic deletion, CLI/UI integration implemented.
refactory GO local: old download stage/cache pointer and duplicate persistence
paths removed; wire hashes and ordered checkpoint contracts remain unchanged.
sentinel GO behavioral tests: 199 pass; NO-GO publication coverage/mutation gate
until measured floors are met. Clean unit-only coverage is partial evidence.
helmsman GO local performance/failure exercises: 2M-row sample shows about 20%
gain at 8 workers versus prior release. TSan compiled but the runtime and an
isolated assertion-only control both SIGSEGV before tests; no sanitizer pass.
bastion GO local checks: no unsafe project code, safe memmap2 API, denied-lock
fail closed, pointer sharing/lifetime, path/lock/link tests, cached DB audit and
redacted secrets scan. Independent security review is unperformed.
beacon GO local signals: existing structured matching lifecycle/bytes/FPP/rows/
duration plus thread count, resident UI and safe deletion event; no IDs/keys/
secrets in new logs. Hosted metrics/traces/alerts are not deployed or claimed.
inquisitor local review: no unresolved functional must-fix found; publication
remains NO-GO for unmet tier gates. Review fixed Unicode stage slicing,
constructor link side effects, recovery's unnecessary DAT request, authentication
cooldown bypass, and clipped compact deletion choices before final checks.
scribe GO: README, 18-chapter shared manual, recovery runbook, ADR and evidence
updated. conductor: local release binaries built; NOT SHIPPED; no .git/PR/CI,
independent approvals, signing or staged deployment was performed.

See `resident-parallel-verification.md` for exact commands, measured limits,
DoD checkboxes, rollback limits and artifact paths. These verdicts are
self-applied local review, never independent reviewer approvals.
