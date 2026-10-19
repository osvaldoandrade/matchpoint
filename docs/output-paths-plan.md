# Absolute result paths and current-directory exports

2026-10-16. Tier T2: local UI/CLI output behavior, multiple files, additive
optional export flag; no schema/auth/crypto/network/dependency/concurrency change.

Problem: Jobs prints a fictional relative matching/jobs/UUID/result.txt instead
of the real data-root path. Export starts blank and success omits its destination.
CLI output echoes a relative --output unchanged. The user requests absolute
display and ./ as the default destination.

Decision: keep the durable job result in its existing data root, report its
actual absolute path and use ./result-JOB_ID.txt as the default external export.
This avoids replacing job storage or moving user results. TUI E prefills that
destination resolved against the current directory. CLI match and job export
without --output export there; an explicit file overrides it, and an existing
directory such as ./ receives the same generated filename. Display successful
destinations as absolute paths. Existing outputs are still never overwritten.
No automatic TUI export before E/Enter; the job remains durable if export fails.

Ordered self-applied design consultations:
architect GO: keep path resolution at matching CLI/export boundary and use the
actual Service root in UI; result_path becomes absolute without changing root
identity or credential-profile hashing. No stored Job/settings path field.
herald GO: --output stays accepted and explicit destinations retain behavior;
job export --output becomes optional, with documented CWD/UUID default. Existing
copy_result signature and job wire format stay compatible. Additive CLI contract.
scribe GO: this plan, constraints and acceptance precede source changes.
forge GO design: small shared export-target resolver, concrete Path/PathBuf;
UUID-based names prevent collisions across jobs. Preserve no-overwrite publication.

Affected: matching cli.rs, jobs.rs, ui.rs, ui/view.rs, manual.rs; matching tests,
synthetic matching render example, README and this verification record.
Constraints: all product copy English; no animations/palette/font changes; no
real job/config/result modifications by tests, no API calls or secret access.
Render full absolute result path before secondary help/chart copy, including
compact completed-job details. Error/completion feedback must retain destination.

Acceptance: actual root, not a guessed prefix; relative roots/explicit outputs
yield absolute paths; default TUI/CLI exports resolve from ./; explicit directory
and filenames with spaces work; duplicate export never overwrites; complete-only
and checksum checks remain; existing job and external exports are preserved.

Verification: focused local export/CLI/UI regressions using tiny synthetic jobs;
cargo test --offline --all-targets --all-features;
cargo clippy --offline --all-targets --all-features -- -D warnings;
cargo fmt --all -- --check;
cargo run --offline --example matching_review -- target/output-paths-review;
cargo build --offline --release --bin pgm-data --bin pgm-bench;
cargo build --offline --bin pgm-data. Check protected/dependency hashes and inspect
synthetic Jobs/Export renders at 40x18, 60x22, 80x24 and 120x36.

Rollback: restore only task source/docs from /tmp/pgm-output-paths-backup-path
and rebuild. Do not delete/relocate user jobs or exports. No schema migration or
deployment is needed. CI, PR/merge/reviewer/publication gates remain outside this
authorized local task; no independent approval or coverage floor is claimed.

## Ordered local execution ledger

forge GO: shared concrete PathBuf resolver; absolute result lookup preserves
Service.root identity; optional CLI --output uses a '.' default without changing
the existing public field type. Durable storage/publication remains intact.
sentinel GO functional: three new behavioral regressions; 249 tests pass. Actual
CLI child cwd, file bytes, collision refusal and four-size complete paths verified.
bastion GO local: UUID-generated defaults, existing no-overwrite/complete/checksum
guards, no real state writes or credentials/API calls. No auth/profile changes.
beacon GO local UX: truthful absolute Result/Output/Exported receipts; no new
network telemetry or claims about operational service metrics.
inquisitor GO local: helpers shared between CLI/UI, real root instead of guessed
prefix, narrow complete paths prioritized, relative-root identity pinned by test.
scribe GO: README/manual and output-paths-verification.md describe default CWD,
directory override, durability and observed local checks. conductor GO local
debug/release builds only; no repository metadata, CI or publication requested.
These consultations are self-applied, not independent reviews or merge approval.
