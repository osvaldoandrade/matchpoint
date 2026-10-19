# Ledger

## 2026-10-14 — Numeric home and English tabbed workspace

This entry supersedes the earlier presentation acceptance for the current UI. The user rejected the hooded figure and rotating torus, requested numeric mathematical rotation, concise bordered screens, English copy and one tabbed Settings screen. The default scene is now a numeric 4D tesseract; a rotating 3D lattice is available in Appearance. Home offers Jobs, New Job and Settings. Settings contains API, Secrets, Processing and Appearance; API and Secrets deliberately share one staged profile and one Save action.

- Final classification: **T4 by size**. Against `/tmp/pgm-before-numeric-20261014`, the final Rust source/test/example delta is 2,566 added and 832 deleted lines, 3,398 total across 26 files. This is a text comparison, not a Git diff; this checkout has no Git metadata. No new dependency, SQL migration, auth policy, crypto algorithm, production operation or worker topology was introduced.
- Design, invariants and ordered design verdicts: [redesign plan](tui-redesign-plan.md), [ADR](adr/0001-numeric-terminal-workspace.md), [threat-model delta](threat-models/0001-tabbed-terminal-settings.md). The original source snapshot remains the local rollback path; rollback never restores job or Keychain data.
- Actual shared instructions were recovered from `codecompany-claude-skills/skills/_shared/`. Specialist verdicts below are self-applied instruction reviews, separate from the three independent Nielsen evaluations.
- Evaluated freeze: `/tmp/pgm-ui-numeric-12-source-20261014`, aggregate SHA-256 `4cf43d9234bea2ea24639e92983a4f462914293bff9785602995cdd414f77ee9`, 89 files including migrations. Live files matched this manifest before the final documentation entries. Evaluation/ledger documentation is appended afterward; executable code, assets and build inputs must remain identical.

### Final functional verification

