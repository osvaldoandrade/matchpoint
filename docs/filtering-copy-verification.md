# Generic Help and Filtering copy verification

2026-10-16. T2, local copy change. Plan: filtering-copy-plan.md.

Shared TUI/plain/roff Help and Clap examples use generic identifiers. The new
DATASETS AND SCALE chapter covers customer/device/product/account/event keys,
CSV/Parquet, bounded streaming, shared reference memory, matching rules, local
privacy, authenticated loading and probabilistic results. README has corresponding
examples. No throughput benchmark or exact-positive guarantee is claimed.
Home labels, status text and verification progress use Filtering. Synthetic
screens use generic customer/product keys. Existing CLI bloom commands, wire
magic and contract paths remain compatible technical identifiers.

The new chapter is appended: all 20 pre-existing chapter indices remain intact,
including contextual Help destinations. Navigation/search/wrapping remain shared.
No source processing, download, crypto, persistence or export behavior changed.

| Command / check | Result |
| --- | --- |
| `cargo test --offline --all-targets --all-features` | 249 passed, 0 failed |
| `cargo test --offline --lib matching::manual` | 4 passed after final chapter placement |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo run --offline --example matching_review -- target/filtering-copy-review` | 96 synthetic screens, four viewport sizes |
| `cargo build --offline --release --bin pgm-data --bin pgm-bench` | Passed |
| `cargo build --offline --bin pgm-data` | Passed; launcher binary updated |
| CLI root long/short, match, job create, bloom, bloom build, plain/roff manual | 8 outputs checked; no CPF or Bloom filter/engine wording |
| Protected animation/font/palette/entry-point and Cargo manifest/lock hashes | Unchanged |

Synthetic Home, NAME and DATASETS AND SCALE PNGs were inspected. Home/progress
labels and chapter selection were checked at 40x18, 60x22, 80x24 and 120x36.
All 96 UI snapshots exclude CPF and Bloom filter/engine branding. Output copy
retains original keys, possible-positive semantics and absolute export defaults.
Artifacts: target/filtering-copy-review/. Logs:
/tmp/pgm-filtering-copy-{tests,manual-tests,clippy,fmt,render,release,debug}.log.

Ordered self-applied completion ledger:
- forge GO: authored copy and synthetic render inputs only.
- sentinel GO local: full suite plus final manual navigation/search checks;
  no new production branches, no invented coverage measurement.
- bastion GO local: honest safe-matching claims, producer-rule and probabilistic
  limits retained; no secrets, API calls, real datasets or state modifications.
- beacon GO: existing progress phases retained with readable Filtering labels;
  no new operational code path or telemetry required.
- inquisitor GO local: scoped source diff reviewed, contextual chapter indices
  preserved, protected-file hashes unchanged, generic examples supported.
- conductor GO local: debug/release rebuilt; no Git metadata, CI, independent
  review, PR, publication or deployment acceptance claimed.

Local T2 DoD: plan, copy, Help compatibility, tests/lint/format, rendered checks,
documentation and both builds complete. No follow-up or migration required.
Rollback uses /tmp/pgm-filtering-copy-backup-path and a local rebuild; preserve
jobs, results, exports and settings. The running user terminal was not restarted.
