# PicPay partner splash and dashboards — 2026-10-14

Implement the user's six UI corrections: exact subheadline **Import tools for PicPay Partners**; animation on an explicit Enter-to-open splash; a PicPay-inspired payment-chip/circuit icon; a compact text identity on Home; real ANSI dashboard graphics; and removal of PGM.DATA from visible TUI branding. Display identity is PicPay Partners / PartnerGetMember. Binary names and existing data locations remain compatibility contracts.

**T3** by estimated 1,200–2,000 added/removed LOC across fewer than 20 files in one package. Blast radius: this reversible local terminal presentation. No network/schema/auth/concurrency change. [ADR-0002](adr/0002-splash-and-partner-dashboards.md) is drafted before edits. Source rollback will restore task files and remove only newly added view modules, without touching jobs/preferences/Keychain. This folder has no Git metadata and the request does not authorize publication or production rollout.

Ordered design consultations, self-applied specialist instructions:

1. **Architect — GO:** Splash and dashboard remain private TUI widgets; interactive launch alone enables an ephemeral splash flag. Home charts consume loaded state, without additional I/O. Context map remains CLI/TUI → App → engine/store/provider boundaries.
2. **Curator — GO:** queued = pending + retry-wait; active = in-flight; processed includes succeeded, final failure and UNKNOWN. UNKNOWN stays uncertain. Job lifecycle groups are Wait, Run, Done and Hold; Hold includes paused/interrupted/blocked/failed preparation. Batch maps identify the actually loaded selected-job slice and filter, not a guessed completion percentage.
3. **Herald — GO:** additive Model splash state; Model::new retains programmatic Home construction, while interactive launch starts Splash. Enter advances without opening/executing a job; Q/Escape/Ctrl+C quit; paste and unrelated commands do nothing on Splash. Existing SceneKind serialized values remain readable, with updated presentation labels for the new splash motifs. Current CLI/data/HTTP contracts stay compatible.
4. **Migrator — N/A:** no schema, backfill or preference format change.
5. **Bastion design — GO:** preserve terminal restoration, masking, sanitization and explicit remote guards. Decorative binary pulses are not API/connection telemetry. No secrets/identities enter graphics or status output. Existing threat model applies; new splash input gating is reviewed as a local boundary.
6. **Scribe — GO:** plan and ADR precede implementation; evidence remains synthetic, and previous Nielsen scores are not reused.
7. **Helmsman design — GO:** fixed-size icon painting, bounded chart sampling and O(loaded jobs + visible batches) frame work; no timed splash delay. Test startup/continue/quit restoration, resize, empty/extreme counters and reduced motion. Rollback is local source restoration, not a deployed canary.
8. **Beacon design — GO:** reproducible renders, PTYs and scoped frame timings are the evidence. Existing operation events stay authoritative. Per-frame field logging adds overhead/exposure without a new operational decision; no external collector or alert is added.
9. **Forge — GO:** replace the large Home wordmark/animation with record-outcome bars, job-state histogram and a scoped batch map/activity panel. Render a shaded double-P payment chip with circuit pulses only in Splash; Appearance preview is static. Keep new functions ≤40 lines where practicable and private files ≤500 lines.
10. **Refactory — GO for structure:** this is an intentional feature replacement, not a behavior-preserving refactor. Existing control/storage suites characterize retained behavior; obsolete Home/art assertions are updated for the requested state transitions and chart semantics. Rollback snapshot retains the rejected source; no Git micro-commit or parallel legacy runtime is implied.

Files: `src/tui.rs`, `src/tui/art.rs`, `src/tui/brand.rs`, `src/tui/copy.rs`, `src/tui/settings.rs`, `src/tui/view.rs`, `src/tui/view/home.rs`, `src/tui/view/new_job.rs`, new private `view/splash.rs` and `view/dashboard.rs`, UI tests, `examples/ui_review.rs`, plan/ADR/ledger.

Verification: meaningful transition/ignore-paste/quit tests; chart totals and segment rounding invariants (empty, mixed, tiny/large/negative values); phase-stable Home, animated/fixed Splash, bounded icon clipping; existing secret/import/processing suites. Inspect Splash and empty/loaded/error Home at 120×40, 100×30, 80×24 and 40×18. Synthetic PTYs prove startup gating and immediate Enter transition plus existing forms/terminal restoration. Measure scoped frame CPU with loaded synthetic data, not provider capacity. Full publishing coverage/mutation/CI/reviewer/canary gates remain unverified or outside scope.

```sh
cargo fmt --all -- --check
cargo check --offline --all-targets
cargo test --offline --all-targets --all-features
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo build --offline --bin pgm-data --example ui_review
cargo build --offline --release --bin pgm-data
cargo run --offline --example ui_review -- target/ux-review/splash-dashboard
```

Sentinel/Helmsman verification, Bastion/Beacon review, Inquisitor, Scribe and Conductor scope verdicts follow implementation. Local T3 DoD:

- [x] Classification, placement/context delta, glossary invariants, plan, ADR, threat/rollout/evidence scope and ordered design consultations.
- [x] Exact copy/rebranding, splash lifecycle/icon and real static dashboards implemented.
- [x] Meaningful tests, format/check/lint and debug/release builds pass.
- [x] Four-size rendering, scoped performance and synthetic PTY behavior verified.
- [x] Final review, evidence limits and source rollback recorded in [the workflow ledger](workflow-ledger.md#picpay-partner-splash-and-ansi-dashboards--2026-10-14).