| Check | Result and scope |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --offline --all-targets` | PASS |
| `cargo test --offline --all-targets --all-features` | PASS, 114 tests: 74 unit, 2 chaos, 10 configuration, 5 OAuth, 2 package, 21 pipeline |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | PASS |
| `cargo build --offline --bin pgm-data` | PASS; current local debug binary rebuilt |
| `cargo build --offline --release --bin pgm-data` | PASS; optimized local binary rebuilt from the same source, replacing the earlier presentation build |
| `cargo run --offline --example ui_review -- target/ux-review/numeric-12` | PASS; synthetic import, invalid input, defaults, pause of active after another job is selected, resume to COMPLETED, export and input/export checksum |
| `cargo run --offline --example ui_review -- --latency target/ux-review/numeric-12/latency.json` | PASS; 64 samples per scenario. Selection + SQLite refresh + render p95 60.938 ms with 150 jobs and 63.729 ms with a selected million-record job. Home CPU + TestBackend frame/diff/flush p95 0.234–0.787 ms across both scenes and four sizes. These are local synthetic timings, not native Ghostty paint or production capacity. |
| `python3 /tmp/pgm-numeric-12-pty-check.py` | PASS, seven checks: home, Settings, Secrets tab, masked literal paste, normal exit, alternate-screen/bracketed-paste restoration, no literal synthetic secret in output. Own isolated PTY and loopback fixture. |
| `python3 /tmp/pgm-numeric-12-release-pty.py` | PASS, the same seven navigation/masking/exit checks on the actual release binary with a fresh temporary data home; no real jobs, provider calls or Keychain writes. |
| `gitleaks detect --no-git --redact --source src --report-format json --report-path target/ux-review/numeric-12/gitleaks.json` | PASS, no leaks found |
| `ghostty +validate-config --config-file=packaging/ghostty/pgm-ansi.ghostty` | PASS using installed Ghostty CLI. Font listing previously confirmed IBM VGA 8x16. |
| `cargo audit` | PASS for unchanged Cargo.lock: 380 dependencies, 1,310 advisories, zero vulnerabilities/warnings; database last updated 2026-10-13. Original report: `target/ux-review/numeric-6/dependency-audit.json`. |

The 156 numeric-12 PNGs are generated from the actual Ratatui TestBackend with the licensed VGA font. They cover 120×40, 100×30, 80×24 and 40×18; buffer tests also cover smaller fallback. They are rendered evidence, not screenshots of a native Ghostty window. The native-app tool denied Ghostty access. No real credential, Keychain write or PicPay request was used in validation.

Meaningful regressions cover rotation and reduced motion, settings tabs/cursors/drafts, shared-profile Save, notices/errors scoped by group, foreground/background contrast, malformed API/processing preference recovery without content disclosure, old preference compatibility, import override precedence, help shortcuts, current-job locks, operation-origin messages, large terminal-state counters and variable-height batch pagination, full hash comparison, literal paste and graceful pause/shutdown.

### Ordered implementation review verdicts

| Review | Verdict |
| --- | --- |
| Sentinel | GO for measured local functional and failure checks. T4 coverage/mutation floors are **not passed**: the exploratory 65-unit-test run on an earlier numeric source covered 185/332 changed branch outcomes (55.72%); it is not a final-source or whole-package result. Mutation and MC/DC remain unmeasured. |
| Helmsman | GO for bounded geometry, local frame/navigation timings, synthetic fixture failure paths and terminal restoration. No production peak multiplier, canary or production rollback drill is claimed. |
| Bastion | GO for local preserved auth gates, exact metadata-profile matching, masked/zeroized secret entry, safe paste/argv and scans. Independent human security release approval and signed SBOM remain pending. |
| Beacon | GO for measured local counters, origin-bound results, safe fixture output and timings. External collectors, production alerts and native paint were not measured. |
| Inquisitor | GO for the local testable feature; NO-GO for publication readiness under the unverified coverage/mutation/security/distribution gates. No publication was requested. |
| Scribe | GO for exact launch instructions, English UI scope, versioned evidence and explicit limits. Nielsen results are recorded separately without rounding to the target. |
| Conductor | Publication N/A: no Git metadata, commit, PR, CI promotion or production release in this task. |

### Local Definition of Done

- [x] T4 classification, plan, ADR, threat model, ordered instruction reviews and retained source rollback.
- [x] Requested numeric art, bordered English home/Jobs, tabbed Settings, font/motion choices, real progress, threads, actionable errors and checksum implemented.
- [x] Format, check, lint, 114 tests, build, scans, rendered evidence, synthetic runtime and local performance checks pass.
- [x] Three independent Nielsen final evaluations above the requested threshold on the same unchanged source: A 9.87, B 9.84, C 9.895; aggregate 9.868333… without rounding for approval. Own-test provenance and low-priority recommendations are recorded in [evaluation history](ux-evaluation.md).
- [ ] Native Ghostty visual acceptance, final-source coverage/mutation floors, independent human security and provider acceptance remain outside the completed verification. This local delivery makes no release-readiness claim.

The numeric-9 counter truncation identified by evaluator B is fixed in this snapshot. The summary expands only when required, and batch page size follows its actual height. Full counters are covered at one million, twenty million and the integer limit. All earlier scores remain in the evaluation history; current scores must refer to numeric-12. The advanced prompt and launcher error are in English. Extreme error layouts preserve all numbers and useful content; detail scrolling advances by at most the visible row count.

## 2026-10-14 — Slash commands and macOS keyboard shortcuts

The terminal will offer `/` commands with a filtered suggestion menu and Ctrl shortcuts so operators can reach help, panels, API configuration and job actions without function keys or macOS workspace shortcuts. The implementation stays in the terminal port and delegates to the existing command executor; it does not change API contracts, credentials, job persistence or remote execution policy.

- Classification: T2; estimated delta at most 800 added/removed lines, nine files, one Rust package; local terminal blast radius; no new dependencies, network boundaries, schema changes or production data.
- Constraints: preserve plain CLI commands, optional F-key aliases, secret masking/history filtering, configuration form focus, graceful shutdown and job guards. No installation or publication.
- Placement: private `src/tui/commands.rs` owns the finite command catalogue, completion and shortcuts. `src/tui.rs` retains execution; `src/tui/view.rs` renders the menu using existing Ratatui widgets. Command tests live in `src/tui/commands_tests.rs`.
- Affected existing files: `src/tui.rs`, `src/tui/view.rs`, `src/cli.rs`, `README.md`, `docs/operacao.md`, this ledger. Existing `src/tui/tests.rs` tests remain compatibility checks.
- Design: `/help`, `/panel`, `/api`, `/import`, `/jobs`, `/status`, `/run`, `/pause`, `/resume`, `/merge`, `/doctor`, `/quit`. Prefix filtering; arrows select; Tab accepts; Enter accepts an incomplete name and executes an exact name; Escape dismisses before clearing. Import and merge insert editable drafts. Existing `/job ...` and `/auth ...` syntax delegates to the unchanged CLI parser.
- Shortcuts: Ctrl+G help, Ctrl+P panel, Ctrl+O API, Ctrl+N CSV, Ctrl+R run, Ctrl+X pause, Ctrl+B/F previous/next job, Ctrl+U clear input; Ctrl+C retains graceful exit. Avoid Ctrl+H/I/J/M and Ctrl+S/Q terminal control aliases.
- Acceptance: menu filtering/selection/scrolling and no-match feedback; no action from Tab or incomplete Enter; job-required actions fail without selection; normal history remains usable; dismissed menu reopens on edits; form typing remains isolated; no secret in suggestions/screens/history; small terminals render without panic.
- Test plan: table-driven aliases/shortcuts plus keyboard, render and existing configuration/job regression tests; real PTY smoke against an isolated temporary data home; no provider calls.
- Verification: `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test --all-targets --all-features`; `cargo build --release`; Python PTY smoke. Measure coverage if available, otherwise record it as unmeasured.
- Rollback: restore the task's temporary source backup, remove only the two new command modules and rebuild. Preserve all application data.

Design consultations in order (skill instructions applied locally, no delegated agents):

| Order | Skill | Verdict / evidence |
| --- | --- | --- |
| 1 | architect | GO — retain terminal-port placement and existing executor; T-1, T-3, T-10; `src/tui.rs`, `src/repl.rs` |
| 2 | herald | GO — additive local input aliases; plain CLI, legacy shortcuts and wire contracts stay compatible; no public API change or deprecation |
| 3 | scribe | GO — this design narrative precedes code; operator instructions will name exact keys and completion semantics |
| 4 | forge | GO — static catalogue, bounded selection, existing widgets and parser; no dependency or engine changes |
| 5 | sentinel | GO for test plan — table-driven positive/negative aliases, menu/history transitions, secret and terminal-size regressions; verification results pending |

The shared thresholds and ADR triggers were recovered at `codecompany-claude-skills/skills/_shared/`; the installed skill symlinks do not expose them at `~/.claude/skills/_shared/`. No ADR trigger fires for additive private terminal input. Publication, CI and human-review gates are N/A for this local folder without Git metadata. Coverage floors will only be claimed when measured.

- Source rollback backup: `/var/folders/14/52y2hz4j46s7_h04n92vcrx00000gn/T/pgm-data-slash-backup-tdq_pktb`.
- bastion: GO for local input — static suggestions, 8,192-byte input cap, existing parser/job/busy guards, secret-safe rendering/history and isolated form focus; threat-model delta in `docs/threat-models/0002-api-configuration.md`.
- beacon: GO for local scope — existing job counters/events and command errors remain the evidence. Per-keystroke logs/metrics/traces are inappropriate for local text editing, can expose secrets and add no operation boundary (T-6 permits a written reason). No new I/O or engine operation needs instrumentation.

### Verification and local Definition of Done

- Final classification remains T2: 11 source/test/document files, under 800 added/removed lines against the task backup; one Rust package. The security delta and CLI-message regression added two files to the original estimate. Changed Rust files stay below 500 lines; added source lines stay below 120 columns.
- `cargo fmt --all -- --check`: PASS. `cargo clippy --all-targets --all-features -- -D warnings`: PASS.
- `cargo test --all-targets --all-features`: PASS, 79 tests (40 unit, 2 chaos, 10 configuration, 5 OAuth, 2 package, 20 pipeline). Loopback fixture binding requires running outside the filesystem/network sandbox; the initial PermissionDenied was environmental. Final string formatting also passed all 40 library tests.
- `cargo build --release`: PASS; launch with `./target/release/pgm-data tui`.
- `python3 /var/folders/14/52y2hz4j46s7_h04n92vcrx00000gn/T/pgm-data-slash-checks-xwrm7ajv/pty-smoke.py`: PASS, 20 checks on the final release binary; menu filtering, scrolling, Tab, form cancellation/masking, Ctrl shortcuts, resize, /quit, Ctrl+C and terminal restoration. Synthetic temporary data only. Capture: `target/slash-menu.txt`; results: `target/slash-pty-smoke.json`.
- Coverage: `cargo test --offline --lib --message-format=json` through the temporary Rust compiler wrapper (`-C instrument-coverage -Z coverage-options=branch`), then bundled `llvm-profdata merge -sparse` and `llvm-cov export`. PASS: 76/86 changed branch outcomes, 88.37%; command module line coverage 95.21%. Report: `target/slash-coverage/summary.json`. This measures unit tests and changed branch source locations; whole-package coverage and mutation score are unmeasured.
- sentinel: GO for local behavior and the T2 changed-branch floor. bastion/beacon: GO for the documented local scope. inquisitor: GO for local unpublished implementation; corrected form-before-menu dispatch and added regression coverage for Ctrl+U, Ctrl+F and Escape with a pending slash query. scribe: GO for exact operator instructions and bounded claims. conductor: publication N/A; no Git metadata, commit, PR, CI or installation was requested/performed.

- [x] T2 classification, problem, constraints, acceptance criteria, placement, plan and ordered consultations recorded.
- [x] Slash commands, autocomplete/menu, replacement shortcuts and backwards compatibility implemented and tested.
- [x] Format, lint, tests, release build, real terminal smoke and changed-branch coverage at least 85% pass.
- [x] Input/security review, telemetry rationale, self-review, documentation and source-backup rollback recorded.
- [ ] Whole-package coverage at least 80% and mutation floors remain unmeasured. This local result does not claim publication readiness; Git/commit/PR/CI/human-approval/merge/branch-deletion gates are N/A here.

## 2026-10-13 — API configuration and terminal UX

- Classification: T4; signal: explicitly enabled external calls on the processing path and destination/identity authorization checks. Expected scope: under 2,500 added/removed lines, one local Rust package; no SQLite migration or production data mutation during this task.
- Goal: colored terminal UI, discoverable credential/URL configuration through Clap and F3, saved settings inherited by new jobs, default batches of 1,000 records.
- Non-goals: provider response invention, live PicPay requests, distribution signing, publishing, installation, credential rotation, rewriting existing jobs.
- Files: `src/{cli,engine,jobcfg,httpcall,repl,tui}.rs`, `src/tui/{configure,view}.rs`, `tests/{configuration,pipeline}.rs`, README, operation guide, ADR 0007, threat-model delta, this ledger.
- Constraints: preserve Keychain-only secret storage and old CLI commands; use confirmed existing URLs; remote defaults OFF; no added dependencies; no Git repository exists in this folder.
- Design: API/profile persistence remains in `App`; ports render and collect input. See ADR 0007 for acceptance criteria, network boundary, rollout, and rollback.
- Verification: `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test --all-targets --all-features`; `cargo build --release`; local synthetic benchmark; terminal smoke test.
- Source rollback backup: `/var/folders/14/52y2hz4j46s7_h04n92vcrx00000gn/T/pgm-data-ui-backup-j9avdt2g`.

Design consultations, in order (performed through the skill instructions; no delegated agents):

| Order | Skill | Verdict / decision |
| --- | --- | --- |
| 1 | architect | GO — retain existing App/port/HTTP/Keychain placement; no context split |
| 2 | curator | GO — batch = up to 1,000 source records, not attempts; UNKNOWN is not a customer verdict |
| 3 | herald | GO for additive CLI/profile API; production response interpretation still NEEDS-INFO; no breaking HTTP contract |
| 4 | migrator | GO — additive serde flag defaults OFF; no SQL/schema/backfill or rewriting old jobs |
| 5 | bastion | GO for design — exact enabled-profile checks, HTTPS, hidden input, no plaintext secret persistence |
| 6 | sentry | GO for design — pre-mortem and minute 5/15/60 actions in threat model; no live incident |
| 7 | helmsman | GO for local design — bounded existing workers/timeouts; disable remote and drain for rollback; production ramp not executed |
| 8 | beacon | GO for local design — TUI shows API access, job counters, progress and batches; no secrets in events |
| 9 | scribe | GO — ADR 0007 and threat-model delta written before implementation |
| 10 | forge | GO for implementation plan — existing concrete config, shared App, no new dependency |
| 11 | refactory | GO — existing suite characterizes current behavior (55 passing tests); constrain extraction to terminal rendering/form |
| 12 | sentinel | GO for test plan — profile/URL/identity rejection matrix, masked/cancelled input, exact batch boundaries, existing OAuth/failure tests |
| 13 | usher | GO for terminal design — high-contrast palette, labels plus colors, keyboard navigation; screen-reader result not claimed |

Shared TENETS/THRESHOLDS/ADR_TRIGGERS reference files are absent from the installed skill folders; the skill-local thresholds and this ADR govern the recorded scope. Git/PR/CI/merge/release gates do not apply to local unpublished edits; production sign-offs, coverage/mutation floors, and staged rollout must not be represented as completed.

### Verification and review — 2026-10-14 UTC

The local implementation passes the following checks. The operator still owns provider acceptance and production rollout; no actual credential or PicPay call entered the tests.

- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --all-targets --all-features -- -D warnings`: PASS.
- `cargo test --all-targets --all-features`: PASS, 72 tests (33 unit, 2 chaos, 10 configuration, 5 OAuth, 2 package, 20 pipeline).
- `cargo build --release`: PASS; `target/release/pgm-data` rebuilt from final source.
- `cargo audit`: PASS; 380 dependencies, 1,310 advisories; no added Cargo dependencies.
- `./target/release/pgm-bench --rows 150000`: PASS against the local synthetic fixture, COMPLETED, 150,000 successes; prepare 623 ms, run 7,049 ms, merge 124 ms, final RSS 111,738,880 bytes. This is not a PicPay capacity result.
- PTY smoke: PASS at 120×32; F3, keyboard input, masking, Escape cancellation, Ctrl+C, terminal mode restoration, no secret in history and no configuration/Keychain writes. The harness removes NO_COLOR to test the requested palette.
- Render review: PASS at 120×34; standard 120×24 test also verifies the saved API URL and UNKNOWN count stay visible. Demo preview: `target/ui-preview.png` (synthetic display, not a live account).
- Contrast calculation: all six foreground colors meet 4.5:1 against the application background; minimum 8.09:1. Native screen-reader acceptance and full localization were not measured/implemented.
- Final scope adds private CLI prompt, App configuration, terminal/session/render/form/test modules to keep UI files below 500 lines; also changes `src/main.rs` for secret-safe argv errors and `src/runner.rs` so ambiguous 5xx still trips the breaker. Tests and docs explain the observable changes. All original files have a rollback copy at the path above.
- Final consults: sentinel GO for measured functional/contract/local-load checks (coverage and mutation unmeasured); bastion GO for local guards and audit (independent sign-off pending); beacon GO for local counters and UI; inquisitor GO for local use, NO-GO for production readiness; scribe GO for operator docs and ADR; conductor N/A for publication because this directory has no Git metadata and no publication was requested.

