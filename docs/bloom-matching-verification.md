# Local matching handoff — 2026-10-15

The executable now downloads a versioned GZip DAT once, verifies and loads its Bloom bit array, and streams CSV or Parquet keys through local membership checks. New Job exposes dataset, key column and optional transformation; Settings owns DAT limits/source, CSV/default key rules, checkpoint interval, optional Keychain bearer and appearance. Output preserves original keys/order/duplicates, with one `key: true|false` per record. `true` is probabilistic (0.1% target); errors never become `false`.

Task: product/format/runtime replacement, **T4**. Local implementation and verification are complete. No live PicPay endpoint, production dataset, deployment, PR/commit, independent approval, release signing/notarization, coverage or mutation threshold is asserted. This checkout has no `.git`; existing source was snapshotted before changes. Historical job/config/Keychain namespaces were preserved.

## Changed files and decisions

- `src/matching/{format,rules,dataset,source,jobs,mod}.rs`: bounded PGMBLOOM/GZip parser and producer, projected Parquet/CSV streaming, explicit transforms, source/cache locks, private snapshots, immutable filter pins and durable ordered checkpoints.
- `src/matching/cli.rs`, `src/matching/ui.rs`, `src/matching/ui/view.rs`: matching-only executable commands, English Home/New Job/Jobs/Settings, asynchronous work, cancellation/drain, masked secrets, slash suggestions, export/checksum and persistent selection. Existing splash, palette, fields and motion are reused.
- `src/{main,lib,engine,tui}.rs`, `src/engine/matching.rs`, narrowly scoped TUI module visibility: route the active application into the new context while retaining historical library APIs. `launch_legacy` is a library recovery entrypoint; main never calls it.
- `Cargo.toml`, `Cargo.lock`: exact flate2 1.1.10 with miniz_oxide; aligned Arrow/Parquet 59.3.0 for all available standard codecs; compatible rustls 0.23.46 security patch. New/resolved dependency licenses and provenance are recorded in [dependency review](../target/bloom-verification/dependency-review.json). Reviewed license families: MIT, Apache-2.0, BSD, ISC and Zlib; no new copyleft family was introduced.
- `src/bin/pgm-bench.rs`, `Makefile`: the active benchmark now exercises local matching with no HTTP calls.
- `tests/matching.rs`, `tests/fixtures/bloom/`, matching unit/UI tests, `examples/matching_review.rs`: new behavioral/independent-format/network/failure/render evidence. One historical test now observes an actual in-flight fixture request before cancellation instead of assuming a fixed 200 ms delay is sufficient.
- `README.md`, packaging README/Ghostty launcher, [ADR-0008](adr/0008-local-bloom-matching.md), [format](contracts/bloom-dat-v1.md), glossary, threat model and recovery runbook: current workflow and operational boundary. Saved appearance writes a dedicated Ghostty profile; the launcher applies it on reopen.

Design/plan preceded source edits: [plan and ordered consultations](bloom-matching-plan.md). Skills were self-applied: code-workflow → architect → curator → herald → migrator → bastion → sentry → helmsman → beacon → scribe → quartermaster → pathfinder → forge/refactory; final reviews follow below. These are not independent reviewer approvals.

## Verification

