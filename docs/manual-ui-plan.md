# MATCHPOINT manual and identity

Replace the repeated partner heading with one MATCHPOINT title and an
Ultra-fast dataset matching headline. Replace the clipped Help paragraph with
a chapter browser and searchable, wrapped pager. The local implementation
owner will verify the change before handing it back for terminal testing.

Initial tier T2; reclassified T3 when the measured task delta reached 819 lines,
including exhaustive content and tests. Final scope stays <=2500 lines and <=20 files, additive CLI
surface, no persistence/network/security boundary or matching hot-path change.
The checkout has no .git; task-only copies provide rollback. No publication.

## Placement and contract

`src/matching/manual.rs` owns static English chapters, text/roff output and
the pager. The UI adapter renders the pager; `ui.rs` routes keys and preserves
the previous form while Help is open. `manual` and `manual --roff` print before
opening application storage or Keychain. Existing CLI names/flags remain.
`--help` retains Clap usage/validation and points to the complete manual.

Acceptance: one product title per screen; all current commands, settings,
transforms, limits, auth modes, files, recovery and result semantics documented;
chapters, less-style scrolling, whole-manual literal search and next/previous
matches work at 40x18, 60x22, 80x24 and 120x36. Escape/q returns to the prior
screen without losing drafts. Help never runs commands or changes a job.
Animations, matching algorithms, credentials and persisted namespaces stay intact.

## Ordered consult ledger

Self-applied skills, not independent reviewer approvals:
1. architect GO: keep documentation/pager in the presentation adapter (T-1/T-3).
2. herald GO: additive `manual [--roff]`; no removal/deprecation or wire change.
3. scribe GO: one authored English source, Unix sections and actionable examples;
   exact source code is authoritative for defaults and failure behavior.
4. forge GO: concrete bounded pager, no dependency or background task.
5. sentinel GO for local behavior: 184 tests; manual module 98.21% branches.
6. bastion GO: bounded inert queries; scans pass; no credentials/data access.
7. beacon GO: T-6 exception; no per-keystroke logging or telemetry.
8. inquisitor GO for local handoff: English copy, renders, protected hashes.
9. conductor: local handoff only; Git/CI/reviewer/merge gates unavailable.

T3 design re-audit in order: architect GO (same presentation boundary), curator
GO (possible membership is not database truth), herald GO (additive manual),
migrator N/A (no data change), bastion GO (static content, 128-byte query,
no execution or credential copy), scribe GO (ADR-0013), helmsman GO (local
reversible replacement; retain Ctrl+C cleanup), beacon GO (T-6 exception:
inert local manual needs no logs/metrics/traces; never record search strings).
Then forge -> sentinel -> helmsman -> bastion -> beacon -> inquisitor -> scribe
-> conductor. CI, independent approval and production rollout are not claimed.

Verification: `cargo fmt --all -- --check`;
`cargo clippy --all-targets --all-features -- -D warnings`;
`cargo test --all-targets --all-features`;
`cargo build --release --bin pgm-data --bin pgm-bench`;
`cargo run --example matching_review -- target/manual-review`;
CLI help/manual/roff smoke, `mandoc -Tlint`, protected-file hash comparison.
Coverage and publication status must be reported as measured, never inferred.

Rollback: restore only files listed in `/tmp/pgm-manual-backup-path`, remove
new task files and rebuild; do not touch settings, Keychain, jobs or datasets.

Final evidence and scoped DoD: `docs/manual-verification.md`.