Definition of Done, T4 scope:

- [x] Classification, plan, ordered consultations, ADR, threat-model delta, rollback, and provider-boundary decisions recorded.
- [x] UI/configuration/batch behavior implemented; local format, lint, tests, build, dependency audit, rendering, keyboard, failure and load checks pass.
- [x] Backward-compatible CLI usage and disabled-by-default old profiles tested; old job configurations/schema require no migration or data recovery.
- [ ] Coverage/mutation floors, independent security/human review, full accessibility acceptance, signed SBOM/distribution, and production provider contract validation remain unmeasured/pending; no release-readiness claim.
- [ ] Production staged ramp, synthetic production probes, paging, reconciliation window, and production rollback drill not executed. There is no production rollout in this task. Git/PR/CI/merge gates are N/A for this local folder.

- classification: tier=T4
- signals: new system, >2500 LOC, auth and secrets, SQLite schema, external HTTP on the hot path, concurrency pool, macOS package
- time: 2026-10-13

## Consults

| Order | Skill | Verdict |
| --- | --- | --- |
| 1 | architect | GO — one process, ACL at the HTTP boundary |
| 2 | curator | GO — glossary; `is_customer` is not implied by HTTP |
| 3 | herald | GO for the simulated fixture; NO-GO for production interpretation |
| 4 | migrator | GO — schema v1 up/down; no destructive production migration |
| 5 | bastion | GO for the local design with remote execution blocked |
| 6 | sentry | GO — pre-mortem in `docs/operacao.md`; no live incident |
| 7 | helmsman | GO — jittered retry, breaker, graceful stop; no cloud canary in this version |
| 8 | beacon | GO — JSON-line log and in-process p50/p95; the TUI is the dashboard |
| 9 | scribe | GO — ADRs and the operator note |
| 10 | pathfinder | GO — bounded queues, one SQLite thread, fixed worker count |
| 11 | steward | GO — transition tables |
| 12 | oracle | GO — partial indexes; EXPLAIN test looks for `idx_records_pending` |
| 13 | quartermaster | GO — exact versions, no extra HTTP framework |
| 14 | sentinel | GO for the simulated suite; mutation score not measured |
| 15 | chaos | GO for injected commit failure, lock conflict, prepare interruption |
| 16 | release-captain | NO-GO for a notarized release; local artifacts only |
| 17 | inquisitor | GO for simulated use; NO-GO for production readiness |

