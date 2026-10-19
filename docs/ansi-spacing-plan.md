# Smaller ASCII geometry and aligned spacing — 2026-10-14

The maintainer will reduce the rotating art and refine its ASCII stroke vocabulary, then align the spacing around labels, notes and actions. The user asked for a smaller, less coarse rendering and more symmetric presentation. This is local TUI presentation work, not a new scene or a change to jobs, APIs, secrets, settings persistence or keyboard semantics.

Classification: **T2**; estimated 300–600 added/removed lines across six files in one Rust package. Changes remain below the private rendering boundary with no dependency, public contract, storage, network or concurrency change. No Git metadata or publication request is present.

Affected files: `src/tui/art.rs`, `src/tui/view.rs`, `src/tui/view/new_job.rs`, `src/tui/view/form_field.rs` if field spacing needs adjustment, this plan and `docs/workflow-ledger.md`. Existing shape preferences and reduced-motion behavior stay compatible. Rollback will restore only these task-owned source files from a temporary snapshot and rebuild offline, preserving jobs and preferences.

Ordered design consultations, self-applied from the previously read specialist instructions:

1. **Architect — GO.** Keep scale, projection and directional glyph selection inside the existing bounded Canvas. Keep layout helpers private to `tui::view`; share Settings/New Job spacing without a new domain model.
2. **Herald — GO.** No public symbols or serialized enum changes; Enter/Escape/Tab, slash commands and existing SceneKind labels/preferences stay compatible.
3. **Scribe — GO.** This plan precedes implementation; report synthetic render/PTY evidence explicitly and do not reuse earlier Nielsen scores or claim native Ghostty inspection.
4. **Forge — GO.** Apply a uniform 0.70 factor to projected coordinates; replace all-digit edge fills with direction/depth-aware ASCII strokes and hexadecimal vertex markers. Use the same content inset, column gutter and message inset for Settings and New Job. Center the home scene/menu pair and shorten surplus menu padding. Preserve bounded raster sampling and existing sanitization/masking.

Acceptance: both scenes are smaller and still rotate; ASCII strokes remain single-column printable characters; numeric vertex markers remain; reduced motion produces identical frames; error recovery, focused fields and navigation remain visible at 40×18. Review home, Settings and New Job at 120×40, 100×30, 80×24 and 40×18. Keep at least one numeric/raster behavior check for the algorithm change, but add no implementation-mirroring tests for padding alone. Run the existing keyboard/import/secret suites and synthetic PTY checks.

Verification commands:

```sh
cargo fmt --all -- --check
cargo check --offline --all-targets
cargo test --offline --all-targets --all-features
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo build --offline --bin pgm-data --example ui_review
cargo build --offline --release --bin pgm-data
cargo run --offline --example ui_review -- target/ux-review/ascii-spacing
```

Sentinel, Bastion, Beacon, Inquisitor and Conductor follow implementation in that order. Coverage, mutation, independent review, CI and publishing gates are not inferred from successful local style checks. A presentation-only Beacon rationale avoids recording per-frame input values or adding render-loop logs.

Local T2 DoD:

- [x] Classification, placement, written plan and ordered design consultations.
- [x] Scale/ASCII rendering and shared spacing implemented.
- [x] Format, check, tests, lint and both builds pass.
- [x] Four-size render inspection and synthetic PTY control checks pass.
- [x] Final scoped consultations, measurements/limits and rollback recorded.

Results are recorded in [workflow-ledger.md](workflow-ledger.md#smaller-ascii-geometry-and-aligned-spacing--2026-10-14). At 100×30 the default home art's occupied bounds changed from 31×16 to 21×11 cells. The 0.70 projection scale combines with the balanced columns; raster rounding makes the occupied-cell ratios approximate. Jobs/Home status areas now reserve two lines for an active or pending operation, and detail paging uses the same inset as rendering. These are presentation changes within the original scope.

This is the local delivery checklist. Coverage/mutation floors, CI, independent reviewer approval and the full publishing DoD were not measured or claimed for this request.
