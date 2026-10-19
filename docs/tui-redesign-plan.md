# English numeric TUI redesign — 2026-10-14

The application maintainer will replace the rejected figure and menu organization with a numeric 4D/3D home, an operational Jobs workspace and a single tabbed Settings screen. The user owns visual acceptance. Source review, runtime evidence and the three independent Nielsen evaluations own the completion evidence. The target exceeds 9.8; no score from the rejected layout counts toward this target. This plan implements the full request and leaves processing behavior under the existing engine.

## Classification

Feature and presentation refactor, **T3**: estimated 1,800–2,500 added/removed lines across fewer than 20 files, with secret-form presentation and reversible preference additions. No wire/API break, schema migration, new dependency, production-data operation, auth policy change, crypto change or new worker topology. Reclassify before further implementation if the actual delta exceeds 2,500 lines or another higher-tier signal appears.

## Design and sequence

Home displays rotating numeric geometry and three primary paths: Jobs, New job and Settings. Jobs keeps selection and its details together at 80+ columns; 40-column layouts preserve a clear list/detail transition. Settings groups API, Secrets, Processing and Appearance into tabs. Keyboard focus separates tab selection from field editing, retains staged values, and provides explicit Save/Cancel. All application-owned visible copy uses English; filenames, IDs and historical event data remain faithful to their source.

Implement the numeric scene first and inspect frames before expanding the layout. Then compose Settings from the existing bounded editors, revise Jobs/Batches, translate UI/CLI presentation, and complete preference compatibility and negative-path tests. Keep checksum, pause/resume, remote authorization and secret handling under their existing guards. Use synthetic data and MemorySecrets for verification.

Affected files: `src/tui/art.rs`, `src/tui/view.rs`, `src/tui.rs`, new `src/tui/settings.rs` and focused view modules where useful; `src/tui/configure.rs`, `appearance.rs`, `forms.rs`, `feedback.rs`, `commands.rs`, `session.rs`, `work.rs`; `src/engine/tuning.rs` and CLI presentation/default inheritance as needed; UI tests, `examples/ui_review.rs`, README and operation/evaluation notes.

## Consultation ledger

Ordered design consultations use the actual specialist instructions, with Rust equivalents for language-specific Go commands. Architect: **GO**, keep presentation in the TUI adapter; no new context/service. Curator: **GO**, Job/Batch/Reserved/Threads meanings and invariants above. Herald: **GO**, preserve wire and CLI flag contracts; only additive optional preferences. Bastion: **GO for this design**, retain existing authorization checks, masked/zeroized inputs, safe argv and synthetic-only tests; implementation review remains required. Scribe: **GO**, this plan and ADR record the narrative, rollback and evidence limits. Helmsman, Beacon, Forge, Refactory, Sentinel, Bastion review and Inquisitor follow in the prescribed order. Usher and Algorithmist cover terminal accessibility/copy and numeric geometry. Three existing Nielsen evaluators remain independent and read-only.

## Verification commands and acceptance

Helmsman design: **GO**, keep bounded worker draining and background file work; verify terminal restoration and local source rollback. Beacon design: **GO**, retain job counters/audit events and add scoped frame/navigation timing and safe review spans to the reproducible harness; external collectors and per-frame persisted logs would add an unrelated dependency and overhead to this local TUI. Forge design: **GO**, concrete bounded editors/widgets, no new pool or dependency. Algorithmist: **GO**, sampled geometry has fixed budgets (at most 3,456 lattice samples or 32 bounded edges), bounded depth grid and linear drawing cost. Usher: **GO for design**, English copy catalog, named fields, visible focus, gray/green contrast and fixed reduced-motion frames; manual/runtime audit remains required.

```sh
cargo fmt --all -- --check
cargo check --offline --all-targets
cargo test --offline --all-targets --all-features
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo build --offline --bin pgm-data
cargo run --offline --example ui_review -- target/ux-review/numeric-1
cargo run --offline --example ui_review -- --latency target/ux-review/numeric-1/latency.json
cargo run --offline --example ui_review -- --live
/Applications/Ghostty.app/Contents/MacOS/ghostty +validate-config --config-file=packaging/ghostty/pgm-ansi.ghostty
gitleaks detect --no-git --redact --source src
```

Verify numeric rotation and fixed reduced-motion frames; tabs/focus/save/cancel and unchanged secret masking; valid/invalid defaults and old preference files; source override precedence; no accidental command execution on paste/menu keys; selection, execution, pause of active after changing selection, result origin, resume, export and input/export checksum. Inspect 120×40, 100×30, 80×24, 40×18 and smaller fallback. Measure render/frame and input/refresh latency with synthetic archives, including one million records. Record coverage/mutation with Rust tooling where applicable and report exact measurement scope. Native Ghostty observation is unavailable through the current capture tool; do not call generated PNGs native screenshots.