Substitutions: none. Topologist does not apply (one disk, one process). Ledger: API unit cost is unknown because the remote call is disabled.

## Measured on this machine

Local fixture only. These numbers are not a PicPay capacity claim.

100 000 rows, 2026-10-13, release binary, concurrency 32, RPS cap 20 000, `synchronous=FULL`:

- prepare 423 ms
- run 5 077 ms (about 19 700 completed calls per second over that interval)
- merge 101 ms
- RSS at the end: 85 934 080 bytes
- state COMPLETED, succeeded 100 000

20 000 000 rows, same binary and caps, wall clock `real 2296.44 s`:

- synth 88 003 ms
- prepare 106 590 ms
- run 2 069 502 ms (20 000 000 / 2 069.502 s ≈ 9 664 completed calls per second over the whole run)
- merge 29 687 ms
- RSS at the end, from `ps`: 346 800 128 bytes
- job directory: 5 933 439 788 bytes
- state COMPLETED, succeeded 20 000 000
- merged sha256 `ede36d9f152646335df213f197de2bd1936c29e0096f345875e64daf7fe4a4ff`

Commit p50/p95 and HTTP p50/p95 were not printed by this harness. The run used the default group commit of 500 rows or 250 ms and `synchronous=FULL`.

## Not done

- Provider validation of the real endpoint.
- Mutation testing.
- Developer ID Installer signature and notarization. The executable is signed with Developer ID Application. The PKG is unsigned. Gatekeeper approval was not checked.
- Independent human security sign-off.
- Staged production ramp. There is no production path to ramp.

## New Job Settings style — 2026-10-14

Classification: **T2**, one package, about 300 added/removed lines across five task files. [The written plan](new-job-ui-plan.md) preceded source edits. The design consultations were Architect, Herald, Scribe and Forge, all GO for private presentation changes. These verdicts apply specialist instructions locally; they are not independent agent or human sign-offs.

New Job now uses the exact Settings field widget: individual borders, selected title/value contrast, safe text and cursor windowing. It also uses the branded outer border, field position hint, Create job/Back action row and wide Context panel. Compact layouts keep the focused field visible. Import parsing, defaults, Enter/Escape/Tab, job persistence and execution handlers were not changed.

Verification artifacts: `target/ux-review/new-job-style/`.

- `cargo fmt --all -- --check`, `cargo check --offline --all-targets`, `cargo test --offline --all-targets --all-features`, `cargo clippy --offline --all-targets --all-features -- -D warnings`: PASS. Tests: 114 (74 unit, 40 integration); existing meaningful suites were retained without adding style-mirroring tests.
- `cargo build --offline --bin pgm-data`, `cargo build --offline --release --bin pgm-data`: PASS. Normal, uninstrumented binaries were restored after the coverage experiment; the normal `ui_review` example was also rebuilt.
- `cargo run --offline --example ui_review -- target/ux-review/new-job-style`: PASS. Filled/error layouts reviewed at 120×40, 100×30, 80×24 and 40×18. All 20 Settings SVGs (five states × four sizes) are byte-identical to the immediately preceding numeric-12 implementation. Generated VGA PNGs are TestBackend renders, not native Ghostty screenshots.
- Synthetic interactive PTYs, 100×30 and 40×18: PASS, 24 assertions total. Empty/missing path errors, path containing spaces, field borders, threads editing, third-field reachability, validation cycling, Escape without creating, Enter local creation and terminal restoration passed. Temporary data and MemorySecrets only; no real provider requests or Keychain changes. The first driver attempts needed startup synchronization and a page-title assertion that did not match the home menu label; final controls passed without source changes.
- `gitleaks detect --no-git --redact --source src --report-format json --report-path target/ux-review/new-job-style/gitleaks.json`: PASS, no leaks.

Post-implementation consultations, in order:

1. **Sentinel — GO for local functional delivery.** LLVM instrumentation of the 74 unit tests plus the real render/loopback harness measured `form_field.rs`: 100% lines, 91.67% branches; `new_job.rs`: 86.96% lines, 50% branches. Combined new-module scope: 89.47% lines, 70.83% branches. This does not meet the T2 changed-branch floor; package coverage and mutation are unmeasured. No merge or publication readiness is asserted.
2. **Bastion — GO for this delta.** Settings supplies masked values to the shared widget; existing safe text/cursor handling was preserved. No auth, secret-storage, API or trust-boundary change; existing `docs/threat-models/0001-tabbed-terminal-settings.md` remains applicable.
3. **Beacon — GO with presentation-only T6 rationale.** The new paths only draw existing state and add no I/O or operation. Existing import events remain the operation evidence. Per-frame logs/metrics/traces would add render overhead and risk recording paths or sensitive field values without a new operational decision to support.
4. **Inquisitor — GO for local rendering/control consistency.** Reviewed the final private extraction and removed flat import branch, inspected four-size renders, Settings parity and PTY evidence. Full publishing gates are not satisfied; no prior Nielsen score is reused for this revision.
5. **Conductor — N/A.** This directory has no Git metadata; no commit, PR, CI or publication was requested. Independent reviewer approval and merge cleanup were not performed.

