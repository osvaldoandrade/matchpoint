# Jade terminal theme and interactions — 2026-10-14

Implement the user's requested local presentation revision: New Job precedes Jobs and is the initial Home action; replace saturated green surfaces with graphite, restrained jade/mint accents, slate borders and semantic chart colors; add short focus transitions and bounded title highlights. The user owns visual acceptance; implementation and verification belong to this local task. Target full labels/shortcuts at four terminal sizes and fixed frames when motion is off. Restore task-owned source and rebuild within ten minutes if controls, readability or reduced motion regress.

**T2**: estimated 400–650 added/removed LOC, up to sixteen files in one Rust package. No public CLI/HTTP, schema, auth, secret-storage, network or concurrency change. Private ephemeral interaction timing adds no persistence format. This is local implementation, with no Git metadata or publication request; CI, human reviewer approval, coverage/mutation publishing floors are outside the local completion claim.

Ordered design consultations, self-applied specialist instructions:

1. **Architect — GO** (T-1/T-3/T-10): shared color tokens live in `src/tui/palette.rs`; deterministic presentation effects live in a private `view/motion.rs`, consuming existing frame ticks and one nonpersisted key timestamp. View → Model/engine direction and all processing boundaries remain. Keep graphs independent of decorative phases.
2. **Herald — GO** (T-1/T-7): Home order is New Job / Jobs / Settings, with N/J/S unchanged; Enter and 1/2/3 follow displayed order. The first Enter after Splash still only opens Home. Centralize commands and copy order; explicit numeric-order change is authorized by the menu correction. Existing preferences, command completion, masking and job execution contracts remain.
3. **Scribe — GO** (canon ch. 02/05/10/15): this one-page plan precedes edits, distinguishes synthetic previews from native screenshots and measures readability/control behavior. This user's request supersedes ADR-0002's static decorative Home title, while record values/charts stay static. No ADR trigger for network/data/dependency changes applies.
4. **Forge — GO** (T-1/T-10): truecolor graphite/panel/selected surfaces, mint-white text and restrained semantic accents; thin neutral borders; title color sweep without erased/typed text; focus color settles in at most eight 33 ms ticks, with no delay to dispatch. All motion honors Appearance Off and `/motion`. Shared Settings/New Job fields receive the same focus treatment. Reuse the existing event loop, not new timers/tasks/dependencies.

Affected files: `src/tui.rs`, `src/tui/palette.rs`, `src/tui/brand.rs`, `src/tui/copy.rs`, `src/tui/view.rs`, `src/tui/view/home.rs`, `src/tui/view/splash.rs`, `src/tui/view/dashboard.rs`, `src/tui/view/form_field.rs`, `src/tui/view/new_job.rs`, new `src/tui/view/motion.rs`, `src/tui/ux_tests.rs`, `examples/ui_review.rs`, `packaging/ghostty/pgm-ansi.ghostty`, this plan and `docs/workflow-ledger.md`.

Acceptance: displayed order and initial Enter reach New Job; arrows/numeric/N/J/S dispatch correct actions without executing jobs; all text remains present at each animation phase; only title/focus styles change, with dashboard counters/geometry stable; Off yields identical frames even after interaction; required text/focus color pairs have at least 4.5:1 contrast. Inspect empty/loaded/error Home, Splash, Settings and New Job at 120×40, 100×30, 80×24 and 40×18. Update the review exporter for real RGB ANSI/SVG colors. Exercise synthetic PTYs for startup gating, menu dispatch, form focus/cancel/create, resizing, normal exit and terminal restoration. Keep assertions meaningful; no tests mirroring padding or exact color literals.

Verification commands:

```sh
cargo fmt --all -- --check
cargo check --offline --all-targets
cargo test --offline --all-targets --all-features
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo build --offline --bin pgm-data --example ui_review
cargo build --offline --release --bin pgm-data
cargo run --offline --example ui_review -- target/ux-review/jade-interactions
python3 /tmp/pgm-jade-interactions-pty.py
gitleaks detect --no-git --redact --source src --report-format json --report-path target/ux-review/jade-interactions/gitleaks.json
```

Sentinel → Bastion → Beacon → Inquisitor → Conductor consultations follow implementation. Beacon's presentation-only T6 rationale: effects introduce no operation/I/O; reproducible frames, timing and PTYs provide local evidence. Per-frame logging would add overhead and expose fields without a new operational decision. Existing operation events remain authoritative.

Local T2 DoD:

- [x] Classification, placement/contract note, written plan, ordered design consultations and rollback scope.
- [x] Menu order, coherent theme and shared bounded text/focus effects implemented.
- [x] Meaningful tests, format/check/lint, normal debug/release builds and leak scan pass.
- [x] Four-size render/contrast/motion and synthetic PTY checks pass.
- [x] Final reviews, measurement limits and source hashes/rollback recorded in [the workflow ledger](workflow-ledger.md#jade-theme-and-menu-interactions--2026-10-14).
