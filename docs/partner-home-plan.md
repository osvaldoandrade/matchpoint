# PartnerGetMember home — 2026-10-14

The maintainer will replace the sparse, misleading Command Center home with an English PicPay PartnerGetMember import workspace. The user requested useful home information, a better logo/art, a menu bar with contextual descriptions, headline/subheadline and vivid ANSI green depth. This is local presentation and navigation work; jobs, API contracts, credentials and persistence stay under their existing handlers.

**T2**, estimated 600–800 added/removed lines across fewer than 15 files in one Rust package. Private rendering, copy, palette and compatible Left/Right menu aliases; no dependency, public API, schema, network or concurrency boundary change. Reclassify before exceeding 800 lines. No Git metadata or publication request is present.

Files: `src/tui/view.rs`, new private `src/tui/view/home.rs`, `src/tui/brand.rs`, `src/tui/palette.rs`, `src/tui/art.rs`, `src/tui/copy.rs`, `src/tui.rs`, existing UI tests, `examples/ui_review.rs`, this plan and `docs/workflow-ledger.md`.

Ordered design consultations, self-applied from the specialist instructions already read:

1. **Architect — GO.** Move home rendering below `tui::view`; compute metrics from loaded JobStatus values only. Keep the animated geometry secondary to an actual PGM wordmark. No render-loop database or network calls.
2. **Herald — GO.** Keep existing J/N/S, Enter, Tab, Up/Down, Escape, slash and quit semantics. Add compatible Left/Right menu selection. SceneKind serialization and processing behavior stay compatible.
3. **Scribe — GO.** This plan precedes edits; distinguish synthetic frames/PTYs from native Ghostty acceptance and do not reuse previous Nielsen scores.
4. **Forge — GO.** Build a shaded block-letter PGM mark with a bounded extrusion/highlight, a coherent ANSI-256 green ramp and reduced-motion support. Compose headline, horizontal menu, brand/process panel, actual workspace/selected-job data, contextual status bar and footer. Compact/error layouts prioritize navigation, descriptions, recovery and exit.

UX-copy consultation: English, direct operational tone. Recommended headline **PartnerGetMember**, subheadline **Partner data import for PicPay**, menu **Jobs / New Job / Settings**. Status descriptions explain review/import/configure actions. Alternatives considered: "Partner data importer" (less specific product identity) and "PicPay data workspace" (less clear import purpose). Selected wording names the real product and avoids implying connection health or production readiness. API status describes enabled configuration only. Localization note: preserve PartnerGetMember/PicPay names; compact English labels use the same terms as existing screens.

Acceptance: Command Center is absent; headline/subheadline fit all supported sizes; each selected option changes the status description through real key events; a branded shaded logo, real jobs/batches/open-record metrics and API setup state fill the home; current job/activity are visible when space allows. Text contrast stays readable; low-contrast green shades are decorative only. Motion changes the geometry/highlight, and reduced motion fixes both. Retain full recovery and Q/Enter controls at 40×18. Synthetic fixtures only.

```sh
cargo fmt --all -- --check
cargo check --offline --all-targets
cargo test --offline --all-targets --all-features
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo build --offline --bin pgm-data --example ui_review
cargo build --offline --release --bin pgm-data
cargo run --offline --example ui_review -- target/ux-review/partner-home
```

Inspect 120×40, 100×30, 80×24 and 40×18: empty/loaded/selected-menu/error/motion states. Extend meaningful menu/metric/motion checks; no padding-mirroring tests. Update the review exporter to preserve ANSI-256 shades in SVG/ANSI evidence. Sentinel, Bastion, Beacon, Inquisitor and Conductor follow implementation in order; record scope and unmeasured publishing gates. Do not log render-loop field values.

Before edits, snapshot task-owned files. Rollback restores only those files and removes the three new private modules, then rebuilds offline; preserve jobs/preferences/Keychain. User owns visual acceptance; local delivery does not assert the full publishing DoD.

- [x] T2 classification, placement, written plan and ordered design consultations.
- [x] Product home, logo, ANSI palette, menu/status and real workspace information implemented.
- [x] Tests, format/check/lint and debug/release builds pass.
- [x] Four-size visual review and synthetic PTY checks pass.
- [x] Final scoped consultations, evidence/limits and rollback recorded.

Results: [workflow-ledger.md](workflow-ledger.md#partnergetmember-home--2026-10-14). Full publishing gates, branch/package coverage, mutation, CI and independent reviewer approval are not claimed. The user remains the judge of the new visual identity.