T2 local Definition of Done:

- [x] Written classification, placement and design plan; ordered scoped consultations.
- [x] Shared Settings/New Job field style implemented; controls and masking preserved.
- [x] Format, check, 114 tests, lint, debug/release builds, four-size render and two-size PTY verification.
- [x] Coverage evidence and its limits recorded; local review and rollback available.
- [ ] Full T2 publishing DoD: changed-branch floor unmet; package coverage, mutation, CI and independent reviewer approval unverified. Publication steps are outside this local request.

Rollback snapshot: `/var/folders/14/52y2hz4j46s7_h04n92vcrx00000gn/T/pgm-new-job-style-before-jrb75db0`. Restore only the saved `src/tui/view.rs`, remove the new `src/tui/view/form_field.rs` and `src/tui/view/new_job.rs`, then rebuild offline. Preserve jobs, preferences and Keychain. Source/artifact hashes are recorded in `target/ux-review/new-job-style/source-manifest.json`.

## Smaller ASCII geometry and aligned spacing — 2026-10-14

Classification: **T2**, one Rust package; rendering-only scope exceeds 200 added/removed lines. [The written plan](ansi-spacing-plan.md) preceded implementation. Architect, Herald, Scribe and Forge returned GO for the private design through self-applied specialist instructions. No delegated or human sign-off is claimed.

`src/tui/art.rs` now applies a uniform **0.70** projection scale, adapts stroke sampling to projected edge length, and chooses single-column ASCII `- _ | / \\ . :` by direction/depth/subcell position. Hexadecimal vertex markers preserve the numeric character of both shapes. Rotation, reduced motion, serialized scene choices and bounded Canvas allocation remain compatible. The raster bound is at most 160×60 cells, 3,456 stroke samples and 27 vertex markers, independent of job data.

`src/tui/view.rs` and `src/tui/view/new_job.rs` share equal horizontal content margins, a field/Context column gutter and note padding aligned with field values/actions. The home art and its shorter Navigation box occupy balanced columns with matching centers; footer shortcuts are centered. Status rendering reserves two lines for active/pending operations, and Jobs detail pagination calculates the same inset as its renderer.

Artifacts: `target/ux-review/ascii-spacing/` contains command logs, 156 SVG/PNG TestBackend renders, ASCII-footprint measurements and PTY captures. At 100×30 the default home art shrank from **31×16 to 21×11 occupied cells**; the lattice changed from 41×19 to 27×13. Uniform scale combines with the new column fit, so cell bounds are not an exact percentage transformation after rounding. PNGs are synthetic VGA renders, not native Ghostty screenshots.

Verification:

- `cargo fmt --all -- --check`: PASS.
- `cargo check --offline --all-targets`: PASS.
- `cargo test --offline --all-targets --all-features`: PASS, 114 tests (74 unit + 40 integration). The existing art behavior test now verifies printable ASCII, numeric vertices, directional strokes, changing motion and identical reduced-motion frames. Existing norm/projection, import, keyboard, masking, compact recovery and batch paging tests pass.
- `cargo clippy --offline --all-targets --all-features -- -D warnings`: PASS.
- `cargo build --offline --bin pgm-data --example ui_review`: PASS.
- `cargo build --offline --release --bin pgm-data`: PASS.
- `cargo run --offline --example ui_review -- target/ux-review/ascii-spacing`: PASS. Home, Settings and New Job inspected at 120×40, 100×30, 80×24 and 40×18; animated home, Appearance preview and compact validation errors also inspected.
- Synthetic PTYs at 100×30 and 40×18: PASS, 24 assertions total, including field reachability, threads editing, validation cycling, path with spaces, missing/empty errors, cancel without creating, local creation, normal exit and terminal restoration. Fixtures use temporary data and MemorySecrets.
- `gitleaks detect --no-git --redact --source src --report-format json --report-path target/ux-review/ascii-spacing/gitleaks.json`: PASS, no leaks. No dependency or credential-storage change.

Final consultations, in order:

1. **Sentinel — GO for local functional delivery.** Existing meaningful suites and synthetic render/PTY evidence pass. Branch/package coverage and mutation were not measured for this revision; full T2 publishing floors are not asserted.
2. **Bastion — GO for this presentation delta.** Safe text/cursor and secret masking remain in the unchanged shared field component. Inputs are not logged. There is no trust-boundary change; the existing tabbed-settings threat model remains applicable.
3. **Beacon — GO with presentation-only T6 rationale.** Rendering introduces no I/O operation or operational decision. Per-frame structured logs/metrics/traces would add render overhead and risk exposing field values; existing import/job operation events remain the evidence.
4. **Inquisitor — GO for local implementation.** Reviewed the projection/stroke changes, bounded sampling, shared geometry, matching pagination inset, four-size images and control evidence. Earlier Nielsen ratings are not reused. Independent review and publication readiness are not claimed.
5. **Conductor — N/A.** No Git metadata, publication request, commit, PR or CI execution in this task.

Local T2 Definition of Done:

- [x] Classification, placement, written plan and ordered scoped consultations.
- [x] Smaller ASCII art, numeric vertices and aligned shared spacing implemented.
- [x] Format, check, 114 tests, lint, normal debug/release builds and leak scan.
- [x] Four-size render inspection, measured smaller footprints and two-size PTY controls.
- [x] Review scope, measurement limits and rollback recorded.
- [ ] Full publishing DoD: branch/package coverage, mutation, CI and independent reviewer approval unmeasured; publication is outside this local request.

Rollback snapshot: `/var/folders/14/52y2hz4j46s7_h04n92vcrx00000gn/T/pgm-ascii-spacing-before-i2kgf62_`. Restore only `src/tui/art.rs`, `src/tui/view.rs` and `src/tui/view/new_job.rs`, then rebuild offline. Preserve jobs, preferences and Keychain. Final source hashes and LOC classification are in `target/ux-review/ascii-spacing/source-manifest.json`.

## PartnerGetMember home — 2026-10-14

Classification: **T2**, one Rust package, fewer than 800 added/removed lines across eleven files; [written plan](partner-home-plan.md) recorded before implementation. Ordered Architect, Herald, Scribe and Forge consultations returned GO through self-applied specialist instructions; UX-copy informed the product headline and action descriptions. No independent agent/human sign-off is claimed.

