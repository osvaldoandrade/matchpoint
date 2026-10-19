# Primary results in the launch directory

2026-10-16. T2 local fix; design: cwd-results-plan.md. New executable jobs pin
their actual primary result to ./result-JOB_ID.txt at preparation. They publish
there atomically and show its absolute path. match --output overrides that
primary destination. Application storage retains input/checkpoints; old jobs
without output_path retain their original internal result location. No backfill
or real user-state changes were performed.

The Ghostty launcher now passes --working-directory explicitly. Installed
Ghostty 1.2.3 documentation confirms macOS open launches otherwise default to
home (Contents/Resources/man/man5/ghostty.5, working-directory). This launcher
selects the project directory. Animation/font/palette settings are unchanged.

| Command / check | Result |
| --- | --- |
| `cargo test --offline --test matching_outputs` | 4 passed |
| `cargo test --offline --all-targets --all-features` | 253 passed, 0 failed |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo run --offline --example matching_review -- target/cwd-results-review` | Passed; Jobs/Export at four sizes |
| `cargo build --offline --release --bin pgm-data --bin pgm-bench` | Passed |
| `cargo build --offline --bin pgm-data` | Passed; launcher binary updated |
| `zsh -n packaging/ghostty/run-ansi.command` | Passed |
| Final match help and plain/roff manual | Primary-output defaults correct; generic Filtering copy retained |

Behavioral tests exercise CLI creation in an isolated launch directory, pause,
process restart and resume from another CWD; actual output bytes/path and absence
of an internal completed copy; explicit file/directory targets, collisions both
before and after preparation, checksum tampering, failed-publication recovery,
self-export, deletion preserving published output, and legacy/empty results.
UI tests assert New Job pins CWD and that Jobs renders the actual external result
at 40x18, 60x22, 80x24 and 120x36. Compact/wide PNGs were inspected. No tests
produced top-level workspace results or contacted a remote API. The second-volume
publication fallback was reviewed and reuses the tested atomic export helper;
no separate writable filesystem was mounted for this verification.

Ordered self-check ledger: forge GO implementation; sentinel GO local functional
checks; bastion GO (threat-models/0004-primary-results.md); beacon GO, existing
Publishing phase and payload-free completion/error logs retained; inquisitor GO,
scope/compatibility/atomicity reviewed; conductor GO local debug/release builds.
Protected animation/font/palette, Cargo dependency, DAT transport, OAuth and
crypto hashes match the backup. No independent review, branch-coverage floor,
CI, PR, release signing or deployment acceptance is claimed.

DoD T2 local: plan/contract, implementation, regression tests, lint/format,
rendered output, docs and both builds complete. Logs:
/tmp/pgm-cwd-results-{focused,tests,clippy,fmt,render,release,debug,launcher}.log.
Artifacts: target/cwd-results-review/. Rollback source backup:
/tmp/pgm-cwd-results-backup-path; preserve all user jobs/results/settings.
