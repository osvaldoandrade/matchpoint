# New Job visual consistency — 2026-10-14

The maintainer will make New Job use the Settings field renderer and page conventions in this local task. The current three fields use flat labels and values; the target gives all three their own outlined field, with the same title contrast, selected value, spacing and footer actions as Settings. The user authorized this reversible presentation change. Enter still creates the job and Escape returns; API, import validation, defaults, job state and persistence remain under their existing handlers.

## Classification and placement

**T2**, estimated 200–350 added/removed lines across five files in one Rust package. The shared field extraction and dedicated New Job layout set the LOC signal; no dependency, public API, persistence, network, security-policy or concurrency boundary changes. Git metadata is absent. This scope is local implementation and verification; no commit, PR, installation or production publication is requested.

Affected files: `src/tui/view.rs`, new private `src/tui/view/form_field.rs` and `src/tui/view/new_job.rs`, this plan and `docs/workflow-ledger.md`. Existing form/input models remain unchanged. The field component replaces the Settings drawing block, so both screens share the same implementation instead of copying styles. New Job uses the branded double outer border, three-row field boxes, visible field position, green Create job/Back actions, and a compact Context panel on wide terminals. Compact layouts scroll fields with focus and reserve the full validation/recovery message.

## Ordered design consultations

1. Architect — **GO**: keep private rendering below `tui::view`, with dependency direction into the existing Model/ImportForm. No context or engine restructure.
2. Herald — **GO**: private widget symbols only; preserve CLI flags, Enter/Escape/Tab, errors and saved job formats. No wire contract or deprecation applies.
3. Scribe — **GO**: this narrative precedes implementation; record exact verification scope and rollback, with no reuse of prior-version Nielsen ratings.
4. Forge design — **GO**: extract the existing Settings field widget, compose New Job from that concrete widget, and remove the obsolete flat New Job branch. Keep new functions within 40 lines, four parameters and 120 columns.

Forge implementation plan: (1) retain the existing field's masking, sanitization, byte cursor and contrast in a private component; (2) give New Job the matching page/field/action layout; (3) inspect blank/editing/error renders at 120×40, 100×30, 80×24 and 40×18, then rebuild both local binaries. Sentinel, Bastion, Beacon, Inquisitor and Conductor verdicts follow implementation in that order. These are self-applied specialist instructions, not delegated agent approvals.

## Verification and acceptance

```sh
cargo fmt --all -- --check
cargo check --offline --all-targets
cargo test --offline --all-targets --all-features
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo build --offline --bin pgm-data
cargo build --offline --release --bin pgm-data
cargo run --offline --example ui_review -- target/ux-review/new-job-style
```

Use the existing meaningful import/path/threads/cancel/secret/contrast suites and loopback review harness; do not add implementation-mirroring tests for a style-only change. Verify field labels remain readable during focus, long paths retain cursor/window behavior, all fields stay reachable in small terminals, validation and Enter/Escape still work, and Settings secret values remain masked. Measure changed-branch coverage with existing LLVM tooling where feasible and report its scope; do not infer whole-package or mutation floors. Generated VGA PNGs are TestBackend renders, not native Ghostty screenshots.

## Rollback and local Definition of Done

Before implementation, copy task-owned source to a temporary snapshot. If controls, masking or geometry regress, restore only those source files and remove only the two new private modules, then rebuild offline within five minutes; preserve all job/preferences/Keychain data. The snapshot path and verification verdicts will be recorded in the ledger.

- [x] T2 classification, placement, plan and ordered design consultations written before code.
- [x] New Job and Settings share field rendering; New Job page matches the requested visual style.
- [x] Format/check/tests/lint/build and four-size rendered checks pass.
- [x] Sentinel, Bastion, Beacon, Inquisitor and Conductor scope verdicts recorded.
- [x] Coverage measurement/limits and rollback evidence recorded; publication gates remain N/A for this local task.

Completion evidence is in [workflow-ledger.md](workflow-ledger.md#new-job-settings-style--2026-10-14). This checklist describes local delivery, not the full publishing DoD: the branch-coverage floor is not met, and package coverage, mutation, CI and independent reviewer approval are not claimed.