All commands ran locally against synthetic/temporary data, memory secrets and loopback servers.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo check --offline --all-targets` | PASS |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | PASS, zero warnings |
| `cargo test --offline --all-targets --all-features` | PASS, **141 tests**: 89 library + 51 integration + 1 example; [log](../target/bloom-verification/tests.log) |
| `cargo build --offline --release --bin pgm-data --bin pgm-bench` | PASS; [binary/source fingerprints](../target/bloom-verification/receipt.json) |
| `cargo audit --no-fetch` | PASS, 392 resolved dependencies, zero advisories after refreshing the advisory database earlier in this run |
| `gitleaks detect --no-git --redact --source src` and same for `tests` | PASS, no leaks; JSON reports in `target/bloom-verification/` |
| `zsh -n packaging/ghostty/run-ansi.command` | PASS |
| `cargo run --offline --example matching_review -- target/bloom-review-final` | PASS; 28 SVG/text frames at 40×18, 60×22, 80×24 and 120×36, rasterized with the bundled VGA font |
| Actual release executable under an owned synthetic PTY | PASS, 10 checks; [receipt](../target/bloom-pty-final/checks.json) |
| Actual release CLI with one loopback GET followed by offline matching | PASS, exact output, no POST and no ANSI in piped output; [receipt](../target/bloom-cli-checks.json) |

The protocol is checked against independent Python stdlib GZip/hash/encoding vectors for identity, SHA-256 hex and raw SHA-256, plus corrupt header/checksum/CRC/length/trailer cases, allocation caps, capacity overflow, transform/trim mismatch and 3,000 seeded adversarial parser mutations. Inserted keys have no false negatives in seeded tests; disjoint samples constrain observed false positives.

CSV tests cover names/indexes, headers/delimiters, multiline/escaped keys, leading zeros, duplicate/order preservation and invalid records. Parquet tests cover projection, UTF-8/integer keys, null rejection, all six codecs (uncompressed, Snappy, GZip, LZ4, Zstd, Brotli), row groups and a 20,000-row dataset paused after row 8,193 and resumed across batch/group boundaries in exact order. Jobs verify locks, immutable pins, missing/corrupt inputs/outputs, committed-prefix truncation, publication-before-metadata crash recovery and preservation of existing export destinations. New exports are private before data is written and publish without clobbering an existing destination.

PTY checks exercised the active splash/Home, DAT download, invalid-path draft recovery, New Job execution, checksum, export paths with spaces, secret masking/clearing, slash completion, resize to 40×18 and clean exit. One GET, zero POSTs; identifiers absent from lifecycle logs, synthetic secret absent from terminal bytes. These are PTY/TestBackend checks, **not native Ghostty visual acceptance**.

## Large synthetic dataset

Final release command: `/usr/bin/time -l target/release/pgm-bench --rows 20000000 --output target/bloom-bench-20m-verified`.

| Measurement | Observed locally |
|---|---:|
| Input rows | 20,000,000 |
| Filter inserted keys | 2,000,000 |
| Input snapshot/prepare | 0.869 s |
| Matching (including input verification, checkpoints and publication) | **7.504 s** |
| Matching throughput | **2.665 million rows/s** |
| Whole benchmark, including generation and output validation | 10.894 s |
| Maximum resident set | 13,647,872 bytes (~13.0 MiB) |
| Filter bit array / compressed DAT | 3,710,512 / 3,714,000 bytes |
| False negatives among inserted keys | **0** |
| False positives among 18M excluded keys | 14,367 (~0.0798%) |
| HTTP requests | **0** |
| Result count / SHA-256 verification | PASS |

[Raw measurements](../target/bloom-verification/benchmark-20m.json), [OS resource measurement](../target/bloom-verification/benchmark-20m.time). This is one synthetic CSV run on this local machine, not a production SLO or Parquet throughput claim. Random filter seeds can change observed false positives. Checkpoints default to 100,000 rows (configurable 1..1,000,000), plus pause/EOF; recovery replays uncommitted rows from a verified prefix without duplicating published output.

## Final ordered review and local DoD

1. **Sentinel — local GO; release thresholds unverified.** Behavioral suites, independent vectors, parser adversarial smoke, fault/recovery and terminal checks passed. Full statement/branch/MC/DC coverage, mutation scores and long fuzz campaigns were not measured.
2. **Helmsman — local GO.** One work slot, joined tasks on cancellation/error/exit, bounded DAT download/bit-array allocation, projected Parquet batches, immutable pins and measured 20M-row behavior. No production canary/bake or field workload SLO applies.
3. **Bastion — local GO.** HTTPS/no redirect/URL credentials, source-bound sensitive bearer header, private directories/files, checksum/CRC/limits, escaped output, no-clobber export publication, audit and leak scans passed. Existing rustls findings were patched. No separate SAST/penetration review or independent approval is claimed.
4. **Beacon — local GO.** Executable lifecycle events write structured fields to private `matching/events.log`; job metadata exposes counts/state/checksums and the UI shows true/false/progress/FPP. PTY checks confirm keys/credentials stay out of that log. External collectors/paging are not configured.
5. **Inquisitor — local review complete; merge/release NO-GO pending publishing gates.** Reviewed requirements/contracts, test evidence then domain/service/adapters. Fixed corrupt-cache repair, secret editor clearing, async error draft retention, active selection, export permission/publication races and crash recovery. No independent CODEOWNER approval, CI coverage/mutation evidence or signed release is available.
6. **Scribe — GO.** Contract, ADR, glossary, threat delta, recovery, current README and this evidence are recorded. `docs/ai/` does not exist and was not created.
7. **Conductor — local handoff only.** No commit, PR, merge, tag, signing, notarization or deployment was requested/performed. All shipping gates remain explicitly separate.

- [x] Design and format documented before source edits.
- [x] CSV/Parquet → configurable key/transform → local Bloom → ordered original-key booleans implemented.
- [x] Active CLI/TUI, cancellation, recovery, safety and synthetic volume checks pass.
- [x] Historical user data preserved; source rollback is available.
- [ ] Real endpoint/producer interoperability, native Ghostty/user visual acceptance and production release gates — not exercised here.

## Rollback

The pre-edit snapshot is `/var/folders/14/52y2hz4j46s7_h04n92vcrx00000gn/T/pgm-bloom-before-u75hia36`; additional touched existing files were added before modifying them. [Changed existing paths](../target/bloom-verification/existing-file-changes.json) and [post-edit source fingerprints](../target/bloom-verification/receipt.json) identify the task scope. Compare fingerprints before restoring, so newer user edits are preserved. Restore those saved source/Cargo/README/packaging/test files, remove only newly introduced matching modules/tests/example if unchanged, and rebuild. Keep all user job/data directories, `matching/`, preferences and Keychain. The old binary cannot process new matching jobs; their snapshots/results remain on disk for a later matching build. No destructive SQL/data rollback is needed.