Home now identifies **PartnerGetMember / PicPay partner data import**. It replaces Command Center and the floating Navigation box with a horizontal menu, a shaded PGM wordmark, import/batch/export guidance, real loaded workspace metrics, selected-job context/recent activity and a contextual status description. The menu distributes options across wide terminals, preserves compact labels and accepts compatible Left/Right aliases. Enter/J/N/S and existing forms still use their original handlers.

The bounded PGM mark has solid letter faces, two-cell extrusion, an animated highlight and fixed reduced-motion output. An ANSI-256 green ramp supplies vivid highlights, face tones and darker depth, with the existing numeric geometry kept secondary. The review exporter preserves the indexed colors in SVG and ANSI evidence. Workspace totals use loaded job counters and saturating arithmetic without new I/O; API Enabled describes saved remote setup, not a verified connection or credential test.

Artifacts: `target/ux-review/partner-home/` contains 168 SVG/PNG TestBackend frames, command logs, PTY captures, contrast results and the final source manifest. Home empty/loaded/menu/motion states were inspected at 120×40, 100×30, 80×24 and 40×18; compact recovery is exercised by existing tests and import PTYs. These are synthetic VGA renders, not native Ghostty screenshots. Earlier Nielsen scores are not applied to this revision.

Verification:

- `cargo fmt --all -- --check`: PASS.
- `cargo check --offline --all-targets`: PASS.
- `cargo test --offline --all-targets --all-features`: PASS, **115 tests** (75 unit + 40 integration). New meaningful coverage exercises descriptions through actual menu keys in four sizes, backward/wrapping selection, Enter dispatch, real job/batch/record/open/unknown counts and API setup state. Existing motion, masking, import/recovery, settings and paging checks pass.
- `cargo clippy --offline --all-targets --all-features -- -D warnings`: PASS.
- `cargo build --offline --bin pgm-data --example ui_review` and `cargo build --offline --release --bin pgm-data`: PASS.
- `cargo run --offline --example ui_review -- target/ux-review/partner-home`: PASS, synthetic loopback harness only.
- Synthetic PTYs at 100×30 and 40×18: PASS, **36 assertions** covering product copy, all three contextual descriptions, Left/Right navigation without creating jobs, import fields/errors/cancel/local creation, normal exit and terminal restoration. Temporary data and MemorySecrets only.
- Home text/highlight/status contrast: minimum **5.804:1** for measured color pairs. Decorative logo/mesh shadows use darker greens. Native accessibility/paint acceptance is unmeasured.
- `gitleaks detect --no-git --redact --source src --report-format json --report-path target/ux-review/partner-home/gitleaks.json`: PASS, no leaks.

Final consultations, in order: **Sentinel GO for local functional delivery**, measured tests/renders/PTYS pass; branch/package coverage and mutation unmeasured. **Bastion GO for this delta**, existing masking/safe text remain, home shows counters/setup state rather than identities/secrets; no trust-boundary change. **Beacon GO with presentation-only T6 rationale**, per-frame logs/metrics would add overhead or expose values, while existing operation events remain the evidence. **Inquisitor GO for local layout/control consistency**, final code, four-size frames, color encoding and control evidence reviewed; user visual acceptance and full publishing gates are not asserted. **Conductor N/A**, no Git metadata, commit, PR, CI or publication request.

Local T2 Definition of Done:

- [x] Classification, placement, written plan, copy decisions and ordered scoped consultations.
- [x] Product home, logo/green depth, menu descriptions and actual workspace data implemented.
- [x] Format/check, 115 tests, lint, normal debug/release builds and leak scan pass.
- [x] Four-size visual inspection, text contrast and two-size PTY verification pass.
- [x] Evidence limits, source hashes and rollback recorded.
- [ ] Full publishing DoD: branch/package coverage, mutation, CI and independent reviewer approval unmeasured; publishing is outside this local request.

Rollback snapshot: `/var/folders/14/52y2hz4j46s7_h04n92vcrx00000gn/T/pgm-partner-home-before-hsde1ck1`. Restore the seven saved task-owned files and remove only new `src/tui/view/home.rs`, `src/tui/brand.rs` and `src/tui/palette.rs`, then rebuild offline. Preserve jobs/preferences/Keychain. Final manifest: `target/ux-review/partner-home/source-manifest.json`.

## PicPay partner splash and ANSI dashboards — 2026-10-14

Classification: **T3**, one Rust package, fewer than 1,500 added/removed lines across fifteen changed task files. [The written plan](splash-dashboard-plan.md) and [ADR-0002](adr/0002-splash-and-partner-dashboards.md) preceded source edits. Ordered Architect, Curator, Herald, Migrator (N/A), Bastion design, Scribe, Helmsman design, Beacon design, Forge and Refactory consultations applied specialist instructions locally. These are self-reviews, not independent agent or human sign-offs. Prior Nielsen scores do not apply to this revision.

The visible identity is **PicPay Partners / PartnerGetMember**, with the exact subtitle **Import tools for PicPay Partners**. PGM.DATA and the giant Home wordmark were removed from visible TUI branding. Interactive startup now opens a splash with a shaded double-P payment chip, circuit pins and binary pulses in green; Enter immediately opens the workspace, Space controls motion, and Escape/Q/Ctrl+C quit. Decorative animation stays in Splash; Appearance shows a static preview. Persisted scene values, binary names and data paths remain readable. Splash ignores pasted or unrelated commands and Enter does not create or execute a job.

Home replaces the numeric table with a processed-record gauge, record-outcome distribution, job lifecycle bars and, when space permits, a selected-job batch map and activity panel. Charts consume loaded state without I/O or invented trends. Queue includes pending/retry-wait; active is in-flight; UNKNOWN remains separately labelled and does not become a confirmed outcome. Totals use nonnegative u128 aggregation and chart rounding conserves available width. The batch map describes its loaded selected-job slice/filter. Existing menu descriptions, Settings and New Job borders/controls remain.

Artifacts: `target/ux-review/splash-dashboard/` contains 184 SVGs and 184 VGA PNGs, command logs, render checks, scoped frame timings, PTY captures and the final source manifest. These are TestBackend renders and synthetic PTYs, not native Ghostty screenshots. Splash and empty/loaded/error Home layouts were reviewed at 120×40, 100×30, 80×24 and 40×18. Home SVGs are byte-identical across animation phases at all four sizes.

Verification:

- `cargo fmt --all -- --check`, `cargo check --offline --all-targets`, `cargo test --offline --all-targets --all-features`, `cargo clippy --offline --all-targets --all-features -- -D warnings`: PASS. **118 tests** (78 unit + 40 integration). Meaningful new checks cover splash transitions/ignored paste/quit, animated versus fixed Splash, phase-stable Home, clipped icon painting, exact chart allocation and UNKNOWN/large/negative counter semantics. Existing masking, import, processing, settings and paging suites pass.
- `cargo build --offline --bin pgm-data --example ui_review` and `cargo build --offline --release --bin pgm-data`: PASS. Normal, uninstrumented binaries are updated.
- `cargo run --offline --example ui_review -- target/ux-review/splash-dashboard`: PASS, temporary loopback/MemorySecrets harness only.
- Synthetic PTYs: **48 assertions PASS**. The 100×30 and 40×18 flows verify splash startup/input gating, Enter to Home, product copy, contextual menu descriptions, navigation without creating jobs, shared form fields, missing/empty path errors, paths with spaces, cancel, local creation, normal exit and terminal restoration. Separate Splash Escape/Ctrl+C runs also exit normally and restore the terminal. No real provider requests or Keychain changes.
- Scoped frame CPU/TestBackend timing: 12 scenarios × 64 samples, **225 loaded synthetic jobs** (1.5× the preceding 150-job scenario), across four sizes and Home/two Splash motifs. Worst p95 **1.194 ms**, maximum **1.374 ms**. This excludes native terminal painting and does not establish provider or production capacity.
- `gitleaks detect --no-git --redact --source src --report-format json --report-path target/ux-review/splash-dashboard/gitleaks.json`: PASS, no leaks. No dependency, auth, worker, schema or credential-storage change.

Final consultations, in order:

1. **Sentinel — GO for local functional delivery.** Tests, command logs, four-size renders and PTYs pass. Changed-branch/package coverage and mutation were not measured for this revision; full T3 publishing floors are not asserted.
2. **Helmsman — GO for local startup/render behavior.** Explicit continuation has no timer delay; clipping/compact recovery, reduced motion, quit/restoration and the scoped 1.5× synthetic load are verified. Fixed chip geometry and loaded-data chart work stay bounded. There is no deployment, canary, production rollback or provider load claim.
3. **Bastion — GO for this delta.** Existing masking/sanitization remain, splash paste/command gating is covered, and graphs contain counters/setup state rather than secrets or customer identities. Decorative pulses imply no connection/auth check. Existing `docs/threat-models/0001-tabbed-terminal-settings.md` applies; leak scan passes and no external trust boundary changed.
4. **Beacon — GO with presentation-only T6 rationale.** New widgets perform no operation or I/O. Reproducible renders, timings and PTY evidence cover local presentation; existing import operation events remain authoritative. Per-frame logs/metrics/traces would add overhead and risk recording field values without supporting a new operational decision.
5. **Inquisitor — GO for local implementation.** Reviewed final source, exact product copy, retained serialized choices, chart arithmetic/semantics, Splash control dispatch, four-size layouts and form/navigation evidence. No independent review, visual user acceptance or prior numeric-art rating is asserted.
6. **Scribe — GO.** Plan, ADR, command/result artifacts, measurement limits, local DoD and source rollback are recorded consistently.
7. **Conductor — N/A.** No Git metadata, publication request, commit, PR, CI execution or merge cleanup in this task.

Local T3 Definition of Done:

- [x] Classification, placement/context, domain invariants, written plan/ADR and ordered scoped consultations.
- [x] All six requested changes implemented: copy, splash, payment/circuit motif, compact identity, ANSI charts and visible rebranding.
- [x] Format/check, 118 tests, lint, normal debug/release builds and leak scan pass.
- [x] Four-size render review, phase-stable Home, scoped 1.5× load and 48 PTY assertions pass.
- [x] Final local review, evidence limits, hashes and reversible source rollback recorded.
- [ ] Full publishing DoD: coverage/mutation floors, CI and independent reviewer approval remain unverified; staging/canary/on-call/publication steps are outside this local request.

Rollback snapshot: `/var/folders/14/52y2hz4j46s7_h04n92vcrx00000gn/T/pgm-splash-dashboard-before-1x1j9lz2`. Restore only saved task-owned files, remove the two new `src/tui/view/splash.rs` and `src/tui/view/dashboard.rs`, and rebuild offline. Keep the new plan/ADR as evidence; never replace jobs, preferences or Keychain. The snapshot includes unchanged `settings_tests.rs`, which requires no restoration. Final source/artifact hashes and line counts: `target/ux-review/splash-dashboard/source-manifest.json`.

## Jade theme and menu interactions — 2026-10-14

Classification: **T2**, one package, fewer than 500 added/removed lines across sixteen changed task files. [The written plan](jade-interactions-plan.md) preceded code edits; Architect, Herald, Scribe and Forge consulted in order through self-applied specialist instructions. No independent agent/human review is claimed. The user requested this local presentation change; publication and production are outside scope.

Home now orders **New Job / Jobs / Settings**, with New Job initially selected. Enter and numeric shortcuts follow that order; N/J/S keep their existing actions. The graphite truecolor theme uses slate borders, layered dark panels, jade selection, mint highlights and separate record-state accents. The shared Settings/New Job field widget retains safe text, masking/cursor handling and legible inactive labels. Ghostty's bundled profile uses matching background, text, palette and cursor colors.

Titles in Home and Splash show a bounded color sweep while every character stays present. Menu, detail/Settings tabs and field borders ease their focus highlight to jade within eight existing 33 ms ticks, without delaying dispatch. One private, nonpersisted interaction timestamp reuses the existing event loop; no timer, task, dependency, operation or I/O is added. Appearance Off and `/motion` disable the effects. Dashboard values and geometry stay independent of decorative phases; this supersedes ADR-0002's static decorative Home title only.

Artifacts: `target/ux-review/jade-interactions/` contains 204 SVGs and 204 VGA PNGs, RGB/ANSI exports, logs, contrast/render/frame checks, synthetic PTYs and the source manifest. These are TestBackend renders, not native Ghostty screenshots. Reviewed empty/loaded Home, Splash, Settings, New Job and recovery layouts across 120×40, 100×30, 80×24 and 40×18. The visual pass corrected inactive field titles to use readable muted text separately from border color.

Verification:

- `cargo fmt --all -- --check`, `cargo check --offline --all-targets`, `cargo test --offline --all-targets --all-features`, `cargo clippy --offline --all-targets --all-features -- -D warnings`: PASS. **122 tests** (81 unit + 40 integration + 1 example). Tests cover menu order/default/numeric/shifted shortcuts, bounded settling/tick wrap, full Unicode title text across 400 phases, fixed reduced motion, unchanged dashboard cells and RGB export fidelity; existing import/processing/masking suites pass. Initial motion comparison sampled two positions after the title sweep had ended; final checks compare an idle and an active phase.
- `cargo build --offline --bin pgm-data --example ui_review` and `cargo build --offline --release --bin pgm-data`: PASS, normal uninstrumented binaries updated. `cargo run --offline --example ui_review -- target/ux-review/jade-interactions`: PASS, isolated loopback/MemorySecrets fixtures.
- `python3 /tmp/pgm-jade-interactions-pty.py`: PASS, **52 assertions** across 100×30 and 40×18, including resize/restore, splash gating, default Enter to New Job, reordered descriptions, navigation without creating jobs, field validation/cancel/local creation and terminal restoration. No real provider requests or Keychain changes.
- Four-size render checks: reduced-motion Home frames are byte-identical across phases; focus frames differ. Required text/status/focus colors: **minimum 5.453:1 across 19 measured pairs**. Decorative border/art shadows are excluded. Local TestBackend timing: 12 scenarios × 64 samples, 225 synthetic jobs; worst p95 **1.282 ms**, maximum **1.409 ms**. This excludes native paint/provider capacity.
- `gitleaks detect --no-git --redact --source src --report-format json --report-path target/ux-review/jade-interactions/gitleaks.json`: PASS, no leaks.

Final consultations, in order: **Sentinel GO for local functional delivery**, tests/renders/PTYS pass; changed-branch/package coverage and mutation are unmeasured, so full T2 publishing floors are not asserted. **Bastion GO for this delta**, no trust-boundary/auth/dependency change; existing tabbed-settings threat model, masking/sanitization and explicit request controls remain. **Beacon GO with presentation-only T6 rationale**, effects add no operation/I/O; reproducible frames/timings/PTYS cover local behavior, and per-frame field logging would add overhead/exposure without an operational decision. **Inquisitor GO for local implementation** (T-1/T-3/T-8/T-10), final tests/diff/private placement/bounded effects/four-size controls reviewed; new effect functions and shared field renderer fit the 40-line cap. **Conductor N/A**, no Git metadata, commit/PR/CI/merge or publication request.

Local T2 Definition of Done:

- [x] Classification, placement/contract notes, written plan and ordered scoped consultations.
- [x] New Job first, graphite/jade theme and shared text/focus interactions implemented.
- [x] Format/check, 122 tests, lint, normal debug/release builds and leak scan pass.
- [x] Four-size render/motion/contrast checks, scoped timings and 52 PTY assertions pass.
- [x] Final local review, evidence limits, hashes and source rollback recorded.
- [ ] Full publishing DoD: coverage/mutation floors, CI and independent reviewer approval unverified; publishing is outside this local request.

Rollback: `/var/folders/14/52y2hz4j46s7_h04n92vcrx00000gn/T/pgm-jade-interactions-before-e63r5fx2`. Restore only the fourteen saved task files, remove new `src/tui/view/motion.rs`, and rebuild offline within ten minutes on a control/readability/motion regression. Keep the new plan as evidence; preserve jobs/preferences/Keychain. Final source/artifact hashes and counts: `target/ux-review/jade-interactions/source-manifest.json`.

## Partner title and one-line shortcuts — 2026-10-14

Pre-edit classification: **T1**, estimated fewer than 160 added/removed lines across five files (`src/tui/copy.rs`, `src/tui/view/home.rs`, `src/tui/view/splash.rs`, `src/tui/ux_tests.rs`, this ledger), one Rust package. The user explicitly requests the title **Partner Get Partner**, no splash pause hint, and Home shortcuts in one tab-separated row. Presentation-only scope; no public API, persistence, auth, dependency, network or execution change. Git metadata/publication are absent.

**Forge design — GO** (self-applied specialist instructions, T-1/T-10): update the shared title, retain the splash continuation/quit affordances, reserve one Home footer row and expand tab separators to terminal columns before painting. Use compact shortcut labels at 40 columns so quit/slash/help remain visible on that same row. This avoids rendering literal terminal control characters or clipping essential controls. Existing keyboard/form handlers stay authoritative.

Written plan before source edits: (1) update English copy and remove only the splash pause/resume hint; (2) render tab-aligned Home shortcuts in one row, using compact labels where required; (3) extend the existing viewport/shortcut checks to cover title, same-row controls and hint absence, then verify and review. Roll back task-owned source from a temporary snapshot and rebuild offline; preserve jobs/preferences/Keychain. Sentinel → Inquisitor → Conductor follow in order; verdicts and the snapshot path will be appended here. These are local self-reviews, not independent reviewer approvals.

Verification commands: `cargo fmt --all -- --check`; `cargo test --offline --all-targets --all-features`; `cargo clippy --offline --all-targets --all-features -- -D warnings`; `cargo build --offline --bin pgm-data --example ui_review`; `cargo build --offline --release --bin pgm-data`; `cargo run --offline --example ui_review -- target/ux-review/one-line-shortcuts`. Inspect 120×40, 100×30, 80×24 and 40×18 Home/Splash renders. Full publishing coverage/CI/reviewer gates are unmeasured or outside this local request.

Result: shared Home/Splash title is **Partner Get Partner**; Splash shows only `Esc quit` below its continuation action; Home shortcuts occupy one row with eight-column tab stops. Compact labels keep Enter, Q, slash and help on that same row. The extra Home row returns to dashboard content.

Post-implementation consultations, in order: **Sentinel GO for local functional delivery**, 122 tests (81 unit + 40 integration + 1 example), including the extended four-viewport same-row/title/hint checks; format, Clippy, normal debug/release builds and the render harness pass. **Inquisitor GO for local scope** (T-1/T-8/T-10), reviewed the five-file delta, explicit tab expansion, compact labels, retained command handlers and Home/Splash evidence. **Conductor N/A**, no Git metadata or publishing request; CI/coverage floors and reviewer approval remain unmeasured for publication. Reviews are self-applied, not independent sign-offs.

Artifacts: `target/ux-review/one-line-shortcuts/verification-commands.json`, `copy-layout-checks.json`, `render-checks.json` and 204 SVG/PNG TestBackend renders. Copy/layout checks pass at 120×40, 100×30, 80×24 and 40×18; compact Home and Splash were also visually inspected. No native Ghostty capture is claimed. No extra padding/color-mirroring tests or provider calls were added.

Local T1 DoD:

- [x] Classification, written plan and ordered Forge/Sentinel/Inquisitor/Conductor consultations.
- [x] All three requested presentation changes implemented with compact controls visible.
- [x] Format, 122 tests, lint, normal debug/release builds and four-size render checks pass.
- [x] Review, evidence limits and reversible source snapshot recorded.
- [ ] Full publishing DoD: coverage/CI/reviewer gates unverified; no publication requested.

Rollback snapshot: `/var/folders/14/52y2hz4j46s7_h04n92vcrx00000gn/T/pgm-one-line-shortcuts-before-e25haddv`. Restore the four saved source/test files and rebuild offline; keep this task record and preserve jobs/preferences/Keychain. Final hashes/counts: `target/ux-review/one-line-shortcuts/source-manifest.json`.