## Rollback and Definition of Done

Create a source snapshot before edits. Restore only task-owned source and rebuild if interaction or processing invariants regress; preserve all job and secret data. The ADR records release/staging gates that do not apply to this local scope. Completion requires the plan/ADR, ordered consultation verdicts, verified implementation, meaningful tests and scans, rendered/runtime/performance evidence, all explicit user requirements, and the three independent scores on unchanged source above the requested average threshold. Final reporting includes the final T4 local checklist and unresolved limitations rather than claiming unperformed release checks.

## Reclassification — T4 local presentation replacement

The measured code/test/example delta reached 2,480 lines before the remaining regression cases and evidence updates. Completing the authorized replacement will exceed 2,500; classification is now **T4** by LOC, before further implementation. No production risk, network/auth policy, cryptography or destructive migration is inferred from this size classification. The existing design artifacts are re-reviewed with the stricter scope below.

Ordered design re-review: Architect **GO**, the design/ADR plus alternatives and evaluation criteria form the local RFC: visible numeric rotation, three primary paths, all-English copy, staged tabs, clear selected-job context and preserved engine invariants. Curator **GO**, failure catalogue covers draft loss, wrong job action/result, import/export/checksum failure, invalid settings, secret disclosure and frame latency. Herald **GO**, compatibility includes existing CLI flags, old processing JSON with only threads, old session JSON without scene/font/motion and current additive preferences; no wire versions change. Migrator **GO**, optional local preference expansion has serde defaults; no schema/backfill/dual-write/reconciliation applies. Bastion design **GO**, STRIDE remains in the linked threat model; metadata-only save compares the exact saved identity/URLs and requires a stored secret reference. Sentry **GO for design**, pre-mortem and local recovery below. Helmsman **GO for design**, bounded geometry, retained shutdown/draining and local source rollback. Beacon **GO for design**, reproducible timings and fixture results are the local evidence; no production alerts/probes apply. Scribe **GO**, amended ADR and this plan record the scope, limits and rationale. Forge **GO**, finish the requested feature on this local checkout. Refactory **GO**, prior source snapshots retain the old path for rollback; this is an intentional feature replacement, not a behavior-preserving refactoring PR. Deletion targets obsolete presentation paths only; engine contracts remain characterized by existing tests.

Pre-mortem: in the first five minutes, stop interaction if a draft saves unexpectedly, credentials appear in artifacts, terminal recovery fails or a wrong job action appears. Within fifteen minutes, restore only source from `/tmp/pgm-before-numeric-20261014`, rebuild offline and verify the fixture. At sixty minutes, keep the reverted binary and inspect isolated reproductions before another local replacement. Source RTO target is ten minutes; data RPO is zero because source rollback never restores jobs or Keychain data. These are local recovery targets, not production SLO measurements.

The production/publication-specific T4 gates remain inapplicable under ADR delivery scope. Rust coverage/mutation tooling is assessed during Sentinel verification; unmeasured floors must stay unverified. No release, merge, canary or signed acceptance is claimed. The requested three independent Nielsen reviews remain separate from these self-applied specialist checks.

## Implementation and final evidence

The final local source is numeric-12, aggregate SHA-256 `4cf43d9234bea2ea24639e92983a4f462914293bff9785602995cdd414f77ee9`. The default tesseract and alternate lattice replace the torus. Settings has staged groups, dirty indicators, group-bound validation and invalid-profile recovery. The final source passes 114 tests, format/check/lint/build, secret scanning, synthetic PTY and four-size rendered/performance checks. Exact commands, instruction-review verdicts, local Definition of Done and unmet publication gates are in [the workflow ledger](workflow-ledger.md). Three independent Nielsen results remain separate in [the evaluation history](ux-evaluation.md); no earlier rejected-version score approves this source. Only evidence documentation is edited after freezing the implementation.

Numeric-10 fixes an independently reproduced counter truncation in 40-column terminal states. Summary lines expand according to actual content, and batch page size uses that same geometry. New tests preserve million-record and 19-digit values and visit every batch at all four supported sizes. Prior ratings are retained; all three reviewers must reconfirm this unchanged implementation.

Final numeric-12 also translates the advanced prompt/launcher error and characterizes compact error layouts through the integer boundary. Shared area calculation preserves summary/content, and PageUp/Down never advances farther than the visible detail rows. The final 114-test and 156-render evidence is recorded under `target/ux-review/numeric-12/`; no source or asset is edited after this final freeze.

Final independent confirmation: A 9.87, B 9.84 and C 9.895, all strictly above 9.8 on the same numeric-12 freeze with no code/asset drift. The local objective is complete. Remaining low-priority recommendations, native visual acceptance and unmet publication-specific checks remain explicit in the evaluation/ledger; no production approval is inferred.
