# Generic matching Help and Filtering UI

2026-10-16. T2: six product/help/render files, copy only; no public signature,
data format, persistence, authentication, crypto, network or matching change.

Goal: present ultra-fast, safe matching for datasets from any domain supplied as
CSV or Parquet, against very large reference collections. Replace CPF-specific
examples and visible engine branding with generic keys and Filtering.

Ordered self-consultations: architect GO, retain the shared manual source for
TUI/plain/roff and existing UI label/progress boundaries. Herald GO, preserve
executable command names, wire identifiers and flag semantics. Scribe GO, record
this plan before implementation. Forge GO design, edit authored copy only.

Affected: src/matching/{manual,cli,progress}.rs, src/matching/ui/view.rs,
README.md, examples/matching_review.rs and these local verification notes.
Add practical customer/device/product examples and explain bounded streaming,
shared memory, local key privacy, authenticated loading and possible matches.
Preserve true/false semantics, exact producer/consumer key rules and limitations.
Do not promise exact membership, unlimited RAM or measured throughput.

Constraints: English product copy; animations, palette, fonts, dependency files,
existing CLI bloom commands, PGMBLOOM contracts and user state remain intact.
No API calls, secret access, native terminal control or real dataset reads.

Acceptance: no CPF wording in Help/README/Clap examples; no Bloom filter/engine
branding in UI labels or progress; runnable generic CSV/Parquet examples;
manual navigation/search/wrapping and result/export contracts preserved.

Verification: cargo test --offline --all-targets --all-features;
cargo clippy --offline --all-targets --all-features -- -D warnings;
cargo fmt --all -- --check;
cargo run --offline --example matching_review -- target/filtering-copy-review;
cargo build --offline --release --bin pgm-data --bin pgm-bench;
cargo build --offline --bin pgm-data. Inspect emitted CLI/plain/roff help and
synthetic Home/Help at compact and wide sizes. No new logic means no new
branch-coverage claim or tests that merely duplicate static text.

Rollback: restore only the six source/docs files from the backup recorded in
/tmp/pgm-filtering-copy-backup-path and rebuild. Preserve user data/settings.
Sentinel, bastion, beacon, inquisitor and conductor checks follow implementation;
these are local self-consultations, not independent review or publication gates.
