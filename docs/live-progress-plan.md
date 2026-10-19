# Operation progress instead of a generic busy notice

2026-10-16. Tier T3: bounded progress channel across existing asynchronous/network
and blocking matching work. Goal: expose the real phase, quantities, elapsed time,
last HTTP failure and retry state without falsely labelling remote waits as local
matching. No server repair, automatic preference/job changes, new pool, dependency,
auth/crypto policy change, DAT persistence or animation redesign.

Read-only diagnosis before edits: current job 4b745ba8 is PREPARED, 0/20,000,000
processed. Latest run acquired OAuth, then the DAT endpoint returned HTTP 504 after
60 seconds and entered retry. Its TLS socket remained established and process
CPU/RSS were low. The generic Working locally notice never changes during headers,
transfer or recovery. This is a remote wait hidden by incorrect client feedback,
not evidence that matching itself is taking minutes. The server is not modified.

## Design and ordered consultations

architect GO: operation-scoped observer on a cloned Service, separate from stored
settings/jobs/auth/resident state; bounded watch channel retains one safe snapshot.
curator GO: progress is observation, never a checkpoint or authentication signal.
herald GO: optional read model only; no endpoint/settings/job wire change.
migrator GO: no persisted progress, migration or job recreation.
bastion GO design: enums, counters, timings/status and typed job UUID only; never
client ID/secret, key/token, URL, row values, paths or raw errors. No extra HTTP.
scribe GO: record exact wait cause and what the local change can prove.
helmsman GO: show elapsed/header wait/retry count/status and body bytes/rate; keep
all existing deadlines, retry limits, cancellation and same-UUID recovery.
beacon GO: live stage/units/rates and existing safe lifecycle events, no telemetry
service or high-volume row logs. UI updates on its existing timer.
pathfinder GO: one latest-value watch channel per worker, no new thread/task pool;
no receiver borrow held over await, and updates per body chunk/8192-row wave.
usher GO: English copy, existing palette/borders, meaningful unknown-total state,
readable counters at 40x18 through 120x36, no animated-logo change.
Next: forge -> refactory -> sentinel -> helmsman -> bastion -> beacon -> inquisitor
-> scribe -> conductor. Consultations are self-applied, not independent approvals.

Observe copy, dataset scan, OAuth acquisition, DAT headers, download, retry wait,
decryption/Bloom validation, input/checkpoint validation, matching and publication.
Downloads aggregate unique logical progress across parallel parts; restart removes
the discarded prefix instead of inflating progress. Do not count the 1-byte probe.
Show HTTP 504/retry state even on the following header wait. Compute elapsed time
at render/tick time so silence from a server still produces a changing clock.
Zero known total is not 100 percent; rate/ETA must not invent a total. Completion
or failure removes the live observer from the UI; stopping overrides its status.

Affected: matching progress model, Service, jobs/source/download/recovery, UI/view,
synthetic render example, meaningful HTTP/UI/progress regressions, shared manual
and recovery runbook. Backup `/tmp/pgm-live-progress-backup-path`.

## Verification and rollback

Test delayed headers with an intermediate 504, progressing/delayed bodies,
recovery/full restart, bounded parallel aggregation, dataset scanning/matching,
same-job retry/cancel and fresh observer isolation; render at four supported sizes.
Check no secrets/free text in the progress type and no additional API calls.
Run cargo test --offline --all-targets --all-features;
cargo clippy --offline --all-targets --all-features -- -D warnings;
cargo fmt --all -- --check; cargo audit --no-fetch;
gitleaks dir src --no-banner --redact;
cargo build --offline --release --bin pgm-data --bin pgm-bench;
cargo build --offline --bin pgm-data. Record actual counts/results and visual QA.
No CI, native Ghostty acceptance, independent review or production rollout claim.

Rollback: restore only task source/docs from backup and rebuild. User preferences,
jobs/checkpoints/exports, credentials and the 20M source CSV are untouched. No
background process is killed or restarted; reopen the app to use the new binary.

## Local execution ledger

forge GO: optional scoped reporter; typed read model, bounded latest-value watch;
copy chunks, existing row waves and transfer chunks publish observations only.
refactory GO local scope: retain existing ownership/pools/checkpoints; no broad
restructure of the existing UI, auth, crypto or matching implementations.
sentinel GO functional: 246 all-target/all-feature tests, including 8 new cases
and extended real HTTP recovery/parallel assertions. Coverage/mutation publication
floors are unmeasured; no production approval is claimed.
helmsman GO local: existing retry/deadline/cancellation bounds are unchanged;
safe stop and same-UUID recovery are visible, and the runbook is updated.
bastion GO local: numeric/enum/timing/typed-UUID snapshot only, no payload/free-text
errors or credential access; locked RAM/auth/GCM validation remain intact.
beacon GO local: safe static stage logs, live clocks/rates and actual quantities;
no high-volume row logs or unsolicited telemetry/network exporter.
inquisitor GO local behavior: tests and scoped diff reviewed; aggregate/restart
bytes remain accurate, old HTTP status cannot mislabel a body interruption,
header-to-body recovery changes the stage and compact screens retain dataset
identity. This is self-review, not independent approval or a merge gate result.
scribe GO: shared help/README/runbook and verification record updated in English.
conductor GO local build only: debug/release rebuilt; no repository metadata,
publishing, CI, signing or rollout. See live-progress-verification.md for evidence
and remaining publication gates.
