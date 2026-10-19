# Publish job results in the launch directory

2026-10-16. T2: local filesystem behavior, additive optional job metadata and
CLI/UI changes; under 800 lines and 20 files. No migration/backfill, DAT/auth,
crypto, concurrency, input/rule or dependency change. No Git checkout is present.

Problem: the prior change defaulted only explicit exports to ./ while New Job
still published its result under Application Support. New executable jobs must
publish their actual result to ./result-JOB_ID.txt and show its absolute path.

Architect GO: capture and persist the absolute primary destination when preparing
a UI/CLI job. Keep checkpoint/input storage under the existing app data root.
Use existing atomic no-overwrite publication, with a staged copy only across
filesystems. Preserve legacy jobs and low-level library constructors; absent
destination metadata resolves to the original internal result.txt.
Herald GO: existing CLI commands/signatures remain accepted; match --output
selects the primary result, not an additional copy. New optional output_path
metadata reads old jobs without rewriting them. Run/resume report actual output.
Scribe GO: plan and acceptance recorded before implementation. Forge GO design:
centralize destination/copy/publication helpers; UI reads the pinned job path.
Ordered remaining self-checks: sentinel, bastion, beacon, inquisitor, conductor.
These are self-applied checks, not independent review/publication acceptance.

Affected: matching jobs/cli/mod/ui/ui-view/manual, a small matching/output module,
README, synthetic review example and focused output integration/UI tests.
The Ghostty launcher will explicitly pass its already-selected project working
directory so macOS app launch cannot silently substitute the home directory.
Constraints: no real jobs, outputs, settings or secrets touched; animations,
fonts, palette, download/auth/crypto unchanged. No native app restart or API call.

Acceptance: CLI job create and UI New Job default to launch CWD, not data root;
absolute pinned path survives pause/restart and a different resumer CWD; match
--output supports explicit files/directories; complete result is never published
early or over an existing path. Checksums verify the actual completed file.
Legacy internal results remain readable. Explicit exports remain no-clobber;
exporting an already-published result to itself is a verified no-op. Deleting a
stored job leaves externally published output intact, as with existing exports.
Jobs renders the real destination across compact/wide layouts.

Verification: focused synthetic CLI-CWD/prepare/pause/resume/publication/checksum,
collision/recovery/legacy and UI tests, followed by
cargo test --offline --all-targets --all-features;
cargo clippy --offline --all-targets --all-features -- -D warnings;
cargo fmt --all -- --check;
cargo run --offline --example matching_review -- target/cwd-results-review;
cargo build --offline --release --bin pgm-data --bin pgm-bench;
cargo build --offline --bin pgm-data. Compare protected-file hashes.

Risk: an unwritable/removed destination can stop final publication; checkpoints
remain for retry and no completed file is silently redirected to app storage.
Rollback: restore files from /tmp/pgm-cwd-results-backup-path, remove only newly
added source/test files, and rebuild. Never delete/move user results or jobs.
An older executable does not understand new output_path jobs; use a repaired
current build for their resume. No production rollout or migration is requested.
