# MATCHPOINT manual verification

The local application now shows one MATCHPOINT frame title with the headline
Ultra-fast dataset matching. F1 opens a contextual chapter in a 15-chapter
English manual. Chapters, less-style scrolling, whole-manual literal search,
highlighted matches and q/Esc return work without executing job commands or
losing form drafts. `manual` and `manual --roff` print the same content before
opening application storage. Existing command help now describes its options.

## Reproduce the local checks

| Command / evidence | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --all-targets --all-features -- -D warnings` | Pass |
| `cargo test --all-targets --all-features` | 184 passed, 0 failed |
| `cargo build --release --bin pgm-data --bin pgm-bench` | Pass |
| `cargo audit --no-fetch` | Pass against the local advisory database |
| `gitleaks dir src --no-banner --redact` | No leaks |
| `cargo run --example matching_review -- target/manual-review` | Four sizes: 40x18, 60x22, 80x24, 120x36 |
| `pgm-data manual --roff` then `mandoc -Tlint` | Exit 0, no diagnostics |
| All 25 manual/help CLI paths with an unusable `PGM_DATA_HOME` | Pass; no storage required |
| Owned PTY release-binary smoke | Login, F1, search, q return, Ctrl+C exit 0, alternate screen restored |
| Seven protected animation/art/runtime hashes | All unchanged |

Source-based instrumentation uses Rust 1.94 with the installed LLVM tools:
`RUSTC_BOOTSTRAP=1 RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch'
CARGO_TARGET_DIR=target/manual-coverage
LLVM_PROFILE_FILE='target/manual-verification/clean-%p-%m.profraw'
cargo test --offline --lib`, followed by `llvm-profdata merge` and
`llvm-cov export` on that library-test executable. The clean single-binary
report has no profile mismatch warnings: the new manual module covers 226/228
lines (99.12%), 55/56 branch outcomes (98.21%) and 14/14 functions.
This is a module result, not a whole-application coverage claim or mutation score.

Synthetic render artifacts live in `target/manual-review/`; command output,
PTY evidence and coverage JSON live in `target/manual-verification/`.
The PNG render uses the bundled VGA font; it is TestBackend evidence, not native
Ghostty acceptance. The review also checked the compact search and chapter end.
Matching format/provider reader, source handling, jobs and dependency lock hashes
match the prior verified baseline; no user datasets or credentials were changed.

## Workflow and delivery status

T2 was promoted to T3 after the measured documentation/test delta exceeded 800
lines. ADR-0013 records placement, compatibility, threat delta and rollback.
Self-applied consults in order: architect, curator, herald, bastion, scribe,
helmsman, beacon, forge, sentinel, helmsman, bastion, beacon, inquisitor, scribe,
conductor. All give GO for the evidenced local handoff; this is not independent
approval. Migrator/refactory and network load/contract rollout gates do not
apply: no migration, existing-code replacement, network or matching-path change.
T-6 telemetry exception: static local documentation needs no operational logs,
metrics or traces; search strings are never logged. No feature flag is added
for the immediate local presentation change the user requested.

Local DoD: [x] classification/plan/ADR, [x] identity/manual/navigation,
[x] tests/lint/build/CLI/man/PTY, [x] bounds/resize/draft checks,
[x] animation and data-path preservation, [x] task-only backup rollback path.
Publication DoD remains unverified: package-wide coverage floors, mutation
campaign, independent reviews, CI/PR/merge, staging rollback and canary sign-off.
The directory has no Git repository and this task publishes nothing.
Use the task backup identified by `/tmp/pgm-manual-backup-path` to restore only
the affected source/docs and rebuild. Settings, Keychain, jobs and datasets
remain in their existing namespace. No tickets or remote changes were created.
